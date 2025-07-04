use crate::span::Spanned;

#[derive(Debug)]
pub struct Punctuated<T, P> {
    values: Vec<(T, P)>,
    last: Option<Box<T>>,
}

impl<T, P> Punctuated<T, P> {
    pub fn new() -> Self {
        Self {
            values: Vec::new(),
            last: None,
        }
    }

    pub fn push(&mut self, value: T) {
        assert!(
            self.last.is_none(),
            "value pushed without a previous punctuation"
        );
        self.last = Some(Box::new(value));
    }

    pub fn push_punct(&mut self, punct: P) {
        let Some(value) = self.last.take() else {
            panic!("punctuation pushed without a previous value");
        };
        self.values.push((*value, punct));
    }

    pub fn is_empty(&self) -> bool {
        self.values.len() == 0 && self.last.is_none()
    }

    pub fn len(&self) -> usize {
        self.values.len() + if self.last.is_some() { 1 } else { 0 }
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.values
            .iter()
            .map(|(t, _)| t)
            .chain(self.last.as_deref())
    }

    pub fn span_start(&self) -> Option<usize>
    where
        T: Spanned,
        P: Spanned,
    {
        if let Some((t, _)) = self.values.first() {
            Some(t.span_start())
        } else {
            self.last.as_deref().map(T::span_start)
        }
    }

    pub fn span_end(&self) -> Option<usize>
    where
        T: Spanned,
        P: Spanned,
    {
        if let Some(last) = &self.last {
            Some(last.span_end())
        } else if let Some((_, k)) = self.values.last() {
            Some(k.span_end())
        } else {
            None
        }
    }
}

impl<T, P> Default for Punctuated<T, P> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone, P: Clone> Clone for Punctuated<T, P> {
    fn clone(&self) -> Self {
        Self {
            values: self.values.clone(),
            last: self.last.clone(),
        }
    }
}
