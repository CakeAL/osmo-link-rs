use osmo_core::{Device, GpsSample};

use crate::status::{MobileMode, MobileStatus};

#[derive(uniffi::Record)]
pub struct MobileDevice {
    pub id: String,
    pub name: String,
    pub rssi: Option<i16>,
}

impl From<&Device> for MobileDevice {
    fn from(device: &Device) -> Self {
        Self {
            id: device.id.clone(),
            name: device.name.clone(),
            rssi: device.rssi,
        }
    }
}

#[derive(uniffi::Record)]
pub struct LocationSample {
    pub timestamp_ms: i64,
    pub longitude: f64,
    pub latitude: f64,
    pub altitude_m: f64,
    pub north_mps: f64,
    pub east_mps: f64,
    pub down_mps: f64,
    pub vertical_accuracy_m: f64,
    pub horizontal_accuracy_m: f64,
    pub speed_accuracy_mps: f64,
    pub satellites: u32,
}

impl From<LocationSample> for GpsSample {
    fn from(sample: LocationSample) -> Self {
        Self {
            timestamp_ms: sample.timestamp_ms,
            longitude: sample.longitude,
            latitude: sample.latitude,
            altitude_m: sample.altitude_m,
            north_mps: sample.north_mps,
            east_mps: sample.east_mps,
            down_mps: sample.down_mps,
            vertical_accuracy_m: sample.vertical_accuracy_m,
            horizontal_accuracy_m: sample.horizontal_accuracy_m,
            speed_accuracy_mps: sample.speed_accuracy_mps,
            satellites: sample.satellites,
        }
    }
}

#[derive(uniffi::Enum)]
pub enum MobileEvent {
    Devices {
        devices: Vec<MobileDevice>,
        scanning: bool,
        connecting: bool,
    },
    Pairing {
        code: u16,
        device: String,
        first_time: bool,
        auto: bool,
    },
    Connected {
        name: String,
        model: String,
        modes: Vec<MobileMode>,
    },
    ConnectFailed {
        message: String,
        auto: bool,
    },
    ScanFailed {
        message: String,
    },
    Status {
        status: MobileStatus,
    },
    ModeInfo {
        name: String,
        parameters: String,
    },
    Version {
        product: String,
        details: String,
    },
    Command {
        action: String,
        error: Option<String>,
    },
    Warning {
        message: String,
    },
    GpsError {
        message: String,
    },
    Reconnecting {
        attempt: u8,
    },
    Reconnected,
    Disconnected {
        reason: Option<String>,
    },
}

#[uniffi::export(with_foreign)]
pub trait MobileListener: Send + Sync {
    fn on_event(&self, event: MobileEvent);
    fn set_ble_scanning(&self, enabled: bool) -> Option<String>;
}
