use core::fmt;

use rox_diag::{Diagnostic, Formatter};

use crate::DecodeError;

#[derive(Debug)]
pub enum RuntimeError {
    StackOverflow,
    StackUnderflow,
    BytecodeOutOfBounds,
    ConstantOutOfBounds,
    Decode(DecodeError),
}

impl From<DecodeError> for RuntimeError {
    fn from(value: DecodeError) -> Self {
        Self::Decode(value)
    }
}

impl Diagnostic for RuntimeError {
    fn fmt(&self, f: &mut Formatter<'_, '_>) -> fmt::Result {
        match self {
            Self::StackOverflow => f.error(format_args!("stack overflow"))?,
            Self::StackUnderflow => f.error(format_args!("stack underflow"))?,
            Self::BytecodeOutOfBounds => {
                f.error(format_args!("bytecode out of bounds"))?
            }
            Self::ConstantOutOfBounds => {
                f.error(format_args!("constant out of bounds"))?
            }
            Self::Decode(e) => f.error(format_args!("decode error: {e}"))?,
        }

        Ok(())
    }
}
