//! Rust BLE remote controller for the public DJI R SDK protocol.
mod ble;
mod camera;
pub mod enums;
mod error;
mod gps;
pub mod protocol;
mod status;

pub use ble::{Device, Scanner};
pub use camera::{Camera, ConnectOptions, ConnectionState, ControllerIdentity, Event, Version};
pub use enums::*;
pub use error::{Error, Result};
pub use gps::GpsSample;
pub use status::{CameraStatus, ModeInfo};

/// Initialize the btleplug Java/JNI backend from the Android application thread.
#[cfg(target_os = "android")]
pub use btleplug::platform::init as init_android;
