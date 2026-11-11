//! Android JNI bridge for `osmo-core`.
//!
//! Kotlin drives everything through `com.cakeal.osmolink.bridge.OsmoNative`.
//! Native calls only start work; results and live state are pushed back to the
//! app through `BridgeListener.onEvent(type, json)`.
//!
//! Threading model:
//! * JNI entry points run on the Java thread and only validate input and spawn.
//! * One Tokio runtime owns BLE work, mirroring the macOS CLI.
//! * Callbacks attach the calling thread to the JVM before touching Java objects.
use std::{
    future::Future,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use jni::{
    Env, EnvUnowned, JavaVM,
    errors::ThrowRuntimeExAndDefault,
    jni_sig, jni_str,
    objects::{JObject, JString, JValue},
    refs::Global,
    sys::{jboolean, jint},
};
use osmo_core::{
    Camera, CameraMode, ConnectOptions, ControllerIdentity, Device, Error, Event, GpsSample,
    Scanner, StatusPushMode, VerificationMode,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use mobile_status::{mode_catalog, selectable_modes, status_json};

struct BridgeState {
    devices: Vec<Device>,
    camera: Option<Arc<Camera>>,
    scanning: bool,
}

static STATE: Mutex<BridgeState> = Mutex::new(BridgeState {
    devices: Vec::new(),
    camera: None,
    scanning: false,
});
static VM: OnceLock<JavaVM> = OnceLock::new();
static LISTENER: Mutex<Option<Arc<Global<JObject<'static>>>>> = Mutex::new(None);
static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
static STATE_DIR: OnceLock<PathBuf> = OnceLock::new();
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

fn emit(kind: &str, payload: Value) {
    let Some(vm) = VM.get() else {
        return;
    };
    let listener = LISTENER
        .lock()
        .ok()
        .and_then(|listener| listener.as_ref().map(Arc::clone));
    let Some(listener) = listener else {
        return;
    };
    let _ = vm.attach_current_thread(|env| send_event(env, listener.as_obj(), kind, &payload));
}

fn send_event(
    env: &mut Env,
    listener: &JObject,
    kind: &str,
    payload: &Value,
) -> jni::errors::Result<()> {
    let kind = JString::from_str(env, kind)?;
    let payload = JString::from_str(env, payload.to_string())?;
    env.call_method(
        listener,
        jni_str!("onEvent"),
        jni_sig!("(Ljava/lang/String;Ljava/lang/String;)V"),
        &[JValue::Object(&kind), JValue::Object(&payload)],
    )?;
    Ok(())
}

fn read_string(env: &mut Env, value: &JString) -> jni::errors::Result<String> {
    let chars = value.mutf8_chars(env)?;
    let text: &jni::strings::JNIStr = &chars;
    Ok(text.to_str().into_owned())
}

fn run_bool<'local, F>(unowned: &mut EnvUnowned<'local>, action: F) -> jboolean
where
    F: FnOnce(&mut Env<'local>) -> jni::errors::Result<jboolean>,
{
    unowned
        .with_env(action)
        .resolve::<ThrowRuntimeExAndDefault>()
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
    runtime.spawn(async move {
        match future.await {
            Ok(()) => emit("command", json!({ "action": action, "ok": true })),
            Err(error) => emit(
                "command",
                json!({ "action": action, "ok": false, "error": error.to_string() }),
            ),
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
        emit("gpsError", json!({ "message": message }));
    }
}

#[cfg(target_os = "android")]
fn platform_init(env: &mut Env) -> Result<(), String> {
    btleplug::platform::init(env).map_err(|error| format!("Bluetooth init failed: {error}"))
}

/// Host builds exist only so the crate can be type-checked outside Android.
#[cfg(not(target_os = "android"))]
fn platform_init(_env: &mut Env) -> Result<(), String> {
    Ok(())
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_cakeal_osmolink_bridge_OsmoNative_nativeInit<'local>(
    mut unowned: EnvUnowned<'local>,
    _this: JObject<'local>,
    state_dir: JString<'local>,
    listener: JObject<'local>,
) -> jboolean {
    run_bool(&mut unowned, |env| {
        let _ = VM.set(env.get_java_vm()?);
        let directory = read_string(env, &state_dir)?;
        let _ = STATE_DIR.set(PathBuf::from(directory));
        let Ok(mut slot) = LISTENER.lock() else {
            return Ok(false);
        };
        *slot = Some(Arc::new(env.new_global_ref(&listener)?));
        drop(slot);

        if RUNTIME.get().is_none() {
            let built = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .thread_name("osmo-bridge")
                .build();
            match built {
                Ok(runtime) => {
                    let _ = RUNTIME.set(runtime);
                }
                Err(error) => {
                    let _ = send_event(
                        env,
                        &listener,
                        "error",
                        &json!({ "message": format!("runtime init failed: {error}") }),
                    );
                    return Ok(false);
                }
            }
        }
        if let Err(message) = platform_init(env) {
            let _ = send_event(env, &listener, "error", &json!({ "message": message }));
            return Ok(false);
        }
        Ok(true)
    })
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_cakeal_osmolink_bridge_OsmoNative_nativeScan<'local>(
    mut unowned: EnvUnowned<'local>,
    _this: JObject<'local>,
    seconds: jint,
    auto_connect: jboolean,
) -> jboolean {
    run_bool(&mut unowned, |_env| {
        let Some(runtime) = RUNTIME.get() else {
            return Ok(false);
        };
        {
            let Ok(mut state) = STATE.lock() else {
                return Ok(false);
            };
            if state.scanning || state.camera.is_some() {
                return Ok(false);
            }
            state.scanning = true;
            state.devices.clear();
        }
        let seconds = seconds.clamp(1, 60) as u64;
        runtime.spawn(async move {
            let inner = tokio::spawn(async move {
                let scanner = Scanner::new().await?;
                scanner.scan(Duration::from_secs(seconds)).await
            });
            let result = match inner.await {
                Ok(result) => result,
                Err(error) => {
                    let message = if error.is_panic() {
                        let payload = error.into_panic();
                        payload
                            .downcast_ref::<&str>()
                            .map(|text| text.to_string())
                            .or_else(|| payload.downcast_ref::<String>().cloned())
                            .unwrap_or_else(|| "native task panicked".into())
                    } else {
                        "native task cancelled".into()
                    };
                    emit(
                        "error",
                        json!({ "message": format!("scan failed: {message}") }),
                    );
                    if let Ok(mut state) = STATE.lock() {
                        state.scanning = false;
                    }
                    emit("devices", json!({ "devices": [] }));
                    return;
                }
            };
            match result {
                Ok(devices) => {
                    let list: Vec<Value> = devices
                        .iter()
                        .map(|device| {
                            json!({
                                "id": device.id,
                                "name": device.name,
                                "rssi": device.rssi,
                            })
                        })
                        .collect();
                    let reconnect = if auto_connect {
                        STATE_DIR.get().and_then(|dir| {
                            let saved = load_saved(dir);
                            preferred_paired(
                                devices.iter().map(|device| device.id.as_str()),
                                &saved,
                            )
                            .map(|index| (devices[index].clone(), dir.clone(), saved))
                        })
                    } else {
                        None
                    };
                    if let Ok(mut state) = STATE.lock() {
                        state.scanning = false;
                        state.devices = devices;
                    }
                    emit("devices", json!({ "devices": list }));
                    // Publish the scan list before connecting so a failed
                    // automatic attempt falls back to manual selection.
                    if let Some((device, dir, saved)) = reconnect {
                        start_connect(device, dir, saved, true);
                    }
                }
                Err(error) => {
                    if let Ok(mut state) = STATE.lock() {
                        state.scanning = false;
                    }
                    // Always close the scan in the UI, even when discovery fails.
                    emit("devices", json!({ "devices": [] }));
                    emit("error", json!({ "message": error.to_string() }));
                }
            }
        });
        Ok(true)
    })
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_cakeal_osmolink_bridge_OsmoNative_nativeConnect<'local>(
    mut unowned: EnvUnowned<'local>,
    _this: JObject<'local>,
    index: jint,
) -> jboolean {
    run_bool(&mut unowned, |_env| {
        let Some(dir) = STATE_DIR.get() else {
            return Ok(false);
        };
        if RUNTIME.get().is_none() {
            return Ok(false);
        }
        let device = {
            let Ok(state) = STATE.lock() else {
                return Ok(false);
            };
            if state.camera.is_some() || index < 0 {
                return Ok(false);
            }
            state.devices.get(index as usize).cloned()
        };
        let Some(device) = device else {
            emit(
                "error",
                json!({ "message": "camera is no longer in the scan list" }),
            );
            return Ok(false);
        };

        let dir = dir.clone();
        let saved = load_saved(&dir);
        Ok(start_connect(device, dir, saved, false))
    })
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
    emit(
        "pairing",
        json!({
            "code": options.verification_code,
            "device": device.name,
            "firstTime": first_time,
            "auto": auto,
        }),
    );

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
                    state.camera = Some(camera.clone());
                }
                emit(
                    "connected",
                    json!({
                        "modelName": format!("{:?}", camera.device_id),
                        "modelCode": camera.device_id.code(),
                        "name": device.name,
                        "modes": mode_catalog(camera.device_id),
                    }),
                );
                spawn_event_pump(camera.clone(), epoch);

                if let Err(error) = camera
                    .subscribe_status(StatusPushMode::PeriodicAndChanges)
                    .await
                {
                    emit("warning", json!({ "message": error.to_string() }));
                }
                match camera.version().await {
                    Ok(version) => emit(
                        "version",
                        json!({ "product": version.product, "details": version.details }),
                    ),
                    Err(error) => emit("warning", json!({ "message": error.to_string() })),
                }
            }
            Err(error) => {
                emit(
                    "connectFailed",
                    json!({ "message": error.to_string(), "auto": auto }),
                );
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
            match events.recv().await {
                Ok(Event::Status(status)) => emit("status", status_json(&status)),
                Ok(Event::ModeInfo(info)) => emit(
                    "modeInfo",
                    json!({ "name": info.name, "parameters": info.parameters }),
                ),
                Ok(Event::Reconnecting(attempt)) => {
                    emit("reconnecting", json!({ "attempt": attempt }))
                }
                Ok(Event::Reconnected) => emit("reconnected", json!({})),
                Ok(Event::Warning(message)) => emit("warning", json!({ "message": message })),
                Ok(Event::Disconnected(reason)) => {
                    if EPOCH.load(Ordering::SeqCst) == epoch {
                        if let Ok(mut state) = STATE.lock() {
                            state.camera = None;
                        }
                        emit("disconnected", json!({ "reason": reason }));
                    }
                    break;
                }
                Ok(Event::Frame(_)) => {}
                Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => emit(
                    "warning",
                    json!({ "message": format!("event backlog: skipped {skipped}") }),
                ),
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    if EPOCH.load(Ordering::SeqCst) == epoch {
                        if let Ok(mut state) = STATE.lock() {
                            state.camera = None;
                        }
                        emit("disconnected", json!({ "reason": "closed" }));
                    }
                    break;
                }
            }
        }
    });
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_cakeal_osmolink_bridge_OsmoNative_nativeDisconnect<'local>(
    mut unowned: EnvUnowned<'local>,
    _this: JObject<'local>,
) -> jboolean {
    run_bool(&mut unowned, |_env| {
        // A later connect must win over any in-flight pairing attempt.
        let _ = EPOCH.fetch_add(1, Ordering::SeqCst);
        let camera = STATE.lock().ok().and_then(|mut state| state.camera.take());
        if let Some(camera) = camera {
            camera.stop_gps();
            if let Some(runtime) = RUNTIME.get() {
                runtime.spawn(async move {
                    match Arc::try_unwrap(camera) {
                        Ok(camera) => {
                            let _ = camera.disconnect().await;
                        }
                        Err(camera) => drop(camera),
                    }
                });
            }
        }
        emit("disconnected", json!({ "reason": "user" }));
        Ok(true)
    })
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_cakeal_osmolink_bridge_OsmoNative_nativeRecordStart<'local>(
    mut unowned: EnvUnowned<'local>,
    _this: JObject<'local>,
) -> jboolean {
    run_bool(&mut unowned, |_env| {
        let Some(camera) = camera() else {
            return Ok(false);
        };
        spawn_command(
            "record.start",
            async move { camera.start_recording().await },
        );
        Ok(true)
    })
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_cakeal_osmolink_bridge_OsmoNative_nativeRecordStop<'local>(
    mut unowned: EnvUnowned<'local>,
    _this: JObject<'local>,
) -> jboolean {
    run_bool(&mut unowned, |_env| {
        let Some(camera) = camera() else {
            return Ok(false);
        };
        spawn_command("record.stop", async move { camera.stop_recording().await });
        Ok(true)
    })
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_cakeal_osmolink_bridge_OsmoNative_nativeShutter<'local>(
    mut unowned: EnvUnowned<'local>,
    _this: JObject<'local>,
) -> jboolean {
    run_bool(&mut unowned, |_env| {
        let Some(camera) = camera() else {
            return Ok(false);
        };
        spawn_command("shutter", async move { camera.shutter().await });
        Ok(true)
    })
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_cakeal_osmolink_bridge_OsmoNative_nativeSetMode<'local>(
    mut unowned: EnvUnowned<'local>,
    _this: JObject<'local>,
    code: jint,
) -> jboolean {
    run_bool(&mut unowned, |_env| {
        let Some(camera) = camera() else {
            return Ok(false);
        };
        let mode = u8::try_from(code).ok().map(CameraMode::from_code);
        let Some(mode) =
            mode.filter(|mode| selectable_modes(camera.device_id).any(|item| item == *mode))
        else {
            emit(
                "command",
                json!({ "action": "mode", "ok": false, "error": "unsupported camera mode" }),
            );
            return Ok(false);
        };
        spawn_command("mode", async move {
            let status = camera
                .status()
                .ok_or(Error::InvalidInput("waiting for camera status"))?;
            if status.is_capturing() {
                return Err(Error::InvalidInput("stop capturing before changing mode"));
            }
            camera.set_mode(mode).await
        });
        Ok(true)
    })
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_cakeal_osmolink_bridge_OsmoNative_nativeVersion<'local>(
    mut unowned: EnvUnowned<'local>,
    _this: JObject<'local>,
) -> jboolean {
    run_bool(&mut unowned, |_env| {
        let (Some(camera), Some(runtime)) = (camera(), RUNTIME.get()) else {
            return Ok(false);
        };
        runtime.spawn(async move {
            match camera.version().await {
                Ok(version) => emit(
                    "version",
                    json!({ "product": version.product, "details": version.details }),
                ),
                Err(error) => emit("warning", json!({ "message": error.to_string() })),
            }
        });
        Ok(true)
    })
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_cakeal_osmolink_bridge_OsmoNative_nativeGpsUpdate<'local>(
    mut unowned: EnvUnowned<'local>,
    _this: JObject<'local>,
    sample: JString<'local>,
) -> jboolean {
    run_bool(&mut unowned, |env| {
        let text = read_string(env, &sample)?;
        let sample: GpsSample = match serde_json::from_str(&text) {
            Ok(sample) => sample,
            Err(error) => {
                report_gps_error(format!("invalid location sample: {error}"));
                return Ok(false);
            }
        };
        let Some(camera) = camera() else {
            return Ok(false);
        };
        match camera.update_gps(&sample) {
            Ok(()) => {
                if let Ok(mut last) = GPS_ERROR.lock() {
                    *last = None;
                }
                Ok(true)
            }
            Err(error) => {
                report_gps_error(error.to_string());
                Ok(false)
            }
        }
    })
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_cakeal_osmolink_bridge_OsmoNative_nativeGpsStop<'local>(
    mut unowned: EnvUnowned<'local>,
    _this: JObject<'local>,
) -> jboolean {
    run_bool(&mut unowned, |_env| {
        if let Some(camera) = camera() {
            camera.stop_gps();
        }
        emit("gps", json!({ "active": false }));
        Ok(true)
    })
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
