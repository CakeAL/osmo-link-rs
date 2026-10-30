use crate::{Error, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CameraStatus {
    pub mode: u8,
    pub state: u8,
    pub resolution: u8,
    pub fps: u8,
    pub eis: u8,
    pub record_seconds: u16,
    pub fov: u8,
    pub photo_ratio: u8,
    pub countdown_seconds: u16,
    pub timelapse_interval: u16,
    pub timelapse_duration: u16,
    pub remaining_mb: u32,
    pub remaining_photos: u32,
    pub remaining_seconds: u32,
    pub user_mode: u8,
    pub power_mode: u8,
    pub next_mode: u8,
    pub temperature_warning: u8,
    pub photo_countdown_ms: u32,
    pub loop_seconds: u16,
    pub battery_percent: u8,
}

impl CameraStatus {
    pub(crate) fn decode(b: &[u8]) -> Result<Self> {
        if b.len() < 38 {
            return Err(Error::Malformed("camera status"));
        }
        let u16_at = |i| u16::from_le_bytes(b[i..i + 2].try_into().unwrap());
        let u32_at = |i| u32::from_le_bytes(b[i..i + 4].try_into().unwrap());
        Ok(Self {
            mode: b[0],
            state: b[1],
            resolution: b[2],
            fps: b[3],
            eis: b[4],
            record_seconds: u16_at(5),
            fov: b[7],
            photo_ratio: b[8],
            countdown_seconds: u16_at(9),
            timelapse_interval: u16_at(11),
            timelapse_duration: u16_at(13),
            remaining_mb: u32_at(15),
            remaining_photos: u32_at(19),
            remaining_seconds: u32_at(23),
            user_mode: b[27],
            power_mode: b[28],
            next_mode: b[29],
            temperature_warning: b[30],
            photo_countdown_ms: u32_at(31),
            loop_seconds: u16_at(35),
            battery_percent: b[37],
        })
    }

    pub fn is_capturing(&self) -> bool {
        self.state == 3
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModeInfo {
    pub name: String,
    pub parameters: String,
}

impl ModeInfo {
    pub(crate) fn decode(b: &[u8]) -> Result<Self> {
        if b.len() < 46 || b[0] != 1 || b[23] != 2 || b[1] > 20 || b[24] > 20 {
            return Err(Error::Malformed("mode information"));
        }
        Ok(Self {
            name: String::from_utf8_lossy(&b[2..2 + usize::from(b[1])])
                .trim_end_matches('\0')
                .into(),
            parameters: String::from_utf8_lossy(&b[25..25 + usize::from(b[24])])
                .trim_end_matches('\0')
                .into(),
        })
    }
}
