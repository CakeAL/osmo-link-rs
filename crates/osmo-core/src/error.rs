use crate::protocol::ProtocolError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Bluetooth(#[from] btleplug::Error),
    #[error(transparent)]
    Protocol(#[from] ProtocolError),
    #[error("no Bluetooth adapter available")]
    NoAdapter,
    #[error("camera service or characteristics missing")]
    MissingCharacteristics,
    #[error("camera disconnected")]
    Disconnected,
    #[error("{0} timed out; the camera may have received the command")]
    Timeout(&'static str),
    #[error("camera rejected pairing")]
    PairingRejected,
    #[error("malformed {0}")]
    Malformed(&'static str),
    #[error("camera returned code {0:#04x}")]
    Camera(u16),
    #[error("invalid input: {0}")]
    InvalidInput(&'static str),
}
