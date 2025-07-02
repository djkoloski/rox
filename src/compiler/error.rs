use core::fmt;

use crate::{
    diagnostic::{Context, Diagnostic},
    span::Span,
};

#[derive(Debug)]
pub enum CompileError {
    UndefinedVariable { span: Span },
}

impl Diagnostic for CompileError {
    fn fmt(
        &self,
        c: &mut Context<'_>,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
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
        }
        Ok(())
    }
}
