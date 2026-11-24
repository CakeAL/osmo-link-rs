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
    /// Wall clock time from the MP4 sample timeline, anchored on the embedded
    /// GPS datetimes. Keeping the timeline as the clock preserves the real
    /// sub-second spacing of the samples and keeps one corrupt embedded
    /// timestamp from reordering the whole track.
    pub time: OffsetDateTime,
    pub latitude: f64,
    pub longitude: f64,
    pub altitude_m: f64,
    pub accelerometer1: Option<[f32; 3]>,
    pub accelerometer2: Option<[f32; 3]>,
    pub derivatives: Option<[f32; 3]>,
}

/// Metadata and GPS points extracted from one MP4 file.
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
    /// Hours subtracted from the embedded GPS time. DJI writes a naive
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
    let mut fixes = Vec::new();
    let mut embedded_times = Vec::new();

    for sample in &samples {
        let message = match protobuf::parse_generic_message(&sample.data) {
            Ok(message) => message,
            Err(_) => {
                skipped_samples += 1;
                continue;
            }
        };

        update_metadata(&mut extraction, &message);

        for gps in message.gps_info {
            match gps_fix(gps) {
                Some((fix, datetime)) => {
                    if let Some(datetime) = datetime {
                        embedded_times.push((sample.time, datetime));
                    }
                    fixes.push((sample.time, fix));
                }
                None => skipped_samples += 1,
            }
        }
    }

    extraction.skipped_samples = skipped_samples;
    if fixes.is_empty() {
        return Err(Error::NoGpsPoints);
    }

    let anchor = anchor_nanos(&embedded_times).ok_or(Error::MissingTimestamps)?;
    let offset_ms = (options.timezone_offset_hours * 3_600_000.0).round() as i64;
    let base =
        OffsetDateTime::from_unix_timestamp_nanos(anchor - i128::from(offset_ms) * 1_000_000)
            .map_err(|_| Error::InvalidTimestamp("embedded GPS time is out of range".to_owned()))?;

    let mut points = Vec::with_capacity(fixes.len());
    for (track_time, fix) in fixes {
        let time = base.checked_add(track_time).ok_or_else(|| {
            Error::InvalidTimestamp(format!(
                "MP4 timeline at {} s is out of range",
                track_time.as_seconds_f64()
            ))
        })?;
        points.push(fix.at(time));
    }
    extraction.points = points;

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

/// Absolute time of track time zero, estimated from the embedded GPS clock.
///
/// DJI stores that clock truncated to whole seconds, so every sample implies a
/// slightly different offset. The median keeps the occasional corrupt
/// timestamp (`12:06:12` in the middle of a `02:06` minute is a real example)
/// from moving the whole track, and the half second centres the estimate in
/// the one-second window lost to truncation.
fn anchor_nanos(embedded_times: &[(Duration, OffsetDateTime)]) -> Option<i128> {
    let mut offsets: Vec<i128> = embedded_times
        .iter()
        .map(|(track_time, datetime)| {
            datetime.unix_timestamp_nanos() - track_time.whole_nanoseconds()
        })
        .collect();
    offsets.sort_unstable();

    let median = *offsets.get(offsets.len() / 2)?;
    Some(median + 500_000_000)
}

/// Positional part of one `gps_info` entry, before its time is known.
struct GpsFix {
    latitude: f64,
    longitude: f64,
    altitude_m: f64,
    accelerometer1: Option<[f32; 3]>,
    accelerometer2: Option<[f32; 3]>,
    derivatives: Option<[f32; 3]>,
}

impl GpsFix {
    fn at(self, time: OffsetDateTime) -> GpsPoint {
        GpsPoint {
            time,
            latitude: self.latitude,
            longitude: self.longitude,
            altitude_m: self.altitude_m,
            accelerometer1: self.accelerometer1,
            accelerometer2: self.accelerometer2,
            derivatives: self.derivatives,
        }
    }
}

/// Decode one `gps_info` entry, keeping its embedded timestamp when it parses.
fn gps_fix(gps: protobuf::GpsInfo) -> Option<(GpsFix, Option<OffsetDateTime>)> {
    let remote = gps.remote_gps_info?;
    let coordinates = remote.coordinates?;
    let info = coordinates.info?;
    let (Some(latitude), Some(longitude)) = (info.latitude, info.longitude) else {
        return None;
    };

    // DJI stores degrees in the current Osmo Action schemas (`coordinate_units = 1`).
    if !latitude.is_finite()
        || !longitude.is_finite()
        || !(-90.0..=90.0).contains(&latitude)
        || !(-180.0..=180.0).contains(&longitude)
    {
        return None;
    }

    // A corrupt embedded clock is not a reason to drop the fix: the sample
    // timeline still knows when it happened, and `anchor_nanos` ignores it.
    let datetime = coordinates
        .datetime
        .as_ref()
        .and_then(|datetime| parse_datetime(&datetime.datetime).ok());

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

    Some((
        GpsFix {
            latitude,
            longitude,
            altitude_m: f64::from(coordinates.gps_altitude_mm) / 1000.0,
            accelerometer1,
            accelerometer2,
            derivatives: remote.derivatives.map(|value| [value.x, value.y, value.z]),
        },
        datetime,
    ))
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
    use prost::Message;
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

    /// Embedded clock at whole seconds for samples 0.4 s apart.
    fn truncated_anchors(base: OffsetDateTime, count: i64) -> Vec<(Duration, OffsetDateTime)> {
        (0..count)
            .map(|index| {
                let track_time = Duration::milliseconds(index * 400);
                (track_time, base + Duration::seconds(index * 400 / 1000))
            })
            .collect()
    }

    #[test]
    fn anchors_the_track_timeline_on_the_embedded_clock() {
        let base = datetime!(2026-11-05 02:06:01 UTC);
        let anchor = anchor_nanos(&truncated_anchors(base, 50)).expect("anchor");

        let error = anchor - base.unix_timestamp_nanos();
        assert!(error.abs() <= 500_000_000, "anchor is off by {error} ns");
    }

    #[test]
    fn anchor_ignores_a_corrupt_embedded_timestamp() {
        let base = datetime!(2026-11-05 02:06:01 UTC);
        let mut embedded = truncated_anchors(base, 50);
        let (track_time, _) = embedded[10];
        embedded[10] = (track_time, base + Duration::hours(10));

        let anchor = anchor_nanos(&embedded).expect("anchor");
        let error = anchor - base.unix_timestamp_nanos();
        assert!(error.abs() <= 500_000_000, "anchor is off by {error} ns");
    }

    #[test]
    fn anchor_needs_at_least_one_timestamp() {
        assert_eq!(anchor_nanos(&[]), None);
    }

    fn boxed(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let size = u32::try_from(8 + payload.len()).expect("test box fits in u32");
        let mut out = Vec::with_capacity(8 + payload.len());
        out.extend_from_slice(&size.to_be_bytes());
        out.extend_from_slice(kind);
        out.extend_from_slice(payload);
        out
    }

    fn full_box(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let mut body = vec![0u8; 4];
        body.extend_from_slice(payload);
        boxed(kind, &body)
    }

    fn sample_message(datetime: Option<&str>, latitude: f64) -> Vec<u8> {
        protobuf::GenericMessage {
            video_global_info: Some(protobuf::VideoGlobalInfo {
                module_info: vec![protobuf::ModuleInfo {
                    proto_name: "dvtm_ac206.proto".to_owned(),
                    serial_number: "9KRVP8A00B06LC".to_owned(),
                    camera_name: "DJI OsmoAction6".to_owned(),
                }],
            }),
            video_stream_info: None,
            gps_info: vec![protobuf::GpsInfo {
                camera_info: None,
                remote_gps_info: Some(protobuf::RemoteGpsInfo {
                    coordinates: Some(protobuf::Coordinates {
                        info: Some(protobuf::CoordinatesInfo {
                            coordinate_units: 1,
                            latitude: Some(latitude),
                            longitude: Some(114.248_571_8),
                        }),
                        gps_altitude_mm: 52_287,
                        datetime: datetime.map(|datetime| protobuf::CoordinatesDatetime {
                            datetime: datetime.to_owned(),
                        }),
                    }),
                    derivatives: None,
                }),
            }],
        }
        .encode_to_vec()
    }

    /// Minimal MP4 with one `djmd` track holding `samples` at 29.97 fps.
    fn mp4_with_samples(samples: &[Vec<u8>]) -> Vec<u8> {
        let mut mdat_payload = Vec::new();
        let mut offsets = Vec::new();
        for sample in samples {
            offsets.push(u32::try_from(8 + mdat_payload.len()).expect("offset fits in u32"));
            mdat_payload.extend_from_slice(sample);
        }
        let mdat = boxed(b"mdat", &mdat_payload);

        let mut stsd = vec![0, 0, 0, 1];
        stsd.extend_from_slice(&20u32.to_be_bytes());
        stsd.extend_from_slice(b"djmd");
        stsd.extend_from_slice(&[0u8; 8]);

        let mut stsz = vec![0u8; 4];
        stsz.extend_from_slice(
            &u32::try_from(samples.len())
                .expect("count fits")
                .to_be_bytes(),
        );
        for sample in samples {
            stsz.extend_from_slice(
                &u32::try_from(sample.len())
                    .expect("size fits")
                    .to_be_bytes(),
            );
        }

        let mut stco = u32::try_from(offsets.len())
            .expect("count fits")
            .to_be_bytes()
            .to_vec();
        for offset in offsets {
            stco.extend_from_slice(&offset.to_be_bytes());
        }

        // One sample per chunk, like DJI writes them.
        let mut stsc = 1u32.to_be_bytes().to_vec();
        stsc.extend_from_slice(&1u32.to_be_bytes());
        stsc.extend_from_slice(&1u32.to_be_bytes());
        stsc.extend_from_slice(&1u32.to_be_bytes());

        let mut stts = 1u32.to_be_bytes().to_vec();
        stts.extend_from_slice(
            &u32::try_from(samples.len())
                .expect("count fits")
                .to_be_bytes(),
        );
        stts.extend_from_slice(&1001u32.to_be_bytes());

        let mut mdhd = vec![0u8; 8];
        mdhd.extend_from_slice(&30_000u32.to_be_bytes());
        mdhd.extend_from_slice(&0u32.to_be_bytes());

        let stbl = boxed(
            b"stbl",
            &[
                full_box(b"stsd", &stsd),
                full_box(b"stsz", &stsz),
                full_box(b"stco", &stco),
                full_box(b"stsc", &stsc),
                full_box(b"stts", &stts),
            ]
            .concat(),
        );
        let mdia = boxed(
            b"mdia",
            &[full_box(b"mdhd", &mdhd), boxed(b"minf", &stbl)].concat(),
        );

        [mdat, boxed(b"moov", &boxed(b"trak", &mdia))].concat()
    }

    fn extract_from_temp_mp4(name: &str, samples: &[Vec<u8>]) -> Result<Extraction> {
        let path = std::env::temp_dir().join(format!(
            "osmo_gps_extract_gpx_{}_{name}.mp4",
            std::process::id()
        ));
        std::fs::write(&path, mp4_with_samples(samples)).expect("write test mp4");
        let extraction = extract_gps(&path);
        let _ = std::fs::remove_file(&path);
        extraction
    }

    #[test]
    fn keeps_a_fix_whose_embedded_timestamp_jumps() {
        // 30 samples roughly one 29.97 fps frame apart, each reporting the
        // embedded clock truncated to whole seconds. Sample 10 is corrupt,
        // exactly like the `12:06:12` sample seen in a real file. Like the
        // camera, the file repeats each fix for every frame, so it carries
        // two fixes, not thirty.
        let base = datetime!(2026-11-05 02:06:01 UTC);
        let format = format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");
        let mut samples = Vec::new();
        for index in 0..30 {
            let datetime = base + Duration::milliseconds(index * 33);
            let text = if index == 10 {
                "2026-11-05 12:06:12".to_owned()
            } else {
                datetime.format(&format).expect("format timestamp")
            };
            let latitude = if index < 15 {
                22.708_072_2
            } else {
                22.709_072_2
            };
            samples.push(sample_message(Some(&text), latitude));
        }

        let extraction = extract_from_temp_mp4("corrupt", &samples).expect("extract");

        assert_eq!(extraction.proto_name, "dvtm_ac206.proto");
        // The corrupt timestamp is not a reason to drop its GPS fix.
        assert_eq!(extraction.points.len(), 30);

        // Points stay on the sample timeline, so the corrupt embedded value
        // neither reorders the track nor moves its anchor.
        assert!(
            (extraction.points[0].time - base).abs() <= Duration::milliseconds(500),
            "anchor moved to {}",
            extraction.points[0].time
        );
        for pair in extraction.points.windows(2) {
            assert_eq!(
                (pair[1].time - pair[0].time).whole_milliseconds(),
                33,
                "spacing changed at {}",
                pair[0].time
            );
        }

        // Linear resampling used to reject the whole file here.
        let points = resample(&extraction.points, 2.0, ResampleMethod::Linear).expect("resample");
        let latitudes: Vec<f64> = points.iter().map(|point| point.latitude).collect();
        assert_eq!(latitudes, vec![22.708_072_2, 22.709_072_2]);

        // The per-frame repeats are one fix each, not thirty points.
        let fixes = resample(&extraction.points, 2.0, ResampleMethod::None).expect("none");
        assert_eq!(fixes.len(), 2);
    }

    #[test]
    fn rejects_a_track_without_embedded_timestamps() {
        let samples = [
            sample_message(None, 22.708_072_2),
            sample_message(None, 22.709_072_2),
        ];

        let error = extract_from_temp_mp4("no-time", &samples).expect_err("no anchor");
        assert!(matches!(error, Error::MissingTimestamps));
    }
}
