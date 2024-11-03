pub struct Position {
    line_start: usize,
    line_end: usize,
    line_number: usize,
    column: usize,
}

impl Position {
    pub fn in_text(text: &str, offset: usize) -> Self {
        let mut matches = text[..offset].rmatch_indices('\n').fuse();

        let line_start = matches.next().map(|(p, _)| p + 1).unwrap_or(0);
        Self {
            line_start,
            line_end: offset + text[offset..].find('\n').unwrap_or(text.len() - offset),
            line_number: 1 + matches.count(),
            column: offset - line_start,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Span {
    start: usize,
    end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        debug_assert!(start <= end);

        Self {
            start,
            end,
        }
    }

    pub fn merge(begin: Span, end: Span) -> Self {
        Self {
            start: begin.start,
            end: end.end,
        }
    }

    pub fn eof() -> Self {
        Self {
            start: usize::MAX,
            end: usize::MAX,
        }
    }

    fn is_eof(&self) -> bool {
        self.start == usize::MAX && self.end == usize::MAX
    }

    pub fn get<'t>(&self, text: &'t str) -> &'t str {
        if self.is_eof() {
            &""
        } else {
            &text[self.start..self.end]
        }
    }

    pub fn indicate(&self, text: &str) {
        let (start, end) = if self.is_eof() {
            (Position::in_text(text, text.len()), Position::in_text(text, text.len()))
        } else {
            (Position::in_text(text, self.start), Position::in_text(text, self.end))
        };

        let width = u32::max(start.line_number.ilog10(), end.line_number.ilog10()) + 1;

        if start.line_number == end.line_number {
            println!(
                "{:width$} | {}",
                start.line_number,
                &text[start.line_start..start.line_end],
                width = width as usize,
            );

            println!(
                "{:width$} | {:column$}{:^^length$} ",
                "",
                "",
                "",
                width = width as usize,
                column = start.column,
                length = end.column - start.column,
            );
        } else {
            todo!()
        }
    }
}

pub struct Spanned<T> {
    pub inner: T,
    pub span: Span,
}
