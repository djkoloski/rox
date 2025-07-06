#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Decoration {
    id: usize,
}

pub struct Decorator {
    next_id: usize,
}

impl Decorator {
    pub fn new() -> Self {
        Self { next_id: 0 }
    }

    pub fn decorate(&mut self) -> Decoration {
        let id = self.next_id;
        self.next_id += 1;
        Decoration { id }
    }
}

impl Default for Decorator {
    fn default() -> Self {
        Self::new()
    }
}
