//! Shared JSON projections of protocol enums for the mobile apps.
//!
//! Labels stay in English and generic; localization belongs to the app layer.
use osmo_core::{
    ActionPhotoSize, CameraMode, CameraModel, CameraStatus, CaptureFormat, CaptureRate, EisMode,
    Osmo360PhotoSize, PhotoRatio, VideoResolution,
};
use serde_json::{Value, json};

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

pub fn mode_catalog(model: CameraModel) -> Vec<Value> {
    selectable_modes(model)
        .map(|mode| json!({ "code": mode.code(), "name": format!("{mode:?}") }))
        .collect()
}

pub fn status_json(status: &CameraStatus) -> Value {
    json!({
        "modeCode": status.mode.code(),
        "modeName": format!("{:?}", status.mode),
        "photoMode": status.mode.uses_photo_parameters(),
        "stateCode": status.state.code(),
        "stateName": format!("{:?}", status.state),
        "capturing": status.is_capturing(),
        "resolution": resolution_label(status.resolution, status.mode, status.photo_ratio),
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
        assert_eq!(
            mode_catalog(CameraModel::Osmo360)[0],
            json!({ "code": CameraMode::PanoramicVideo.code(), "name": "PanoramicVideo" })
        );
        assert!(mode_catalog(CameraModel::Unknown(0)).is_empty());
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
