use core::fmt;

use crate::{
    diagnostic::{Context, Diagnostic},
    interpreter::eval::{Function, Value},
    span::Span,
};

#[derive(Debug)]
pub enum InterpretError {
    ExpectedBoolean {
        span: Span,
        actual: Value,
    },
    ExpectedNumber {
        span: Span,
        actual: Value,
    },
    ExpectedString {
        span: Span,
        actual: Value,
    },
    ExpectedNumberOrString {
        span: Span,
        actual: Value,
    },
    ExpectedFunction {
        span: Span,
        actual: Value,
    },
    DivideByZero(Span),
    UndefinedVariable {
        span: Span,
    },
    UninitializedVariable(Span),
    IncorrectFunctionArity {
        span: Span,
        callee: Function,
        expected: usize,
        actual: usize,
    },
}

impl Diagnostic for InterpretError {
    fn fmt(
        &self,
        c: &mut Context<'_>,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            Self::ExpectedBoolean { span, actual } => {
                c.error(
                    f,
                    format_args!(
                        "expected expression to evaluate to a boolean"
                    ),
                )?;
                c.span(
                    *span,
                    f,
                    format_args!(
                        "this expression evaluated to '{actual}' instead of a \
                         boolean"
                    ),
                )?;
            }
            Self::ExpectedNumber { span, actual } => {
                c.error(
                    f,
                    format_args!("expected expression to evaluate to a number"),
                )?;
                c.span(
                    *span,
                    f,
                    format_args!(
                        "this expression evaluated to '{actual}' instead of a \
                         number"
                    ),
                )?;
            }
            Self::ExpectedString { span, actual } => {
                c.error(
                    f,
                    format_args!("expected expression to evaluate to a string"),
                )?;
                c.span(
                    *span,
                    f,
                    format_args!(
                        "this expression evaluated to '{actual}' instead of a \
                         string"
                    ),
                )?;
            }
            Self::ExpectedNumberOrString { span, actual } => {
                c.error(
                    f,
                    format_args!(
                        "expected expression to evaluate to a number or a \
                         string"
                    ),
                )?;
                c.span(
                    *span,
                    f,
                    format_args!(
                        "this expression evaluated to '{actual}' instead of a \
                         number or string"
                    ),
                )?;
            }
            Self::ExpectedFunction { span, actual } => {
                c.error(
                    f,
                    format_args!(
                        "expected callee expression to evaluate to a function"
                    ),
                )?;
                c.span(
                    *span,
                    f,
                    format_args!(
                        "this expression evaluated to '{actual}' instead of a \
                         function"
                    ),
                )?;
            }
            Self::DivideByZero(span) => {
                c.error(f, format_args!("attempted to divide by zero"))?;
                c.span(
                    *span,
                    f,
                    format_args!("this expression evaluated to zero"),
                )?;
            }
            Self::UndefinedVariable { span } => {
                c.error(
                    f,
                    format_args!(
                        "undefined variable '{}'",
                        span.get(c.source())
                    ),
                )?;
                c.span(
                    *span,
                    f,
                    format_args!("this variable has not been defined"),
                )?;
            }
            Self::UninitializedVariable(span) => {
                c.error(
                    f,
                    format_args!("variable not initialized before use"),
                )?;
                c.span(
                    *span,
                    f,
                    format_args!(
                        "'{}' was declared, but wasn't assigned a value \
                         before being used here",
                        span.get(c.source())
                    ),
                )?;
            }
            Self::IncorrectFunctionArity {
                span,
                callee,
                expected,
                actual,
            } => {
                c.error(
                    f,
                    format_args!("function callee has incorrect arity"),
                )?;
                c.span(
                    *span,
                    f,
                    format_args!(
                        "'{callee}' has arity {expected}, but was called with \
                         arity {actual}"
                    ),
                )?;
            }
        }
        Ok(())
    }
}
