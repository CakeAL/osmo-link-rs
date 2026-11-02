use std::{collections::VecDeque, pin::Pin, time::Duration};

use btleplug::{
    api::{
        Central, CentralState, CharPropFlags, Characteristic, Manager as _, Peripheral as _,
        ScanFilter, ValueNotification, WriteType,
    },
    platform::{Adapter, Manager, Peripheral},
};
use futures_util::{Stream, StreamExt};
use tokio::time::{sleep, timeout};
use uuid::Uuid;

use crate::{
    Error, Result,
    protocol::{Decoder, Frame},
};

fn uuid(short: u16) -> Uuid {
    Uuid::from_u128(((short as u128) << 96) | 0x0000_1000_8000_0080_5f9b_34fb)
}

#[derive(Debug, Clone)]
pub struct Device {
    /// Opaque platform ID. macOS/iOS do not expose the camera's BLE MAC address.
    pub id: String,
    pub name: String,
    pub rssi: Option<i16>,
    pub(crate) peripheral: Peripheral,
}

pub struct Scanner {
    adapter: Adapter,
}

impl Scanner {
    pub async fn new() -> Result<Self> {
        let adapter = Manager::new()
            .await?
            .adapters()
            .await?
            .into_iter()
            .next()
            .ok_or(Error::NoAdapter)?;
        Ok(Self { adapter })
    }

    pub async fn scan(&self, duration: Duration) -> Result<Vec<Device>> {
        // CoreBluetooth start_scan only queues a request: it can succeed while
        // Bluetooth is off or the terminal has no permission, then report no devices.
        let state = timeout(Duration::from_secs(3), self.adapter.adapter_state())
            .await
            .map_err(|_| Error::Timeout("Bluetooth adapter state"))??;
        tracing::debug!(?state, "Bluetooth scan adapter state");
        match state {
            CentralState::PoweredOn => {}
            CentralState::PoweredOff => {
                return Err(Error::AdapterNotReady("turn on Bluetooth on the host"));
            }
            CentralState::Unknown => {
                return Err(Error::AdapterNotReady(
                    "check Bluetooth permission for this app; the adapter may also be initializing or resetting",
                ));
            }
        }
        self.adapter.start_scan(ScanFilter::default()).await?;
        sleep(duration).await;
        self.adapter.stop_scan().await?;
        let mut devices = Vec::new();
        let peripherals = self.adapter.peripherals().await?;
        let visible = peripherals.len();
        for peripheral in peripherals {
            let Some(properties) = peripheral.properties().await? else {
                continue;
            };
            if properties.manufacturer_data.contains_key(&0x08aa)
                || properties
                    .local_name
                    .as_deref()
                    .is_some_and(|name| name.contains("Osmo") || name.contains("DJI"))
            {
                tracing::debug!(id = %peripheral.id(), name = ?properties.local_name,
                    manufacturer_data = ?properties.manufacturer_data,
                    services = ?properties.services, "DJI scan candidate");
            }
            // btleplug separates the first two manufacturer bytes into company ID.
            if properties
                .manufacturer_data
                .get(&0x08aa)
                .is_some_and(|data| data.get(2) == Some(&0xfa))
            {
                devices.push(Device {
                    id: peripheral.id().to_string(),
                    // Display fallback only; names never decide compatibility.
                    name: properties
                        .local_name
                        .or(properties.advertisement_name)
                        .unwrap_or_else(|| "Unnamed DJI camera".into()),
                    rssi: properties.rssi,
                    peripheral,
                });
            }
        }
        devices.sort_by_key(|d| std::cmp::Reverse(d.rssi));
        tracing::debug!(
            visible,
            matched = devices.len(),
            "Bluetooth scan results (backend cache)"
        );
        Ok(devices)
    }
}

pub(crate) struct Link {
    pub peripheral: Peripheral,
    write: Characteristic,
    write_type: WriteType,
    notifications: Pin<Box<dyn Stream<Item = ValueNotification> + Send>>,
    decoder: Decoder,
    frames: VecDeque<Frame>,
}

impl Link {
    pub async fn open(device: &Device) -> Result<Self> {
        let peripheral = device.peripheral.clone();
        let result = timeout(Duration::from_secs(15), async {
            peripheral.connect().await?;
            peripheral.discover_services().await?;
            let characteristics = peripheral.characteristics();
            let notify = characteristics
                .iter()
                .find(|c| c.service_uuid == uuid(0xfff0) && c.uuid == uuid(0xfff4))
                .ok_or(Error::MissingCharacteristics)?;
            let write = characteristics
                .iter()
                .find(|c| c.service_uuid == uuid(0xfff0) && c.uuid == uuid(0xfff5))
                .ok_or(Error::MissingCharacteristics)?
                .clone();
            let write_type = if write
                .properties
                .contains(CharPropFlags::WRITE_WITHOUT_RESPONSE)
            {
                WriteType::WithoutResponse
            } else if write.properties.contains(CharPropFlags::WRITE) {
                WriteType::WithResponse
            } else {
                return Err(Error::MissingCharacteristics);
            };
            let notifications = peripheral.notifications().await?;
            peripheral.subscribe(notify).await?;
            Ok(Self {
                peripheral: peripheral.clone(),
                write,
                write_type,
                notifications,
                decoder: Decoder::default(),
                frames: VecDeque::new(),
            })
        })
        .await
        .unwrap_or(Err(Error::Timeout("BLE connection")));
        if result.is_err() {
            let _ = timeout(Duration::from_secs(3), peripheral.disconnect()).await;
        }
        result
    }

    pub async fn send(&mut self, frame: &Frame) -> Result<()> {
        let bytes = frame.encode()?;
        let size = usize::from(self.peripheral.mtu().saturating_sub(3)).max(20);
        timeout(Duration::from_secs(3), async {
            for chunk in bytes.chunks(size) {
                self.peripheral
                    .write(&self.write, chunk, self.write_type)
                    .await?;
            }
            Ok(())
        })
        .await
        .map_err(|_| Error::Timeout("BLE write"))?
    }

    pub async fn receive(&mut self) -> Result<Frame> {
        loop {
            if let Some(frame) = self.frames.pop_front() {
                return Ok(frame);
            }
            let notification = self.notifications.next().await.ok_or(Error::Disconnected)?;
            if notification.uuid == uuid(0xfff4) {
                self.frames.extend(self.decoder.push(&notification.value));
            }
        }
    }

    pub async fn close(&mut self) {
        let _ = timeout(Duration::from_secs(3), self.peripheral.disconnect()).await;
    }
}
