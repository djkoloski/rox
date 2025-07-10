use core::fmt;
use std::time::SystemTimeError;

use rox_diag::{Diagnostic, Formatter, Span};

use crate::{DecodeError, InvalidNativeFunction, UnpackedValue};

#[derive(Debug)]
pub struct RuntimeDiagnostic {
    error: RuntimeError,
    span: Span,
}

impl RuntimeDiagnostic {
    pub fn new(error: RuntimeError, span: Span) -> Self {
        Self { error, span }
    }
}

impl Diagnostic for RuntimeDiagnostic {
    fn fmt(&self, f: &mut Formatter<'_, '_>) -> fmt::Result {
        f.error(format_args!("{}", self.error))?;
        f.span_error(
            self.span,
            format_args!("while executing this operation"),
        )?;

        Ok(())
    }
}

#[derive(Debug)]
pub enum RuntimeError {
    Decode(DecodeError),
    SystemTimeError(SystemTimeError),
    InvalidNativeFunction(InvalidNativeFunction),

    StackOverflow,
    StackUnderflow,
    ConstantOutOfBounds,
    ExpectedFloat { actual: UnpackedValue },
    ExpectedString { actual: UnpackedValue },
    ExpectedFloatOrString { actual: UnpackedValue },
    ExpectedVariableName { index: usize },
    ExpectedFunction { actual: UnpackedValue },
    GlobalAlreadyDefined { name: String, value: UnpackedValue },
    UndefinedGlobal { name: String },
    LocalVariableOutOfBounds { index: usize },
    TooFewArguments { arity: usize },
}

impl From<DecodeError> for RuntimeError {
    fn from(value: DecodeError) -> Self {
        Self::Decode(value)
    }
}

impl From<SystemTimeError> for RuntimeError {
    fn from(value: SystemTimeError) -> Self {
        Self::SystemTimeError(value)
    }
}

impl From<InvalidNativeFunction> for RuntimeError {
    fn from(value: InvalidNativeFunction) -> Self {
        Self::InvalidNativeFunction(value)
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Decode(e) => write!(f, "decode error: {e}")?,
            Self::SystemTimeError(e) => write!(f, "system time error: {e}")?,
            Self::InvalidNativeFunction(e) => {
                write!(f, "invalid native function: {}", e.index)?
            }

            Self::StackOverflow => write!(f, "stack overflow")?,
            Self::StackUnderflow => write!(f, "stack underflow")?,
            Self::ConstantOutOfBounds => write!(f, "constant out of bounds")?,
            Self::ExpectedFloat { actual } => {
                write!(f, "expected float, got {actual:?}")?;
            }
            Self::ExpectedString { actual } => {
                write!(f, "expected string, got {actual:?}")?;
            }
            Self::ExpectedFloatOrString { actual } => {
                write!(f, "expected float or string, got {actual:?}")?;
            }
            Self::ExpectedVariableName { index } => {
                write!(f, "expected a variable name in constant {index}")?;
            }
            Self::ExpectedFunction { actual } => {
                write!(f, "expected function, got {actual:?}")?;
            }
            Self::GlobalAlreadyDefined { name, value } => {
                write!(
                    f,
                    "global variable '{name}' was already defined with value \
                     {value:?}"
                )?;
            }
            Self::UndefinedGlobal { name } => {
                write!(f, "global variable '{name}' was undefined")?;
            }
            Self::LocalVariableOutOfBounds { index } => {
                write!(f, "local variable #{index} was out-of-bounds")?;
            }
            Self::TooFewArguments { arity } => {
                write!(
                    f,
                    "too few arguments to call function of arity {arity}"
                )?;
            }
        }

        Ok(())
    }
}
