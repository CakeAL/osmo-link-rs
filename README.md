# osmo-link-rs

最小 Rust Osmo 蓝牙遥控 core，以及用于 macOS 真机调试的 CLI。
一个 Cargo workspace，两个 crate：`osmo-core`、`macos-cli`。

## macOS 运行

```sh
cargo run -p macos-cli
```

打开相机无线连接；首次连接时在相机确认配对：

```text
scan
connect 0
version
wait 1
status
mode video
record start
wait 3
record stop
wait 2
status
disconnect
quit
```

也可以 `cargo run -p macos-cli -- --scan` 或
`cargo run -p macos-cli -- --connect OsmoAction4-B15B`。
`help` 列出命令；`watch on` 持续显示状态。
扫描默认 10 秒；`scan 20` 可延长到 20 秒（允许 1–60 秒）。
设备是否匹配由 DJI 厂商广播标记决定，不依赖设备名称或名称前缀。
未提供名称时显示 `Unnamed DJI camera`，这只是界面占位文字。
支持从标准输入管道逐行执行命令，`wait` 期间继续收状态、发送 GPS。

macOS 需允许运行 CLI 的终端使用蓝牙。独立应用需在 Info.plist 配置
`NSBluetoothAlwaysUsageDescription`。本项目已在当前 Mac 的 Codex 执行环境中
获得蓝牙访问并完成 Action 4 配对。
若扫描提示 adapter not ready，请检查运行 CLI 的终端（Terminal、iTerm、VS Code 等）
是否拥有蓝牙权限，以及主机蓝牙是否开启。若状态正常但为空，保持相机亮屏、
开启无线连接、断开其他遥控器/App 后尝试 `scan 20`。
`RUST_LOG=osmo_core=debug cargo run -p macos-cli` 会输出适配器状态及 DJI 候选广播。

配对身份与历史默认保存于：
`~/Library/Application Support/osmo-link-rs/controller.json`。
可用 `OSMO_STATE_DIR` 指定目录；保持该文件可以复用遥控器身份。
这只是 DJI 协议身份，不依赖读取或伪造 Mac 的蓝牙 MAC 地址。

## 功能

| 能力 | API / CLI |
| --- | --- |
| 发现相机、鉴权连接 | `Scanner`、`Camera::connect` / `scan`、`connect` |
| 版本、状态 | `version`、`status`、`statuses`、`events` / `version`、`status`、`watch` |
| 明确开始、停止拍录 | `start_recording`、`stop_recording` / `record start`、`record stop` |
| 拍录键、QS | `shutter`、`quick_switch` / `shutter`、`qs` |
| 任意公开按键事件及按下/松开 | `key_event`、`key_state` / `key ...` |
| 模式切换 | `set_mode(u8)` / `mode video`、`mode photo`、`mode 0x0a` 等 |
| GPS 单次、持续输入 | `send_gps`、`update_gps`、`stop_gps` / `gps once`、`gps replay`、`gps stop` |
| 电源模式、重启 | `set_power_mode`、`sleep`、`reboot` / `sleep`、`reboot` |
| 断开、连接状态 | `disconnect`、`connection_state`、`connections` / `disconnect` |

状态支持 `1D02` 的完整字段，以及 `1D06` 的新模式名称、参数。
模式值与其他功能受相机型号、固件支持范围限制；core 保留原始数值。
`shutter` 是按键切换；需要明确开始/停止时使用 `record start|stop`。
拍录命令遵循当前拍摄模式，要录像先切换到视频模式。

电源命令 `001A` 的 `3` 是休眠，公开协议没有独立的彻底断电命令。
休眠请求发出后暂停写入，成功应答或结果不确定时均保持暂停；休眠后不自动重连。
广播唤醒需要 DJI 指定的厂商广播，`btleplug` 没有广播发送能力；
`wake`、完整 `snapshot` 流程会明确返回 `Unsupported`。
Apple 公开 CoreBluetooth 广播接口只支持名称和服务 UUID，后续移动端也不能
直接假定厂商广播可用。原始 Snapshot 按键编码已支持，但不等于完成唤醒流程。

意外断开最多重连两次并恢复状态订阅，配对被拒绝即停止。
控制命令超时不自动重发，避免重复拍录；GPS 需重连后的新输入才恢复。
调用 `disconnect().await` 等待清理；丢弃 `Camera` 也会通知后台任务断开。

## GPS

默认向相机每 **100ms** 发送最近一次有效定位，即 **10Hz**。
使用 `ConnectOptions::gps_interval` 可配置 100ms 到 1s。
采样频率、BLE 发送频率、视频最终存储频率是三个不同概念。
本库不采集定位、不插值，也不伪造新的采样时间；定位源由调用方提供。
两个输入更新之间最多复用最近定位 2 秒，之后暂停发送。

手机系统定位不保证每秒产生 10 个新定位。Android 的请求间隔只是期望值；
iOS 标准定位接口不能指定保证 10Hz。原生定位接入时还需处理权限、后台生命周期、
速度分量、精度以及卫星数等字段的来源，不能编造缺失字段。

CLI 支持 JSONL，每行一个 `GpsSample`，时间戳为 UTC Unix 毫秒：

```json
{"timestamp_ms":1793361600000,"longitude":113.5,"latitude":22.5,"altitude_m":20.0,"north_mps":1.0,"east_mps":0.0,"down_mps":0.0,"vertical_accuracy_m":2.0,"horizontal_accuracy_m":3.0,"speed_accuracy_mps":0.2,"satellites":12}
```

上例只用于说明格式。`examples/dji-gps.jsonl` 则转换自 DJI 官方的历史 GPS
测试帧，包含六个真实采样值，仍仅用于传输测试；它不是本机实时定位。

```text
gps replay examples/dji-gps.jsonl
wait 1
record start
wait 3
record stop
gps stop
```

回放保留原始采样时间与间隔；连接 worker 独立按 10Hz 发送最新样本。
经纬度转换为度 × 10⁷，高度/精度为毫米，速度为厘米/秒，日期时间按协议 UTC+8
转换并处理跨日。拒绝非法坐标、NaN、超出编码范围以及无卫星的输入。

发送成功表示 BLE 写入完成，GPS 无强制应答；并不证明视频已写入 GPS。
视频中的 GPS 采样频率及 Mimo 仪表盘效果需读取录制文件另行验证。

## core 接入

```rust,no_run
use std::time::Duration;
use osmo_core::{Camera, ConnectOptions, ControllerIdentity, Scanner};

# async fn example() -> osmo_core::Result<()> {
let scanner = Scanner::new().await?;
let devices = scanner.scan(Duration::from_secs(5)).await?;
// 身份由宿主生成一次并保存，后续复用。
let identity = ControllerIdentity::default();
if let Some(device) = devices.first() {
    let camera = Camera::connect(device, ConnectOptions::new(identity)).await?;
    camera.subscribe_status(3).await?;
    let _events = camera.events();
    camera.set_mode(1).await?;
    camera.start_recording().await?;
    // 定位服务回调：camera.update_gps(&sample)?;
    camera.stop_recording().await?;
    camera.disconnect().await?;
}
# Ok(())
# }
```

core 使用宿主的 Tokio runtime。单个 worker 负责收发、CRC 和序列号应答关联。
协议流解析支持拆包、多帧、CRC 错误后恢复；未公开/未知消息通过 `Event::Frame`
保留。`Event::Warning` 报告不能解析的状态。

## Android / iOS

core 没有 macOS 专属业务代码，BLE 后端由 btleplug 选择。本阶段不包含手机 App
或 Swift/Kotlin FFI；移动端应用仍需原生外壳。

- iOS：原生应用负责蓝牙和定位权限，链接 Rust 库并提供 Tokio 生命周期。
- Android：原生应用负责 Nearby devices/定位等所需权限；先按 btleplug 0.13.5
  的 Java/JNI 初始化要求调用 `osmo_core::init_android`，再启动扫描。
- 安装对应 Rust target 后可做 core 编译检查，例如
  `cargo check -p osmo-core --target aarch64-apple-ios`、
  `cargo check -p osmo-core --target aarch64-linux-android`。
  这些命令不代表已经生成可安装 App。

移动端编译/运行结果详见 [验证记录](docs/macos-validation.md)，不以 macOS 测试
代替手机验证。[btleplug 平台接入文档](https://docs.rs/btleplug/0.13.5/btleplug/)。

## 开发检查

```sh
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

仅保留三个针对性测试：官方协议帧、GPS 单位/日期转换、模式切换应答兼容。
CLI 默认显示主动控制命令发送的 DATA 数据段，以及按序列号和命令号匹配到的应答 DATA：

```text
TX DATA=1D 04 33 FF 00 00 05 00 00 00 00
RX DATA=1D 03 00
```

DATA 为 `CmdSet + CmdID + Payload`，不含帧头、长度、保留字段、CRC。
后台状态推送、自动状态订阅、定时 GPS 推送及无关应答不显示。
`gps once` 显示单次发送的数据；它不要求应答。`scan`、`status` 等本地操作不会伪造协议帧。
上例展示实测的模式切换应答差异，保留相机实际返回的命令号。
所有型号的 `1D04` 请求均接受相同序列号的 `1D04` 或 `1D03` 应答；
返回码仍独立检查，`00` 为成功，非零报告错误。
TX 在尝试写入前记录，发送失败仍会报告错误。

```sh
cargo run -p macos-cli                              # 命令 DATA 显示
RUST_LOG=warn cargo run -p macos-cli                # 关闭 DATA 显示
RUST_LOG=osmo_core=debug cargo run -p macos-cli      # 加上扫描诊断
cargo run -p macos-cli 2>commands.log               # 保存命令 DATA 日志
```

协议来源：[DJI 官方示例](https://github.com/dji-sdk/Osmo-GPS-Controller-Demo)。
参考修订固定为 `92fe23e5a749f189593f980a26a105c3bb66aa1c`；
上游许可与测试数据归属见 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。
分步提交记录见 [PROGRESS.md](PROGRESS.md)。

协议：
https://github.com/dji-sdk/Osmo-GPS-Controller-Demo/blob/main/docs/protocol_data_segment_CN.md
