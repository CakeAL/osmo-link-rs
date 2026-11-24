use time::{Duration, OffsetDateTime};

use crate::GpsPoint;
use crate::error::{Error, Result};

const MAX_OUTPUT_POINTS: usize = 10_000_000;

/// How decoded GPS samples are reduced before writing GPX.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResampleMethod {
    /// Keep every distinct fix.
    None,
    /// Keep the fix closest to every `1 / output_frequency` seconds.
    Discard,
    /// Interpolate at a fixed output interval.
    Linear,
}

impl std::str::FromStr for ResampleMethod {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "none" => Ok(Self::None),
            "discard" => Ok(Self::Discard),
            "linear" => Ok(Self::Linear),
            _ => Err(Error::Argument(format!(
                "unknown resampling method {value:?}; use none, discard or linear"
            ))),
        }
    }
}

impl std::fmt::Display for ResampleMethod {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::None => "none",
            Self::Discard => "discard",
            Self::Linear => "linear",
        })
    }
}

/// Resample raw GPS points.
///
/// DJI writes one `gps_info` entry per video frame and repeats the last fix
/// while no new one arrives, so a fix that updates once per second occupies
/// roughly thirty consecutive samples. Those repeats carry no new position:
/// they are collapsed before resampling, and the output never repeats the
/// previous position back to back.
pub fn resample(
    points: &[GpsPoint],
    output_frequency: f64,
    method: ResampleMethod,
) -> Result<Vec<GpsPoint>> {
    if method != ResampleMethod::None && (!output_frequency.is_finite() || output_frequency <= 0.0)
    {
        return Err(Error::Argument(
            "output frequency must be a positive finite number".to_owned(),
        ));
    }
    if points.windows(2).any(|pair| pair[1].time < pair[0].time) {
        return Err(Error::UnsortedTimestamps);
    }

    let fixes = coalesce(points);
    if method == ResampleMethod::None || fixes.len() < 2 {
        return Ok(fixes);
    }

    if method == ResampleMethod::Discard {
        discard(&fixes, output_frequency)
    } else {
        linear(&fixes, output_frequency)
    }
}

/// Collapse samples that only repeat the previous fix.
fn coalesce(points: &[GpsPoint]) -> Vec<GpsPoint> {
    let mut fixes: Vec<GpsPoint> = Vec::new();
    for point in points {
        push_fix(&mut fixes, point.clone());
    }
    fixes
}

fn same_position(left: &GpsPoint, right: &GpsPoint) -> bool {
    left.latitude == right.latitude
        && left.longitude == right.longitude
        && left.altitude_m == right.altitude_m
}

/// Push a point unless it repeats the position of the previous one.
fn push_fix(output: &mut Vec<GpsPoint>, point: GpsPoint) {
    if !output
        .last()
        .is_some_and(|last| same_position(last, &point))
    {
        output.push(point);
    }
}

fn discard(points: &[GpsPoint], output_frequency: f64) -> Result<Vec<GpsPoint>> {
    let start = points[0].time;
    let count = grid_count(points, output_frequency)?;
    let mut output = Vec::with_capacity(count.min(points.len()));
    let mut cursor = 0usize;

    for index in 0..count {
        let time = grid_time(start, index, output_frequency);
        while cursor + 1 < points.len()
            && distance_nanos(points[cursor + 1].time, time)
                < distance_nanos(points[cursor].time, time)
        {
            cursor += 1;
        }
        push_fix(&mut output, points[cursor].clone());
    }

    Ok(output)
}

fn linear(points: &[GpsPoint], output_frequency: f64) -> Result<Vec<GpsPoint>> {
    let start = points[0].time;
    let end = points[points.len() - 1].time;
    let count = grid_count(points, output_frequency)?;

    let mut output = Vec::with_capacity(count);
    let mut cursor = 0usize;

    for index in 0..count {
        let time = grid_time(start, index, output_frequency);
        if time > end {
            break;
        }

        while cursor + 1 < points.len() && points[cursor + 1].time < time {
            cursor += 1;
        }

        if points[cursor].time == time {
            push_fix(&mut output, points[cursor].clone());
            continue;
        }
        let Some(right) = points.get(cursor + 1) else {
            push_fix(&mut output, points[cursor].clone());
            continue;
        };

        let left = &points[cursor];
        let span_ms = (right.time - left.time).whole_milliseconds();
        let fraction = if span_ms <= 0 {
            0.0
        } else {
            (time - left.time).whole_milliseconds() as f64 / span_ms as f64
        };

        push_fix(
            &mut output,
            GpsPoint {
                time,
                latitude: lerp(left.latitude, right.latitude, fraction),
                longitude: lerp(left.longitude, right.longitude, fraction),
                altitude_m: lerp(left.altitude_m, right.altitude_m, fraction),
                accelerometer1: lerp_vec3(left.accelerometer1, right.accelerometer1, fraction),
                accelerometer2: lerp_vec3(left.accelerometer2, right.accelerometer2, fraction),
                derivatives: lerp_vec3(left.derivatives, right.derivatives, fraction),
            },
        );
    }

    Ok(output)
}

fn grid_count(points: &[GpsPoint], output_frequency: f64) -> Result<usize> {
    let total_seconds = (points[points.len() - 1].time - points[0].time).as_seconds_f64();
    let count = ((total_seconds * output_frequency).floor() as usize).saturating_add(1);
    if count > MAX_OUTPUT_POINTS {
        return Err(Error::TooManyOutputPoints(count));
    }
    Ok(count)
}

fn grid_time(start: OffsetDateTime, index: usize, output_frequency: f64) -> OffsetDateTime {
    let offset_ms = (index as f64 / output_frequency * 1000.0).round() as i64;
    start + Duration::milliseconds(offset_ms)
}

fn distance_nanos(left: OffsetDateTime, right: OffsetDateTime) -> i128 {
    (left - right).whole_nanoseconds().abs()
}

fn lerp(left: f64, right: f64, fraction: f64) -> f64 {
    left + (right - left) * fraction
}

fn lerp_vec3(left: Option<[f32; 3]>, right: Option<[f32; 3]>, fraction: f64) -> Option<[f32; 3]> {
    match (left, right) {
        (Some(left), Some(right)) => Some([
            lerp(f64::from(left[0]), f64::from(right[0]), fraction) as f32,
            lerp(f64::from(left[1]), f64::from(right[1]), fraction) as f32,
            lerp(f64::from(left[2]), f64::from(right[2]), fraction) as f32,
        ]),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use time::macros::datetime;

    use super::*;

    fn point(seconds: i64, latitude: f64) -> GpsPoint {
        let time = datetime!(2026-01-01 00:00:00 UTC) + Duration::seconds(seconds);
        GpsPoint {
            time,
            latitude,
            longitude: 114.0,
            altitude_m: 50.0,
            accelerometer1: Some([1.0, 2.0, 3.0]),
            accelerometer2: None,
            derivatives: None,
        }
    }

    fn at(milliseconds: i64, latitude: f64) -> GpsPoint {
        GpsPoint {
            time: datetime!(2026-01-01 00:00:00 UTC) + Duration::milliseconds(milliseconds),
            ..point(0, latitude)
        }
    }

    #[test]
    fn linear_interpolates_half_seconds() {
        let points = vec![point(0, 0.0), point(1, 1.0), point(2, 2.0)];
        let output = resample(&points, 2.0, ResampleMethod::Linear).expect("resample");

        assert_eq!(output.len(), 5);
        let latitudes: Vec<f64> = output.iter().map(|point| point.latitude).collect();
        assert_eq!(latitudes, vec![0.0, 0.5, 1.0, 1.5, 2.0]);
        assert_eq!(output[1].accelerometer1, Some([1.0, 2.0, 3.0]));
        assert_eq!(output[1].accelerometer2, None);
        assert_eq!(output[4].time, datetime!(2026-01-01 00:00:02 UTC));
    }

    #[test]
    fn collapses_a_fix_repeated_over_consecutive_frames() {
        // 30 fps file: one fix per second occupies ~30 identical samples.
        let points = vec![
            at(0, 0.0),
            at(33, 0.0),
            at(66, 0.0),
            at(1000, 1.0),
            at(1033, 1.0),
        ];
        let output = resample(&points, 2.0, ResampleMethod::None).expect("none");

        assert_eq!(output.len(), 2);
        assert_eq!(output[0].time, datetime!(2026-01-01 00:00:00 UTC));
        assert_eq!(output[1].time, datetime!(2026-01-01 00:00:01 UTC));
    }

    #[test]
    fn linear_interpolates_between_fixes_instead_of_repeating_one() {
        // A held from 0ms to 200ms, then B from 400ms: at 5 Hz the midpoint
        // must interpolate between A and B rather than emit A twice.
        let points = vec![
            at(0, 0.0),
            at(100, 0.0),
            at(200, 0.0),
            at(400, 1.0),
            at(500, 1.0),
        ];
        let output = resample(&points, 5.0, ResampleMethod::Linear).expect("linear");

        let latitudes: Vec<f64> = output.iter().map(|point| point.latitude).collect();
        assert_eq!(latitudes, vec![0.0, 0.5, 1.0]);
    }

    #[test]
    fn discard_picks_the_fix_closest_to_each_interval() {
        let points = vec![at(0, 0.0), at(900, 1.0), at(1400, 2.0), at(2000, 3.0)];
        let output = resample(&points, 1.0, ResampleMethod::Discard).expect("discard");

        let times: Vec<OffsetDateTime> = output.iter().map(|point| point.time).collect();
        assert_eq!(
            times,
            vec![
                datetime!(2026-01-01 00:00:00 UTC),
                datetime!(2026-01-01 00:00:00.900 UTC),
                datetime!(2026-01-01 00:00:02 UTC),
            ]
        );
        assert_eq!(output[1].latitude, 1.0);
    }

    #[test]
    fn discard_never_repeats_the_same_fix() {
        let points = vec![at(0, 0.0), at(1000, 1.0), at(2000, 2.0)];
        let output = resample(&points, 5.0, ResampleMethod::Discard).expect("discard");
        assert_eq!(output.len(), 3);
    }

    #[test]
    fn none_keeps_every_distinct_fix() {
        let points = vec![point(0, 0.0), point(1, 1.0)];
        let output = resample(&points, 2.0, ResampleMethod::None).expect("none");
        assert_eq!(output, points);
    }

    #[test]
    fn rejects_unsorted_points() {
        let points = vec![point(1, 0.0), point(0, 1.0)];
        let error = resample(&points, 2.0, ResampleMethod::Linear).expect_err("unsorted");
        assert!(matches!(error, Error::UnsortedTimestamps));
    }
}
