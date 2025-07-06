use core::fmt;

use rox_diag::{Diagnostic, Formatter, Span};

#[derive(Debug)]
pub enum ParseError {
    ExpectedExpression(Span),
    UnterminatedGroup { start: Span, end: Span },
    UnterminatedStatement { stmt: Span, next: Span },
    UnterminatedBlock { start: Span, end: Span },
    ExpectedIdent(Span),
    ExpectedLeftParen(Span),
    ExpectedLeftBrace(Span),
    InvalidAssignmentTarget(Span),
    ExpectedSemicolon(Span),
    ExpectedDot(Span),
}

impl Diagnostic for ParseError {
    fn fmt(&self, f: &mut Formatter<'_, '_>) -> fmt::Result {
        match self {
            Self::ExpectedExpression(span) => {
                f.error(format_args!("unexpected token"))?;
                f.span_error(
                    *span,
                    format_args!(
                        "expected expression, found '{}'",
                        span.get(f.source())
                    ),
                )?;
            }
            Self::UnterminatedGroup { start, end } => {
                f.error(format_args!("unterminated group"))?;
                f.span_error(
                    *start,
                    format_args!(
                        "the group started here is missing a closing ')'"
                    ),
                )?;
                f.span_error(
                    *end,
                    format_args!(
                        "insert a ')' before '{}'",
                        end.get(f.source())
                    ),
                )?;
            }
            Self::UnterminatedStatement { stmt, next } => {
                f.error(format_args!("unterminated statement"))?;
                f.span_error(
                    *stmt,
                    format_args!(
                        "this statement was followed by '{}' instead of ';'",
                        next.get(f.source())
                    ),
                )?;
            }
            Self::UnterminatedBlock { start, end } => {
                f.error(format_args!("unterminated block"))?;
                f.span_error(
                    *start,
                    format_args!(
                        "the block started here is missing a closing '}}'"
                    ),
                )?;
                f.span_error(
                    *end,
                    format_args!(
                        "insert a '}}' before '{}'",
                        end.get(f.source())
                    ),
                )?;
            }
            Self::ExpectedIdent(span) => {
                f.error(format_args!("expected identifier"))?;
                f.span_error(
                    *span,
                    format_args!("expected an identifier here"),
                )?;
            }
            Self::ExpectedLeftParen(span) => {
                f.error(format_args!("missing opening parenthesis"))?;
                f.span_error(*span, format_args!("expected a '(' here"))?;
            }
            Self::ExpectedLeftBrace(span) => {
                f.error(format_args!("missing opening brace"))?;
                f.span_error(*span, format_args!("expected a '{{' here"))?;
            }
            Self::InvalidAssignmentTarget(span) => {
                f.error(format_args!("invalid assignment target"))?;
                f.span_error(
                    *span,
                    format_args!(
                        "this is the left-hand side of an assignment \
                         expression, but is not a place"
                    ),
                )?;
            }
            Self::ExpectedSemicolon(span) => {
                f.error(format_args!("missing semicolon"))?;
                f.span_error(*span, format_args!("expected a ';' here"))?;
            }
            Self::ExpectedDot(span) => {
                f.error(format_args!("missing dot"))?;
                f.span_error(*span, format_args!("expected a '.' here"))?;
            }
        }
        Ok(())
    }
}
