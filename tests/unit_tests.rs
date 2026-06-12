use serde::{Deserialize, Serialize};
use std::fmt;
use std::io::{Cursor, Read};
use std::ops::Deref;

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct CompactSize {
    pub value: u64,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum BitcoinError {
    InsufficientBytes,
    InvalidFormat,
}

impl CompactSize {
    pub fn new(value: u64) -> Self {
        Self { value }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        if self.value <= 0xFC {
            vec![self.value as u8]
        } else if self.value <= 0xFFFF {
            let mut buf = Vec::with_capacity(3);
            buf.push(0xFD);
            buf.extend((self.value as u16).to_le_bytes());
            buf
        } else if self.value <= 0xFFFFFFFF {
            let mut buf = Vec::with_capacity(5);
            buf.push(0xFE);
            buf.extend((self.value as u32).to_le_bytes());
            buf
        } else {
            let mut buf = Vec::with_capacity(9);
            buf.push(0xFF);
            buf.extend(self.value.to_le_bytes());
            buf
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<(Self, usize), BitcoinError> {
        if bytes.is_empty() {
            return Err(BitcoinError::InsufficientBytes);
        }

        let mut cursor = Cursor::new(bytes);
        let mut leading_byte = [0u8; 1];
        let _ = cursor.read_exact(&mut leading_byte);

        match leading_byte[0] {
            0x00..=0xFC => Ok((
                Self {
                    value: leading_byte[0] as u64,
                },
                1,
            )),
            0xFD => {
                let mut buf = [0u8; 2];
                cursor.read_exact(&mut buf).unwrap();

                Ok((
                    Self {
                        value: u16::from_le_bytes(buf) as u64,
                    },
                    3,
                ))
            }
            0xFE => {
                let mut buf = [0u8; 4];
                cursor.read_exact(&mut buf).unwrap();

                Ok((
                    Self {
                        value: u32::from_le_bytes(buf) as u64,
                    },
                    5,
                ))
            }
            0xFF => {
                let mut buf = [0u8; 8];
                cursor.read_exact(&mut buf).unwrap();

                Ok((
                    Self {
                        value: u64::from_le_bytes(buf),
                    },
                    9,
                ))
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Txid(pub [u8; 32]);

impl Serialize for Txid {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let hex_string = hex::encode(self.0);
        serializer.serialize_str(&hex_string)
    }
}

impl<'de> Deserialize<'de> for Txid {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let hex_string = String::deserialize(deserializer)?;
        let bytes = hex::decode(&hex_string).expect("Failed to parse");

        if bytes.len() != 32 {
            Err(serde::de::Error::custom("Byte length not sufficient"))
        } else {
            let mut buf = [0u8; 32];
            buf.copy_from_slice(&bytes);
            Ok(Txid(buf))
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct OutPoint {
    pub txid: Txid,
    pub vout: u32,
}

impl OutPoint {
    pub fn new(txid: [u8; 32], vout: u32) -> Self {
        Self {
            txid: Txid(txid),
            vout,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::new();

        buf.extend(self.txid.0);
        buf.extend(self.vout.to_le_bytes());
        buf
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<(Self, usize), BitcoinError> {
        if bytes.len() != 36 {
            return Err(BitcoinError::InsufficientBytes);
        }

        let mut cursor = Cursor::new(bytes);

        let mut txid_buf = [0u8; 32];
        cursor.read_exact(&mut txid_buf).unwrap();

        let mut vout_buf = [0u8; 4];
        cursor.read_exact(&mut vout_buf).unwrap();

        let vout = u32::from_le_bytes(vout_buf);

        Ok((
            Self {
                vout,
                txid: Txid(txid_buf),
            },
            36,
        ))
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Script {
    pub bytes: Vec<u8>,
}

impl Script {
    pub fn new(bytes: Vec<u8>) -> Self {
        Script { bytes }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let compact_size = CompactSize::new(self.bytes.len() as u64).to_bytes();

        let mut buf = Vec::new();
        buf.extend(compact_size);
        buf.extend(&self.bytes);

        buf
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<(Self, usize), BitcoinError> {
        let (compact_size, prefix_len) = CompactSize::from_bytes(bytes)?;
        let script_len = compact_size.value as usize;

        let start = prefix_len;
        let end = start + script_len;

        if bytes.len() < end {
            return Err(BitcoinError::InsufficientBytes);
        }

        let script_bytes = bytes[start..end].to_vec();

        Ok((
            Self {
                bytes: script_bytes,
            },
            prefix_len + script_len,
        ))
    }
}

impl Deref for Script {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.bytes
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct TransactionInput {
    pub previous_output: OutPoint,
    pub script_sig: Script,
    pub sequence: u32,
}

impl TransactionInput {
    pub fn new(previous_output: OutPoint, script_sig: Script, sequence: u32) -> Self {
        Self {
            previous_output,
            script_sig,
            sequence,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();

        let outpoint = self.previous_output.to_bytes();
        buf.extend_from_slice(&outpoint);

        let script = self.script_sig.to_bytes();
        buf.extend_from_slice(&script);

        let sequence = self.sequence.to_le_bytes();
        buf.extend(sequence);

        buf
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<(Self, usize), BitcoinError> {
        let mut offset = 0;

        let (outpoint, outpoint_size) = OutPoint::from_bytes(&bytes[offset..])?;
        offset += outpoint_size;

        let (script, script_size) = Script::from_bytes(&bytes[offset..])?;
        offset += script_size;

        // Sequence: 4 bytes LE
        if bytes.len() < offset + 4 {
            return Err(BitcoinError::InsufficientBytes);
        }
        let mut sequence_bytes = [0u8; 4];
        sequence_bytes.copy_from_slice(&bytes[offset..offset + 4]);
        offset += 4;

        let sequence = u32::from_le_bytes(sequence_bytes);

        let input = Self {
            previous_output: outpoint,
            script_sig: script,
            sequence,
        };

        Ok((input, offset))
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct BitcoinTransaction {
    pub version: u32,
    pub inputs: Vec<TransactionInput>,
    pub lock_time: u32,
}

impl BitcoinTransaction {
    pub fn new(version: u32, inputs: Vec<TransactionInput>, lock_time: u32) -> Self {
        Self {
            version,
            inputs,
            lock_time,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();

        // version (4 bytes LE)
        buf.extend(self.version.to_le_bytes());

        // input count as CompactSize
        let input_count = CompactSize::new(self.inputs.len() as u64).to_bytes();
        buf.extend(input_count);

        // each input serialized
        for input in &self.inputs {
            buf.extend(input.to_bytes());
        }

        // lock_time (4 bytes LE)
        buf.extend(self.lock_time.to_le_bytes());

        buf
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<(Self, usize), BitcoinError> {
        let mut offset = 0;

        // version (4 bytes LE)
        if bytes.len() < 4 {
            return Err(BitcoinError::InsufficientBytes);
        }
        let mut version_bytes = [0u8; 4];
        version_bytes.copy_from_slice(&bytes[offset..offset + 4]);
        let version = u32::from_le_bytes(version_bytes);
        offset += 4;

        // input count CompactSize
        let (input_count, prefix_len) = CompactSize::from_bytes(&bytes[offset..])?;
        offset += prefix_len;

        let mut inputs = Vec::new();
        for _ in 0..input_count.value {
            let (input, input_size) = TransactionInput::from_bytes(&bytes[offset..])?;
            offset += input_size;
            inputs.push(input);
        }

        // lock_time (4 bytes LE)
        if bytes.len() < offset + 4 {
            return Err(BitcoinError::InsufficientBytes);
        }
        let mut lock_time_bytes = [0u8; 4];
        lock_time_bytes.copy_from_slice(&bytes[offset..offset + 4]);
        let lock_time = u32::from_le_bytes(lock_time_bytes);
        offset += 4;

        Ok((
            Self {
                version,
                inputs,
                lock_time,
            },
            offset,
        ))
    }
}

impl fmt::Display for BitcoinTransaction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "BitcoinTransaction {{")?;
        writeln!(f, "  Version: {}", self.version)?;
        writeln!(f, "  Inputs ({}):", self.inputs.len())?;
        for (i, input) in self.inputs.iter().enumerate() {
            writeln!(
                f,
                "    [{}] Previous Output Txid: {}, Previous Output Vout: {}",
                i,
                hex::encode(input.previous_output.txid.0),
                input.previous_output.vout
            )?;
            writeln!(
                f,
                "         Script Sig: len={}, bytes={:?}",
                input.script_sig.bytes.len(),
                input.script_sig.bytes
            )?;
            writeln!(f, "         Sequence: {}", input.sequence)?;
        }
        writeln!(f, "  Lock Time: {}", self.lock_time)?;
        write!(f, "}}")
    }
}
