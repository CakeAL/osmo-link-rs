# iOS 应用

## 功能

原生 SwiftUI 应用位于 [ios](../ios)，最低支持 iOS 16.0。iOS 26 起系统导航和拍摄按钮使用
Liquid Glass，旧系统使用原生控件和系统材质。首次搜索时授权蓝牙，选择相机后在相机端
确认配对码 `2333`。配对身份保存在应用沙盒中。搜索时实时显示相机，扫描期间即可手动连接。
开启「自动连接已配对相机」时，找到上次使用的相机就立即连接。如果 5 秒内没找到它，
才从发现的其他已配对相机里选择信号最强的一台。相机状态决定
主按钮是拍照、开始录像还是停止录像。设置页提供外观选项、实时活动开关、低电量提醒、
高温通知和系统权限入口。相机电量低于设置阈值时（默认 10%），应用会震动并发送通知；
相机温度升到「无法录制」或「即将关机」时也会震动并通知，可在设置页关闭。
相机页与发现页底部都会显示最近事件，记录连接、断开、重连、模式切换、拍照录像，
以及电量与温度提醒，按时间倒序保留最近 20 条。
在当前模式一栏上下滑动可展开带图标的滚轮，松手切换到中央高亮的模式。也可点开滚轮后选择；
模式行的拖动优先于页面滚动。

iOS 16.1 起，应用前台收到录像状态后可启动实时活动，显示拍摄模式、时长与电量。
支持灵动岛的 iPhone 显示紧凑及展开布局，锁定屏幕也会显示实时活动。
停止录像或断开连接会结束活动，点按活动可返回遥控页。应用在后台时能否及时收到
相机状态，取决于系统调度及蓝牙连接。

连接相机后，应用会自动请求「使用 App 期间」定位权限并开始手机 GPS 注入，
相机页的「定位注入」开关可随时关闭。iOS 的公开定位接口不提供参与定位的卫星数，
Osmo Link 将该字段固定发送为 `0` 表示不可用；坐标、时间、速度与精度仍来自
Core Location。相机页会明确提示当前卫星数只能注入 0。

## 侧载

1. 在 [GitHub Releases](https://github.com/CakeAL/osmo-link-rs/releases/latest) 下载 `*-ios-unsigned.ipa`。
2. 在 Mac 或 Windows 上打开 Sideloadly，用 USB 连接并解锁 iPhone，按提示信任电脑。
3. 将 IPA 拖进 Sideloadly，选择设备，用自己的 Apple 账号登录并安装。工具会为该设备签名。
4. 按提示在 iPhone 的「设置 → 隐私与安全性 → 开发者模式」启用开发者模式，并在「通用 → VPN 与设备管理」信任你的开发者身份。

免费 Apple 账号的开发描述文件有效期为 7 天，可以在到期前用同一账号续签。录制实时活动依赖
IPA 内的 `RecordingWidget`，侧载工具如果询问是否移除扩展，需要保留扩展才能使用该功能。
[苹果 Personal Team 说明](https://developer.apple.com/help/account/basics/about-your-developer-account) ·
[Sideloadly](https://sideloadly.io/) ·
[苹果开发者模式说明](https://developer.apple.com/documentation/xcode/enabling-developer-mode-on-a-device)

## 构建

用 Xcode 26 或更新版本打开 [OsmoLink.xcodeproj](../ios/OsmoLink.xcodeproj)，选择
`OsmoLink` scheme。真机运行需为应用及 `RecordingWidget` 设置同一个签名团队。
Xcode 构建阶段会编译并链接 `mobile-bridge` 的 Rust 静态库，同时生成 Swift 绑定。

```sh
rustup target add aarch64-apple-ios
open ios/OsmoLink.xcodeproj

# 在 Apple Silicon 上构建 iOS 模拟器版本
rustup target add aarch64-apple-ios-sim
```

下面的命令生成未签名的 arm64 真机 IPA，用于下载后侧载。

```sh
xcodebuild -project ios/OsmoLink.xcodeproj -scheme OsmoLink -configuration Release \
  -sdk iphoneos -derivedDataPath ios/build/DerivedData \
  CODE_SIGNING_ALLOWED=NO CODE_SIGNING_REQUIRED=NO build
mkdir -p ios/build/Payload
ditto ios/build/DerivedData/Build/Products/Release-iphoneos/OsmoLink.app \
  ios/build/Payload/OsmoLink.app
(cd ios/build && zip -qry OsmoLink-unsigned.ipa Payload)
```

Intel 模拟器构建使用 `x86_64-apple-ios` target。蓝牙配对、拍摄及灵动岛实机表现待验证。
