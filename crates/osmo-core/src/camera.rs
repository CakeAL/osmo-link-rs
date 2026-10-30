use std::{collections::HashMap, time::Duration};

use btleplug::api::Peripheral as _;
use tokio::{
    sync::{broadcast, mpsc, oneshot, watch},
    time::{Instant, interval, timeout},
};

use crate::{CameraStatus, Device, Error, GpsSample, ModeInfo, Result, ble::Link, protocol::Frame};

/// Application-owned identity, independent of the host's private Bluetooth MAC.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct ControllerIdentity {
    pub device_id: u32,
    pub address: [u8; 6],
}

impl Default for ControllerIdentity {
    fn default() -> Self {
        let random = uuid::Uuid::new_v4();
        let bytes = random.as_bytes();
        let mut address: [u8; 6] = bytes[4..10].try_into().unwrap();
        address[0] = (address[0] | 2) & !1;
        Self {
            device_id: u32::from_le_bytes(bytes[..4].try_into().unwrap()),
            address,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConnectOptions {
    pub identity: ControllerIdentity,
    pub first_pair: bool,
    pub verification_code: u16,
    /// GPS transmission period. Default: 100ms (10Hz); maximum: one second.
    pub gps_interval: Duration,
}

impl ConnectOptions {
    pub fn new(identity: ControllerIdentity) -> Self {
        Self {
            identity,
            first_pair: false,
            verification_code: (uuid::Uuid::new_v4().as_u128() % 10000) as u16,
            gps_interval: Duration::from_millis(100),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    Frame(Frame),
    Status(CameraStatus),
    ModeInfo(ModeInfo),
    Warning(String),
    Disconnected(String),
    Reconnecting(u8),
    Reconnected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Connected,
    Sleeping,
    Reconnecting,
    Disconnected,
}

#[derive(Debug, Clone)]
pub struct Version {
    pub product: String,
    pub details: String,
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum Key {
    Shutter = 1,
    Qs = 2,
    Snapshot = 3,
}

#[derive(Debug, Clone, Copy)]
#[repr(u16)]
pub enum KeyEvent {
    Short = 0,
    Long = 1,
    Double = 2,
    Triple = 3,
    Quadruple = 4,
}

struct Request {
    command: [u8; 2],
    payload: Vec<u8>,
    wait: bool,
    reply: oneshot::Sender<Result<Frame>>,
}

struct Pending {
    command: [u8; 2],
    sleep: bool,
    deadline: Instant,
    reply: oneshot::Sender<Result<Frame>>,
}

#[derive(Clone)]
struct GpsInput {
    payload: Vec<u8>,
    received_at: Instant,
}

pub struct Camera {
    pub device_id: u32,
    commands: mpsc::Sender<Request>,
    events: broadcast::Sender<Event>,
    status: watch::Receiver<Option<CameraStatus>>,
    gps: watch::Sender<Option<GpsInput>>,
    connection: watch::Receiver<ConnectionState>,
    shutdown: Option<oneshot::Sender<()>>,
    task: Option<tokio::task::JoinHandle<()>>,
}

impl Camera {
    pub async fn connect(device: &Device, options: ConnectOptions) -> Result<Self> {
        if options.verification_code > 9999 {
            return Err(Error::InvalidInput("verification code must be 0..9999"));
        }
        if options.gps_interval < Duration::from_millis(100)
            || options.gps_interval > Duration::from_secs(1)
        {
            return Err(Error::InvalidInput("GPS interval must be 100ms..1s"));
        }
        let mut link = Link::open(device).await?;
        let result = timeout(Duration::from_secs(35), handshake(&mut link, &options))
            .await
            .unwrap_or(Err(Error::Timeout("DJI pairing")));
        let device_id = match result {
            Ok(id) => id,
            Err(error) => {
                link.close().await;
                return Err(error);
            }
        };
        let (commands, receiver) = mpsc::channel(8);
        let (events, _) = broadcast::channel(64);
        let (status_tx, status) = watch::channel(None);
        let (gps, gps_rx) = watch::channel(None);
        let (connection_tx, connection) = watch::channel(ConnectionState::Connected);
        let (shutdown, stop) = oneshot::channel();
        let task = tokio::spawn(
            Worker {
                link,
                requests: receiver,
                events: events.clone(),
                status: status_tx,
                gps: gps_rx,
                connection: connection_tx,
                stop,
                device: device.clone(),
                options,
                device_id,
                seq: 1,
                subscription: None,
                gps_valid_from: Instant::now(),
            }
            .run(),
        );
        Ok(Self {
            device_id,
            commands,
            events,
            status,
            gps,
            connection,
            shutdown: Some(shutdown),
            task: Some(task),
        })
    }

    pub fn events(&self) -> broadcast::Receiver<Event> {
        self.events.subscribe()
    }

    pub fn connection_state(&self) -> ConnectionState {
        *self.connection.borrow()
    }

    pub fn connections(&self) -> watch::Receiver<ConnectionState> {
        self.connection.clone()
    }

    fn ensure_connected(&self) -> Result<()> {
        match self.connection_state() {
            ConnectionState::Connected => Ok(()),
            ConnectionState::Sleeping => Err(Error::Sleeping),
            ConnectionState::Reconnecting => Err(Error::Reconnecting),
            ConnectionState::Disconnected => Err(Error::Disconnected),
        }
    }

    /// Explicitly close BLE and wait for worker cleanup. Drop also requests cleanup.
    pub async fn disconnect(mut self) -> Result<()> {
        self.stop_gps();
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        if let Some(mut task) = self.task.take() {
            match timeout(Duration::from_secs(8), &mut task).await {
                Ok(result) => result.map_err(|_| Error::Disconnected)?,
                Err(_) => {
                    task.abort();
                    return Err(Error::Timeout("disconnect"));
                }
            }
        }
        Ok(())
    }

    /// DJI wake needs custom manufacturer advertising, absent from btleplug.
    pub async fn wake(&self) -> Result<()> {
        Err(Error::Unsupported("DJI manufacturer wake advertising"))
    }

    pub async fn snapshot(&self) -> Result<()> {
        self.wake().await?;
        self.key_event(Key::Snapshot, KeyEvent::Short).await
    }

    /// Update the latest fix. The worker transmits at the configured period
    /// (default 10Hz), for at most 2 seconds after the last input update.
    pub fn update_gps(&self, sample: &GpsSample) -> Result<()> {
        self.ensure_connected()?;
        self.gps.send_replace(Some(GpsInput {
            payload: sample.encode()?,
            received_at: Instant::now(),
        }));
        Ok(())
    }

    pub fn stop_gps(&self) {
        self.gps.send_replace(None);
    }

    /// Send one fix without enabling periodic transmission.
    pub async fn send_gps(&self, sample: &GpsSample) -> Result<()> {
        self.command([0, 0x17], sample.encode()?, false).await?;
        Ok(())
    }

    pub fn status(&self) -> Option<CameraStatus> {
        self.status.borrow().clone()
    }

    pub fn statuses(&self) -> watch::Receiver<Option<CameraStatus>> {
        self.status.clone()
    }

    pub async fn version(&self) -> Result<Version> {
        let frame = self.command([0, 0], vec![], true).await?;
        let b = &frame.payload;
        if b.len() < 18 {
            return Err(Error::Malformed("version response"));
        }
        let code = u16::from_le_bytes([b[0], b[1]]);
        if code != 0 {
            return Err(Error::Camera(code));
        }
        Ok(Version {
            product: String::from_utf8_lossy(&b[2..18])
                .trim_end_matches('\0')
                .into(),
            details: String::from_utf8_lossy(&b[18..])
                .replace('\0', " ")
                .trim()
                .into(),
        })
    }

    async fn control(&self, command: [u8; 2], payload: Vec<u8>) -> Result<()> {
        let frame = self.command(command, payload, true).await?;
        check_code(&frame.payload)
    }

    fn target_payload(&self, value: Option<u8>) -> Vec<u8> {
        let mut b = self.device_id.to_le_bytes().to_vec();
        if let Some(value) = value {
            b.push(value);
        }
        b.extend_from_slice(&[0; 4]);
        b
    }

    /// Explicit start/stop, rather than toggling the shutter button.
    pub async fn start_recording(&self) -> Result<()> {
        self.control([0x1d, 3], self.target_payload(Some(0))).await
    }
    pub async fn stop_recording(&self) -> Result<()> {
        self.control([0x1d, 3], self.target_payload(Some(1))).await
    }
    pub async fn set_mode(&self, mode: u8) -> Result<()> {
        self.control([0x1d, 4], self.target_payload(Some(mode)))
            .await
    }
    pub async fn reboot(&self) -> Result<()> {
        self.control([0, 0x16], self.target_payload(None)).await
    }
    pub async fn sleep(&self) -> Result<()> {
        self.set_power_mode(3).await
    }

    pub async fn set_power_mode(&self, mode: u8) -> Result<()> {
        if mode != 0 && mode != 3 {
            return Err(Error::InvalidInput("power mode must be 0 or 3"));
        }
        self.control([0, 0x1a], vec![mode]).await
    }

    pub async fn key_event(&self, key: Key, event: KeyEvent) -> Result<()> {
        let mut b = vec![key as u8, 1];
        b.extend_from_slice(&(event as u16).to_le_bytes());
        self.control([0, 0x11], b).await
    }

    pub async fn key_state(&self, key: Key, pressed: bool) -> Result<()> {
        let mut b = vec![key as u8, 0];
        b.extend_from_slice(&u16::from(!pressed).to_le_bytes());
        self.control([0, 0x11], b).await
    }

    pub async fn shutter(&self) -> Result<()> {
        self.key_event(Key::Shutter, KeyEvent::Short).await
    }
    pub async fn quick_switch(&self) -> Result<()> {
        self.key_event(Key::Qs, KeyEvent::Short).await
    }

    /// 0=off, 1=once, 2=periodic, 3=periodic plus changes; fixed 2Hz.
    /// State is delivered via events; an optional subscription ACK is ignored.
    pub async fn subscribe_status(&self, mode: u8) -> Result<()> {
        if mode > 3 {
            return Err(Error::InvalidInput("status mode must be 0..3"));
        }
        self.command([0x1d, 5], vec![mode, 20, 0, 0, 0, 0], false)
            .await?;
        Ok(())
    }

    pub(crate) async fn command(
        &self,
        command: [u8; 2],
        payload: Vec<u8>,
        wait: bool,
    ) -> Result<Frame> {
        self.ensure_connected()?;
        let (reply, response) = oneshot::channel();
        self.commands
            .send(Request {
                command,
                payload,
                wait,
                reply,
            })
            .await
            .map_err(|_| Error::Disconnected)?;
        response.await.map_err(|_| Error::Disconnected)?
    }
}

impl Drop for Camera {
    fn drop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
    }
}

fn check_code(payload: &[u8]) -> Result<()> {
    match payload.first() {
        Some(0) => Ok(()),
        Some(code) => Err(Error::Camera((*code).into())),
        None => Err(Error::Malformed("command response")),
    }
}

async fn handshake(link: &mut Link, options: &ConnectOptions) -> Result<u32> {
    let mut payload = Vec::with_capacity(33);
    payload.extend_from_slice(&options.identity.device_id.to_le_bytes());
    payload.push(6);
    payload.extend_from_slice(&options.identity.address);
    payload.extend_from_slice(&[0; 10]);
    payload.extend_from_slice(&0u32.to_le_bytes()); // fw_version=0 disables accessory updates
    payload.extend_from_slice(&[0, u8::from(options.first_pair)]);
    payload.extend_from_slice(&options.verification_code.to_le_bytes());
    payload.extend_from_slice(&[0; 4]);
    link.send(&Frame {
        kind: 2,
        seq: 1,
        command: [0, 0x19],
        payload,
    })
    .await?;
    loop {
        let frame = link.receive().await?;
        if frame.command != [0, 0x19] {
            continue;
        }
        if frame.is_ack() {
            if frame.seq != 1 {
                continue;
            }
            let code = *frame
                .payload
                .get(4)
                .ok_or(Error::Malformed("pairing response"))?;
            if code != 0 {
                return Err(Error::Camera(code.into()));
            }
            continue;
        }
        if frame.payload.len() < 33 || frame.payload[26] != 2 {
            return Err(Error::Malformed("pairing confirmation"));
        }
        if u16::from_le_bytes([frame.payload[27], frame.payload[28]]) != 0 {
            return Err(Error::PairingRejected);
        }
        let device_id = u32::from_le_bytes(frame.payload[..4].try_into().unwrap());
        let mut payload = options.identity.device_id.to_le_bytes().to_vec();
        payload.extend_from_slice(&[0; 5]); // success + single-camera index 0
        link.send(&Frame {
            kind: 0x20,
            seq: frame.seq,
            command: frame.command,
            payload,
        })
        .await?;
        return Ok(device_id);
    }
}

struct Worker {
    link: Link,
    requests: mpsc::Receiver<Request>,
    events: broadcast::Sender<Event>,
    status: watch::Sender<Option<CameraStatus>>,
    gps: watch::Receiver<Option<GpsInput>>,
    connection: watch::Sender<ConnectionState>,
    stop: oneshot::Receiver<()>,
    device: Device,
    options: ConnectOptions,
    device_id: u32,
    seq: u16,
    subscription: Option<u8>,
    gps_valid_from: Instant,
}

impl Worker {
    async fn run(mut self) {
        let mut retries = 0u8;
        let final_reason = 'sessions: loop {
            let (reason, intentional, sleeping) = self.serve().await;
            self.status.send_replace(None);
            self.link.close().await;
            if intentional || sleeping || self.requests.is_closed() {
                break reason;
            }
            self.connection.send_replace(ConnectionState::Reconnecting);
            // Never replay control requests across a connection boundary.
            while let Ok(request) = self.requests.try_recv() {
                let _ = request.reply.send(Err(Error::Disconnected));
            }
            let mut connected = false;
            while retries < 2 {
                retries += 1;
                let _ = self.events.send(Event::Reconnecting(retries));
                let mut options = self.options.clone();
                options.first_pair = false;
                let attempt = async {
                    let mut link = Link::open(&self.device).await?;
                    let result = timeout(Duration::from_secs(35), handshake(&mut link, &options))
                        .await
                        .unwrap_or(Err(Error::Timeout("DJI reconnect")));
                    match result {
                        Ok(id) if id == self.device_id => Ok(link),
                        result => {
                            link.close().await;
                            Err(match result {
                                Err(e) => e,
                                _ => Error::Malformed("changed camera ID"),
                            })
                        }
                    }
                };
                let result = tokio::select! {
                    _ = &mut self.stop => break 'sessions "controller closed".to_string(),
                    result = attempt => result,
                };
                match result {
                    Ok(link) => {
                        self.link = link;
                        self.seq = self.seq.wrapping_add(1);
                        if let Some(mode) = self.subscription {
                            let frame = Frame {
                                kind: 0,
                                seq: self.seq,
                                command: [0x1d, 5],
                                payload: vec![mode, 20, 0, 0, 0, 0],
                            };
                            if let Err(error) = self.link.send(&frame).await {
                                self.link.close().await;
                                let _ = self.events.send(Event::Warning(error.to_string()));
                                continue;
                            }
                        }
                        // Ignore any fix supplied before the new connection was ready.
                        self.gps_valid_from = Instant::now();
                        self.connection.send_replace(ConnectionState::Connected);
                        let _ = self.events.send(Event::Reconnected);
                        connected = true;
                        break;
                    }
                    Err(Error::PairingRejected) => {
                        break 'sessions "camera rejected reconnection".to_string();
                    }
                    Err(error) => {
                        let _ = self.events.send(Event::Warning(error.to_string()));
                    }
                }
            }
            if !connected {
                break format!("{reason}; reconnect attempts exhausted");
            }
        };
        self.link.close().await;
        self.connection.send_replace(ConnectionState::Disconnected);
        self.requests.close();
        while let Ok(request) = self.requests.try_recv() {
            let _ = request.reply.send(Err(Error::Disconnected));
        }
        let _ = self.events.send(Event::Disconnected(final_reason));
    }

    fn next_seq(&mut self, pending: &HashMap<u16, Pending>) -> u16 {
        loop {
            self.seq = self.seq.wrapping_add(1);
            if !pending.contains_key(&self.seq) {
                return self.seq;
            }
        }
    }

    async fn serve(&mut self) -> (String, bool, bool) {
        let mut pending = HashMap::<u16, Pending>::new();
        let mut tick = interval(Duration::from_millis(250));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut gps_tick = interval(self.options.gps_interval);
        gps_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let ready_at = self.gps_valid_from;
        let mut sleeping = false;
        let mut sleep_requested = false;
        let mut intentional = false;
        let reason = loop {
            tokio::select! {
                _ = &mut self.stop => { intentional = true; break "controller closed".to_string(); }
                _ = gps_tick.tick() => {
                    let latest = self.gps.borrow().clone();
                    if let Some(fix) = latest.filter(|fix| !sleeping && !sleep_requested && fix.received_at >= ready_at && fix.received_at.elapsed() < Duration::from_secs(2)) {
                        let seq = self.next_seq(&pending);
                        let frame = Frame { kind: 0, seq, command: [0, 0x17], payload: fix.payload };
                        if let Err(error) = self.link.send(&frame).await { break error.to_string(); }
                    }
                }
                request = self.requests.recv() => {
                    let Some(request) = request else { intentional = true; break "controller closed".to_string(); };
                    if request.reply.is_closed() { continue; }
                    if sleeping || sleep_requested {
                        let _ = request.reply.send(Err(Error::Sleeping));
                        continue;
                    }
                    let seq = self.next_seq(&pending);
                    let is_sleep = request.command == [0, 0x1a] && request.payload == [3];
                    let subscription = if request.command == [0x1d, 5] { request.payload.first().copied() } else { None };
                    let frame = Frame { kind: if request.wait { 2 } else { 0 }, seq, command: request.command, payload: request.payload };
                    if let Err(error) = self.link.send(&frame).await {
                        let reason = error.to_string(); let _ = request.reply.send(Err(error)); break reason;
                    }
                    if let Some(mode) = subscription { self.subscription = Some(mode); }
                    sleep_requested |= is_sleep;
                    if request.wait {
                        pending.insert(seq, Pending { command: request.command, sleep: is_sleep, deadline: Instant::now() + Duration::from_secs(3), reply: request.reply });
                    } else { let _ = request.reply.send(Ok(frame)); }
                }
                incoming = self.link.receive() => {
                    match incoming {
                        Ok(frame) => {
                            if frame.is_ack() && pending.get(&frame.seq).is_some_and(|p| ack_matches(self.device_id, p.command, frame.command)) {
                                let entry = pending.remove(&frame.seq).unwrap();
                                if entry.sleep {
                                    sleep_requested = false;
                                    if check_code(&frame.payload).is_ok() {
                                        sleeping = true;
                                        self.connection.send_replace(ConnectionState::Sleeping);
                                    }
                                }
                                let _ = entry.reply.send(Ok(frame));
                            } else if !frame.is_ack() {
                                let decoded = match frame.command {
                                    [0x1d, 2] => CameraStatus::decode(&frame.payload).map(|s| {
                                        sleeping |= s.power_mode == 3;
                                        if sleeping { self.connection.send_replace(ConnectionState::Sleeping); }
                                        self.status.send_replace(Some(s.clone()));
                                        Event::Status(s)
                                    }),
                                    [0x1d, 6] => ModeInfo::decode(&frame.payload).map(Event::ModeInfo),
                                    _ => Ok(Event::Frame(frame.clone())),
                                };
                                let known_status = matches!(frame.command, [0x1d, 2] | [0x1d, 6]) && decoded.is_ok();
                                let _ = self.events.send(decoded.unwrap_or_else(|e| Event::Warning(e.to_string())));
                                if known_status && frame.kind & 0x1f >= 2 && !sleeping && !sleep_requested {
                                    let ack = Frame { kind: 0x20, seq: frame.seq, command: frame.command, payload: vec![0] };
                                    if let Err(error) = self.link.send(&ack).await { break error.to_string(); }
                                }
                            } else { let _ = self.events.send(Event::Frame(frame)); }
                        }
                        Err(error) => break error.to_string(),
                    }
                }
                _ = tick.tick() => {
                    let expired: Vec<_> = pending.iter().filter(|(_, p)| p.deadline <= Instant::now()).map(|(seq, _)| *seq).collect();
                    for seq in expired {
                        if let Some(p) = pending.remove(&seq) {
                            // On an uncertain sleep outcome, stop all writes until reconnect.
                            if p.sleep { sleeping = true; self.connection.send_replace(ConnectionState::Sleeping); }
                            let _ = p.reply.send(Err(Error::Timeout("command response")));
                        }
                    }
                    let connected = timeout(Duration::from_secs(2), self.link.peripheral.is_connected()).await;
                    if !matches!(connected, Ok(Ok(true))) { break "Bluetooth link lost".to_string(); }
                }
            }
        };
        for (_, p) in pending {
            let _ = p.reply.send(Err(Error::Disconnected));
        }
        (reason, intentional, sleeping || sleep_requested)
    }
}

fn ack_matches(camera_id: u32, requested: [u8; 2], received: [u8; 2]) -> bool {
    requested == received
        // Action 4 firmware 03.04.80.15 returns 1D03 for 1D04 mode switches.
        // The caller also requires the exact sequence number; all other IDs stay strict.
        || (camera_id == 0xff33 && requested == [0x1d, 4] && received == [0x1d, 3])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action4_mode_ack_is_narrowly_scoped() {
        assert!(ack_matches(0xff33, [0x1d, 4], [0x1d, 3]));
        assert!(!ack_matches(0xff44, [0x1d, 4], [0x1d, 3]));
        assert!(!ack_matches(0xff33, [0x1d, 3], [0x1d, 4]));
        assert!(!ack_matches(0xff33, [0, 0x1a], [0x1d, 3]));
    }
}
