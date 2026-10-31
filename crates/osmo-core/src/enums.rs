//! Named values from DJI's [DATA protocol document](https://github.com/dji-sdk/Osmo-GPS-Controller-Demo/blob/main/docs/protocol_data_segment_CN.md).
//!
//! `code()` returns the protocol value, which is not necessarily a physical unit
//! (for example, frame rates are indices). Unknown received values stay intact;
//! command APIs reject them rather than sending undocumented settings.
use std::fmt;

macro_rules! protocol_enum {
    ($(#[$meta:meta])* pub enum $name:ident : $wire:ty { $($(#[$variant_meta:meta])* $variant:ident = $value:expr),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum $name {
            $($(#[$variant_meta])* $variant,)+
            Unknown($wire),
        }

        impl $name {
            pub const fn from_code(code: $wire) -> Self {
                match code { $($value => Self::$variant,)+ other => Self::Unknown(other) }
            }

            pub const fn code(self) -> $wire {
                match self { $(Self::$variant => $value,)+ Self::Unknown(code) => code }
            }

            /// Reject unknown values when constructing outgoing commands.
            pub fn checked_code(self) -> crate::Result<$wire> {
                match self {
                    Self::Unknown(_) => Err(crate::Error::InvalidInput(concat!("unknown ", stringify!($name)))),
                    _ => Ok(self.code()),
                }
            }
        }

        impl From<$wire> for $name {
            fn from(code: $wire) -> Self { Self::from_code(code) }
        }

        impl From<$name> for $wire {
            fn from(value: $name) -> Self { value.code() }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{self:?}") }
        }
    };
}

protocol_enum! {
    /// Camera IDs. Osmo Nano has no assigned supported ID in this document.
    pub enum CameraModel: u32 {
        Action4 = 0xff33,
        Action5Pro = 0xff44,
        Action6 = 0xff55,
        Osmo360 = 0xff66,
    }
}

protocol_enum! {
    /// Common return codes; u16 also accommodates the version-query ACK field.
    pub enum ReturnCode: u16 {
        Success = 0,
        ParseError = 1,
        ExecutionFailed = 2,
        UndefinedError = 0xff,
    }
}

protocol_enum! {
    pub enum VerificationMode: u8 {
        Automatic = 0,
        Required = 1,
        Result = 2,
    }
}

protocol_enum! {
    /// Meaning of verify_data only when verify_mode is Result.
    pub enum VerificationResult: u16 {
        Allowed = 0,
        Rejected = 1,
    }
}

protocol_enum! {
    pub enum Key: u8 {
        Shutter = 1,
        Qs = 2,
        Snapshot = 3,
    }
}

protocol_enum! {
    pub enum KeyReportMode: u8 {
        State = 0,
        Event = 1,
    }
}

protocol_enum! {
    pub enum KeyState: u16 {
        Pressed = 0,
        Released = 1,
    }
}

protocol_enum! {
    pub enum KeyEvent: u16 {
        Short = 0,
        Long = 1,
        Double = 2,
        Triple = 3,
        Quadruple = 4,
    }
}

protocol_enum! {
    pub enum RecordControl: u8 {
        Start = 0,
        Stop = 1,
    }
}

protocol_enum! {
    pub enum StatusPushMode: u8 {
        Off = 0,
        Once = 1,
        Periodic = 2,
        PeriodicAndChanges = 3,
    }
}

protocol_enum! {
    /// Wire unit is 0.1Hz; only 20 (2Hz) is documented as supported.
    pub enum StatusPushFrequency: u8 {
        Hz2 = 20,
    }
}

protocol_enum! {
    pub enum CameraMode: u8 {
        SlowMotion = 0x00,
        Video = 0x01,
        Timelapse = 0x02,
        Photo = 0x05,
        Hyperlapse = 0x0a,
        LiveStream = 0x1a,
        UvcLiveStream = 0x23,
        SuperNight = 0x28,
        SubjectTracking = 0x34,
        PanoramicVideo = 0x38,
        PanoramicHyperlapse = 0x3a,
        Selfie = 0x3c,
        PanoramicPhoto = 0x3f,
        UltraWideVideo = 0x41,
        TimeFreeze = 0x43,
        PanoramicSuperNight = 0x44,
        SingleLensSuperNight = 0x4a,
    }
}

impl CameraMode {
    pub const fn uses_photo_parameters(self) -> bool {
        matches!(
            self,
            Self::Photo | Self::PanoramicPhoto | Self::SubjectTracking
        )
    }
}

protocol_enum! {
    pub enum CameraState: u8 {
        ScreenOff = 0,
        LiveView = 1,
        Playback = 2,
        Capturing = 3,
        PreRecording = 5,
    }
}

protocol_enum! {
    pub enum VideoResolution: u8 {
        P1080 = 10,
        K4Ratio16x9 = 16,
        K27Ratio16x9 = 45,
        P1080Ratio9x16 = 66,
        K27Ratio9x16 = 67,
        K27Ratio4x3 = 95,
        K4Ratio4x3 = 103,
        K4Ratio9x16 = 109,
    }
}

protocol_enum! {
    pub enum ActionPhotoSize: u8 {
        Medium = 3,
        Large = 4,
    }
}

protocol_enum! {
    pub enum Osmo360PhotoSize: u8 {
        Standard12Mp = 2,
        Wide20Mp = 3,
        UltraWide30Mp = 4,
    }
}

protocol_enum! {
    /// fps_idx codes, not literal frames per second.
    pub enum FrameRate: u8 {
        Fps24 = 1,
        Fps25 = 2,
        Fps30 = 3,
        Fps48 = 4,
        Fps50 = 5,
        Fps60 = 6,
        Fps100 = 10,
        Fps120 = 7,
        Fps200 = 19,
        Fps240 = 8,
    }
}

impl FrameRate {
    pub const fn frames_per_second(self) -> Option<u16> {
        match self {
            Self::Fps24 => Some(24),
            Self::Fps25 => Some(25),
            Self::Fps30 => Some(30),
            Self::Fps48 => Some(48),
            Self::Fps50 => Some(50),
            Self::Fps60 => Some(60),
            Self::Fps100 => Some(100),
            Self::Fps120 => Some(120),
            Self::Fps200 => Some(200),
            Self::Fps240 => Some(240),
            Self::Unknown(_) => None,
        }
    }
}

protocol_enum! {
    pub enum EisMode: u8 {
        Off = 0,
        RockSteady = 1,
        HorizonSteady = 2,
        RockSteadyPlus = 3,
        HorizonBalancing = 4,
    }
}

protocol_enum! {
    pub enum PhotoRatio: u8 {
        Ratio4x3 = 0,
        Ratio16x9 = 1,
    }
}

protocol_enum! {
    pub enum UserMode: u8 {
        General = 0,
        Custom1 = 1,
        Custom2 = 2,
        Custom3 = 3,
        Custom4 = 4,
        Custom5 = 5,
    }
}

impl UserMode {
    /// The protocol explicitly says invalid user modes are treated as 0.
    pub const fn normalized(self) -> Self {
        match self {
            Self::Unknown(_) => Self::General,
            mode => mode,
        }
    }
}

protocol_enum! {
    pub enum PowerMode: u8 {
        Normal = 0,
        Sleep = 3,
    }
}

protocol_enum! {
    pub enum TemperatureStatus: u8 {
        Normal = 0,
        Warning = 1,
        TooHotToRecord = 2,
        ShutdownImminent = 3,
    }
}

protocol_enum! {
    /// Documented photo-countdown display values, in milliseconds.
    pub enum PhotoCountdown: u32 {
        HalfSecond = 500,
        OneSecond = 1000,
        TwoSeconds = 2000,
        ThreeSeconds = 3000,
        FiveSeconds = 5000,
        TenSeconds = 10000,
    }
}

protocol_enum! {
    /// Loop recording duration in seconds, including Off/Max sentinels.
    pub enum LoopRecordingDuration: u16 {
        Off = 0,
        FiveMinutes = 300,
        TwentyMinutes = 1200,
        OneHour = 3600,
        Max = 0xffff,
    }
}

protocol_enum! {
    /// Tags in the new 1D06 camera status message.
    pub enum ModeInfoType: u8 {
        Name = 1,
        Parameters = 2,
    }
}

/// The resolution byte has different meanings for videos and photos/models.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureFormat {
    Video(VideoResolution),
    ActionPhoto(ActionPhotoSize),
    Osmo360Photo(Osmo360PhotoSize),
    Unknown(u8),
}

impl CaptureFormat {
    pub const fn from_code(mode: CameraMode, model: CameraModel, code: u8) -> Self {
        if mode.uses_photo_parameters() {
            match model {
                CameraModel::Action4 | CameraModel::Action5Pro | CameraModel::Action6 => {
                    Self::ActionPhoto(ActionPhotoSize::from_code(code))
                }
                CameraModel::Osmo360 => Self::Osmo360Photo(Osmo360PhotoSize::from_code(code)),
                CameraModel::Unknown(_) => Self::Unknown(code),
            }
        } else if matches!(mode, CameraMode::Unknown(_)) {
            Self::Unknown(code)
        } else {
            Self::Video(VideoResolution::from_code(code))
        }
    }

    pub const fn code(self) -> u8 {
        match self {
            Self::Video(value) => value.code(),
            Self::ActionPhoto(value) => value.code(),
            Self::Osmo360Photo(value) => value.code(),
            Self::Unknown(code) => code,
        }
    }
}

/// The fps_idx byte may encode an FPS index, burst count, or slow-motion factor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureRate {
    Video(FrameRate),
    BurstCount(u8),
    SlowMotionMultiplier(u8),
    Unknown(u8),
}

impl CaptureRate {
    pub const fn from_code(mode: CameraMode, code: u8) -> Self {
        if mode.uses_photo_parameters() {
            Self::BurstCount(code)
        } else if matches!(mode, CameraMode::SlowMotion) {
            Self::SlowMotionMultiplier(code)
        } else if matches!(mode, CameraMode::Unknown(_)) {
            Self::Unknown(code)
        } else {
            Self::Video(FrameRate::from_code(code))
        }
    }

    pub const fn code(self) -> u8 {
        match self {
            Self::Video(value) => value.code(),
            Self::BurstCount(code) | Self::SlowMotionMultiplier(code) | Self::Unknown(code) => code,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn documented_units_and_unknown_values() {
        // FPS indices must not be confused with physical FPS, nor duration sentinels.
        assert_eq!(FrameRate::Fps200.code(), 19);
        assert_eq!(FrameRate::Fps200.frames_per_second(), Some(200));
        assert_eq!(LoopRecordingDuration::FiveMinutes.code(), 300);
        assert_eq!(LoopRecordingDuration::Max.code(), 65535);
        assert_eq!(PhotoCountdown::HalfSecond.code(), 500);
        assert_eq!(StatusPushFrequency::Hz2.code(), 20);
        assert_eq!(CameraMode::SingleLensSuperNight.code(), 0x4a);
        let unknown = CameraMode::from_code(0xe7);
        assert_eq!(unknown, CameraMode::Unknown(0xe7));
        assert_eq!(unknown.code(), 0xe7);
        assert!(unknown.checked_code().is_err());
        assert_eq!(ReturnCode::from_code(0xe4), ReturnCode::Unknown(0xe4));
        assert_eq!(UserMode::from_code(99).normalized(), UserMode::General);
    }
}
