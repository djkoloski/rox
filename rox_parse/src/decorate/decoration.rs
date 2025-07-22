use core::{fmt, marker::PhantomData};

use crate::decorate::DECORATIONS_MAX;

pub struct Decorator {
    counters: Vec<usize>,
}

impl Decorator {
    pub fn new() -> Self {
        Self {
            counters: vec![0; DECORATIONS_MAX],
        }
    }

    pub fn decorate<T: DecorationKind>(&mut self) -> Decoration<T> {
        let counter = &mut self.counters[T::DECORATION_INDEX];
        let index = *counter;
        *counter += 1;

        Decoration {
            index,
            _phantom: PhantomData,
        }
    }

    pub fn count<T: DecorationKind>(&self) -> usize {
        self.counters[T::DECORATION_INDEX]
    }
}

impl Default for Decorator {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Decoration<D> {
    index: usize,
    _phantom: PhantomData<D>,
}

impl<D> Decoration<D> {
    pub fn index(&self) -> usize {
        self.index
    }
}

impl<T> fmt::Debug for Decoration<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Decoration")
    }
}

impl<D> Clone for Decoration<D> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<D> Copy for Decoration<D> {}

pub trait DecorationKind {
    const DECORATION_INDEX: usize;
}
