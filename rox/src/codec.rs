use core::fmt;

#[derive(Debug)]
pub enum DecodeError {
    InsufficientBytes,
    InvalidOpcode { byte: u8 },
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InsufficientBytes => write!(f, "insufficient bytes"),
            Self::InvalidOpcode { byte } => {
                write!(f, "unknown opcode {byte:>4x}")
            }
        }
    }
}

pub trait Codec: Sized {
    fn encode(self, bytes: &mut Vec<u8>);
    fn decode(bytes: &[u8], offset: &mut usize) -> Result<Self, DecodeError>;
}

impl Codec for u8 {
    fn encode(self, bytes: &mut Vec<u8>) {
        bytes.push(self);
    }

    fn decode(bytes: &[u8], offset: &mut usize) -> Result<Self, DecodeError> {
        let result =
            *bytes.get(*offset).ok_or(DecodeError::InsufficientBytes)?;
        *offset += 1;
        Ok(result)
    }
}

#[derive(Debug)]
pub struct U24(u32);

impl From<u32> for U24 {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl From<U24> for u32 {
    fn from(value: U24) -> Self {
        value.0
    }
}

impl Codec for U24 {
    fn encode(self, bytes: &mut Vec<u8>) {
        bytes.extend_from_slice(&self.0.to_le_bytes()[0..3]);
    }

    fn decode(bytes: &[u8], offset: &mut usize) -> Result<Self, DecodeError> {
        Ok(Self(
            (u8::decode(bytes, offset)? as u32)
                | (u8::decode(bytes, offset)? as u32) << 8
                | (u8::decode(bytes, offset)? as u32) << 16,
        ))
    }
}
