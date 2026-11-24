use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use geo_types::Point;
use gpx::{Gpx, GpxVersion, Track, TrackSegment, Waypoint};

use crate::GpsPoint;
use crate::error::Result;

/// Write a GPX 1.1 document with one track and one segment.
pub fn write_gpx<W: Write>(writer: W, points: &[GpsPoint], name: &str) -> Result<()> {
    let mut gpx = Gpx {
        version: GpxVersion::Gpx11,
        creator: Some("osmo_gps_extract_gpx".to_owned()),
        ..Gpx::default()
    };

    let mut track = Track {
        name: (!name.is_empty()).then(|| name.to_owned()),
        ..Track::default()
    };
    let mut segment = TrackSegment::default();

    for point in points {
        let latitude = round(point.latitude, 7);
        let longitude = round(point.longitude, 7);
        let mut waypoint = Waypoint::new(Point::new(longitude, latitude));
        waypoint.elevation = Some(round(point.altitude_m, 3));
        waypoint.time = Some(point.time.into());
        segment.points.push(waypoint);
    }

    track.segments.push(segment);
    gpx.tracks.push(track);

    gpx::write(&gpx, writer)?;
    Ok(())
}

fn round(value: f64, decimals: u32) -> f64 {
    let factor = 10f64.powi(decimals as i32);
    (value * factor).round() / factor
}

/// Write a GPX file.
pub fn write_gpx_file(path: impl AsRef<Path>, points: &[GpsPoint], name: &str) -> Result<()> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    write_gpx(&mut writer, points, name)?;
    writer.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use time::macros::datetime;

    use super::*;

    fn point() -> GpsPoint {
        GpsPoint {
            time: datetime!(2026-10-03 04:00:00 UTC),
            latitude: 22.708_072_2,
            longitude: 114.248_571_8,
            altitude_m: 52.287,
            accelerometer1: Some([0.1, 0.2, 0.3]),
            accelerometer2: None,
            derivatives: Some([13.4, 21.2, 0.0]),
        }
    }

    #[test]
    fn writes_gpx_track() {
        let mut output = Vec::new();
        write_gpx(&mut output, &[point()], "DJI_0003").expect("write gpx");
        let text = String::from_utf8(output).expect("utf-8");
        assert!(text.contains("<gpx"));
        assert!(text.contains("version=\"1.1\""));
        assert!(text.contains("<name>DJI_0003</name>"));
        assert!(text.contains("lat=\"22.7080722\""));
        assert!(text.contains("lon=\"114.2485718\""));
        assert!(text.contains("<ele>52.287</ele>"));
        assert!(text.contains("<time>2026-10-03T04:00:00.000000000Z</time>"));
    }

    #[test]
    fn escapes_track_name() {
        let mut output = Vec::new();
        write_gpx(&mut output, &[], "A & B <test>").expect("write gpx");
        let text = String::from_utf8(output).expect("utf-8");

        assert!(text.contains("A &amp; B &lt;test&gt;"));
    }
}
