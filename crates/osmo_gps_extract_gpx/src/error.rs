use std::result::Result as StdResult;

/// Errors returned by the MP4/protobuf/GPX pipeline.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid MP4: {0}")]
    InvalidMp4(String),

    #[error("invalid DJI protobuf: {0}")]
    Protobuf(String),

    #[error("no `djmd` metadata track found ({0})")]
    MissingMetadataTrack(String),

    #[error("no GPS points found in the metadata track")]
    NoGpsPoints,

    #[error("invalid timestamp {0:?}")]
    InvalidTimestamp(String),

    #[error("invalid argument: {0}")]
    Argument(String),

    #[error("frame rate is required for discard resampling")]
    MissingFrameRate,

    #[error("GPS timestamps are not monotonically increasing")]
    UnsortedTimestamps,

    #[error("too many output points ({0}); lower the frequency")]
    TooManyOutputPoints(usize),

    #[error("GPX error: {0}")]
    Gpx(#[from] gpx::errors::GpxError),
}

pub type Result<T> = StdResult<T, Error>;
