use core::{fmt, marker::PhantomData};

pub struct Decoration<T> {
    index: usize,
    _phantom: PhantomData<T>,
}

impl<T> Decoration<T> {
    pub fn index(&self) -> usize {
        self.index
    }
}

impl<T> fmt::Debug for Decoration<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Decoration")
    }
}

pub struct Decorator {
    counters: Vec<usize>,
}

impl Decorator {
    pub fn new() -> Self {
        Self {
            counters: vec![0; COUNTERS_MAX],
        }
    }

    pub fn decorate<T: DecorationKind>(&mut self) -> Decoration<T> {
        let counter = &mut self.counters[T::COUNTER_INDEX];
        let index = *counter;
        *counter += 1;
        Decoration {
            index,
            _phantom: PhantomData,
        }
    }

    pub fn count<T: DecorationKind>(&self) -> usize {
        self.counters[T::COUNTER_INDEX]
    }
}

impl Default for Decorator {
    fn default() -> Self {
        Self::new()
    }
}

const COUNTERS_MAX: usize = 2;

pub trait DecorationKind {
    const COUNTER_INDEX: usize;
}

pub struct NameResolution;

impl DecorationKind for NameResolution {
    const COUNTER_INDEX: usize = 0;
}

pub struct LocalsCount;

impl DecorationKind for LocalsCount {
    const COUNTER_INDEX: usize = 1;
}
