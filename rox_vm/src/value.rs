use core::fmt;

use crate::RuntimeError;

#[derive(Debug, PartialEq)]
pub enum Constant {
    Float(f64),
    String(String),
    Function(usize),
}

impl fmt::Display for Constant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Float(value) => write!(f, "{value}"),
            Self::String(value) => write!(f, "{value}"),
            Self::Function(index) => write!(f, "<fun {index}>"),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Float(f64),
    Boolean(bool),
    Nil,
    String(usize),
    Function(usize),
    FramePointer(usize),
    InstructionPointer(usize),
}

impl Value {
    pub fn truthiness(&self) -> bool {
        match self {
            Self::Float(_) | Self::String(_) | Self::Function(_) => true,
            Self::Boolean(b) => *b,
            Self::Nil => false,
            Self::FramePointer(_) | Self::InstructionPointer(_) => {
                unreachable!()
            }
        }
    }

    pub fn float(self) -> Result<f64, RuntimeError> {
        match self {
            Self::Float(n) => Ok(n),
            _ => Err(RuntimeError::ExpectedFloat { actual: self }),
        }
    }

    pub fn boolean(self) -> Result<bool, RuntimeError> {
        match self {
            Self::Boolean(b) => Ok(b),
            _ => Err(RuntimeError::ExpectedBoolean { actual: self }),
        }
    }
}
