use core::fmt;

use crate::span::{Position, Span};

pub struct Context<'s> {
    source: &'s str,
}

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
        writeln!(f, "\x1b[31;1merror\x1b[97m: {args}\x1b[0m")
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
            "\x1b[96;1m{:width$} |\x1b[0m",
            "",
            width = width as usize,
        )?;

        writeln!(
            f,
            "\x1b[96;1m{:width$} |\x1b[0m   {}",
            start.line_number,
            &self.source[start.line_start..start.line_end],
            width = width as usize,
        )?;

        if start.line_number == end.line_number {
            writeln!(
                f,
                "\x1b[96;1m{:width$} |\x1b[0m   \
                 {:column$}\x1b[31;1m{:^^length$} {args}\x1b[0m",
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
                "\x1b[96;1m{:width$} |\x1b[0m  \x1b[31;1m_{:_^column$}^\x1b[0m",
                "",
                "",
                width = width as usize,
                column = start.column,
            )?;
            writeln!(
                f,
                "\x1b[96;1m{:width$} |\x1b[0m \x1b[31;1m|\x1b[0m {}",
                end.line_number,
                &self.source[end.line_start..end.line_end],
                width = width as usize,
            )?;
            writeln!(
                f,
                "{:width$} \x1b[96;1m|\x1b[0m \x1b[31;1m|_{:_^column$}^ \
                 {args}\x1b[0m",
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
