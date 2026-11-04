# Android 应用

原生 Kotlin + Jetpack Compose 遥控界面，通过 JNI 调用同一个 `osmo-core`。
界面为单页深色 HUD：未连接时是扫描页，连接后同一屏变为遥控台。

## 结构

| 位置 | 作用 |
| --- | --- |
| [crates/android-bridge](../crates/android-bridge) | JNI cdylib（`libosmo_bridge.so`）：扫描、连接、命令、事件、GPS |
| [android/app](../android/app) | Compose UI、权限、前台服务；状态在 `OsmoRepository`，GPS 在 `GpsTracker` |
| [OsmoRepository.kt](../android/app/src/main/java/com/cakeal/osmolink/data/OsmoRepository.kt) | 进程级状态、GPS、前台服务启停 |
| [OsmoService.kt](../android/app/src/main/java/com/cakeal/osmolink/service/OsmoService.kt) | 前台服务与常驻通知 |
| [android/btleplug-jni](../android/btleplug-jni) | 内置的 btleplug 0.13.5 Java 支持类（上游不发布 AAR） |
| [scripts/build-android.sh](../scripts/build-android.sh) | cargo-ndk 构建脚本，被 Gradle `preBuild` 调用 |

数据流：Kotlin `OsmoNative` 调 JNI → Rust 在 Tokio runtime 上操作相机 →
`BridgeListener.onEvent(type, json)` 回到 Kotlin → `OsmoRepository` 合并成
`StateFlow<UiState>` → Compose 与通知渲染。所有命令立即返回，结果以事件形式回来。

事件类型：`modes`、`devices`、`pairing`、`connected`、`connectFailed`、`status`、
`modeInfo`、`version`、`command`、`warning`、`error`、`gpsError`、`reconnecting`、
`reconnected`、`disconnected`。

## 构建

前置：Rust 的 `aarch64-linux-android` target、Android NDK 29、cargo-ndk、JDK 17+。

```sh
rustup target add aarch64-linux-android
cargo install cargo-ndk
cd android
./gradlew :app:assembleDebug
```

Gradle 在 `preBuild` 执行 `scripts/build-android.sh`，把
`libosmo_bridge.so` 产出到 `app/src/main/jniLibs/arm64-v8a/`，随后正常打包。
只改 Kotlin 时可跳过 Rust：

```sh
./gradlew :app:assembleDebug -PskipRust=true
```

NDK 查找顺序：`ANDROID_NDK_HOME` → 仓库内 `.tools/android-ndk`。
cargo-ndk 查找顺序：`.tools/bin/cargo-ndk` → `PATH`。
`.tools/` 与 `jniLibs/` 都是构建产物，不提交。

安装与运行：

```sh
adb install -r app/build/outputs/apk/debug/app-debug.apk
adb shell am start -n com.cakeal.osmolink/.MainActivity
```

## 后台保活

连接成功或开启定位后，`OsmoRepository` 启动 `OsmoService`；两者都关闭时停止。
服务是前台服务（类型 `connectedDevice|location`），常驻通知显示当前模式、录制时长、
电量、剩余容量/时长与定位频率，并提供「断开」「停止定位」动作。

状态与 GPS 属于进程级 `OsmoRepository`，不跟随 Activity 生命周期，因此退出界面后
连接和定位仍保持。进程被系统杀死后服务不会自动重建（`START_NOT_STICKY`），因为此时
BLE 连接已经丢失。

录制中在 Android 16+ 会申请 Live Update（`Notification.ProgressStyle`、
`setRequestPromotedOngoing`、短文本 `REC`），系统以状态栏胶囊/锁屏卡片呈现；
用户可在系统设置关闭提升，`canPostPromotedNotifications()` 为 false 时自动退回普通通知。
录制时长使用原生 chronometer，不依赖状态推送频率。

Android 12+ 不允许从后台启动前台服务，所以服务只在用户于界面内连接/开启定位时启动。
Android 13+ 首次连接后会请求通知权限；拒绝只会隐藏通知，服务仍运行。

小米/HyperOS 还需在系统设置中允许自启动、省电策略设为「无限制」，并锁定最近任务卡片。

## 首次使用

1. 授予「附近设备」与「位置信息」权限（Android 12+ 为运行时权限）。
2. 唤醒相机、开启无线连接，断开其它遥控器 / Mimo App。
3. 点「搜索相机」，选择设备；首次配对在相机屏幕确认 4 位配对码。
4. 连接后选择模式、按 REC 启停录制，或按快门拍照。
5. 打开「定位注入」后，手机定位按 100ms 目标间隔发送给相机。

GPS 只使用系统 `LocationManager`（无 Play Services 依赖），采样率取决于设备；
界面显示实际收到的频率，不假装固定 10Hz。没有卫星数或定位无效的样本会被丢弃。

## 权限与平台约束

- 前台服务：`FOREGROUND_SERVICE`、`FOREGROUND_SERVICE_CONNECTED_DEVICE`、
  `FOREGROUND_SERVICE_LOCATION`、`POST_NOTIFICATIONS`；服务类型在 manifest 固定为
  `connectedDevice|location`，Android 14+ 必须与 `startForeground` 参数一致。
- Live Update：Android 16+ 需要 `POST_PROMOTED_NOTIFICATIONS`，并先用
  `canPostPromotedNotifications()` 判断用户是否允许。
- 扫描：API 31+ 需要 `BLUETOOTH_SCAN` / `BLUETOOTH_CONNECT`；旧版本需要定位权限。
  本应用始终同时请求定位，因为 GPS 注入本身需要它。
- **不要预检 adapter 状态**：btleplug 的 Android 后端把 `adapter_state()` 固定返回
  `Unknown`，预检会把所有设备误判为不可用。现在直接调用 `start_scan`，并由它把
  「蓝牙未开」「权限被拒」映射成明确错误。
- `btleplug::platform::init` 必须在 Java 线程调用，因此放在 `nativeInit` 中。
- JNI crate 必须与 btleplug 使用同一个 jni 0.22；JNI 符号与 Kotlin `external fun` 一一对应。
- release 开 R8：`src/main/keepRules/rules.keep` 保留 `com.nonpolynomial.**`、
  `io.github.gedgygedgy.**`、`OsmoNative` 和 `BridgeListener` 实现。
- `btleplug` 的 Android 后端要求 minSdk ≥ 24；本工程 minSdk 29。

## 已验证与未验证

已验证（arm64 API 37 模拟器）：

- debug APK 构建、安装、启动；
- 扫描页渲染、扫描中状态、失败后恢复；
- release 构建通过 R8，btleplug 类与原生库仍在 APK 内；
- 暂停在模拟器无可用 BLE 适配器时给出 adapter-not-ready 错误；
- 前台服务启动成功：`dumpsys` 显示 `isForeground=true`、`types=0x18`
  （connectedDevice|location），通知标题/正文符合预期。
- Android 16 Live Update 路径：API 37 模拟器上通知带 `PROMOTED_ONGOING`、
  `ProgressStyle` 模板与 `shortCriticalText=REC`。

未验证（需要真机 + 相机）：

- 真机扫描、配对、录制、模式切换、状态推送的端到端行为；
- GPS 是否真正写入视频素材（发送完成不等于落盘）；
- 真机长时间后台、Doze 与厂商省电策略下的存活；
- 真实连接状态下的通知刷新（模拟器没有 BLE，验证时使用临时演示状态）；
- 各厂商 ROM 对 Live Update 的展示差异（部分系统可能折叠或不显示胶囊）。

模拟器没有可用的 BLE 适配器，不能代替真机结论。

## 排错

| 现象 | 处理 |
| --- | --- |
| 扫描报 Bluetooth is off or unavailable | 打开系统蓝牙，确认「附近设备」权限；模拟器属预期 |
| 扫描报 permission denied | 设置 → 应用 → 权限 → 附近设备；HyperOS 还需允许定位 |
| 小米/HyperOS 搜不到设备 | 关闭省电限制、打开系统定位与蓝牙扫描开关，稍后重扫 |
| 常驻通知不显示 | 授予通知权限；HyperOS 还需允许自启动，否则服务可能被清理 |
| 找不到相机 | 相机亮屏、开无线连接、断开其它遥控器；稍后重扫 |
| 配对失败 | 在相机屏幕确认配对码；拒绝后需重新发起 |
| 连接报 `status: 128` / CCCD 失败 | Android 首次 CCCD 写入的常见错误；已自动重试并重连一次，仍失败可再点连接 |
| 录制命令失败 | 确认当前模式；相机忙碌时可能返回错误，等状态回待机再试 |
| 定位频率低 | 系统定位调度限制；室内/遮挡会掉星 |
| release 崩溃提示类缺失 | 检查 `rules.keep` 是否保留 btleplug 与 JNI 类 |
