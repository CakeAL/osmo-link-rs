use chrono::{DateTime, Datelike, Timelike, Utc};
use serde::{Deserialize, Serialize};

use crate::{Error, Result};

/// A valid GNSS fix. Coordinates use degrees, distances meters, velocities m/s.
/// Accuracy and satellite count must come from the positioning source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpsSample {
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

impl GpsSample {
    pub fn validate(&self) -> Result<()> {
        self.encode().map(|_| ())
    }

    pub(crate) fn encode(&self) -> Result<Vec<u8>> {
        let utc = DateTime::<Utc>::from_timestamp_millis(self.timestamp_ms)
            .filter(|t| (2000..=2099).contains(&t.year()))
            .ok_or(Error::InvalidInput("GPS timestamp must be in 2000..2099"))?;
        // DJI specifies UTC+8. Include date rollover at midnight.
        let local = utc + chrono::Duration::hours(8);
        if !self.longitude.is_finite()
            || !(-180.0..=180.0).contains(&self.longitude)
            || !self.latitude.is_finite()
            || !(-90.0..=90.0).contains(&self.latitude)
        {
            return Err(Error::InvalidInput("invalid GPS coordinates"));
        }
        if self.satellites == 0 {
            return Err(Error::InvalidInput("GPS fix has no satellites"));
        }
        let mut b = Vec::with_capacity(48);
        let date = local.year() * 10000 + local.month() as i32 * 100 + local.day() as i32;
        let time = local.hour() * 10000 + local.minute() * 100 + local.second();
        b.extend_from_slice(&date.to_le_bytes());
        b.extend_from_slice(&(time as i32).to_le_bytes());
        for value in [
            self.longitude * 1e7,
            self.latitude * 1e7,
            self.altitude_m * 1000.0,
        ] {
            if !value.is_finite() || !(i32::MIN as f64..=i32::MAX as f64).contains(&value.round()) {
                return Err(Error::InvalidInput("GPS position out of wire range"));
            }
            b.extend_from_slice(&(value.round() as i32).to_le_bytes());
        }
        for value in [self.north_mps, self.east_mps, self.down_mps] {
            let scaled = (value * 100.0) as f32;
            if !scaled.is_finite() {
                return Err(Error::InvalidInput("invalid GPS velocity"));
            }
            b.extend_from_slice(&scaled.to_le_bytes());
        }
        for value in [
            self.vertical_accuracy_m * 1000.0,
            self.horizontal_accuracy_m * 1000.0,
            self.speed_accuracy_mps * 100.0,
        ] {
            if !value.is_finite() || !(0.0..=u32::MAX as f64).contains(&value) {
                return Err(Error::InvalidInput("invalid GPS accuracy"));
            }
            b.extend_from_slice(&(value.round() as u32).to_le_bytes());
        }
        b.extend_from_slice(&self.satellites.to_le_bytes());
        Ok(b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn units_date_rollover_and_invalid_fix() {
        let mut sample = GpsSample {
            timestamp_ms: DateTime::parse_from_rfc3339("2026-10-30T20:01:02Z")
                .unwrap()
                .timestamp_millis(),
            longitude: 113.5,
            latitude: -22.5,
            altitude_m: 12.345,
            north_mps: 1.25,
            east_mps: -2.0,
            down_mps: 0.0,
            vertical_accuracy_m: 1.0,
            horizontal_accuracy_m: 2.0,
            speed_accuracy_mps: 0.1,
            satellites: 12,
        };
        let b = sample.encode().unwrap();
        assert_eq!(b.len(), 48);
        assert_eq!(i32::from_le_bytes(b[0..4].try_into().unwrap()), 20261031);
        assert_eq!(i32::from_le_bytes(b[4..8].try_into().unwrap()), 40102);
        assert_eq!(i32::from_le_bytes(b[8..12].try_into().unwrap()), 1135000000);
        assert_eq!(
            i32::from_le_bytes(b[12..16].try_into().unwrap()),
            -225000000
        );
        assert_eq!(i32::from_le_bytes(b[16..20].try_into().unwrap()), 12345);
        assert_eq!(f32::from_le_bytes(b[20..24].try_into().unwrap()), 125.0);
        sample.latitude = f64::NAN;
        assert!(sample.validate().is_err());
        sample.latitude = 0.0;
        sample.satellites = 0;
        assert!(sample.validate().is_err());
    }
}
