//! Unencrypted DJI R SDK frames, with little endian fields and DJI CRC seeds.
use thiserror::Error;

const MIN_FRAME: usize = 18;
const MAX_FRAME: usize = 1023;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ProtocolError {
    #[error("invalid frame length")]
    Length,
    #[error("unsupported protocol version, flags, or encryption")]
    Unsupported,
    #[error("invalid frame checksum")]
    Checksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub kind: u8,
    pub seq: u16,
    pub command: [u8; 2],
    pub payload: Vec<u8>,
}

impl Frame {
    pub fn is_ack(&self) -> bool {
        self.kind & 0x20 != 0
    }

    pub fn encode(&self) -> Result<Vec<u8>, ProtocolError> {
        let len = MIN_FRAME + self.payload.len();
        if len > MAX_FRAME {
            return Err(ProtocolError::Length);
        }
        if self.kind & 0xc0 != 0 {
            return Err(ProtocolError::Unsupported);
        }
        let mut bytes = vec![0xaa];
        bytes.extend_from_slice(&(len as u16).to_le_bytes());
        bytes.extend_from_slice(&[self.kind, 0, 0, 0, 0]);
        bytes.extend_from_slice(&self.seq.to_le_bytes());
        bytes.extend_from_slice(&crc16(&bytes).to_le_bytes());
        bytes.extend_from_slice(&self.command);
        bytes.extend_from_slice(&self.payload);
        bytes.extend_from_slice(&crc32(&bytes).to_le_bytes());
        Ok(bytes)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, ProtocolError> {
        if !(MIN_FRAME..=MAX_FRAME).contains(&bytes.len()) || bytes[0] != 0xaa {
            return Err(ProtocolError::Length);
        }
        let length = u16::from_le_bytes([bytes[1], bytes[2]]);
        if length & 0x3ff != bytes.len() as u16 {
            return Err(ProtocolError::Length);
        }
        if length >> 10 != 0 || bytes[4] != 0 || bytes[3] & 0xc0 != 0 {
            return Err(ProtocolError::Unsupported);
        }
        let end = bytes.len() - 4;
        if crc16(&bytes[..10]) != u16::from_le_bytes([bytes[10], bytes[11]])
            || crc32(&bytes[..end]) != u32::from_le_bytes(bytes[end..].try_into().unwrap())
        {
            return Err(ProtocolError::Checksum);
        }
        Ok(Self {
            kind: bytes[3],
            seq: u16::from_le_bytes([bytes[8], bytes[9]]),
            command: [bytes[12], bytes[13]],
            payload: bytes[14..end].to_vec(),
        })
    }
}

/// A bounded stream decoder. BLE notifications may split or combine frames.
#[derive(Default)]
pub struct Decoder {
    buffer: Vec<u8>,
}

impl Decoder {
    pub fn push(&mut self, input: &[u8]) -> Vec<Frame> {
        let mut frames = Vec::new();
        for byte in input {
            self.buffer.push(*byte);
            loop {
                let Some(start) = self.buffer.iter().position(|b| *b == 0xaa) else {
                    self.buffer.clear();
                    break;
                };
                self.buffer.drain(..start);
                if self.buffer.len() < 12 {
                    break;
                }
                let length = u16::from_le_bytes([self.buffer[1], self.buffer[2]]);
                let len = (length & 0x3ff) as usize;
                if !(MIN_FRAME..=MAX_FRAME).contains(&len)
                    || length >> 10 != 0
                    || self.buffer[4] != 0
                    || crc16(&self.buffer[..10])
                        != u16::from_le_bytes([self.buffer[10], self.buffer[11]])
                {
                    self.buffer.remove(0);
                    continue;
                }
                if self.buffer.len() < len {
                    break;
                }
                match Frame::decode(&self.buffer[..len]) {
                    Ok(frame) => {
                        frames.push(frame);
                        self.buffer.drain(..len);
                    }
                    Err(_) => {
                        self.buffer.remove(0);
                    }
                }
            }
        }
        frames
    }
}

fn crc16(input: &[u8]) -> u16 {
    let mut crc = 0x3aa3u16;
    for b in input {
        crc ^= *b as u16;
        for _ in 0..8 {
            crc = (crc >> 1) ^ if crc & 1 != 0 { 0xa001 } else { 0 };
        }
    }
    crc
}

fn crc32(input: &[u8]) -> u32 {
    let mut crc = 0x3aa3u32;
    for b in input {
        crc ^= *b as u32;
        for _ in 0..8 {
            crc = (crc >> 1) ^ if crc & 1 != 0 { 0xedb88320 } else { 0 };
        }
    }
    crc
}

#[cfg(test)]
mod tests {
    use super::*;

    // Official mode-switch example, including its nonzero reserved bytes.
    const SAMPLE: &[u8] = &[
        0xaa, 0x1b, 0, 1, 0, 0, 0, 0, 5, 0, 0x57, 0xee, 0x1d, 4, 0, 0, 0x33, 0xff, 0x0a, 1, 0x47,
        0x39, 0x36, 0xf4, 0xfa, 0xe1, 0xd0,
    ];

    #[test]
    fn official_frame_and_stream_recovery() {
        let frame = Frame::decode(SAMPLE).unwrap();
        assert_eq!(frame.command, [0x1d, 4]);
        assert_eq!(frame.encode().unwrap(), SAMPLE);
        let mut broken = SAMPLE.to_vec();
        broken[15] ^= 1;
        assert_eq!(Frame::decode(&broken), Err(ProtocolError::Checksum));
        for split in 0..SAMPLE.len() {
            let mut decoder = Decoder::default();
            assert!(decoder.push(&[0x55, 0, 0xaa, 0, 0]).is_empty());
            assert!(decoder.push(&broken).is_empty());
            assert!(decoder.push(&SAMPLE[..split]).is_empty());
            assert_eq!(decoder.push(&SAMPLE[split..]), vec![frame.clone()]);
            assert_eq!(
                decoder.push(&[SAMPLE, SAMPLE].concat()),
                vec![frame.clone(), frame.clone()]
            );
        }
    }
}
