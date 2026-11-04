# AGENTS.md

给 AI/自动化编码代理的项目须知。人类读者请先看 [README.md](README.md)。

## 项目是什么

`osmo-link-rs` 是一个最小化的 Rust 库，用公开的 DJI R SDK BLE 协议遥控 Osmo 相机
（Action 4 为已验证机型），外加 macOS 调试 CLI 和 Android 原生 App。

- 一个 Cargo workspace（resolver 3，edition 2024），三个 crate：
  - `osmo-core`（[crates/osmo-core](crates/osmo-core)）：协议、BLE、相机控制、GPS。**无平台专属业务代码。**
  - `macos-cli`（[crates/macos-cli](crates/macos-cli)）：命令行外壳、配对身份持久化、GPS 回放。
  - `android-bridge`（[crates/android-bridge](crates/android-bridge)）：JNI cdylib，向 Kotlin 暴露异步 JSON 事件。
- `android/`（[android](android)）是独立 Gradle 工程：`:app`（Compose HUD）、
  `:btleplug-jni`（内置的 btleplug Java 支持类）。原生库由 cargo-ndk 构建。

## 常用命令

```sh
cargo run -p macos-cli                 # 交互式 CLI；命令 DATA 默认打印
cargo run -p macos-cli -- --scan       # 只扫描后退出
cargo run -p macos-cli -- --connect OsmoAction4-B15B

cargo fmt --all                        # 格式化（无 rustfmt.toml，用默认规则）
cargo test --workspace                 # 6 个针对性测试，全部离线
cargo clippy --workspace --all-targets -- -D warnings
cargo check -p osmo-core --target aarch64-apple-ios
cargo check -p android-bridge --target aarch64-linux-android

cd android && ./gradlew :app:assembleDebug   # 先构建 Rust，再打包 APK
cd android && ./gradlew :app:assembleDebug -PskipRust=true   # 只改 Kotlin 时
```

`RUST_LOG` 默认过滤为 `warn,osmo_core::commands=info`；`RUST_LOG=warn` 关闭命令 DATA，
`RUST_LOG=osmo_core=debug` 打开扫描诊断。CLI 日志写 stderr，正常输出写 stdout。

## 代码地图

| 文件 | 职责 |
| --- | --- |
| [protocol.rs](crates/osmo-core/src/protocol.rs) | 帧编解码、CRC16/CRC32、`Decoder` 流式拆包/多帧/重同步 |
| [ble.rs](crates/osmo-core/src/ble.rs) | `Scanner` 扫描与 DJI 广播匹配、`Link` 连接/MTU 分片写/通知组装 |
| [camera.rs](crates/osmo-core/src/camera.rs) | 鉴权握手、单 worker 收发循环、`Camera` 公开 API、重连 |
| [status.rs](crates/osmo-core/src/status.rs) | `1D02` 状态、`1D06` 模式信息解码 |
| [enums.rs](crates/osmo-core/src/enums.rs) | `protocol_enum!` 宏及全部协议枚举、`CaptureFormat`/`CaptureRate` |
| [gps.rs](crates/osmo-core/src/gps.rs) | `GpsSample` 校验与 48 字节编码、UTC+8 转换 |
| [error.rs](crates/osmo-core/src/error.rs) | `Error`/`Result` |
| [macos-cli/src/main.rs](crates/macos-cli/src/main.rs) | 命令解析、事件循环、`controller.json`、JSONL 回放 |
| [android-bridge/src/lib.rs](crates/android-bridge/src/lib.rs) | JNI 入口、JVM 回调、扫描/连接/命令/GPS 转发 |
| [android-bridge/src/status.rs](crates/android-bridge/src/status.rs) | `CameraStatus` 的 JSON 投影与模式目录 |
| [android/app](android/app/src/main/java/com/cakeal/osmolink) | Compose UI、`OsmoRepository`、前台服务、权限、`GpsTracker` |
| [docs/macos-validation.md](docs/macos-validation.md) | 真机证据与**未**验证范围 |
| [PROGRESS.md](PROGRESS.md) | 按步骤追加的实现日志（英文） |

任务 → 改哪里：协议字段 → `protocol.rs`；扫描/连接问题 → `ble.rs`；命令语义与重连 →
`camera.rs`；状态字段含义 → `status.rs` + `enums.rs`；定位编码 → `gps.rs`；CLI 交互 →
`macos-cli/src/main.rs`；Android 界面 → `android/app`；JNI 接口 → `android-bridge`。

## 架构约束

**单 worker 独占 BLE。** `Camera` 只是句柄：命令经 `mpsc` 送入，由 worker 串行发送、
分配序列号、维护 `pending` 表、把 ACK 与状态推送分发到 `broadcast`/`watch` 通道。
不要在 `Camera` 上加直接写 BLE 的旁路，也不要引入第二个读写任务。

**核心库不自建 Tokio runtime。** 使用宿主的 runtime；公开 API 是 async，GPS 更新是同步的
`update_gps`/`stop_gps`。

**命令生命周期：**
- `command(cmd, payload, wait)`：`wait=false` 写入成功即返回；`wait=true` 等待同序列号 ACK，超时 3 秒。
- 超时**不重发**。重发会导致重复拍录，这是刻意的选择，不要"顺手"加重试。
- 重连时清空并失败所有 pending 请求（`Error::Disconnected`），绝不跨连接重放控制命令。
- 不确定的 sleep 结果同样暂停写入，直到重连。

**休眠语义：** `PowerMode::Sleep` 请求发出后立即停止后续写入；成功应答或结果不确定都保持
暂停；休眠后不自动重连。任何新增命令都必须尊重 `sleeping`/`sleep_requested` 拦截。

**重连：** 意外断开最多 2 次，重连成功后恢复状态订阅，`gps_valid_from` 重置（旧定位作废）；
相机拒绝配对立即放弃。改动这里要同步 [README.md](README.md) 的重连段落。

## 协议与硬件规则（易错，先读再改）

- 帧：`0xaa` 头，长度低 10 位，最小 18 / 最大 1023 字节；除帧头字段外**全部小端**。
- CRC16 与 CRC32 初值均为 `0x3aa3`；CRC16 覆盖前 10 字节，CRC32 覆盖除末尾 4 字节外的全部。
- 保留字节在官方样例中非零，解码只校验 `bytes[4] == 0` 与 `kind & 0xc0`，不要收紧成"保留全零"。
- 扫描靠厂商广播，不靠名字：`manufacturer_data[0x08aa][2] == 0xfa`。名字只用于显示，
  不要加名称前缀过滤。
- 服务/特征：service `fff0`，通知 `fff4`，写入 `fff5`（优先 `WRITE_WITHOUT_RESPONSE`）。
- 握手：`Kind=2` 的 `0019`，固定 33 字节载荷；`fw_version=0` 用于关闭配件升级；
  成功标志的 ACK 后仍需确认 `VerificationResult`，再回 `Kind=0x20` 确认帧。
- ACK 匹配：序列号必须精确相等；唯一的放宽是 `1D04`（切模式）接受 `1D03` 应答，
  对所有机型生效。返回码必须单独检查，`00` 才算成功。放宽前需要真机证据。
- 状态推送解码失败不得让 worker 退出：走 `Event::Warning`，未知命令走 `Event::Frame`。
- `1D02` 至少 38 字节；分辨率字节与帧率字节的含义随模式和机型变化，由
  `CaptureFormat`/`CaptureRate` 解释，未知模式/机型保留原始编码。
- 不要发明协议值。`Osmo Nano` 在本协议文档中没有受支持的 ID，故意缺失。
- GPS：48 字节；经纬度 ×10⁷、高度/精度毫米、速度 cm/s；日期时间用 **UTC+8** 并处理跨日；
  拒绝 NaN、越界、以及 `satellites == 0` 的定位。发送成功只代表 BLE 写入，不代表视频已记录。
- GPS 节奏：默认 100ms（10Hz），允许 100ms..1s；最新样本 2 秒后停止复用；
  不插值、不伪造采样时间。
- `wake`/`snapshot` 依赖厂商广播，btleplug 无此能力，必须返回 `Error::Unsupported`；
  不要用可用的近似方案"假装"实现。

## 编码规范

- 格式由 `cargo fmt` 决定；提交前必须 `cargo fmt --all --check` 通过。
- `cargo clippy --workspace --all-targets -- -D warnings` 必须零告警。禁止 `#[allow(...)]`
  掩盖新告警；确实必要时要写明原因。
- 新增协议枚举一律用 `protocol_enum!`，自动获得 `from_code`/`code`/`checked_code`/`Display`/`From`。
  接收未知值保留 `Unknown(code)`；**发送**设置前调用 `checked_code()`。
- 数值转换保持显式：需要单位时用方法（如 `FrameRate::frames_per_second()`），不要把协议索引
  当物理值；索引 19 才是 200fps。
- 错误用 `Error` + `thiserror`；IO/平台错误向上传递，不吞掉。不要为了"跑通"而 `unwrap()`
  掉可能失败的路径（现有 `unwrap()` 都在已校验长度的位置或测试里）。
- 函数保持单职责、模块内私有优先；公开 API 只在 `lib.rs` 重导出。
- 注释写**为什么**，不写复述代码的废话；Rust 代码注释用英文，面向用户的文档用中文。
- 日志：用户可见的协议数据走 `tracing` target `osmo_core::commands`（`TX/RX DATA=` 大写十六进制，
  只含 CmdSet+CmdID+payload）。后台状态推送、订阅、周期 GPS 不打印。

## 验证与文档纪律

- 本地可验证：fmt、6 个单元测试、clippy、两个交叉 target 的 `cargo check`、
  `./gradlew :app:assembleDebug`（含 R8 的 release 也应能构建）。
- 硬件相关（扫描、配对、录制、休眠、GPS 落盘）**必须真机验证**；不跑硬件就不要在文档里
  写"已验证"。新增未验证能力时，同步更新 [docs/macos-validation.md](docs/macos-validation.md)
  的"未验证的范围"。
- 只把宿主发送日志当作发送节奏证据，不当作空口抓包或"视频已写入 GPS"的证据。
- 文档改动要同步：行为变化 → README；实现步骤 → PROGRESS.md（英文，按步骤追加，不重写历史）；
  公开 API 变化 → README 的 `core 接入`/枚举章节。
- 上游协议来源固定为 DJI Osmo-GPS-Controller-Demo 修订
  `92fe23e5a749f189593f980a26a105c3bb66aa1c`；引用样例字节或测试数据时保持
  [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) 的归属准确。

## 安全与副作用

- `record start/stop`、`mode`、`sleep`、`reboot`、`shutter` 都会改变真实相机的物理状态。
  未经用户明确要求，不要自动执行真机控制命令；调试时优先只读操作（`scan`、`version`、`status`）。
- 不读取、不伪造宿主蓝牙 MAC；控制器身份是应用生成的 DJI 协议身份，
  默认存于 `~/Library/Application Support/osmo-link-rs/controller.json`（`OSMO_STATE_DIR` 可覆盖）。
  写入用临时文件 + rename 保持原子性。
- 不提交生成物：`target/`、`android/app/build/`、`android/app/src/main/jniLibs/`、
  `.tools/`、`android/local.properties`、`.DS_Store`。
  `android/.idea/` 目前被跟踪，改动机器相关文件时不要顺带提交无关差异。

## Android

- Gradle 工程在 [android](android)，**不在** Cargo workspace 内；不会随 `cargo test` 构建。
- 关键版本：AGP 9.5.0、Kotlin 2.2.10、Compose BOM 2026.02.01、minSdk 29、target 37，
  namespace/applicationId 为 `com.cakeal.osmolink`。
- 模块：`:app`（UI/ViewModel/GPS/权限）、`:btleplug-jni`（上游 Java 类，**不要修改**，
  升级时随 btleplug 版本整体替换）。
- 原生库：`crates/android-bridge` 生成 `libosmo_bridge.so`；Gradle `preBuild` 调
  [scripts/build-android.sh](scripts/build-android.sh)。NDK 默认 `.tools/android-ndk`，
  可用 `ANDROID_NDK_HOME` 覆盖；`-PskipRust=true` 跳过 Rust。
- JNI 约定：Kotlin [OsmoNative.kt](android/app/src/main/java/com/cakeal/osmolink/bridge/OsmoNative.kt)
  的 `external fun` 必须与 `Java_com_cakeal_osmolink_bridge_OsmoNative_*` 一一对应；
  `jni` crate 版本必须与 btleplug 一致（0.22），否则 `Env` 类型不兼容。
- `btleplug::platform::init` 必须在 Java 线程调用（现位于 `nativeInit`）；
  它依赖内置的 `com.nonpolynomial.**` 与 `io.github.gedgygedgy.**` 类。
- 事件协议：`onEvent(type, json)`。新增事件类型时同步 `OsmoRepository.onEvent`，
  未识别类型会被静默忽略。
- release 启用 R8，`app/src/main/keepRules/rules.keep` 负责保留 btleplug 与 JNI 类；
  新增 JNI 类或回调接口时同步更新该文件。
- 状态与 GPS 属于 `OsmoRepository`（`data/`），不要搬回 ViewModel，否则退出界面会断连。
- `OsmoService` 在 `connected || gpsEnabled` 时前台运行；通知文案由纯函数
  `NotificationContent.kt` 生成，修改文案要同步 `NotificationContentTest`。
- manifest 的前台服务类型固定为 `connectedDevice|location`，改动需同步 `startForeground` 参数。
- 录制中在 Android 16+ 走 `Notification.ProgressStyle` + `setRequestPromotedOngoing`，
  需要 `POST_PROMOTED_NOTIFICATIONS` 与 `canPostPromotedNotifications()` 检查；
  旧系统或用户关闭提升时退回 compat 通知，不要假定一定会上岛/胶囊。
- 模拟器没有可用 BLE 适配器，只能验证 UI 与错误路径；扫描/配对/录制必须真机验证。
- 不要依赖 `Central::adapter_state()`：btleplug 的 Android 后端固定返回 `Unknown`，
  扫描前必须跳过该检查，让 `start_scan` 报告蓝牙关闭/权限错误。

## 提交与完成标准

- 提交信息用 Conventional Commits 前缀（`feat:`、`fix:`、`docs:`、`refactor:`），
  一个自洽步骤一个提交。
- 完成一项改动前依次确认：`cargo fmt --all --check`、`cargo test --workspace`、
  `cargo clippy --workspace --all-targets -- -D warnings` 全绿；涉及 API/行为时文档已同步；
  涉及硬件时明确标注"未真机验证"。
