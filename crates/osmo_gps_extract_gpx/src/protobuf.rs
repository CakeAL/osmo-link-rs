use prost::Message;

use crate::error::{Error, Result};

#[derive(Clone, PartialEq, Message)]
pub(crate) struct GenericMessage {
    #[prost(message, optional, tag = "1")]
    pub(crate) video_global_info: Option<VideoGlobalInfo>,
    #[prost(message, optional, tag = "2")]
    pub(crate) video_stream_info: Option<VideoStreamInfo>,
    #[prost(message, repeated, tag = "3")]
    pub(crate) gps_info: Vec<GpsInfo>,
}

#[derive(Clone, PartialEq, Message)]
pub(crate) struct VideoGlobalInfo {
    #[prost(message, repeated, tag = "1")]
    pub(crate) module_info: Vec<ModuleInfo>,
}

#[derive(Clone, PartialEq, Message)]
pub(crate) struct ModuleInfo {
    #[prost(string, tag = "1")]
    pub(crate) proto_name: String,
    #[prost(string, tag = "5")]
    pub(crate) serial_number: String,
    #[prost(string, tag = "10")]
    pub(crate) camera_name: String,
}

#[derive(Clone, PartialEq, Message)]
pub(crate) struct VideoStreamInfo {
    #[prost(message, optional, tag = "3")]
    pub(crate) details: Option<VideoStreamDetails>,
}

#[derive(Clone, PartialEq, Message)]
pub(crate) struct VideoStreamDetails {
    #[prost(float, optional, tag = "3")]
    pub(crate) frame_rate: Option<f32>,
}

#[derive(Clone, PartialEq, Message)]
pub(crate) struct GpsInfo {
    #[prost(message, optional, tag = "2")]
    pub(crate) camera_info: Option<CameraInfo>,
    #[prost(message, optional, tag = "4")]
    pub(crate) remote_gps_info: Option<RemoteGpsInfo>,
}

#[derive(Clone, PartialEq, Message)]
pub(crate) struct CameraInfo {
    #[prost(message, optional, tag = "9")]
    pub(crate) accelerometer1: Option<Accelerometer>,
    #[prost(message, optional, tag = "10")]
    pub(crate) accelerometer2: Option<Accelerometer>,
}

#[derive(Clone, PartialEq, Message)]
pub(crate) struct Accelerometer {
    #[prost(float, tag = "2")]
    pub(crate) x: f32,
    #[prost(float, tag = "3")]
    pub(crate) y: f32,
    #[prost(float, tag = "4")]
    pub(crate) z: f32,
}

#[derive(Clone, PartialEq, Message)]
pub(crate) struct RemoteGpsInfo {
    #[prost(message, optional, tag = "2")]
    pub(crate) coordinates: Option<Coordinates>,
    #[prost(message, optional, tag = "3")]
    pub(crate) derivatives: Option<Derivative>,
}

#[derive(Clone, PartialEq, Message)]
pub(crate) struct Coordinates {
    #[prost(message, optional, tag = "1")]
    pub(crate) info: Option<CoordinatesInfo>,
    #[prost(uint32, tag = "2")]
    pub(crate) gps_altitude_mm: u32,
    #[prost(message, optional, tag = "6")]
    pub(crate) datetime: Option<CoordinatesDatetime>,
}

#[derive(Clone, PartialEq, Message)]
pub(crate) struct CoordinatesInfo {
    #[prost(uint32, tag = "1")]
    pub(crate) coordinate_units: u32,
    #[prost(double, optional, tag = "2")]
    pub(crate) latitude: Option<f64>,
    #[prost(double, optional, tag = "3")]
    pub(crate) longitude: Option<f64>,
}

#[derive(Clone, PartialEq, Message)]
pub(crate) struct CoordinatesDatetime {
    #[prost(string, tag = "1")]
    pub(crate) datetime: String,
}

#[derive(Clone, PartialEq, Message)]
pub(crate) struct Derivative {
    #[prost(float, tag = "1")]
    pub(crate) x: f32,
    #[prost(float, tag = "2")]
    pub(crate) y: f32,
    #[prost(float, tag = "3")]
    pub(crate) z: f32,
}

/// Decode one `djmd` sample.
pub(crate) fn parse_generic_message(data: &[u8]) -> Result<GenericMessage> {
    GenericMessage::decode(data).map_err(|error| Error::Protobuf(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_message() -> GenericMessage {
        GenericMessage {
            video_global_info: Some(VideoGlobalInfo {
                module_info: vec![ModuleInfo {
                    proto_name: "dvtm_ac206.proto".to_owned(),
                    serial_number: "9KRVP8A00B06LC".to_owned(),
                    camera_name: "DJI OsmoAction6".to_owned(),
                }],
            }),
            video_stream_info: Some(VideoStreamInfo {
                details: Some(VideoStreamDetails {
                    frame_rate: Some(29.97),
                }),
            }),
            gps_info: vec![GpsInfo {
                camera_info: Some(CameraInfo {
                    accelerometer1: Some(Accelerometer {
                        x: 0.000_183_6,
                        y: 0.046_016_93,
                        z: -0.243_561_55,
                    }),
                    accelerometer2: None,
                }),
                remote_gps_info: Some(RemoteGpsInfo {
                    coordinates: Some(Coordinates {
                        info: Some(CoordinatesInfo {
                            coordinate_units: 1,
                            latitude: Some(22.708_072_2),
                            longitude: Some(114.248_571_8),
                        }),
                        gps_altitude_mm: 52_287,
                        datetime: Some(CoordinatesDatetime {
                            datetime: "2026-11-05 02:42:52".to_owned(),
                        }),
                    }),
                    derivatives: Some(Derivative {
                        x: 13.4,
                        y: 21.2,
                        z: 0.0,
                    }),
                }),
            }],
        }
    }

    #[test]
    fn round_trips_generic_message() {
        let message = sample_message();
        let mut bytes = message.encode_to_vec();
        // Unknown field 99 is skipped by prost.
        bytes.extend([0x98, 0x06, 0x07]);

        let decoded = parse_generic_message(&bytes).expect("valid message");
        assert_eq!(decoded, message);
    }

    #[test]
    fn rejects_truncated_message() {
        let error = parse_generic_message(&[0x0a, 0x05, 0x01]).expect_err("truncated");
        assert!(error.to_string().contains("buffer"));
    }
}
