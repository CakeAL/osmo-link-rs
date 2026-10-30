use osmo_core::{
    Camera, ConnectOptions, ControllerIdentity, Device, Event, GpsSample, Key, KeyEvent, Scanner,
};
use serde::{Deserialize, Serialize};
use std::{
    io::{self, BufRead, IsTerminal, Write},
    path::PathBuf,
    time::Duration,
};
use tokio::{
    sync::{broadcast, mpsc},
    time::{Instant, interval},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Default, Serialize, Deserialize)]
struct SavedState {
    identity: ControllerIdentity,
    paired: Vec<String>,
}
struct Replay {
    samples: Vec<GpsSample>,
    index: usize,
    started: Instant,
}
struct App {
    scanner: Scanner,
    devices: Vec<Device>,
    camera: Option<Camera>,
    events: Option<broadcast::Receiver<Event>>,
    saved: SavedState,
    state_path: PathBuf,
    replay: Option<Replay>,
    watch: bool,
}

impl App {
    async fn new() -> Result<Self> {
        let directory = match std::env::var_os("OSMO_STATE_DIR") {
            Some(path) => PathBuf::from(path),
            None => PathBuf::from(std::env::var_os("HOME").ok_or("set HOME or OSMO_STATE_DIR")?)
                .join("Library/Application Support/osmo-link-rs"),
        };
        std::fs::create_dir_all(&directory)?;
        let state_path = directory.join("controller.json");
        let saved = match std::fs::read(&state_path) {
            Ok(bytes) => serde_json::from_slice(&bytes)?,
            Err(e) if e.kind() == io::ErrorKind::NotFound => SavedState::default(),
            Err(e) => return Err(e.into()),
        };
        let app = Self {
            scanner: Scanner::new().await?,
            devices: vec![],
            camera: None,
            events: None,
            saved,
            state_path,
            replay: None,
            watch: false,
        };
        app.save()?;
        Ok(app)
    }
    fn save(&self) -> Result<()> {
        let temporary = self.state_path.with_extension("json.tmp");
        std::fs::write(&temporary, serde_json::to_vec_pretty(&self.saved)?)?;
        std::fs::rename(temporary, &self.state_path)?;
        Ok(())
    }
    fn camera(&self) -> Result<&Camera> {
        self.camera.as_ref().ok_or_else(|| "connect first".into())
    }
    async fn scan(&mut self) -> Result<()> {
        println!("Scanning for DJI cameras (5 seconds)…");
        self.devices = self.scanner.scan(Duration::from_secs(5)).await?;
        for (i, d) in self.devices.iter().enumerate() {
            println!("{i}  {}  {}  RSSI {:?}", d.id, d.name, d.rssi);
        }
        if self.devices.is_empty() {
            println!("No supported camera found; enable the camera's wireless connection.");
        }
        Ok(())
    }
    async fn connect(&mut self, selector: Option<&str>) -> Result<()> {
        if self.camera.is_some() {
            return Err("disconnect the current camera first".into());
        }
        if self.devices.is_empty() {
            self.scan().await?;
        }
        let device = match selector {
            Some(s) => self
                .devices
                .iter()
                .enumerate()
                .find(|(i, d)| i.to_string() == s || d.id == s || d.name == s)
                .map(|(_, d)| d.clone()),
            None if self.devices.len() == 1 => self.devices.first().cloned(),
            _ => None,
        }
        .ok_or("select a camera: connect <index|id|name>")?;
        let mut options = ConnectOptions::new(self.saved.identity);
        options.first_pair = !self.saved.paired.contains(&device.id);
        println!(
            "Connecting to {}; code {:04}. Allow the connection on the camera if prompted.",
            device.name, options.verification_code
        );
        let camera = Camera::connect(&device, options).await?;
        println!(
            "DJI authenticated; camera ID {:#010x}; GPS send rate 10Hz",
            camera.device_id
        );
        self.events = Some(camera.events());
        self.camera = Some(camera);
        if !self.saved.paired.contains(&device.id) {
            self.saved.paired.push(device.id);
            self.save()?;
        }
        self.camera()?.subscribe_status(3).await?;
        Ok(())
    }
    async fn command(&mut self, line: &str) -> Result<()> {
        let mut parts = line.split_whitespace();
        let Some(command) = parts.next() else {
            return Ok(());
        };
        match command {
            "help" => help(),
            "scan" => self.scan().await?,
            "connect" => self.connect(parts.next()).await?,
            "version" => println!("{:#?}", self.camera()?.version().await?),
            "status" => println!("{:#?}", self.camera()?.status()),
            "watch" => {
                self.watch = parts.next() != Some("off");
                println!("Status output: {}", self.watch);
            }
            "record" => {
                match parts.next() {
                    Some("start") => self.camera()?.start_recording().await?,
                    Some("stop") => self.camera()?.stop_recording().await?,
                    _ => return Err("usage: record start|stop".into()),
                }
                println!("Recording command acknowledged");
            }
            "shutter" => {
                self.camera()?.shutter().await?;
                println!("Shutter acknowledged");
            }
            "qs" => {
                self.camera()?.quick_switch().await?;
                println!("QS acknowledged");
            }
            "mode" => {
                let value = parts
                    .next()
                    .ok_or("usage: mode video|photo|slow|timelapse|hyperlapse|<number>")?;
                let mode = match value {
                    "video" => 1,
                    "photo" => 5,
                    "slow" => 0,
                    "timelapse" => 2,
                    "hyperlapse" => 10,
                    value => number(value)?,
                };
                self.camera()?.set_mode(mode).await?;
                println!("Mode command acknowledged");
            }
            "key" => {
                let key = match parts.next() {
                    Some("shutter") => Key::Shutter,
                    Some("qs") => Key::Qs,
                    Some("snapshot") => Key::Snapshot,
                    _ => return Err("expected shutter|qs|snapshot".into()),
                };
                match parts.next() {
                    Some("press") => self.camera()?.key_state(key, true).await?,
                    Some("release") => self.camera()?.key_state(key, false).await?,
                    event => {
                        let event = match event {
                            Some("short") => KeyEvent::Short,
                            Some("long") => KeyEvent::Long,
                            Some("double") => KeyEvent::Double,
                            Some("triple") => KeyEvent::Triple,
                            Some("quad") => KeyEvent::Quadruple,
                            _ => {
                                return Err(
                                    "expected short|long|double|triple|quad|press|release".into()
                                );
                            }
                        };
                        self.camera()?.key_event(key, event).await?;
                    }
                }
                println!("Key acknowledged");
            }
            "gps" => {
                let action = parts
                    .next()
                    .ok_or("usage: gps replay|once <file.jsonl>, or gps stop")?;
                if action == "stop" {
                    self.replay = None;
                    self.camera()?.stop_gps();
                    println!("GPS stopped");
                } else {
                    let samples = load_gps(&parts.collect::<Vec<_>>().join(" "))?;
                    match action {
                        "once" => {
                            self.camera()?.send_gps(&samples[0]).await?;
                            println!("GPS fix written");
                        }
                        "replay" => {
                            self.camera()?;
                            println!(
                                "Replaying {} recorded fixes at their original intervals; BLE sends at 10Hz. Test data only.",
                                samples.len()
                            );
                            self.replay = Some(Replay {
                                samples,
                                index: 0,
                                started: Instant::now(),
                            });
                        }
                        _ => return Err("expected gps replay|once|stop".into()),
                    }
                }
            }
            "sleep" => {
                self.replay = None;
                self.camera()?.sleep().await?;
                println!("Sleep acknowledged");
            }
            "reboot" => {
                self.replay = None;
                self.camera()?.reboot().await?;
                println!("Restart acknowledged");
            }
            "wake" => self.camera()?.wake().await?,
            "snapshot" => self.camera()?.snapshot().await?,
            "disconnect" => {
                self.replay = None;
                self.events = None;
                if let Some(camera) = self.camera.take() {
                    camera.disconnect().await?;
                }
                println!("Disconnected");
            }
            _ => return Err("unknown command; type help".into()),
        }
        Ok(())
    }
    fn advance_replay(&mut self) -> Result<()> {
        let Some(replay) = &mut self.replay else {
            return Ok(());
        };
        let elapsed = replay.started.elapsed().as_millis() as i64;
        let origin = replay.samples[0].timestamp_ms;
        let mut latest = None;
        while replay.index < replay.samples.len()
            && replay.samples[replay.index].timestamp_ms - origin <= elapsed
        {
            latest = Some(&replay.samples[replay.index]);
            replay.index += 1;
        }
        if let Some(sample) = latest {
            self.camera
                .as_ref()
                .ok_or("camera disconnected")?
                .update_gps(sample)?;
        }
        if replay.index == replay.samples.len()
            && elapsed > replay.samples.last().unwrap().timestamp_ms - origin + 1100
        {
            if let Some(camera) = &self.camera {
                camera.stop_gps();
            }
            self.replay = None;
            println!("GPS replay complete");
        }
        Ok(())
    }
    fn event(&mut self, event: Event) {
        match event {
            Event::Status(s) if self.watch => println!(
                "state={} mode={:#04x} recording={} time={}s battery={}% power={}",
                s.state,
                s.mode,
                s.is_recording(),
                s.record_seconds,
                s.battery_percent,
                s.power_mode
            ),
            Event::ModeInfo(info) if self.watch => println!("{} {}", info.name, info.parameters),
            Event::Reconnecting(attempt) => {
                self.replay = None;
                if let Some(camera) = &self.camera {
                    camera.stop_gps();
                }
                eprintln!("Reconnecting (attempt {attempt}/2)…");
            }
            Event::Reconnected => println!("Reconnected; status subscription restored"),
            Event::Warning(warning) => eprintln!("Warning: {warning}"),
            Event::Disconnected(reason) => {
                eprintln!("Disconnected: {reason}");
                self.replay = None;
                self.events = None;
                self.camera = None;
            }
            _ => {}
        }
    }
}
fn number(value: &str) -> Result<u8> {
    Ok(if let Some(hex) = value.strip_prefix("0x") {
        u8::from_str_radix(hex, 16)?
    } else {
        value.parse()?
    })
}
fn load_gps(path: &str) -> Result<Vec<GpsSample>> {
    let input = std::fs::read_to_string(path)?;
    let mut samples: Vec<GpsSample> = vec![];
    for (i, line) in input
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
    {
        let sample: GpsSample =
            serde_json::from_str(line).map_err(|e| format!("GPS line {}: {e}", i + 1))?;
        sample
            .validate()
            .map_err(|e| format!("GPS line {}: {e}", i + 1))?;
        if samples
            .last()
            .is_some_and(|previous| previous.timestamp_ms >= sample.timestamp_ms)
        {
            return Err("GPS timestamps must increase".into());
        }
        samples.push(sample);
    }
    if samples.is_empty() {
        return Err("GPS file is empty".into());
    }
    Ok(samples)
}
fn help() {
    println!(
        "Commands:\n  scan | connect [index|id|name] | version | status | watch on|off\n  record start|stop | shutter | mode video|photo|slow|timelapse|hyperlapse|<number> | qs\n  key shutter|qs|snapshot short|long|double|triple|quad|press|release\n  gps replay|once <file.jsonl> | gps stop\n  sleep | reboot | disconnect | wait <seconds> | quit\n  wake | snapshot (manufacturer advertising unsupported by this backend)"
    );
}
#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "warn".into()),
        )
        .with_writer(io::stderr)
        .init();
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.first().is_some_and(|a| a == "--help" || a == "-h") {
        println!(
            "macos-cli [--scan | --connect <index|id|name>]\nRUST_LOG=osmo_core=debug enables frame logs; OSMO_STATE_DIR selects pairing storage."
        );
        help();
        return Ok(());
    }
    let mut app = App::new().await?;
    match args.as_slice() {
        [] => {}
        [scan] if scan == "--scan" => {
            app.scan().await?;
            return Ok(());
        }
        [connect, selector] if connect == "--connect" => app.connect(Some(selector)).await?,
        _ => return Err("usage: macos-cli [--scan | --connect <index|id|name>]".into()),
    }
    let interactive = io::stdin().is_terminal();
    if interactive {
        help();
        print!("osmo> ");
        io::stdout().flush()?;
    }
    let (tx, mut lines) = mpsc::channel(16);
    std::thread::spawn(move || {
        for line in io::stdin().lock().lines() {
            if tx.blocking_send(line).is_err() {
                break;
            }
        }
    });
    let mut tick = interval(Duration::from_millis(100));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut waiting_until = Instant::now();
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => break,
            _ = tick.tick() => if let Err(e) = app.advance_replay() { eprintln!("Error: {e}"); app.replay = None; if let Some(c) = &app.camera { c.stop_gps(); } },
            event = async { match &mut app.events { Some(events) => events.recv().await, None => std::future::pending().await } } => {
                match event {
                    Ok(event) => app.event(event),
                    Err(broadcast::error::RecvError::Lagged(n)) => eprintln!("Skipped {n} old events"),
                    Err(broadcast::error::RecvError::Closed) => app.events = None,
                }
            }
            line = lines.recv(), if waiting_until <= Instant::now() => {
                let Some(line) = line else { break }; let line = line?; let line = line.trim();
                if line == "quit" || line == "exit" { break; }
                if let Some(seconds) = line.strip_prefix("wait ") {
                    match seconds.parse::<f64>() {
                        Ok(s) if s.is_finite() && (0.0..=3600.0).contains(&s) => waiting_until = Instant::now() + Duration::from_secs_f64(s),
                        _ => eprintln!("wait requires 0..3600 seconds"),
                    }
                } else if let Err(e) = app.command(line).await { eprintln!("Error: {e}"); }
                if interactive { print!("osmo> "); io::stdout().flush()?; }
            }
        }
    }
    if let Some(camera) = app.camera.take() {
        camera.disconnect().await?;
    }
    Ok(())
}
