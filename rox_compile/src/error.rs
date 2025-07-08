use core::fmt;

use rox_diag::{Diagnostic, Formatter, Span};

#[derive(Debug)]
pub enum CompileError {
    LocalRedefined { original: Span, redefinition: Span },
}

impl Diagnostic for CompileError {
    fn fmt(&self, f: &mut Formatter<'_, '_>) -> fmt::Result {
        match self {
            Self::LocalRedefined {
                original,
                redefinition,
            } => {
                f.error(format_args!("local variable already defined"))?;
                f.span_error(
                    *redefinition,
                    format_args!(
                        "'{}' was already defined",
                        redefinition.get(f.source())
                    ),
                )?;
                f.span_help(
                    *original,
                    format_args!("previously defined here"),
                )?;
            }
        }

        Ok(())
    }
}
