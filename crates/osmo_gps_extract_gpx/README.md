# osmo_gps_extract_gpx

从 DJI Osmo Action 4 / 5 / 6 的 MP4 中读取内嵌 GPS，写成 GPX。

DJI 把 GPS 和相机遥测放在 `djmd` timed metadata 轨里，内容是按帧写入的
protobuf `GenericMessage`。本 crate：

- 解析 MP4 的 `moov → trak → mdia → minf → stbl`，按 `stsd` 的 sample entry
  `djmd`（而不是轨道序号）选出元数据轨；
- 用 `stsc` + `stsz` + `stco`/`co64` 还原每个 sample 的偏移，读取全部样本；
- 用 `stts` 取每个样本在轨道时间轴上的位置，再用嵌入的 GPS 日期时间确定绝对
  时刻（对全部样本的偏移取中位数，个别写坏的时间戳不会移动整条轨迹）；
- 用 `prost` 按 DJI proto 解码 `GenericMessage.gps_info`；
- 用 `gpx` crate 写 GPX 1.1；
- 默认按 2 Hz 线性插值输出，也支持保留原始点或等间隔丢弃。

Action 4 / 5 / 6 的 GPS 字段编号一致，区别只是 `proto_name`
（`dvtm_ac203.proto` / `dvtm_ac204.proto` / `dvtm_ac206.proto`）。

## 用法

```sh
# 默认 2 Hz、线性插值
cargo run -p osmo_gps_extract_gpx -- input.MP4 output.gpx

# 保留全部原始样本（约视频帧率，例如 29.97 Hz）
cargo run -p osmo_gps_extract_gpx -- --resample none input.MP4 output.gpx

# 按帧率等间隔丢弃
cargo run -p osmo_gps_extract_gpx -- -r discard -f 1 input.MP4 output.gpx

# 嵌入时间减 8 小时（默认 0，即原样输出）
cargo run -p osmo_gps_extract_gpx -- -t 8 input.MP4 output.gpx
```

不传 `output.gpx` 时，输出与输入同名的 `.gpx`。

选项：

| 选项 | 说明 |
| --- | --- |
| `-o, --output <path>` | 输出 GPX 路径 |
| `-f, --frequency <Hz>` | 输出频率，默认 2 |
| `-r, --resample <method>` | `none` / `discard` / `linear`，默认 `linear` |
| `-t, --timezone-offset <h>` | 从嵌入时间中减去的小时数，默认 0 |
| `-h, --help` | 帮助 |
| `-V, --version` | 版本 |

## 作为库使用

```rust
use osmo_gps_extract_gpx::{
    ExtractOptions, ResampleMethod, extract_gps_with_options, resample, write_gpx_file,
};

fn main() -> Result<(), osmo_gps_extract_gpx::Error> {
    let extraction = extract_gps_with_options(
        "input.MP4",
        ExtractOptions { timezone_offset_hours: 0.0 },
    )?;
    let points = resample(
        &extraction.points,
        extraction.frame_rate,
        2.0,
        ResampleMethod::Linear,
    )?;
    write_gpx_file("output.gpx", &points, "DJI track")?;
    Ok(())
}
```

`Extraction` 还带有 `proto_name`、`camera_name`、`serial_number`、
`frame_rate` 和 `skipped_samples`，可用于校验机型或诊断。

## 关于依赖

- GPX 输出使用 [`gpx`](https://crates.io/crates/gpx) 0.10，不再手写 XML。
- protobuf 解码使用 [`prost`](https://crates.io/crates/prost) 0.14。
- 时间使用 `gpx` 依赖的 [`time`](https://crates.io/crates/time) 0.3。

MP4 容器层保留了一个很小的 box 解析器。原因是 `mp4` crate 0.14 在 DJI 的
`udta/meta/hdlr`（32 字节）上会报 `hdlr size too small`，无法直接读取这些文件；
而 `mp4parse` 是流式解析，会顺序读过 16 GB 的 `mdat`。这里只解析 `moov` 和
`stbl`，样本数据仍按偏移直接读取。

## 限制

- 目前验证的是 Action 6 的 `dvtm_ac206.proto`。Action 4/5 的字段路径与它一致，
  但没有在本仓库中做真机文件验证。
- 嵌入的 GPS 时间是整秒精度（`2026-11-05 02:06:01`），绝对时刻只能精确到 1 秒；
  轨迹内部的间隔来自 MP4 时间轴，是准确的。`--timezone-offset` 只是从这个绝对
  时刻中减去指定小时数，不做时区推断。
- DJI 偶尔会写出错误的嵌入时间戳（实测有一次 `12:06:12` 夹在 `02:06` 的一分钟
  里）。导出只会忽略它对绝对时刻的影响，该样本的位置仍然保留。
- `gps_altitude_mm` 是**有符号**值：低于椭球面时 DJI 用 10 字节 varint 写负数，
  按无符号读取会得到四百多万米的错误高程。
- `djmd` 轨的样本通常是 one-sample-per-chunk；实现按 `stsc` 处理多 sample/chunk，
  但这种情况没有真实文件覆盖。
- `gpx` 0.10 没有暴露 `<extensions>` 字段，因此没有输出加速度计/derivatives；
  这些值仍保留在库的 `GpsPoint` 里。
- 读取 16 GB 视频的 7 万多个分散 sample 在 SD 卡上可能需要一两分钟，属于随机读限制。
