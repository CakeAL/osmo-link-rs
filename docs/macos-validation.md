# macOS 验证记录

日期：2026-10-30（Asia/Shanghai）。
主机：Apple Silicon，macOS 27.2，Rust 1.99.1。
设备：Osmo Action 4，名称 OsmoAction4-B15B。
版本应答：`DJI-ACTION4` / `SDK-v1.1 DEBUG AC203-03.04.80.15`。
鉴权返回的相机 ID：`0x0000ff33`。

## 已执行

| 项目 | 结果 |
| --- | --- |
| BLE 扫描 | 发现目标 Action 4 |
| 首次 DJI 配对 | 相机确认后完成双向鉴权 |
| 再次连接 | 使用已保存身份，无需重新确认 |
| 版本查询 | 成功读取上述版本 |
| 状态订阅 | 收到模式、拍录状态、剩余容量、电量等 1D02 数据 |
| 视频录制起停 | 收到成功 ACK，状态实际变化 `mode=01, state=01 → 03 → 01` |
| 拍照模式与快门 | 成功 ACK；剩余照片计数减少 |
| 切回视频模式 | 成功 ACK，状态确认 `mode=01` |
| GPS 持续传输 | 使用 DJI 官方历史样本回放；验证日志内共 146 个 GPS TX 包 |
| GPS 与录像并行 | GPS 回放期间成功录制并停止，状态确认返回待机 |
| GPS 发送节奏 | 同一回放段内相邻间隔中位数 99.943ms，均值 100.704ms；这是 host 发送日志，不是空口抓包 |
| 休眠 | `001A` 返回成功；休眠请求后 TX 包数量为 0 |
| 休眠后再查询版本 | core 返回 Sleeping，未写入 BLE |
| 正常退出 | 等待 BLE 断开清理后 CLI 退出 |

首次无人确认时相机返回了拒绝，程序断开且未自动重复配对。
用户确认准备后再次扫描连接成功。

一次在拍照仍处于忙碌状态时切换视频，相机返回 `0xE4`，core 正确报告错误；
等待相机回到待机后切换成功。命令 ACK 成功不等于相机立即完成状态转换。

## Action 4 模式应答差异

该行为经过独立复测，发送的完整帧为：

```text
AA 1B 00 02 00 00 00 00 03 00 67 4E
1D 04 33 FF 00 00 05 00 00 00 00 DB A7 82 E0
```

发送：SEQ=3，CmdSet=1D，CmdID=04，mode=05（拍照）。
接收解码：SEQ=3，CmdSet=1D，CmdID=03，CmdType=20，payload=00。
随后收到的状态显示 `mode=05`。

当前按用户要求，所有型号的 `1D04` 请求均可由相同序列号的 `1D04` 或 `1D03`
ACK 完成；其他命令仍严格匹配，返回码单独校验。
这里的真机证据来自这台 Action 4，通用兼容规则不代表其他型号已经过真机验证。

## 未验证的范围

- 未读取相机视频文件，未确认内嵌 GPS 采样频率、内容或 Mimo 仪表盘效果。
- GPS 为上游历史测试样本，非当前 Mac 的实时定位；发送完成不证明已被写入素材。
- 未强制断线或做长时间压力测试；重连有两次尝试上限，但未做故障注入验证。
- QS、任意多击/按下松开、重启及新型号的 1D06 已实现，未逐项真机触发。
- `wake`、完整 `snapshot` 依赖的厂商广播不受当前后端支持，明确返回 Unsupported。
- 本文只记录 macOS 测试。Android 固定配对码、默认定位注入和通知的真机验证范围见 [Android 文档](android.md#已验证与未验证)。
- iOS 的扫描、配对、拍摄、后台连接、灵动岛表现和手机 GPS 注入（卫星数固定 0）尚待 iPhone 真机验证。构建与系统版本要求见 [iOS 说明](../README.md#ios)。

## 编译与基础检查

- `cargo fmt --all --check`：通过。
- `cargo test --workspace`：六个针对性测试通过。
- `cargo clippy --workspace --all-targets -- -D warnings`：通过。

- `cargo check -p osmo-core --target aarch64-apple-ios`：通过。
- `cargo check -p osmo-core --target aarch64-linux-android`：通过。

移动端结果是 Rust core 的类型与依赖编译检查，未链接成原生 App，未做真机运行。
