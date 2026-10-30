use std::{collections::HashMap, time::Duration};

use btleplug::api::Peripheral as _;
use tokio::{
    sync::{broadcast, mpsc, oneshot},
    time::{Instant, interval, timeout},
};

use crate::{Device, Error, Result, ble::Link, protocol::Frame};

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
    Disconnected(String),
}

struct Request {
    command: [u8; 2],
    payload: Vec<u8>,
    wait: bool,
    reply: oneshot::Sender<Result<Frame>>,
}

struct Pending {
    command: [u8; 2],
    deadline: Instant,
    reply: oneshot::Sender<Result<Frame>>,
}

pub struct Camera {
    pub device_id: u32,
    commands: mpsc::Sender<Request>,
    events: broadcast::Sender<Event>,
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
        tokio::spawn(run(link, receiver, events.clone()));
        Ok(Self {
            device_id,
            commands,
            events,
        })
    }

    pub fn events(&self) -> broadcast::Receiver<Event> {
        self.events.subscribe()
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
) {
    let mut seq = 1u16;
    let mut pending = HashMap::<u16, Pending>::new();
    let mut tick = interval(Duration::from_millis(250));
    let reason = loop {
        tokio::select! {
            request = requests.recv() => {
                let Some(request) = request else { break "controller closed".to_string(); };
                loop { seq = seq.wrapping_add(1); if !pending.contains_key(&seq) { break; } }
                let frame = Frame { kind: if request.wait { 2 } else { 0 }, seq, command: request.command, payload: request.payload };
                if let Err(error) = link.send(&frame).await {
                    let reason = error.to_string();
                    let _ = request.reply.send(Err(error));
                    break reason;
                }
                if request.wait {
                    pending.insert(seq, Pending { command: request.command, deadline: Instant::now() + Duration::from_secs(3), reply: request.reply });
                } else { let _ = request.reply.send(Ok(frame)); }
            }
            incoming = link.receive() => {
                match incoming {
                    Ok(frame) => {
                        if frame.is_ack() && pending.get(&frame.seq).is_some_and(|p| p.command == frame.command) {
                            let entry = pending.remove(&frame.seq).unwrap();
                            let _ = entry.reply.send(Ok(frame));
                        } else { let _ = events.send(Event::Frame(frame)); }
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
    let _ = events.send(Event::Disconnected(reason));
}
