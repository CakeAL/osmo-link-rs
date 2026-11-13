use std::path::PathBuf;

use osmo_gps_extract_gpx::{
    ExtractOptions, ResampleMethod, Result, extract_gps_with_options, resample, write_gpx_file,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let Some(args) = parse_args()? else {
        return Ok(());
    };

    let extraction = extract_gps_with_options(
        &args.input,
        ExtractOptions {
            timezone_offset_hours: args.timezone_offset,
        },
    )?;
    let raw_count = extraction.points.len();
    let points = resample(&extraction.points, args.frequency, args.method)?;

    let name = args
        .input
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("DJI GPS");
    write_gpx_file(&args.output, &points, name)?;

    println!(
        "相机: {} ({})",
        text_or_unknown(&extraction.camera_name),
        text_or_unknown(&extraction.proto_name)
    );
    if !extraction.serial_number.is_empty() {
        println!("序列号: {}", extraction.serial_number);
    }
    if let Some(frame_rate) = extraction.frame_rate {
        println!("帧率: {frame_rate:.2} Hz");
    }
    println!(
        "GPS: {} 个原始样本 -> {} 个输出点（{}）",
        raw_count,
        points.len(),
        args.method
    );
    if extraction.skipped_samples > 0 {
        println!("跳过: {} 个无效样本/点", extraction.skipped_samples);
    }
    println!("已写入: {}", args.output.display());

    if !extraction.is_supported_model() {
        eprintln!(
            "warning: {} 不在已验证的 Action 4/5/6 proto 列表中，结果可能不完整",
            text_or_unknown(&extraction.proto_name)
        );
    }

    Ok(())
}

struct Args {
    input: PathBuf,
    output: PathBuf,
    frequency: f64,
    method: ResampleMethod,
    timezone_offset: f64,
}

fn parse_args() -> Result<Option<Args>> {
    let mut arguments = std::env::args().skip(1);
    let mut input: Option<PathBuf> = None;
    let mut positional_output: Option<PathBuf> = None;
    let mut output_option: Option<PathBuf> = None;
    let mut frequency = 2.0;
    let mut method = ResampleMethod::None;
    let mut timezone_offset = 0.0;
    let mut positional_only = false;

    while let Some(argument) = arguments.next() {
        if !positional_only {
            match argument.as_str() {
                "-h" | "--help" => {
                    print_help();
                    return Ok(None);
                }
                "-V" | "--version" => {
                    println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
                    return Ok(None);
                }
                "-o" | "--output" => {
                    output_option = Some(PathBuf::from(next_value(&mut arguments, "--output")?));
                    continue;
                }
                "-f" | "--frequency" => {
                    frequency =
                        parse_f64(&next_value(&mut arguments, "--frequency")?, "--frequency")?;
                    continue;
                }
                "-r" | "--resample" => {
                    method = next_value(&mut arguments, "--resample")?.parse()?;
                    continue;
                }
                "-t" | "--timezone-offset" => {
                    timezone_offset = parse_f64(
                        &next_value(&mut arguments, "--timezone-offset")?,
                        "--timezone-offset",
                    )?;
                    continue;
                }
                "--" => {
                    positional_only = true;
                    continue;
                }
                value if value.starts_with('-') => {
                    return Err(osmo_gps_extract_gpx::Error::Argument(format!(
                        "unknown option {value:?}"
                    )));
                }
                _ => {}
            }
        }

        if input.is_none() {
            input = Some(PathBuf::from(argument));
        } else if positional_output.is_none() {
            positional_output = Some(PathBuf::from(argument));
        } else {
            return Err(osmo_gps_extract_gpx::Error::Argument(
                "too many positional arguments".to_owned(),
            ));
        }
    }

    let Some(input) = input else {
        print_help();
        return Err(osmo_gps_extract_gpx::Error::Argument(
            "missing input MP4 file".to_owned(),
        ));
    };

    if method != ResampleMethod::None && (!frequency.is_finite() || frequency <= 0.0) {
        return Err(osmo_gps_extract_gpx::Error::Argument(
            "--frequency must be a positive finite number".to_owned(),
        ));
    }
    if !timezone_offset.is_finite() {
        return Err(osmo_gps_extract_gpx::Error::Argument(
            "--timezone-offset must be a finite number".to_owned(),
        ));
    }

    let output = match (output_option, positional_output) {
        (Some(_), Some(_)) => {
            return Err(osmo_gps_extract_gpx::Error::Argument(
                "output was specified twice".to_owned(),
            ));
        }
        (Some(output), None) => output,
        (None, Some(output)) => output,
        (None, None) => {
            let mut output = input.clone();
            output.set_extension("gpx");
            output
        }
    };

    Ok(Some(Args {
        input,
        output,
        frequency,
        method,
        timezone_offset,
    }))
}

fn next_value(arguments: &mut impl Iterator<Item = String>, option: &str) -> Result<String> {
    arguments
        .next()
        .ok_or_else(|| osmo_gps_extract_gpx::Error::Argument(format!("{option} requires a value")))
}

fn parse_f64(value: &str, option: &str) -> Result<f64> {
    value.parse::<f64>().map_err(|_| {
        osmo_gps_extract_gpx::Error::Argument(format!("{option} expects a number, got {value:?}"))
    })
}

fn text_or_unknown(value: &str) -> &str {
    if value.is_empty() { "未知" } else { value }
}

fn print_help() {
    println!(
        "\
osmo_gps_extract_gpx - 从 DJI Osmo Action 4/5/6 的 MP4 提取 GPS 并写 GPX

用法:
  osmo_gps_extract_gpx <input.mp4> [output.gpx] [选项]

选项:
  -o, --output <path>          输出 GPX 路径（默认与输入同名的 .gpx）
  -f, --frequency <Hz>         discard / linear 的输出频率，默认 2 Hz
  -r, --resample <method>      none | discard | linear（默认 none）
  -t, --timezone-offset <h>    从嵌入时间中减去的小时数，默认 0
  -h, --help                   显示帮助
  -V, --version                显示版本

说明:
  读取第 1 条 sample entry 为 `djmd` 的轨道，按 DJI protobuf 解析
  GenericMessage.gps_info，按 stsc/stsz/stco|co64 定位样本，样本时间取
  stts 时间轴。嵌入的 GPS 日期时间只用来确定轨迹的绝对时刻，因此个别
  写坏的时间戳不会中断导出。嵌入时间按 UTC 原样解释；
  `--timezone-offset 8` 会再减 8 小时。
  相机对每一帧都写一条 gps_info，没有新定位时重复上一条，所以相邻的
  相同定位会被合并成一个点：默认输出即每个真实定位（手机 GNSS 通常
  1 Hz），不会出现重复坐标。需要更高频率时用 `-r linear -f 2` 插值，
  或用 `-r discard -f 1` 按时间抽样。
"
    );
}
