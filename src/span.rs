use core::fmt;

#[derive(Clone, Copy, Debug)]
pub struct Span {
    line: usize,
}

impl Span {
    pub fn for_line(line: usize) -> Self {
        Self { line }
    }

    pub fn eof() -> Self {
        Self { line: usize::MAX }
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}", self.line)
    }
}

pub struct Spanned<T> {
    pub inner: T,
    pub span: Span,
}
