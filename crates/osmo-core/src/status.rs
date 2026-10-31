use crate::{
    CameraMode, CameraModel, CameraState, CaptureFormat, CaptureRate, EisMode, Error,
    LoopRecordingDuration, ModeInfoType, PhotoCountdown, PhotoRatio, PowerMode, Result,
    TemperatureStatus, UserMode,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CameraStatus {
    /// 当前拍摄模式；未知的新固件取值保留在 Unknown 中。
    pub mode: CameraMode,
    /// 相机工作状态，Capturing 同时用于拍照和录像。
    pub state: CameraState,
    /// 视频分辨率或照片尺寸，由模式和相机型号决定。
    pub resolution: CaptureFormat,
    /// 帧率索引、慢动作倍率或连拍张数，由模式决定。
    pub fps: CaptureRate,
    /// 电子防抖模式。
    pub eis: EisMode,
    /// 录像时间（含预录制），单位秒；连拍时为连拍时限，单位毫秒。
    pub record_seconds: u16,
    /// FOV 保留字段，文档没有定义枚举值。
    pub fov: u8,
    pub photo_ratio: PhotoRatio,
    /// 实时倒计时，单位秒。
    pub countdown_seconds: u16,
    /// 静止延时间隔，单位 0.1 秒；运动延时为拍摄速率，0 为 Auto。
    pub timelapse_interval: u16,
    /// 延时录像时长，单位秒。
    pub timelapse_duration: u16,
    /// SD 卡剩余容量，单位 MB。
    pub remaining_mb: u32,
    pub remaining_photos: u32,
    pub remaining_seconds: u32,
    /// 非法值按文档规定归一为 General。
    pub user_mode: UserMode,
    pub power_mode: PowerMode,
    /// QS 预切换目标模式，否则与当前 mode 相同。
    pub next_mode: CameraMode,
    pub temperature_warning: TemperatureStatus,
    pub photo_countdown: PhotoCountdown,
    pub loop_duration: LoopRecordingDuration,
    /// 电池电量百分比，0..100，不是枚举。
    pub battery_percent: u8,
}

impl CameraStatus {
    pub(crate) fn decode(b: &[u8], model: CameraModel) -> Result<Self> {
        if b.len() < 38 {
            return Err(Error::Malformed("camera status"));
        }
        let u16_at = |i| u16::from_le_bytes(b[i..i + 2].try_into().unwrap());
        let u32_at = |i| u32::from_le_bytes(b[i..i + 4].try_into().unwrap());
        let mode = CameraMode::from_code(b[0]);
        Ok(Self {
            mode,
            state: CameraState::from_code(b[1]),
            resolution: CaptureFormat::from_code(mode, model, b[2]),
            fps: CaptureRate::from_code(mode, b[3]),
            eis: EisMode::from_code(b[4]),
            record_seconds: u16_at(5),
            fov: b[7],
            photo_ratio: PhotoRatio::from_code(b[8]),
            countdown_seconds: u16_at(9),
            timelapse_interval: u16_at(11),
            timelapse_duration: u16_at(13),
            remaining_mb: u32_at(15),
            remaining_photos: u32_at(19),
            remaining_seconds: u32_at(23),
            user_mode: UserMode::from_code(b[27]).normalized(),
            power_mode: PowerMode::from_code(b[28]),
            next_mode: CameraMode::from_code(b[29]),
            temperature_warning: TemperatureStatus::from_code(b[30]),
            photo_countdown: PhotoCountdown::from_code(u32_at(31)),
            loop_duration: LoopRecordingDuration::from_code(u16_at(35)),
            battery_percent: b[37],
        })
    }

    pub fn is_capturing(&self) -> bool {
        self.state == CameraState::Capturing
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModeInfo {
    pub name: String,
    pub parameters: String,
}

impl ModeInfo {
    pub(crate) fn decode(b: &[u8]) -> Result<Self> {
        if b.len() < 46
            || ModeInfoType::from_code(b[0]) != ModeInfoType::Name
            || ModeInfoType::from_code(b[23]) != ModeInfoType::Parameters
            || b[1] > 20
            || b[24] > 20
        {
            return Err(Error::Malformed("mode information"));
        }
        Ok(Self {
            name: String::from_utf8_lossy(&b[2..2 + usize::from(b[1])])
                .trim_end_matches('\0')
                .into(),
            parameters: String::from_utf8_lossy(&b[25..25 + usize::from(b[24])])
                .trim_end_matches('\0')
                .into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ActionPhotoSize, FrameRate, Osmo360PhotoSize, VideoResolution};

    #[test]
    fn status_interprets_fields_by_mode_and_model() {
        let mut b = [0; 38];
        b[0] = CameraMode::Photo.code();
        b[1] = CameraState::Capturing.code();
        b[2] = 3;
        b[3] = 1;
        b[27] = 99;
        b[28] = PowerMode::Sleep.code();
        b[30] = TemperatureStatus::TooHotToRecord.code();
        b[31..35].copy_from_slice(&500u32.to_le_bytes());
        b[35..37].copy_from_slice(&65535u16.to_le_bytes());
        let action = CameraStatus::decode(&b, CameraModel::Action4).unwrap();
        let panorama = CameraStatus::decode(&b, CameraModel::Osmo360).unwrap();
        assert_eq!(
            action.resolution,
            CaptureFormat::ActionPhoto(ActionPhotoSize::Medium)
        );
        assert_eq!(
            panorama.resolution,
            CaptureFormat::Osmo360Photo(Osmo360PhotoSize::Wide20Mp)
        );
        assert_eq!(action.fps, CaptureRate::BurstCount(1));
        assert_eq!(action.photo_countdown, PhotoCountdown::HalfSecond);
        assert_eq!(action.loop_duration, LoopRecordingDuration::Max);
        assert_eq!(action.user_mode, UserMode::General);
        assert_eq!(action.power_mode, PowerMode::Sleep);
        assert!(action.is_capturing());

        b[0] = CameraMode::Video.code();
        b[2] = VideoResolution::K4Ratio16x9.code();
        let video = CameraStatus::decode(&b, CameraModel::Action4).unwrap();
        assert_eq!(
            video.resolution,
            CaptureFormat::Video(VideoResolution::K4Ratio16x9)
        );
        assert_eq!(video.fps, CaptureRate::Video(FrameRate::Fps24));
        b[0] = CameraMode::SlowMotion.code();
        assert_eq!(
            CameraStatus::decode(&b, CameraModel::Action4).unwrap().fps,
            CaptureRate::SlowMotionMultiplier(1)
        );

        b[0] = 0xe7;
        b[1] = 0xe8;
        let future = CameraStatus::decode(&b, CameraModel::Unknown(0xff77)).unwrap();
        assert_eq!(future.mode, CameraMode::Unknown(0xe7));
        assert_eq!(future.state, CameraState::Unknown(0xe8));
        assert_eq!(future.resolution.code(), b[2]);
        assert_eq!(future.fps.code(), b[3]);
        assert!(CameraStatus::decode(&b[..37], CameraModel::Action4).is_err());
    }
}
