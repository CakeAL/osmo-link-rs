//! Shared presentation data and mode availability for the mobile apps.
//!
//! Labels stay in English and generic; error details are the exception because
//! both mobile UIs show the same Chinese explanation.
use osmo_core::{
    ActionPhotoSize, CameraMode, CameraModel, CameraStatus, CaptureFormat, CaptureRate, EisMode,
    Error, Osmo360PhotoSize, PhotoRatio, ReturnCode, VideoResolution,
};

/// Mobile mode choices for the model ID returned by the BLE handshake.
pub fn selectable_modes(model: CameraModel) -> impl Iterator<Item = CameraMode> {
    use CameraMode::*;

    const ACTION: &[CameraMode] = &[
        Video,
        Photo,
        SlowMotion,
        Timelapse,
        Hyperlapse,
        SuperNight,
        SubjectTracking,
        LiveStream,
        UvcLiveStream,
    ];
    const OSMO_360: &[CameraMode] = &[
        PanoramicVideo,
        PanoramicPhoto,
        PanoramicHyperlapse,
        Timelapse,
        PanoramicSuperNight,
        Selfie,
        TimeFreeze,
        Video,
        Photo,
        UltraWideVideo,
        SingleLensSuperNight,
        LiveStream,
    ];
    let modes = match model {
        CameraModel::Action4 | CameraModel::Action5Pro | CameraModel::Action6 => ACTION,
        CameraModel::Osmo360 => OSMO_360,
        CameraModel::Unknown(_) => &[],
    };
    modes
        .iter()
        .copied()
        .filter(move |mode| model != CameraModel::Action4 || *mode != SubjectTracking)
}

/// User-facing detail for a failed camera command.
///
/// The original error text is kept so the raw code stays visible. Unknown
/// codes get the storage-card hint because a missing card is the most common
/// undocumented failure on the Action 4; known codes get their own text.
pub fn error_message(error: &Error) -> String {
    let Error::Camera(code) = error else {
        return error.to_string();
    };
    let detail = match code {
        ReturnCode::Success => "相机返回成功状态，无需重试",
        ReturnCode::ParseError => "相机无法解析该命令，请重试；反复失败时请检查相机固件",
        ReturnCode::ExecutionFailed => "相机拒绝执行该命令，请检查拍摄模式、存储卡和相机状态后重试",
        ReturnCode::UndefinedError => "相机返回未定义错误，请重启相机后重试",
        ReturnCode::Unknown(_) => {
            "可能未插卡（存储卡未插入或不可用）；如果已插卡，请检查卡片状态或重启相机后重试"
        }
    };
    format!("{error}：{detail}")
}

#[derive(uniffi::Record)]
pub struct MobileMode {
    pub code: u8,
    pub name: String,
}

pub fn mode_catalog(model: CameraModel) -> Vec<MobileMode> {
    selectable_modes(model)
        .map(|mode| MobileMode {
            code: mode.code(),
            name: format!("{mode:?}"),
        })
        .collect()
}

#[derive(uniffi::Record)]
pub struct MobileStatus {
    pub mode_code: u8,
    pub mode_name: String,
    pub state_name: String,
    pub photo_mode: bool,
    pub capturing: bool,
    pub resolution: String,
    pub fps: String,
    pub eis: String,
    pub record_seconds: u16,
    pub remaining_mb: u32,
    pub remaining_photos: u32,
    pub remaining_seconds: u32,
    pub battery: u8,
    pub temperature_code: u8,
    pub temperature_name: String,
}

impl From<&CameraStatus> for MobileStatus {
    fn from(status: &CameraStatus) -> Self {
        Self {
            mode_code: status.mode.code(),
            mode_name: format!("{:?}", status.mode),
            state_name: format!("{:?}", status.state),
            photo_mode: status.mode.uses_photo_parameters(),
            capturing: status.is_capturing(),
            resolution: resolution_label(status.resolution, status.mode, status.photo_ratio),
            fps: rate_label(status.fps),
            eis: eis_label(status.eis).into(),
            record_seconds: status.record_seconds,
            remaining_mb: status.remaining_mb,
            remaining_photos: status.remaining_photos,
            remaining_seconds: status.remaining_seconds,
            battery: status.battery_percent,
            temperature_code: status.temperature_warning.code(),
            temperature_name: format!("{:?}", status.temperature_warning),
        }
    }
}

fn resolution_label(format: CaptureFormat, mode: CameraMode, ratio: PhotoRatio) -> String {
    let photo_ratio = || match ratio {
        PhotoRatio::Ratio4x3 => "4:3".into(),
        PhotoRatio::Ratio16x9 => "16:9".into(),
        PhotoRatio::Unknown(_) => String::new(),
    };
    match format {
        CaptureFormat::Video(resolution) => match resolution {
            VideoResolution::P1080 => "1080P".into(),
            VideoResolution::K4Ratio16x9 => "4K 16:9".into(),
            VideoResolution::K27Ratio16x9 => "2.7K 16:9".into(),
            VideoResolution::P1080Ratio9x16 => "1080P 9:16".into(),
            VideoResolution::K27Ratio9x16 => "2.7K 9:16".into(),
            VideoResolution::K27Ratio4x3 => "2.7K 4:3".into(),
            VideoResolution::K4Ratio4x3 => "4K 4:3".into(),
            VideoResolution::K4Ratio9x16 => "4K 9:16".into(),
            VideoResolution::Unknown(code) => format!("0x{code:02X}"),
        },
        CaptureFormat::ActionPhoto(size) => match size {
            ActionPhotoSize::Medium => "Medium".into(),
            ActionPhotoSize::Large => "Large".into(),
            ActionPhotoSize::Unknown(_) => photo_ratio(),
        },
        CaptureFormat::Osmo360Photo(size) => match size {
            Osmo360PhotoSize::Standard12Mp => "12MP".into(),
            Osmo360PhotoSize::Wide20Mp => "20MP".into(),
            Osmo360PhotoSize::UltraWide30Mp => "30MP".into(),
            Osmo360PhotoSize::Unknown(_) => photo_ratio(),
        },
        CaptureFormat::Unknown(_) if mode.uses_photo_parameters() => photo_ratio(),
        CaptureFormat::Unknown(code) => format!("0x{code:02X}"),
    }
}

fn rate_label(rate: CaptureRate) -> String {
    match rate {
        CaptureRate::Video(fps) => match fps.frames_per_second() {
            Some(value) => format!("{value} fps"),
            None => format!("fps index {}", fps.code()),
        },
        CaptureRate::BurstCount(count) => format!("{count}-shot burst"),
        CaptureRate::SlowMotionMultiplier(factor) => format!("{factor}x slow motion"),
        CaptureRate::Unknown(code) => format!("0x{code:02X}"),
    }
}

fn eis_label(eis: EisMode) -> &'static str {
    match eis {
        EisMode::Off => "Off",
        EisMode::RockSteady => "RockSteady",
        EisMode::HorizonSteady => "HorizonSteady",
        EisMode::RockSteadyPlus => "RockSteady+",
        EisMode::HorizonBalancing => "HorizonBalancing",
        EisMode::Unknown(_) => "Unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_catalog_uses_the_connected_model() {
        let action4: Vec<_> = selectable_modes(CameraModel::Action4).collect();
        let action5: Vec<_> = selectable_modes(CameraModel::Action5Pro).collect();
        let action6: Vec<_> = selectable_modes(CameraModel::Action6).collect();
        let osmo360: Vec<_> = selectable_modes(CameraModel::Osmo360).collect();
        assert_eq!(action5, action6);
        assert!(action4.contains(&CameraMode::SlowMotion));
        assert!(action4.contains(&CameraMode::SuperNight));
        assert!(!action4.contains(&CameraMode::SubjectTracking));
        assert!(action5.contains(&CameraMode::SubjectTracking));
        assert!(!action5.contains(&CameraMode::PanoramicVideo));
        assert!(!osmo360.contains(&CameraMode::SubjectTracking));
        assert!(osmo360.contains(&CameraMode::PanoramicVideo));
        assert!(osmo360.contains(&CameraMode::Photo));
        let catalog = mode_catalog(CameraModel::Osmo360);
        assert_eq!(catalog[0].code, CameraMode::PanoramicVideo.code());
        assert_eq!(catalog[0].name, "PanoramicVideo");
        assert!(mode_catalog(CameraModel::Unknown(0)).is_empty());
    }

    #[test]
    fn camera_error_message_keeps_the_raw_code_and_adds_the_card_hint() {
        let message = error_message(&Error::Camera(ReturnCode::Unknown(0xd6)));
        assert!(message.contains("0xd6"));
        assert!(message.contains("Unknown(214)"));
        assert!(message.contains("可能未插卡"));
    }

    #[test]
    fn known_camera_errors_get_specific_details() {
        let parse = error_message(&Error::Camera(ReturnCode::ParseError));
        let execution = error_message(&Error::Camera(ReturnCode::ExecutionFailed));
        assert!(parse.contains("解析"));
        assert!(execution.contains("拒绝执行"));
        assert_ne!(parse, execution);
        assert_eq!(error_message(&Error::Disconnected), "camera disconnected");
    }

    #[test]
    fn unknown_photo_size_uses_the_reported_aspect_ratio() {
        let photo = CameraMode::Photo;
        let format = CaptureFormat::from_code(photo, CameraModel::Action4, 0x10);
        assert_eq!(format.code(), 0x10);
        assert_eq!(resolution_label(format, photo, PhotoRatio::Ratio4x3), "4:3");
        assert_eq!(
            resolution_label(format, photo, PhotoRatio::Ratio16x9),
            "16:9"
        );
        assert_eq!(
            resolution_label(format, photo, PhotoRatio::Unknown(0xff)),
            ""
        );

        let large = CaptureFormat::ActionPhoto(ActionPhotoSize::Large);
        assert_eq!(
            resolution_label(large, photo, PhotoRatio::Ratio4x3),
            "Large"
        );

        let video = CameraMode::Video;
        let format = CaptureFormat::from_code(video, CameraModel::Action4, 0x10);
        assert_eq!(
            resolution_label(format, video, PhotoRatio::Ratio4x3),
            "4K 16:9"
        );
    }
}
