//! Shared mobile session and typed Swift/Kotlin interface.
#[cfg(target_os = "android")]
mod android;
mod events;
mod status;

use std::{
    future::Future,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use osmo_core::{
    Camera, CameraMode, ConnectOptions, ConnectionState, ControllerIdentity, Device, Error, Event,
    Scanner, StatusPushMode, VerificationMode,
};
use serde::{Deserialize, Serialize};
use tokio::sync::{OnceCell, oneshot};

pub use events::{LocationSample, MobileDevice, MobileEvent, MobileListener};
pub use status::{MobileMode, MobileStatus};
use status::{error_message, mode_catalog, selectable_modes};

uniffi::setup_scaffolding!();

struct BridgeState {
    devices: Vec<Device>,
    camera: Option<Arc<Camera>>,
    scan: Option<LiveScan>,
    connecting: bool,
    disconnecting: bool,
}

struct LiveScan {
    end: Option<oneshot::Sender<ScanEnd>>,
    #[cfg(target_os = "android")]
    scanner: Option<Arc<Scanner>>,
}

enum ScanEnd {
    Connect(String),
    Failed(i32),
}

static STATE: Mutex<BridgeState> = Mutex::new(BridgeState {
    devices: Vec::new(),
    camera: None,
    scan: None,
    connecting: false,
    disconnecting: false,
});
static LISTENER: OnceLock<Arc<dyn MobileListener>> = OnceLock::new();
static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
static STATE_DIR: OnceLock<PathBuf> = OnceLock::new();
static SCANNER: OnceCell<Arc<Scanner>> = OnceCell::const_new();
/// Bumped for every connect/disconnect so stale futures cannot publish state.
static EPOCH: AtomicU64 = AtomicU64::new(0);
/// Avoid flooding the UI while the location provider is still warming up.
static GPS_ERROR: Mutex<Option<String>> = Mutex::new(None);

/// Controller identity plus pairing history, shared with the CLI's file format.
#[derive(Default, Serialize, Deserialize)]
struct SavedState {
    identity: ControllerIdentity,
    #[serde(default)]
    paired: Vec<String>,
    /// Most recently connected camera, preferred when several saved cameras
    /// are in range at the same time.
    #[serde(default)]
    last: Option<String>,
}

/// Index into the RSSI-sorted scan list of the camera to reconnect to.
///
/// The last connected camera wins when it is in range; otherwise the strongest
/// previously paired camera is used, matching the scan result order.
fn preferred_paired<'a, I>(ids: I, saved: &SavedState) -> Option<usize>
where
    I: IntoIterator<Item = &'a str>,
{
    let ids: Vec<&str> = ids.into_iter().collect();
    if let Some(last) = saved.last.as_deref()
        && let Some(index) = ids.iter().position(|id| *id == last)
    {
        return Some(index);
    }
    ids.iter()
        .position(|id| saved.paired.iter().any(|paired| paired.as_str() == *id))
}

fn load_saved(dir: &Path) -> SavedState {
    match std::fs::read(dir.join("controller.json")) {
        Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_default(),
        Err(_) => {
            // Persist the generated identity immediately so reconnects keep it.
            let saved = SavedState::default();
            let _ = save_saved(dir, &saved);
            saved
        }
    }
}

fn save_saved(dir: &Path, saved: &SavedState) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let temporary = dir.join("controller.json.tmp");
    let bytes = serde_json::to_vec_pretty(saved).map_err(std::io::Error::other)?;
    std::fs::write(&temporary, bytes)?;
    std::fs::rename(temporary, dir.join("controller.json"))
}

fn emit(event: MobileEvent) {
    if let Some(listener) = LISTENER.get() {
        listener.on_event(event);
    }
}

fn camera() -> Option<Arc<Camera>> {
    STATE.lock().ok()?.camera.clone()
}

fn spawn_command<F>(action: &'static str, future: F)
where
    F: Future<Output = osmo_core::Result<()>> + Send + 'static,
{
    let Some(runtime) = RUNTIME.get() else {
        return;
    };
    let epoch = EPOCH.load(Ordering::SeqCst);
    runtime.spawn(async move {
        let result = future.await;
        if EPOCH.load(Ordering::SeqCst) == epoch {
            emit(MobileEvent::Command {
                action: action.into(),
                error: result.err().as_ref().map(error_message),
            });
        }
    });
}

fn report_gps_error(message: String) {
    let changed = match GPS_ERROR.lock() {
        Ok(mut last) => {
            if last.as_deref() == Some(message.as_str()) {
                false
            } else {
                *last = Some(message.clone());
                true
            }
        }
        Err(_) => true,
    };
    if changed {
        emit(MobileEvent::GpsError { message });
    }
}

#[uniffi::export]
pub fn start_session(directory: String, listener: Arc<dyn MobileListener>) -> bool {
    if RUNTIME.get().is_none() {
        let Ok(runtime) = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .thread_name("osmo-mobile")
            .build()
        else {
            return false;
        };
        let _ = RUNTIME.set(runtime);
    }
    let _ = STATE_DIR.set(PathBuf::from(directory));
    LISTENER.set(listener).is_ok()
}

#[uniffi::export]
pub fn start_scan(seconds: u8, auto_connect: bool) -> bool {
    let (Some(runtime), Some(dir)) = (RUNTIME.get(), STATE_DIR.get()) else {
        return false;
    };
    let (end, selection) = oneshot::channel();
    {
        let Ok(mut state) = STATE.lock() else {
            return false;
        };
        if state.scan.is_some() || state.connecting || state.disconnecting || state.camera.is_some()
        {
            return false;
        }
        state.scan = Some(LiveScan {
            end: Some(end),
            #[cfg(target_os = "android")]
            scanner: None,
        });
        state.devices.clear();
    }
    let duration = Duration::from_secs(u64::from(seconds.clamp(1, 60)));
    runtime.spawn(async move {
        match tokio::spawn(scan(duration, selection, dir, auto_connect)).await {
            Ok(Ok(())) => {}
            Ok(Err(error)) => report_scan_failure(error),
            Err(error) => report_scan_failure(error),
        }
    });
    true
}

async fn scan(
    duration: Duration,
    mut selection: oneshot::Receiver<ScanEnd>,
    dir: &Path,
    auto_connect: bool,
) -> osmo_core::Result<()> {
    let saved = load_saved(dir);
    let last = saved.last.as_deref().filter(|_| auto_connect);
    let scanner = SCANNER
        .get_or_try_init(|| async { Scanner::new().await.map(Arc::new) })
        .await?;
    #[cfg(target_os = "android")]
    {
        if let Ok(mut state) = STATE.lock()
            && let Some(scan) = &mut state.scan
        {
            scan.scanner = Some(Arc::clone(scanner));
        }
    }
    let mut end = None;
    let stop = async {
        end = (&mut selection).await.ok();
    };
    let update = |devices: &[Device]| {
        if devices
            .iter()
            .any(|device| Some(device.id.as_str()) == last)
        {
            return true;
        }
        if let Ok(mut state) = STATE.lock() {
            state.devices = devices.to_vec();
        }
        publish_devices(devices, true, false);
        false
    };
    #[cfg(target_os = "android")]
    let devices = scanner
        .scan_android_live(duration, stop, update, set_android_scanning)
        .await?;
    #[cfg(not(target_os = "android"))]
    let devices = scanner.scan_live(duration, stop, update).await?;

    let target = {
        let mut state = STATE.lock().map_err(|_| Error::Disconnected)?;
        // A selection queued as the deadline elapsed still takes priority.
        state.scan = None;
        let target = match end.or_else(|| selection.try_recv().ok()) {
            Some(ScanEnd::Connect(id)) => devices
                .iter()
                .find(|device| device.id == id)
                .map(|device| (device.clone(), false)),
            Some(ScanEnd::Failed(code)) => {
                let devices = state.devices.clone();
                drop(state);
                publish_devices(&devices, false, false);
                emit(MobileEvent::ScanFailed {
                    message: format!("蓝牙扫描失败（代码 {code}）"),
                });
                return Ok(());
            }
            None if auto_connect => {
                preferred_paired(devices.iter().map(|device| device.id.as_str()), &saved)
                    .map(|index| (devices[index].clone(), true))
            }
            None => None,
        };
        state.connecting = target.is_some();
        state.devices.clone_from(&devices);
        target
    };
    publish_devices(&devices, false, target.is_some());
    if let Some((device, auto)) = target {
        start_connect(device, dir.to_path_buf(), saved, auto);
    }
    Ok(())
}

fn publish_devices(devices: &[Device], scanning: bool, connecting: bool) {
    emit(MobileEvent::Devices {
        devices: devices.iter().map(MobileDevice::from).collect(),
        scanning,
        connecting,
    });
}

fn report_scan_failure(error: impl std::fmt::Display) {
    #[cfg(target_os = "android")]
    if let Err(stop_error) = set_android_scanning(false) {
        emit(MobileEvent::Warning {
            message: stop_error.to_string(),
        });
    }
    let devices = match STATE.lock() {
        Ok(mut state) => {
            state.scan = None;
            state.devices.clone()
        }
        Err(_) => Vec::new(),
    };
    publish_devices(&devices, false, false);
    emit(MobileEvent::ScanFailed {
        message: error.to_string(),
    });
}

#[cfg(target_os = "android")]
fn set_android_scanning(enabled: bool) -> osmo_core::Result<()> {
    let listener = LISTENER.get().ok_or(Error::Disconnected)?;
    match listener.set_ble_scanning(enabled) {
        Some(message) => Err(btleplug::Error::RuntimeError(message).into()),
        None => Ok(()),
    }
}

#[uniffi::export]
pub fn scan_failed(code: i32) {
    if let Some(sender) = STATE
        .lock()
        .ok()
        .and_then(|mut state| state.scan.as_mut()?.end.take())
    {
        let _ = sender.send(ScanEnd::Failed(code));
    }
}

#[uniffi::export]
pub fn connect(id: String) -> bool {
    let Some(dir) = STATE_DIR.get() else {
        return false;
    };
    let device = {
        let Ok(mut state) = STATE.lock() else {
            return false;
        };
        if state.camera.is_some() || state.connecting || state.disconnecting {
            return false;
        }
        let Some(device) = state.devices.iter().find(|device| device.id == id).cloned() else {
            return false;
        };
        if let Some(scan) = &mut state.scan {
            return scan
                .end
                .take()
                .is_some_and(|sender| sender.send(ScanEnd::Connect(id)).is_ok());
        }
        state.connecting = true;
        device
    };
    start_connect(device, dir.clone(), load_saved(dir), false)
}

/// Starts one connection attempt and the event pump on success.
///
/// Manual selection and automatic reconnection share this path. The caller
/// passes the pairing snapshot it used so a success persists exactly the
/// identity the camera verified.
fn start_connect(device: Device, dir: PathBuf, mut saved: SavedState, auto: bool) -> bool {
    let Some(runtime) = RUNTIME.get() else {
        return false;
    };
    let first_time = !saved.paired.contains(&device.id);
    let mut options = ConnectOptions::new(saved.identity);
    options.verification_code = 2333;
    options.verification_mode = if first_time {
        VerificationMode::Required
    } else {
        VerificationMode::Automatic
    };
    let identity = options.identity;
    let epoch = EPOCH.fetch_add(1, Ordering::SeqCst) + 1;
    emit(MobileEvent::Pairing {
        code: options.verification_code,
        device: device.name.clone(),
        first_time,
        auto,
    });

    runtime.spawn(async move {
        match Camera::connect(&device, options).await {
            Ok(camera) => {
                if EPOCH.load(Ordering::SeqCst) != epoch {
                    let _ = camera.disconnect().await;
                    return;
                }
                saved.identity = identity;
                if !saved.paired.contains(&device.id) {
                    saved.paired.push(device.id.clone());
                }
                saved.last = Some(device.id.clone());
                let _ = save_saved(&dir, &saved);

                let camera = Arc::new(camera);
                if let Ok(mut state) = STATE.lock() {
                    state.connecting = false;
                    state.camera = Some(camera.clone());
                }
                emit(MobileEvent::Connected {
                    model: format!("{:?}", camera.device_id),
                    name: device.name,
                    modes: mode_catalog(camera.device_id),
                });
                spawn_event_pump(camera.clone(), epoch);

                if let Err(error) = camera
                    .subscribe_status(StatusPushMode::PeriodicAndChanges)
                    .await
                {
                    emit(MobileEvent::Warning {
                        message: error_message(&error),
                    });
                }
                match camera.version().await {
                    Ok(version) => emit(MobileEvent::Version {
                        product: version.product,
                        details: version.details,
                    }),
                    Err(error) => emit(MobileEvent::Warning {
                        message: error_message(&error),
                    }),
                }
            }
            Err(error) => {
                if EPOCH.load(Ordering::SeqCst) != epoch {
                    return;
                }
                if let Ok(mut state) = STATE.lock() {
                    state.connecting = false;
                }
                emit(MobileEvent::ConnectFailed {
                    message: error_message(&error),
                    auto,
                });
            }
        }
    });
    true
}

fn spawn_event_pump(camera: Arc<Camera>, epoch: u64) {
    let Some(runtime) = RUNTIME.get() else {
        return;
    };
    let mut events = camera.events();
    runtime.spawn(async move {
        loop {
            let result = events.recv().await;
            if EPOCH.load(Ordering::SeqCst) != epoch {
                break;
            }
            let event = match result {
                Ok(Event::Status(status)) => MobileEvent::Status {
                    status: MobileStatus::from(&status),
                },
                Ok(Event::ModeInfo(info)) => MobileEvent::ModeInfo {
                    name: info.name,
                    parameters: info.parameters,
                },
                Ok(Event::Reconnecting(attempt)) => MobileEvent::Reconnecting { attempt },
                Ok(Event::Reconnected) => MobileEvent::Reconnected,
                Ok(Event::Warning(message)) => MobileEvent::Warning { message },
                Ok(Event::Disconnected(reason)) => {
                    end_session(epoch, reason);
                    break;
                }
                Ok(Event::Frame(_)) => continue,
                Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                    MobileEvent::Warning {
                        message: format!("event backlog: skipped {skipped}"),
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    end_session(epoch, Error::Disconnected.to_string());
                    break;
                }
            };
            emit(event);
        }
    });
}

fn end_session(epoch: u64, reason: String) {
    if let Ok(mut state) = STATE.lock() {
        if EPOCH.load(Ordering::SeqCst) != epoch {
            return;
        }
        state.camera = None;
    }
    emit(MobileEvent::Disconnected {
        reason: Some(reason),
    });
}

#[uniffi::export]
pub fn disconnect() -> bool {
    let (Some(runtime), Ok(mut state)) = (RUNTIME.get(), STATE.lock()) else {
        return false;
    };
    if state.disconnecting {
        return false;
    }
    let Some(camera) = state.camera.take() else {
        return false;
    };
    state.disconnecting = true;
    let _ = EPOCH.fetch_add(1, Ordering::SeqCst);
    drop(state);

    camera.stop_gps();
    let mut connection = camera.connections();
    runtime.spawn(async move {
        let result = match Arc::try_unwrap(camera) {
            Ok(camera) => camera.disconnect().await,
            Err(camera) => {
                // A command can still hold a reference. Its completion drops
                // the last handle and shuts down the single BLE worker.
                drop(camera);
                tokio::time::timeout(
                    Duration::from_secs(8),
                    connection.wait_for(|state| *state == ConnectionState::Disconnected),
                )
                .await
                .map_err(|_| Error::Timeout("disconnect"))
                .and_then(|result| result.map(|_| ()).map_err(|_| Error::Disconnected))
            }
        };
        if let Ok(mut state) = STATE.lock() {
            state.disconnecting = false;
        }
        emit(MobileEvent::Disconnected {
            reason: result.err().as_ref().map(error_message),
        });
    });
    true
}

#[uniffi::export]
pub fn record(starting: bool) -> bool {
    let Some(camera) = camera() else {
        return false;
    };
    let action = if starting {
        "record.start"
    } else {
        "record.stop"
    };
    spawn_command(action, async move {
        if starting {
            camera.start_recording().await
        } else {
            camera.stop_recording().await
        }
    });
    true
}

#[uniffi::export]
pub fn shutter() -> bool {
    let Some(camera) = camera() else {
        return false;
    };
    spawn_command("shutter", async move { camera.shutter().await });
    true
}

#[uniffi::export]
pub fn capture(mode: u8, capturing: bool) -> bool {
    let Some(camera) = camera() else {
        return false;
    };
    spawn_command("capture", async move {
        capture_and_wait(&camera, mode, capturing).await
    });
    true
}

#[uniffi::export]
pub fn set_mode(code: u8) -> bool {
    let Some(camera) = camera() else {
        return false;
    };
    spawn_command("mode", async move { apply_mode(&camera, code).await });
    true
}

#[uniffi::export]
pub fn update_gps(sample: LocationSample) -> bool {
    let Some(camera) = camera() else {
        return false;
    };
    match camera.update_gps(&sample.into()) {
        Ok(()) => {
            if let Ok(mut last) = GPS_ERROR.lock() {
                *last = None;
            }
            true
        }
        Err(error) => {
            report_gps_error(error.to_string());
            false
        }
    }
}

#[uniffi::export]
pub fn stop_gps() {
    if let Some(camera) = camera() {
        camera.stop_gps();
    }
}

async fn capture_and_wait(camera: &Camera, mode: u8, capturing: bool) -> osmo_core::Result<()> {
    let status = camera
        .status()
        .ok_or(Error::InvalidInput("waiting for camera status"))?;
    status.mode.checked_code()?;
    if status.mode.code() != mode || status.is_capturing() != capturing {
        return Err(Error::InvalidInput("capture state changed on the camera"));
    }
    if status.mode.uses_photo_parameters() {
        if capturing {
            return Err(Error::InvalidInput("camera is already taking a photo"));
        }
        return camera.shutter().await;
    }
    let mut statuses = camera.statuses();
    if capturing {
        camera.stop_recording().await?;
    } else {
        camera.start_recording().await?;
    }
    // An ACK can precede the state push. Keep the capture control disabled
    // until the camera changes state, without retrying the command.
    tokio::time::timeout(
        Duration::from_secs(3),
        statuses.wait_for(|status| {
            status.as_ref().is_none_or(|status| {
                status.mode.code() != mode || status.is_capturing() != capturing
            })
        }),
    )
    .await
    .map_err(|_| Error::Timeout("capture status"))?
    .map_err(|_| Error::Disconnected)?;
    Ok(())
}

async fn apply_mode(camera: &Camera, code: u8) -> osmo_core::Result<()> {
    let mode = CameraMode::from_code(code);
    if !selectable_modes(camera.device_id).any(|item| item == mode) {
        return Err(Error::InvalidInput("unsupported camera mode"));
    }
    let status = camera
        .status()
        .ok_or(Error::InvalidInput("waiting for camera status"))?;
    if status.is_capturing() {
        return Err(Error::InvalidInput("stop capturing before changing mode"));
    }
    camera.set_mode(mode).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn saved(paired: &[&str], last: Option<&str>) -> SavedState {
        SavedState {
            identity: ControllerIdentity::default(),
            paired: paired.iter().map(|id| (*id).to_owned()).collect(),
            last: last.map(str::to_owned),
        }
    }

    #[test]
    fn reads_existing_pairing_history() {
        let data = r#"{"identity":{"device_id":123,"address":[2,3,4,5,6,7]},"paired":["camera"],"last":"camera"}"#;
        let state: SavedState = serde_json::from_str(data).unwrap();

        assert_eq!(state.identity.device_id, 123);
        assert_eq!(state.identity.address, [2, 3, 4, 5, 6, 7]);
        assert_eq!(preferred_paired(["other", "camera"], &state), Some(1));
    }

    #[test]
    fn auto_connect_prefers_the_last_camera_when_it_is_in_range() {
        let state = saved(&["a", "b"], Some("b"));
        assert_eq!(preferred_paired(["a", "b"], &state), Some(1));
    }

    #[test]
    fn auto_connect_falls_back_to_the_strongest_paired_camera() {
        let state = saved(&["a", "b"], Some("c"));
        assert_eq!(preferred_paired(["z", "b", "a"], &state), Some(1));
    }

    #[test]
    fn auto_connect_ignores_cameras_without_pairing_history() {
        let state = saved(&["a"], Some("a"));
        assert_eq!(preferred_paired(["z", "y"], &state), None);
        assert_eq!(preferred_paired(["z"], &saved(&[], None)), None);
    }
}
