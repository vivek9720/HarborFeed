use crate::error::{HarborError, Result};

#[derive(Debug, Clone)]
pub struct BitCursor {
    bits: Vec<u8>,
    pos: usize,
}

impl BitCursor {
    pub fn from_sixbit_payload(payload: &str, fill_bits: u8) -> Result<Self> {
        if fill_bits > 5 {
            return Err(HarborError::InvalidAisPayload);
        }
        let mut bits = Vec::with_capacity(payload.len() * 6);
        for ch in payload.bytes() {
            let value = sixbit_value(ch)?;
            for shift in (0..6).rev() {
                bits.push((value >> shift) & 1);
            }
        }
        for _ in 0..fill_bits {
            let _ = bits.pop();
        }
        Ok(Self { bits, pos: 0 })
    }

    pub fn remaining(&self) -> usize {
        self.bits.len().saturating_sub(self.pos)
    }

    pub fn read_u64(&mut self, width: usize) -> Result<u64> {
        if width > 64 || self.remaining() < width {
            return Err(HarborError::InvalidAisPayload);
        }
        let mut value = 0u64;
        for _ in 0..width {
            value = (value << 1) | self.bits[self.pos] as u64;
            self.pos += 1;
        }
        Ok(value)
    }

    pub fn read_u32(&mut self, width: usize) -> Result<u32> {
        Ok(self.read_u64(width)? as u32)
    }

    pub fn read_u8(&mut self, width: usize) -> Result<u8> {
        Ok(self.read_u64(width)? as u8)
    }

    pub fn read_bool(&mut self) -> Result<bool> {
        Ok(self.read_u64(1)? != 0)
    }

    pub fn read_signed(&mut self, width: usize) -> Result<i32> {
        if width == 0 || width > 31 {
            return Err(HarborError::InvalidAisPayload);
        }
        let raw = self.read_u64(width)? as i32;
        let sign = 1i32 << (width - 1);
        if raw & sign == 0 {
            Ok(raw)
        } else {
            Ok(raw - (1i32 << width))
        }
    }

    pub fn skip(&mut self, width: usize) -> Result<()> {
        let _ = self.read_u64(width)?;
        Ok(())
    }

    pub fn read_text(&mut self, chars: usize) -> Result<String> {
        let mut out = String::new();
        for _ in 0..chars {
            let value = self.read_u8(6)?;
            let b = ais_text_char(value);
            if b != b'@' {
                out.push(b as char);
            }
        }
        Ok(out.trim().to_string())
    }
}

pub fn sixbit_value(ch: u8) -> Result<u8> {
    if !(48..=119).contains(&ch) {
        return Err(HarborError::InvalidAisPayload);
    }
    let mut v = ch - 48;
    if v > 40 {
        v -= 8;
    }
    if v > 63 {
        return Err(HarborError::InvalidAisPayload);
    }
    Ok(v)
}

pub fn ais_text_char(value: u8) -> u8 {
    match value {
        0 => b'@',
        1..=26 => b'A' + value - 1,
        27 => b'[',
        28 => b'\\',
        29 => b']',
        30 => b'^',
        31 => b'_',
        32 => b' ',
        33..=57 => b'!' + (value - 33),
        58..=63 => b':' + (value - 58),
        _ => b' ',
    }
}

pub fn nmea_checksum(body: &str) -> u8 {
    body.bytes().fold(0u8, |acc, b| acc ^ b)
}

pub fn checksum64(data: &[u8]) -> u64 {
    let mut acc = 0x9e37_79b9_7f4a_7c15u64 ^ data.len() as u64;
    for (i, b) in data.iter().enumerate() {
        let lane = (*b as u64).wrapping_add(((i as u64) << 17) ^ 0xa076_1d64_78bd_642f);
        acc ^= lane.rotate_left((i & 31) as u32);
        acc = acc.wrapping_mul(0xe703_7ed1_a0b4_28db).rotate_left(9);
    }
    acc ^ (acc >> 33)
}

pub fn fold_pair(a: u64, b: u64) -> u64 {
    let mut x = a ^ b.rotate_left(17);
    x = x.wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 29)
}
