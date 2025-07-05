use core::fmt;

use crate::RuntimeError;

#[derive(Clone, Copy, Debug)]
pub enum Value {
    Float(f64),
}

impl Value {
    pub fn float(self) -> Result<f64, RuntimeError> {
        match self {
            Self::Float(n) => Ok(n),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Float(value) => write!(f, "{value}"),
        }
    }
}
