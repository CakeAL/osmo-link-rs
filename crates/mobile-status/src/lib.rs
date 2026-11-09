//! Shared JSON projections of protocol enums for the mobile apps.
//!
//! Labels stay in English and generic; localization belongs to the app layer.
use osmo_core::{
    ActionPhotoSize, CameraMode, CameraStatus, CaptureFormat, CaptureRate, EisMode,
    Osmo360PhotoSize, VideoResolution,
};
use serde_json::{Value, json};

/// Every camera mode the protocol documents, in a UI-friendly order.
pub fn mode_catalog() -> Value {
    const MODES: [CameraMode; 17] = [
        CameraMode::Video,
        CameraMode::Photo,
        CameraMode::SlowMotion,
        CameraMode::Timelapse,
        CameraMode::Hyperlapse,
        CameraMode::SuperNight,
        CameraMode::SubjectTracking,
        CameraMode::Selfie,
        CameraMode::PanoramicVideo,
        CameraMode::PanoramicPhoto,
        CameraMode::PanoramicHyperlapse,
        CameraMode::PanoramicSuperNight,
        CameraMode::SingleLensSuperNight,
        CameraMode::UltraWideVideo,
        CameraMode::TimeFreeze,
        CameraMode::LiveStream,
        CameraMode::UvcLiveStream,
    ];
    let modes: Vec<Value> = MODES
        .iter()
        .map(|mode| json!({ "code": mode.code(), "name": format!("{mode:?}") }))
        .collect();
    json!({ "modes": modes })
}

pub fn status_json(status: &CameraStatus) -> Value {
    json!({
        "modeCode": status.mode.code(),
        "modeName": format!("{:?}", status.mode),
        "photoMode": status.mode.uses_photo_parameters(),
        "stateCode": status.state.code(),
        "stateName": format!("{:?}", status.state),
        "capturing": status.is_capturing(),
        "resolution": resolution_label(status.resolution),
        "fps": rate_label(status.fps),
        "eis": eis_label(status.eis),
        "recordSeconds": status.record_seconds,
        "remainingMb": status.remaining_mb,
        "remainingPhotos": status.remaining_photos,
        "remainingSeconds": status.remaining_seconds,
        "battery": status.battery_percent,
        "temperatureCode": status.temperature_warning.code(),
        "temperatureName": format!("{:?}", status.temperature_warning),
        "powerMode": status.power_mode.code(),
        "userMode": status.user_mode.code(),
        "nextMode": status.next_mode.code(),
        "photoRatio": status.photo_ratio.code(),
    })
}

fn resolution_label(format: CaptureFormat) -> String {
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
            ActionPhotoSize::Unknown(code) => format!("0x{code:02X}"),
        },
        CaptureFormat::Osmo360Photo(size) => match size {
            Osmo360PhotoSize::Standard12Mp => "12MP".into(),
            Osmo360PhotoSize::Wide20Mp => "20MP".into(),
            Osmo360PhotoSize::UltraWide30Mp => "30MP".into(),
            Osmo360PhotoSize::Unknown(code) => format!("0x{code:02X}"),
        },
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
