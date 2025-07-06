use core::fmt;

use crate::RuntimeError;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Value {
    Float(f64),
    Boolean(bool),
    Nil,
}

impl Value {
    pub fn truthiness(&self) -> bool {
        match self {
            Self::Float(_) => true,
            Self::Boolean(b) => *b,
            Self::Nil => false,
        }
    }

    pub fn float(self) -> Result<f64, RuntimeError> {
        match self {
            Self::Float(n) => Ok(n),
            _ => Err(RuntimeError::ExpectedFloat(self)),
        }
    }

    pub fn boolean(self) -> Result<bool, RuntimeError> {
        match self {
            Self::Boolean(b) => Ok(b),
            _ => Err(RuntimeError::ExpectedBoolean(self)),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Float(value) => write!(f, "{value}"),
            Self::Boolean(value) => write!(f, "{value}"),
            Self::Nil => write!(f, "<nil>"),
        }
    }
}
