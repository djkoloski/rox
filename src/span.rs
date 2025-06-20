#[derive(Debug)]
pub struct Position {
    pub line_start: usize,
    pub line_end: usize,
    pub line_number: usize,
    pub column: usize,
}

impl Position {
    pub fn from_source(text: &str, offset: usize) -> Self {
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

#[derive(Clone, Copy, Debug)]
pub struct Span {
    start: usize,
    end: usize,
}

impl Span {
    pub fn scan(start: usize, end: usize) -> Self {
        debug_assert!(start <= end);

        Self { start, end }
    }

    pub fn across(left: &impl Spanned, right: &impl Spanned) -> Self {
        Self {
            start: left.span_start(),
            end: right.span_end(),
        }
    }

    pub fn start(&self) -> usize {
        self.start
    }

    pub fn end(&self) -> usize {
        self.end
    }

    pub fn get<'t>(&self, text: &'t str) -> &'t str {
        &text[self.start..self.end]
    }
}

pub trait Spanned {
    fn span_start(&self) -> usize;
    fn span_end(&self) -> usize;

    fn span(&self) -> Span {
        Span::scan(self.span_start(), self.span_end())
    }
}

impl<T: Spanned + ?Sized> Spanned for Box<T> {
    fn span_start(&self) -> usize {
        T::span_start(self)
    }

    fn span_end(&self) -> usize {
        T::span_end(self)
    }
}

impl<T0: Spanned, T1: Spanned> Spanned for (T0, T1) {
    fn span_start(&self) -> usize {
        self.0.span_start()
    }

    fn span_end(&self) -> usize {
        self.1.span_end()
    }
}
