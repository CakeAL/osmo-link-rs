use time::Duration;

use crate::GpsPoint;
use crate::error::{Error, Result};

const MAX_OUTPUT_POINTS: usize = 10_000_000;

/// How raw GPS samples are reduced before writing GPX.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResampleMethod {
    /// Keep every decoded sample.
    None,
    /// Keep one sample every `input_frequency / output_frequency` samples.
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
pub fn resample(
    points: &[GpsPoint],
    input_frequency: Option<f64>,
    output_frequency: f64,
    method: ResampleMethod,
) -> Result<Vec<GpsPoint>> {
    if method == ResampleMethod::None {
        return Ok(points.to_vec());
    }
    if !output_frequency.is_finite() || output_frequency <= 0.0 {
        return Err(Error::Argument(
            "output frequency must be a positive finite number".to_owned(),
        ));
    }
    if points.len() < 2 {
        return Ok(points.to_vec());
    }
    if points.windows(2).any(|pair| pair[1].time < pair[0].time) {
        return Err(Error::UnsortedTimestamps);
    }

    if method == ResampleMethod::Discard {
        discard(points, input_frequency, output_frequency)
    } else {
        linear(points, output_frequency)
    }
}

fn discard(
    points: &[GpsPoint],
    input_frequency: Option<f64>,
    output_frequency: f64,
) -> Result<Vec<GpsPoint>> {
    let input_frequency = input_frequency.ok_or(Error::MissingFrameRate)?;
    if !input_frequency.is_finite() || input_frequency <= 0.0 {
        return Err(Error::MissingFrameRate);
    }
    if output_frequency > input_frequency {
        return Err(Error::Argument(
            "discard output frequency cannot exceed the input frame rate".to_owned(),
        ));
    }

    let step = (input_frequency / output_frequency).round().max(1.0) as usize;
    Ok(points.iter().step_by(step).cloned().collect())
}

fn linear(points: &[GpsPoint], output_frequency: f64) -> Result<Vec<GpsPoint>> {
    let start = points[0].time;
    let end = points[points.len() - 1].time;
    let total_seconds = (end - start).as_seconds_f64();
    let count = ((total_seconds * output_frequency).floor() as usize).saturating_add(1);
    if count > MAX_OUTPUT_POINTS {
        return Err(Error::TooManyOutputPoints(count));
    }

    let mut output = Vec::with_capacity(count);
    let mut cursor = 0usize;

    for index in 0..count {
        let offset_ms = (index as f64 / output_frequency * 1000.0).round() as i64;
        let time = start + Duration::milliseconds(offset_ms);
        if time > end {
            break;
        }

        while cursor + 1 < points.len() && points[cursor + 1].time < time {
            cursor += 1;
        }

        if points[cursor].time == time {
            output.push(points[cursor].clone());
            continue;
        }
        let Some(right) = points.get(cursor + 1) else {
            output.push(points[cursor].clone());
            continue;
        };

        let left = &points[cursor];
        let span_ms = (right.time - left.time).whole_milliseconds();
        let fraction = if span_ms <= 0 {
            0.0
        } else {
            (time - left.time).whole_milliseconds() as f64 / span_ms as f64
        };

        output.push(GpsPoint {
            time,
            latitude: lerp(left.latitude, right.latitude, fraction),
            longitude: lerp(left.longitude, right.longitude, fraction),
            altitude_m: lerp(left.altitude_m, right.altitude_m, fraction),
            accelerometer1: lerp_vec3(left.accelerometer1, right.accelerometer1, fraction),
            accelerometer2: lerp_vec3(left.accelerometer2, right.accelerometer2, fraction),
            derivatives: lerp_vec3(left.derivatives, right.derivatives, fraction),
        });
    }

    Ok(output)
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

    #[test]
    fn linear_interpolates_half_seconds() {
        let points = vec![point(0, 0.0), point(1, 1.0), point(2, 2.0)];
        let output = resample(&points, Some(1.0), 2.0, ResampleMethod::Linear).expect("resample");

        assert_eq!(output.len(), 5);
        let latitudes: Vec<f64> = output.iter().map(|point| point.latitude).collect();
        assert_eq!(latitudes, vec![0.0, 0.5, 1.0, 1.5, 2.0]);
        assert_eq!(output[1].accelerometer1, Some([1.0, 2.0, 3.0]));
        assert_eq!(output[1].accelerometer2, None);
        assert_eq!(output[4].time, datetime!(2026-01-01 00:00:02 UTC));
    }

    #[test]
    fn discard_uses_frame_rate() {
        let points: Vec<GpsPoint> = (0..10).map(|index| point(index, index as f64)).collect();
        let output = resample(&points, Some(30.0), 10.0, ResampleMethod::Discard).expect("discard");

        assert_eq!(output.len(), 4);
        assert_eq!(output[0].latitude, 0.0);
        assert_eq!(output[1].latitude, 3.0);
        assert_eq!(output[2].latitude, 6.0);
        assert_eq!(output[3].latitude, 9.0);
    }

    #[test]
    fn none_keeps_every_point() {
        let points = vec![point(0, 0.0), point(1, 1.0)];
        let output = resample(&points, None, 2.0, ResampleMethod::None).expect("none");
        assert_eq!(output, points);
    }

    #[test]
    fn rejects_unsorted_points() {
        let points = vec![point(1, 0.0), point(0, 1.0)];
        let error =
            resample(&points, Some(1.0), 2.0, ResampleMethod::Linear).expect_err("unsorted");
        assert!(matches!(error, Error::UnsortedTimestamps));
    }
}
