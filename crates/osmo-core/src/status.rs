use crate::{Error, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CameraStatus {
    pub mode: u8,
    // 相机模式：0x00 - 慢动作，0x01 - 视频，0x02 - 静止延时，0x05 - 拍照，0x0A - 运动延时，0x1A - 直播，0x23 - UVC直播，0x28 - 低光视频（超级夜景），0x34 - 人物跟随，其它值 - 使用新协议（参考新相机状态推送1D06）
    pub state: u8,
    // 相机状态：0x00 - 屏幕关闭，0x01 - 直播（包括亮屏未录制），0x02 - 回放，0x03 - 拍照或录像中，0x05 - 预录制中
    pub resolution: u8,
    // 视频分辨率：10 - 1080P，16 - 4K 16:9，45 - 2.7K 16:9，66 - 1080P 9:16，67 - 2.7K 9:16，95 - 2.7K 4:3，103 - 4K 4:3，109 - 4K 9:16；拍照画幅（Osmo Action）：4 - L，3 - M；拍照画幅（Osmo 360）：4 - Ultra Wide 30MP，3 - Wide 20MP，2 - Standard 12MP
    pub fps: u8,
    // 帧率：1 - 24fps，2 - 25fps，3 - 30fps，4 - 48fps，5 - 50fps，6 - 60fps，10 - 100fps，7 - 120fps，19 - 200fps，8 - 240fps；慢动作模式时：倍率 = 帧率/30；拍照模式时：连拍数（1 - 普通拍照只拍一张，>1 - 连拍张数）
    pub eis: u8,
    // 电子防抖模式：0 - 关闭，1 - RS，2 - HS，3 - RS+，4 - HB
    pub record_seconds: u16,
    // 当前录像时间（包括预录制时长），单位：秒；连拍状态下：连拍时限，单位：毫秒
    pub fov: u8,
    // FOV类型，保留字段
    pub photo_ratio: u8,
    // 照片比例：0 - 4:3，1 - 16:9
    pub countdown_seconds: u16,
    // 实时倒计时，单位：秒
    pub timelapse_interval: u16,
    // 静止延时摄影模式下：拍摄时间间隔，单位：0.1秒（例如间隔0.5秒时值为5）；运动延时摄影模式下：拍摄速率（Auto选项下值为0）
    pub timelapse_duration: u16,
    // 延时录像时长，单位：秒
    pub remaining_mb: u32,
    // SD卡剩余容量，单位：MB
    pub remaining_photos: u32,
    // 剩余拍照张数
    pub remaining_seconds: u32,
    // 剩余录像时间，单位：秒
    pub user_mode: u8,
    // 用户模式（非法值按0处理）：0 - 通用模式，1 - 自定义模式1，2 - 自定义模式2，3 - 自定义模式3，4 - 自定义模式4，5 - 自定义模式5
    pub power_mode: u8,
    // 电源模式：0 - 正常工作模式，3 - 休眠模式
    pub next_mode: u8,
    // 预切换标志：在预切换（例如QS）模式下，仅显示模式名称和图标，不展示具体参数。表示即将切换的目标模式；若当前不处于预切换模式，则该字段表示当前模式，与camera_mode保持一致
    pub temperature_warning: u8,
    // 相机发生了错误：0 - 温度正常，1 - 温度警告（可以录制但温度比较高），2 - 温度高（不可以录制），3 - 温度过高（要关机了）
    pub photo_countdown_ms: u32,
    // 拍照倒计时参数，单位：毫秒（遥控器转换为0.5s，1s，2s，3s，5s，10s几个档显示）
    pub loop_seconds: u16,
    // 循环录像时长，单位：秒（遥控器转为off，max，5m，20m，1h几档显示，其中off=0，max=65535）
    pub battery_percent: u8,
    // 相机电池电量：0~100%
}

impl CameraStatus {
    pub(crate) fn decode(b: &[u8]) -> Result<Self> {
        if b.len() < 38 {
            return Err(Error::Malformed("camera status"));
        }
        let u16_at = |i| u16::from_le_bytes(b[i..i + 2].try_into().unwrap());
        let u32_at = |i| u32::from_le_bytes(b[i..i + 4].try_into().unwrap());
        Ok(Self {
            mode: b[0],
            state: b[1],
            resolution: b[2],
            fps: b[3],
            eis: b[4],
            record_seconds: u16_at(5),
            fov: b[7],
            photo_ratio: b[8],
            countdown_seconds: u16_at(9),
            timelapse_interval: u16_at(11),
            timelapse_duration: u16_at(13),
            remaining_mb: u32_at(15),
            remaining_photos: u32_at(19),
            remaining_seconds: u32_at(23),
            user_mode: b[27],
            power_mode: b[28],
            next_mode: b[29],
            temperature_warning: b[30],
            photo_countdown_ms: u32_at(31),
            loop_seconds: u16_at(35),
            battery_percent: b[37],
        })
    }

    pub fn is_capturing(&self) -> bool {
        self.state == 3
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModeInfo {
    pub name: String,
    pub parameters: String,
}

impl ModeInfo {
    pub(crate) fn decode(b: &[u8]) -> Result<Self> {
        if b.len() < 46 || b[0] != 1 || b[23] != 2 || b[1] > 20 || b[24] > 20 {
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
