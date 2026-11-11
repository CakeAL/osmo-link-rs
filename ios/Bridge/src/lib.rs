//! The app owns one Rust runtime and one ordered stream of UI events.
//! Camera remains the sole owner of the BLE command and reconnect worker.
use std::{
    ffi::{CStr, CString, c_char},
    io,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock, Weak},
    time::Duration,
};

use mobile_status::{mode_catalog, selectable_modes, status_json};
use osmo_core::{
    Camera, CameraMode, ConnectOptions, ControllerIdentity, Device, Error, Event, GpsSample,
    Scanner, StatusPushMode, VerificationMode,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::sync::{broadcast, mpsc};

type Callback = extern "C" fn(*const c_char);
static REQUESTS: OnceLock<Option<mpsc::Sender<Request>>> = OnceLock::new();
/// The connected camera. Weak so a disconnect can still consume the session owner.
static CAMERA: Mutex<Option<Weak<Camera>>> = Mutex::new(None);

enum Request {
    Scan { auto_connect: bool },
    Connect(u32),
    Disconnect,
    Capture { mode: u8, capturing: bool },
    Mode(u8),
}

/// Starts the bridge once per app process.
///
/// # Safety
/// The directory must be a readable, NUL-terminated string for this call.
/// The callback must remain callable from the bridge thread for the app's
/// lifetime. It must copy the event before returning.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn osmo_ios_start(directory: *const c_char, callback: Callback) -> bool {
    if directory.is_null() {
        return false;
    }
    let Ok(directory) = (unsafe { CStr::from_ptr(directory) }).to_str() else {
        return false;
    };
    REQUESTS
        .get_or_init(|| {
            let directory = PathBuf::from(directory);
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .ok()?;
            let (sender, receiver) = mpsc::channel(8);
            std::thread::Builder::new()
                .name("osmo-ios".into())
                .spawn(move || runtime.block_on(run(directory, callback, receiver)))
                .ok()?;
            Some(sender)
        })
        .is_some()
}

fn send(request: Request) -> bool {
    REQUESTS
        .get()
        .and_then(Option::as_ref)
        .is_some_and(|sender| sender.try_send(request).is_ok())
}

/// Scans for DJI cameras. When `auto_connect` is set, a previously paired
/// camera found in the scan is connected without waiting for a selection.
#[unsafe(no_mangle)]
pub extern "C" fn osmo_ios_scan(auto_connect: bool) -> bool {
    send(Request::Scan { auto_connect })
}

#[unsafe(no_mangle)]
pub extern "C" fn osmo_ios_connect(index: u32) -> bool {
    send(Request::Connect(index))
}

#[unsafe(no_mangle)]
pub extern "C" fn osmo_ios_disconnect() -> bool {
    send(Request::Disconnect)
}

#[unsafe(no_mangle)]
pub extern "C" fn osmo_ios_capture(mode: u8, capturing: bool) -> bool {
    send(Request::Capture { mode, capturing })
}

#[unsafe(no_mangle)]
pub extern "C" fn osmo_ios_set_mode(code: u8) -> bool {
    send(Request::Mode(code))
}

/// Feeds one `GpsSample` JSON object to the connected camera.
///
/// # Safety
/// `sample` must be a NUL-terminated UTF-8 string valid for this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn osmo_ios_gps_update(sample: *const c_char) -> bool {
    if sample.is_null() {
        return false;
    }
    let Ok(text) = (unsafe { CStr::from_ptr(sample) }).to_str() else {
        return false;
    };
    let Ok(sample) = serde_json::from_str::<GpsSample>(text) else {
        return false;
    };
    let Some(camera) = camera() else {
        return false;
    };
    camera.update_gps(&sample).is_ok()
}

#[unsafe(no_mangle)]
pub extern "C" fn osmo_ios_gps_stop() -> bool {
    if let Some(camera) = camera() {
        camera.stop_gps();
    }
    true
}

fn camera() -> Option<Arc<Camera>> {
    CAMERA
        .lock()
        .ok()
        .and_then(|slot| slot.as_ref().and_then(Weak::upgrade))
}

fn emit(callback: Callback, kind: &str, payload: Value) {
    // JSON escapes embedded NULs before handing the string to Swift.
    if let Ok(event) = CString::new(json!({ "kind": kind, "payload": payload }).to_string()) {
        callback(event.as_ptr());
    }
}

fn report(callback: Callback, kind: &str, error: impl std::fmt::Display) {
    emit(callback, kind, json!({ "message": error.to_string() }));
}

#[derive(Default, Serialize, Deserialize)]
struct Controller {
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
fn preferred_paired<'a, I>(ids: I, controller: &Controller) -> Option<usize>
where
    I: IntoIterator<Item = &'a str>,
{
    let ids: Vec<&str> = ids.into_iter().collect();
    if let Some(last) = controller.last.as_deref()
        && let Some(index) = ids.iter().position(|id| *id == last)
    {
        return Some(index);
    }
    ids.iter().position(|id| {
        controller
            .paired
            .iter()
            .any(|paired| paired.as_str() == *id)
    })
}

impl Controller {
    fn load(directory: &Path) -> io::Result<Self> {
        match std::fs::read(directory.join("controller.json")) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(io::Error::other),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                let controller = Self::default();
                controller.save(directory)?;
                Ok(controller)
            }
            Err(error) => Err(error),
        }
    }

    fn save(&self, directory: &Path) -> io::Result<()> {
        std::fs::create_dir_all(directory)?;
        let temporary = directory.join("controller.json.tmp");
        let bytes = serde_json::to_vec(self).map_err(io::Error::other)?;
        std::fs::write(&temporary, bytes)?;
        std::fs::rename(temporary, directory.join("controller.json"))
    }
}

async fn run(directory: PathBuf, callback: Callback, mut requests: mpsc::Receiver<Request>) {
    let mut devices = Vec::<Device>::new();
    // Reuse one CoreBluetooth adapter across scans and connections.
    let mut scanner = None;
    while let Some(request) = requests.recv().await {
        match request {
            Request::Scan { auto_connect } => {
                devices.clear();
                let result = async {
                    let scanner = match &scanner {
                        Some(scanner) => scanner,
                        None => scanner.insert(Scanner::new().await?),
                    };
                    scanner.scan(Duration::from_secs(5)).await
                }
                .await;
                match result {
                    Ok(found) => {
                        let list: Vec<_> = found
                            .iter()
                            .map(|device| {
                                json!({ "id": device.id, "name": device.name, "rssi": device.rssi })
                            })
                            .collect();
                        devices = found;
                        emit(callback, "devices", json!({ "devices": list }));
                        if auto_connect {
                            reconnect_saved(&directory, &devices, callback, &mut requests).await;
                        }
                    }
                    Err(error) => report(callback, "scanFailed", error),
                }
            }
            Request::Connect(index) => {
                if let Some(device) = devices.get(index as usize).cloned() {
                    match connect(&directory, &device, callback, false).await {
                        Ok(camera) => {
                            session(Arc::new(camera), callback, &mut requests).await;
                            if let Ok(mut slot) = CAMERA.lock() {
                                *slot = None;
                            }
                        }
                        Err(error) => report_connect_failed(callback, false, error),
                    }
                } else {
                    report_connect_failed(callback, false, "请重新搜索相机。");
                }
            }
            Request::Disconnect => emit(callback, "disconnected", json!({})),
            Request::Capture { .. } | Request::Mode(_) => {
                report(callback, "commandFailed", Error::Disconnected)
            }
        }
    }
}

/// Connects to a previously paired camera found in the latest scan.
///
/// Runs the normal session after a successful connection, so the request loop
/// stays blocked exactly as it does for a manual selection.
async fn reconnect_saved(
    directory: &Path,
    devices: &[Device],
    callback: Callback,
    requests: &mut mpsc::Receiver<Request>,
) {
    let controller = match Controller::load(directory) {
        Ok(controller) => controller,
        Err(error) => {
            report(callback, "warning", error);
            return;
        }
    };
    let Some(index) =
        preferred_paired(devices.iter().map(|device| device.id.as_str()), &controller)
    else {
        return;
    };
    let device = devices[index].clone();
    match connect(directory, &device, callback, true).await {
        Ok(camera) => {
            session(Arc::new(camera), callback, requests).await;
            if let Ok(mut slot) = CAMERA.lock() {
                *slot = None;
            }
        }
        Err(error) => report_connect_failed(callback, true, error),
    }
}

fn report_connect_failed(callback: Callback, auto: bool, error: impl std::fmt::Display) {
    emit(
        callback,
        "connectFailed",
        json!({ "message": error.to_string(), "auto": auto }),
    );
}

async fn connect(
    directory: &Path,
    device: &Device,
    callback: Callback,
    auto: bool,
) -> Result<Camera, Box<dyn std::error::Error + Send + Sync>> {
    let mut controller = Controller::load(directory)?;
    let first_time = !controller.paired.contains(&device.id);
    let mut options = ConnectOptions::new(controller.identity);
    options.verification_code = 2333;
    options.verification_mode = if first_time {
        VerificationMode::Required
    } else {
        VerificationMode::Automatic
    };
    emit(
        callback,
        "pairing",
        json!({
            "device": device.name,
            "firstTime": first_time,
            "code": options.verification_code,
            "auto": auto,
        }),
    );
    let camera = Camera::connect(device, options).await?;
    if first_time {
        controller.paired.push(device.id.clone());
    }
    controller.last = Some(device.id.clone());
    if let Err(error) = controller.save(directory) {
        report(callback, "warning", error);
    }
    emit(
        callback,
        "connected",
        json!({
            "name": device.name,
            "model": camera.device_id.to_string(),
            "modes": mode_catalog(camera.device_id),
        }),
    );
    Ok(camera)
}

async fn session(camera: Arc<Camera>, callback: Callback, requests: &mut mpsc::Receiver<Request>) {
    if let Ok(mut slot) = CAMERA.lock() {
        *slot = Some(Arc::downgrade(&camera));
    }
    let mut events = camera.events();
    if let Err(error) = camera
        .subscribe_status(StatusPushMode::PeriodicAndChanges)
        .await
    {
        report(callback, "warning", error);
    }
    if let Ok(version) = camera.version().await {
        emit(callback, "version", json!({ "details": version.details }));
    }
    loop {
        tokio::select! {
            request = requests.recv() => {
                let result = match request {
                    Some(Request::Capture { mode, capturing }) => capture(&camera, mode, capturing).await,
                    Some(Request::Mode(code)) => set_mode(&camera, code).await,
                    Some(Request::Disconnect) | None => {
                        if let Ok(mut slot) = CAMERA.lock() {
                            *slot = None;
                        }
                        // A concurrent GPS sample may still hold a temporary Arc.
                        // In that case dropping the last handle still asks the
                        // worker to shut down; there is no command to retry.
                        let result = match Arc::try_unwrap(camera) {
                            Ok(camera) => camera.disconnect().await,
                            Err(camera) => {
                                camera.stop_gps();
                                Ok(())
                            }
                        };
                        match result {
                            Ok(()) => emit(callback, "disconnected", json!({})),
                            Err(error) => report(callback, "disconnected", error),
                        }
                        return;
                    }
                    Some(Request::Scan { .. } | Request::Connect(_)) => continue,
                };
                match result {
                    Ok(()) => emit(callback, "commandDone", json!({})),
                    Err(error) => report(callback, "commandFailed", error),
                }
            }
            event = events.recv() => match event {
                Ok(Event::Status(status)) => emit(callback, "status", status_json(&status)),
                Ok(Event::ModeInfo(info)) => emit(callback, "modeInfo", json!({ "name": info.name, "parameters": info.parameters })),
                Ok(Event::Reconnecting(attempt)) => emit(callback, "reconnecting", json!({ "attempt": attempt })),
                Ok(Event::Reconnected) => emit(callback, "reconnected", json!({})),
                Ok(Event::Warning(message)) => report(callback, "warning", message),
                Ok(Event::Disconnected(reason)) => {
                    report(callback, "disconnected", reason);
                    return;
                }
                Err(broadcast::error::RecvError::Closed) => {
                    report(callback, "disconnected", Error::Disconnected);
                    return;
                }
                Ok(Event::Frame(_)) | Err(broadcast::error::RecvError::Lagged(_)) => {}
            }
        }
    }
}

async fn capture(camera: &Camera, mode: u8, capturing: bool) -> osmo_core::Result<()> {
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

async fn set_mode(camera: &Camera, code: u8) -> osmo_core::Result<()> {
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

    fn controller(paired: &[&str], last: Option<&str>) -> Controller {
        Controller {
            identity: ControllerIdentity::default(),
            paired: paired.iter().map(|id| (*id).to_owned()).collect(),
            last: last.map(str::to_owned),
        }
    }

    #[test]
    fn auto_connect_prefers_the_last_camera_when_it_is_in_range() {
        let saved = controller(&["a", "b"], Some("b"));
        assert_eq!(preferred_paired(["a", "b"], &saved), Some(1));
    }

    #[test]
    fn auto_connect_falls_back_to_the_strongest_paired_camera() {
        let saved = controller(&["a", "b"], Some("c"));
        assert_eq!(preferred_paired(["z", "b", "a"], &saved), Some(1));
    }

    #[test]
    fn auto_connect_ignores_cameras_without_pairing_history() {
        let saved = controller(&["a"], Some("a"));
        assert_eq!(preferred_paired(["z", "y"], &saved), None);
        assert_eq!(preferred_paired(["z"], &controller(&[], None)), None);
    }
}
