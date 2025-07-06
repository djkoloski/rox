use crate::Punctuated;

pub trait Visit<V: ?Sized> {
    fn accept(&self, visitor: &mut V);
}

impl<V: ?Sized, T: Visit<V>> Visit<V> for Option<T> {
    fn accept(&self, visitor: &mut V) {
        if let Some(node) = self {
            node.accept(visitor);
        }
    }
}

impl<V: ?Sized, T: Visit<V> + ?Sized> Visit<V> for Box<T> {
    fn accept(&self, visitor: &mut V) {
        T::accept(self, visitor)
    }
}

impl<V: ?Sized, T: Visit<V>> Visit<V> for Vec<T> {
    fn accept(&self, visitor: &mut V) {
        for node in self.iter() {
            node.accept(visitor);
        }
    }
}

impl<V: ?Sized, T: Visit<V>, P> Visit<V> for Punctuated<T, P> {
    fn accept(&self, visitor: &mut V) {
        for node in self.iter() {
            node.accept(visitor);
        }
    }
}
