use core::{fmt, num::ParseFloatError};

use rox_diag::{Diagnostic, Formatter, Span};

#[derive(Debug)]
pub enum LexError {
    UnexpectedCharacter { span: Span, char: u8 },
    UnterminatedBlockComment { span: Span },
    UnterminatedString { span: Span },
    InvalidNumber { span: Span, error: ParseFloatError },
}

impl Diagnostic for LexError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedCharacter { span, char } => {
                f.error(format_args!("unexpected character"))?;
                f.span_error(
                    *span,
                    format_args!(
                        "'{}' (0x{char:x}) is not valid syntax",
                        *char as char
                    ),
                )?;
            }
            Self::UnterminatedBlockComment { span } => {
                f.error(format_args!("unterminated block comment"))?;
                f.span_error(
                    *span,
                    format_args!(
                        "this block comment is missing a closing tag (*/)"
                    ),
                )?;
            }
            Self::UnterminatedString { span } => {
                f.error(format_args!("unterminated string"))?;
                f.span_error(
                    *span,
                    format_args!("this string is missing a closing quote (\")"),
                )?;
            }
            Self::InvalidNumber { span, error } => {
                f.error(format_args!("invalid number"))?;
                f.span_error(
                    *span,
                    format_args!(
                        "failed to parse '{}' as a number: {error}",
                        span.get(f.source())
                    ),
                )?;
            }
        }
        Ok(())
    }
}
