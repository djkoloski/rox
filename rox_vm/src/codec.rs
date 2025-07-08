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
pub struct U8(u8);

impl U8 {
    pub const MAX: usize = (1 << 8) - 1;
}

impl From<usize> for U8 {
    fn from(value: usize) -> Self {
        Self(value as u8)
    }
}

impl From<U8> for usize {
    fn from(value: U8) -> Self {
        value.0 as usize
    }
}

impl Codec for U8 {
    fn encode(self, bytes: &mut Vec<u8>) {
        bytes.push(self.0)
    }

    fn decode(bytes: &[u8], offset: &mut usize) -> Result<Self, DecodeError> {
        Ok(Self(u8::decode(bytes, offset)?))
    }
}

#[derive(Debug)]
pub struct U24(usize);

impl U24 {
    pub const MAX: usize = (1 << 24) - 1;
}

impl From<usize> for U24 {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl From<U24> for usize {
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
            (u8::decode(bytes, offset)? as usize)
                | (u8::decode(bytes, offset)? as usize) << 8
                | (u8::decode(bytes, offset)? as usize) << 16,
        ))
    }
}
