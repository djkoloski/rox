use core::ops::Index;

struct Run<T> {
    value: T,
    count: usize,
}

pub struct Rle<T> {
    runs: Vec<Run<T>>,
    len: usize,
}

impl<T> Rle<T> {
    pub fn new() -> Self {
        Self {
            runs: Vec::new(),
            len: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn push(&mut self, value: T)
    where
        T: PartialEq,
    {
        self.extend(value, 1);
    }

    pub fn extend(&mut self, value: T, len: usize)
    where
        T: PartialEq,
    {
        self.len += len;

        if let Some(last) = self.runs.last_mut()
            && last.value == value
        {
            last.count += len;
            return;
        }

        self.runs.push(Run { value, count: len });
    }
}

impl<T> Default for Rle<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Index<usize> for Rle<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        let mut current = 0;
        for run in &self.runs {
            if index < current + run.count {
                return &run.value;
            }
            current += run.count;
        }

        panic!("line index out of bounds");
    }
}
