use core::fmt;

use crate::Span;

#[derive(Debug)]
struct Position {
    pub line_start: usize,
    pub line_end: usize,
    pub line_number: usize,
    pub column: usize,
}

impl Position {
    fn from_source(text: &str, offset: usize) -> Self {
        let mut matches = text[..offset].rmatch_indices('\n').fuse();

        let line_end =
            offset + text[offset..].find('\n').unwrap_or(text.len() - offset);
        if let Some((pos, _)) = matches.next() {
            Self {
                line_start: pos + 1,
                line_end,
                line_number: 2 + matches.count(),
                column: offset - pos - 1,
            }
        } else {
            Self {
                line_start: 0,
                line_end,
                line_number: 1,
                column: offset,
            }
        }
    }
}

pub struct Context<'s> {
    source: &'s str,
}

const BRIGHT_RED: &str = "\x1b[31;1m";
const BRIGHT_WHITE: &str = "\x1b[97m";
const BRIGHT_CYAN: &str = "\x1b[96;1m";
const RESET_COLOR: &str = "\x1b[0m";

impl<'s> Context<'s> {
    pub fn new(source: &'s str) -> Self {
        Self { source }
    }

    pub fn source(&self) -> &'s str {
        self.source
    }

    pub fn error(
        &mut self,
        f: &mut fmt::Formatter<'_>,
        args: fmt::Arguments<'_>,
    ) -> fmt::Result {
        writeln!(f, "{BRIGHT_RED}error{BRIGHT_WHITE}: {args}{RESET_COLOR}")
    }

    pub fn span(
        &mut self,
        span: Span,
        f: &mut fmt::Formatter<'_>,
        args: fmt::Arguments<'_>,
    ) -> fmt::Result {
        let start = Position::from_source(self.source, span.start());
        let end = Position::from_source(self.source, span.end());

        let width =
            u32::max(start.line_number.ilog10(), end.line_number.ilog10()) + 1;

        writeln!(
            f,
            "{BRIGHT_CYAN}{:width$} |{RESET_COLOR}",
            "",
            width = width as usize,
        )?;

        writeln!(
            f,
            "{BRIGHT_CYAN}{:width$} |{RESET_COLOR}   {}",
            start.line_number,
            &self.source[start.line_start..start.line_end],
            width = width as usize,
        )?;

        if start.line_number == end.line_number {
            writeln!(
                f,
                "{BRIGHT_CYAN}{:width$} |{RESET_COLOR}   \
                 {:column$}{BRIGHT_RED}{:^^length$} {args}{RESET_COLOR}",
                "",
                "",
                "",
                width = width as usize,
                column = start.column,
                length = usize::max(1, end.column - start.column),
            )?;
        } else {
            writeln!(
                f,
                "{BRIGHT_CYAN}{:width$} |{RESET_COLOR}  \
                 {BRIGHT_RED}_{:_^column$}^{RESET_COLOR}",
                "",
                "",
                width = width as usize,
                column = start.column,
            )?;
            writeln!(
                f,
                "{BRIGHT_CYAN}{:width$} |{RESET_COLOR} \
                 {BRIGHT_RED}|{RESET_COLOR} {}",
                end.line_number,
                &self.source[end.line_start..end.line_end],
                width = width as usize,
            )?;
            writeln!(
                f,
                "{:width$} {BRIGHT_CYAN}|{RESET_COLOR} \
                 {BRIGHT_RED}|_{:_^column$}^ {args}{RESET_COLOR}",
                "",
                "",
                width = width as usize,
                column = end.column,
            )?;
        }

        Ok(())
    }
}

pub trait Diagnostic {
    fn fmt(
        &self,
        c: &mut Context<'_>,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result;
}
