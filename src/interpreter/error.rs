use core::fmt;

use crate::{
    diagnostic::{Context, Diagnostic},
    interpreter::eval::Value,
    span::Span,
};

#[derive(Debug)]
pub enum InterpretError {
    #[allow(dead_code)]
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
    DivideByZero(Span),
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
            Self::DivideByZero(span) => {
                c.error(f, format_args!("attempted to divide by zero"))?;
                c.span(
                    *span,
                    f,
                    format_args!("this expression evaluated to zero"),
                )?;
            }
        }
        Ok(())
    }
}
