use std::{collections::HashMap, time::Duration};

use btleplug::api::Peripheral as _;
use tokio::{
    sync::{broadcast, mpsc, oneshot, watch},
    time::{Instant, interval, timeout},
};

use crate::{CameraStatus, Device, Error, ModeInfo, Result, ble::Link, protocol::Frame};

/// Application-owned identity, independent of the host's private Bluetooth MAC.
#[derive(Debug, Clone, Copy)]
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
}

impl ConnectOptions {
    pub fn new(identity: ControllerIdentity) -> Self {
        Self {
            identity,
            first_pair: false,
            verification_code: (uuid::Uuid::new_v4().as_u128() % 10000) as u16,
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

pub struct Camera {
    pub device_id: u32,
    commands: mpsc::Sender<Request>,
    events: broadcast::Sender<Event>,
    status: watch::Receiver<Option<CameraStatus>>,
}

impl Camera {
    pub async fn connect(device: &Device, options: ConnectOptions) -> Result<Self> {
        if options.verification_code > 9999 {
            return Err(Error::InvalidInput("verification code must be 0..9999"));
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
        tokio::spawn(run(link, receiver, events.clone(), status_tx));
        Ok(Self {
            device_id,
            commands,
            events,
            status,
        })
    }

    pub fn events(&self) -> broadcast::Receiver<Event> {
        self.events.subscribe()
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
        self.control([0, 0x1a], vec![3]).await
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
    /// The camera responds via status events, not a normal command ACK.
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

async fn run(
    mut link: Link,
    mut requests: mpsc::Receiver<Request>,
    events: broadcast::Sender<Event>,
    status: watch::Sender<Option<CameraStatus>>,
) {
    let mut seq = 1u16;
    let mut pending = HashMap::<u16, Pending>::new();
    let mut tick = interval(Duration::from_millis(250));
    let mut sleeping = false;
    let reason = loop {
        tokio::select! {
            request = requests.recv() => {
                let Some(request) = request else { break "controller closed".to_string(); };
                if sleeping {
                    let _ = request.reply.send(Err(Error::Sleeping));
                    continue;
                }
                loop { seq = seq.wrapping_add(1); if !pending.contains_key(&seq) { break; } }
                let frame = Frame { kind: if request.wait { 2 } else { 0 }, seq, command: request.command, payload: request.payload };
                if let Err(error) = link.send(&frame).await {
                    let reason = error.to_string();
                    let _ = request.reply.send(Err(error));
                    break reason;
                }
                if request.wait {
                    pending.insert(seq, Pending { command: request.command, sleep: request.command == [0, 0x1a] && frame.payload == [3], deadline: Instant::now() + Duration::from_secs(3), reply: request.reply });
                } else { let _ = request.reply.send(Ok(frame)); }
            }
            incoming = link.receive() => {
                match incoming {
                    Ok(frame) => {
                        if frame.is_ack() && pending.get(&frame.seq).is_some_and(|p| p.command == frame.command) {
                            let entry = pending.remove(&frame.seq).unwrap();
                            if entry.sleep && check_code(&frame.payload).is_ok() { sleeping = true; }
                            let _ = entry.reply.send(Ok(frame));
                        } else {
                            if !frame.is_ack() {
                                let decoded = match frame.command {
                                    [0x1d, 2] => CameraStatus::decode(&frame.payload).map(|s| {
                                        sleeping |= s.power_mode == 3;
                                        status.send_replace(Some(s.clone()));
                                        Event::Status(s)
                                    }),
                                    [0x1d, 6] => ModeInfo::decode(&frame.payload).map(Event::ModeInfo),
                                    _ => Ok(Event::Frame(frame.clone())),
                                };
                                let _ = events.send(decoded.unwrap_or_else(|e| Event::Warning(e.to_string())));
                                if frame.kind & 0x1f != 0 && !sleeping {
                                    let ack = Frame { kind: 0x20, seq: frame.seq, command: frame.command, payload: vec![0] };
                                    if let Err(error) = link.send(&ack).await { break error.to_string(); }
                                }
                            } else { let _ = events.send(Event::Frame(frame)); }
                        }
                    }
                    Err(error) => break error.to_string(),
                }
            }
            _ = tick.tick() => {
                let expired: Vec<_> = pending.iter().filter(|(_, p)| p.deadline <= Instant::now()).map(|(seq, _)| *seq).collect();
                for seq in expired {
                    if let Some(p) = pending.remove(&seq) { let _ = p.reply.send(Err(Error::Timeout("command response"))); }
                }
                if !link.peripheral.is_connected().await.unwrap_or(false) { break "Bluetooth link lost".to_string(); }
            }
        }
    };
    for (_, p) in pending {
        let _ = p.reply.send(Err(Error::Disconnected));
    }
    link.close().await;
    status.send_replace(None);
    let _ = events.send(Event::Disconnected(reason));
}
