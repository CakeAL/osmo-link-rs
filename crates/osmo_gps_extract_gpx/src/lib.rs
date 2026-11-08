mod error;
mod gpx;
mod mp4;
mod protobuf;
mod resample;

use std::path::Path;
use time::format_description::well_known::Rfc3339;
use time::macros::format_description;
use time::{Duration, OffsetDateTime, PrimitiveDateTime};

pub use error::{Error, Result};
pub use gpx::{write_gpx, write_gpx_file};
pub use resample::{ResampleMethod, resample};

/// Models whose `djmd` protobuf layout has been checked against public
/// `dvtm_ac203.proto` / `dvtm_ac204.proto` / `dvtm_ac206.proto` definitions.
pub const SUPPORTED_MODELS: [&str; 3] =
    ["dvtm_ac203.proto", "dvtm_ac204.proto", "dvtm_ac206.proto"];

/// One GPS fix decoded from a `djmd` metadata sample.
#[derive(Debug, Clone, PartialEq)]
pub struct GpsPoint {
    pub time: OffsetDateTime,
    pub latitude: f64,
    pub longitude: f64,
    pub altitude_m: f64,
    pub accelerometer1: Option<[f32; 3]>,
    pub accelerometer2: Option<[f32; 3]>,
    pub derivatives: Option<[f32; 3]>,
}

/// Metadata and raw GPS points extracted from one MP4 file.
#[derive(Debug, Clone)]
pub struct Extraction {
    pub proto_name: String,
    pub camera_name: String,
    pub serial_number: String,
    pub frame_rate: Option<f64>,
    pub points: Vec<GpsPoint>,
    pub skipped_samples: usize,
}

impl Extraction {
    /// Whether the embedded `proto_name` is one of the verified Action 4/5/6 schemas.
    pub fn is_supported_model(&self) -> bool {
        SUPPORTED_MODELS.contains(&self.proto_name.as_str())
    }
}

/// Extraction options shared by the library and the CLI.
#[derive(Debug, Clone, Copy, Default)]
pub struct ExtractOptions {
    /// Hours subtracted from the embedded timestamp. DJI writes a naive
    /// `YYYY-MM-DD HH:MM:SS` string; use 0 to keep it verbatim.
    pub timezone_offset_hours: f64,
}

/// Extract every GPS point from the DJI `djmd` track of an MP4 file.
pub fn extract_gps(path: impl AsRef<Path>) -> Result<Extraction> {
    extract_gps_with_options(path, ExtractOptions::default())
}

/// Extract GPS points with an explicit timezone offset.
pub fn extract_gps_with_options(
    path: impl AsRef<Path>,
    options: ExtractOptions,
) -> Result<Extraction> {
    if !options.timezone_offset_hours.is_finite() {
        return Err(Error::Argument(
            "timezone offset must be a finite number".to_owned(),
        ));
    }

    let samples = mp4::read_djmd_samples(path.as_ref())?;
    let mut extraction = Extraction {
        proto_name: String::new(),
        camera_name: String::new(),
        serial_number: String::new(),
        frame_rate: None,
        points: Vec::new(),
        skipped_samples: 0,
    };
    let mut skipped_samples = 0usize;

    for sample in &samples {
        let message = match protobuf::parse_generic_message(sample) {
            Ok(message) => message,
            Err(_) => {
                skipped_samples += 1;
                continue;
            }
        };

        update_metadata(&mut extraction, &message);

        for gps in message.gps_info {
            match gps_to_point(gps, options.timezone_offset_hours) {
                Ok(Some(point)) => extraction.points.push(point),
                Ok(None) => skipped_samples += 1,
                Err(Error::InvalidTimestamp(_)) => skipped_samples += 1,
                Err(error) => return Err(error),
            }
        }
    }

    extraction.skipped_samples = skipped_samples;
    if extraction.points.is_empty() {
        return Err(Error::NoGpsPoints);
    }

    Ok(extraction)
}

fn update_metadata(extraction: &mut Extraction, message: &protobuf::GenericMessage) {
    if let Some(global) = &message.video_global_info
        && let Some(module) = global.module_info.first()
    {
        if extraction.proto_name.is_empty() {
            extraction.proto_name.clone_from(&module.proto_name);
        }
        if extraction.camera_name.is_empty() {
            extraction.camera_name.clone_from(&module.camera_name);
        }
        if extraction.serial_number.is_empty() {
            extraction.serial_number.clone_from(&module.serial_number);
        }
    }

    if let Some(frame_rate) = message
        .video_stream_info
        .as_ref()
        .and_then(|info| info.details.as_ref())
        .and_then(|details| details.frame_rate)
        && frame_rate.is_finite()
        && frame_rate > 0.0
    {
        extraction.frame_rate = Some(f64::from(frame_rate));
    }
}

fn gps_to_point(gps: protobuf::GpsInfo, timezone_offset_hours: f64) -> Result<Option<GpsPoint>> {
    let Some(remote) = gps.remote_gps_info else {
        return Ok(None);
    };
    let Some(coordinates) = remote.coordinates else {
        return Ok(None);
    };
    let Some(info) = coordinates.info else {
        return Ok(None);
    };
    let (Some(latitude), Some(longitude)) = (info.latitude, info.longitude) else {
        return Ok(None);
    };

    // DJI stores degrees in the current Osmo Action schemas (`coordinate_units = 1`).
    if !latitude.is_finite()
        || !longitude.is_finite()
        || !(-90.0..=90.0).contains(&latitude)
        || !(-180.0..=180.0).contains(&longitude)
    {
        return Ok(None);
    }

    let Some(datetime) = coordinates.datetime.as_ref() else {
        return Ok(None);
    };
    let time = match parse_datetime(&datetime.datetime) {
        Ok(time) => time,
        Err(_) => return Ok(None),
    };

    let offset_ms = (timezone_offset_hours * 3_600_000.0).round() as i64;
    let time = time - Duration::milliseconds(offset_ms);

    let (accelerometer1, accelerometer2) = match gps.camera_info {
        Some(camera) => (
            camera
                .accelerometer1
                .map(|value| [value.x, value.y, value.z]),
            camera
                .accelerometer2
                .map(|value| [value.x, value.y, value.z]),
        ),
        None => (None, None),
    };

    Ok(Some(GpsPoint {
        time,
        latitude,
        longitude,
        altitude_m: f64::from(coordinates.gps_altitude_mm) / 1000.0,
        accelerometer1,
        accelerometer2,
        derivatives: remote.derivatives.map(|value| [value.x, value.y, value.z]),
    }))
}

fn parse_datetime(value: &str) -> Result<OffsetDateTime> {
    let formats = [
        format_description!("[year]-[month]-[day] [hour]:[minute]:[second]"),
        format_description!("[year]-[month]-[day] [hour]:[minute]:[second].[subsecond]"),
        format_description!("[year]-[month]-[day]T[hour]:[minute]:[second]"),
        format_description!("[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond]"),
    ];

    for format in formats {
        if let Ok(naive) = PrimitiveDateTime::parse(value, format) {
            return Ok(naive.assume_utc());
        }
    }
    if let Ok(datetime) = OffsetDateTime::parse(value, &Rfc3339) {
        return Ok(datetime);
    }

    Err(Error::InvalidTimestamp(value.to_owned()))
}

#[cfg(test)]
mod tests {
    use time::macros::datetime;

    use super::*;

    #[test]
    fn parses_dji_timestamp_and_subtracts_offset() {
        let time = parse_datetime("2026-11-05 02:42:52").expect("timestamp");
        assert_eq!(time, datetime!(2026-11-05 02:42:52 UTC));

        let time = time - Duration::hours(8);
        assert_eq!(time, datetime!(2026-11-04 18:42:52 UTC));
    }

    #[test]
    fn recognizes_supported_protos() {
        let mut extraction = Extraction {
            proto_name: "dvtm_ac206.proto".to_owned(),
            camera_name: String::new(),
            serial_number: String::new(),
            frame_rate: None,
            points: Vec::new(),
            skipped_samples: 0,
        };
        assert!(extraction.is_supported_model());

        extraction.proto_name = "dvtm_other.proto".to_owned();
        assert!(!extraction.is_supported_model());
    }
}
