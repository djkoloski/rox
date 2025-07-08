use core::fmt;

use rox_diag::{Diagnostic, Formatter, Span};

use crate::{DecodeError, Value};

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
    StackOverflow,
    StackUnderflow,
    BytecodeOutOfBounds,
    ConstantOutOfBounds,
    ObjectOutOfBounds,
    Decode(DecodeError),
    ExpectedFloat { actual: Value },
    ExpectedBoolean { actual: Value },
    ExpectedString { actual: Value },
    ExpectedFloatOrString { actual: Value },
    ExpectedVariableName { index: usize },
    GlobalAlreadyDefined { name: String, value: Value },
    UndefinedGlobal { name: String },
    LocalVariableOutOfBounds { index: usize },
}

impl From<DecodeError> for RuntimeError {
    fn from(value: DecodeError) -> Self {
        Self::Decode(value)
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StackOverflow => write!(f, "stack overflow")?,
            Self::StackUnderflow => write!(f, "stack underflow")?,
            Self::BytecodeOutOfBounds => write!(f, "bytecode out of bounds")?,
            Self::ConstantOutOfBounds => write!(f, "constant out of bounds")?,
            Self::ObjectOutOfBounds => write!(f, "object out of bounds")?,
            Self::Decode(e) => write!(f, "decode error: {e}")?,
            Self::ExpectedFloat { actual } => {
                write!(f, "expected float, got {actual:?}")?;
            }
            Self::ExpectedBoolean { actual } => {
                write!(f, "expected boolean, got {actual:?}")?;
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
        }

        Ok(())
    }
}
