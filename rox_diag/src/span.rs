#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Span {
    start: usize,
    end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
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

    pub fn get<'t>(&self, source: &'t str) -> &'t str {
        &source[self.start..self.end]
    }
}

pub trait Spanned {
    fn span_start(&self) -> usize;
    fn span_end(&self) -> usize;

    fn span(&self) -> Span {
        Span::new(self.span_start(), self.span_end())
    }
}

impl Spanned for Span {
    fn span_start(&self) -> usize {
        self.start()
    }

    fn span_end(&self) -> usize {
        self.end()
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
