//! Native Android objects enter through JNI and stay on the Android side of the API.
use jni::{EnvUnowned, errors::ThrowRuntimeExAndDefault, objects::JObject, sys::jboolean};

use crate::STATE;

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_cakeal_osmolink_bridge_OsmoNative_nativeBluetoothInit<'local>(
    mut unowned: EnvUnowned<'local>,
    _this: JObject<'local>,
) -> jboolean {
    unowned
        .with_env(|env| Ok::<_, jni::errors::Error>(btleplug::platform::init(env).is_ok()))
        .resolve::<ThrowRuntimeExAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_cakeal_osmolink_bridge_OsmoNative_nativeScanResult<'local>(
    mut unowned: EnvUnowned<'local>,
    _this: JObject<'local>,
    result: JObject<'local>,
) -> jboolean {
    unowned
        .with_env(|env| {
            let scanner = STATE
                .lock()
                .ok()
                .and_then(|state| state.scan.as_ref()?.scanner.clone());
            Ok::<_, jni::errors::Error>(
                scanner
                    .is_some_and(|scanner| scanner.report_android_scan_result(env, result).is_ok()),
            )
        })
        .resolve::<ThrowRuntimeExAndDefault>()
}
