use core::fmt;

use rox_diag::{Diagnostic, Formatter, Span};

#[derive(Debug)]
pub enum CompileError {
    UndefinedItem { span: Span },
    ItemRedefined { original: Span, redefinition: Span },
}

impl Diagnostic for CompileError {
    fn fmt(&self, f: &mut Formatter<'_, '_>) -> fmt::Result {
        match self {
            Self::UndefinedItem { span } => {
                f.error(format_args!(
                    "undefined item '{}'",
                    span.get(f.source())
                ))?;
                f.span_error(*span, format_args!("referenced here"))?;
            }
            Self::ItemRedefined {
                original,
                redefinition,
            } => {
                f.error(format_args!("duplicate item definition"))?;
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
