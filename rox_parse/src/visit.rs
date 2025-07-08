use crate::Punctuated;

pub trait Visit<'ast, V: ?Sized> {
    fn accept(&'ast self, visitor: &mut V);
}

impl<'ast, V: ?Sized, T: Visit<'ast, V>> Visit<'ast, V> for Option<T> {
    fn accept(&'ast self, visitor: &mut V) {
        if let Some(node) = self {
            node.accept(visitor);
        }
    }
}

impl<'ast, V: ?Sized, T: Visit<'ast, V> + ?Sized> Visit<'ast, V> for Box<T> {
    fn accept(&'ast self, visitor: &mut V) {
        T::accept(self, visitor)
    }
}

impl<'ast, V: ?Sized, T: Visit<'ast, V>> Visit<'ast, V> for Vec<T> {
    fn accept(&'ast self, visitor: &mut V) {
        for node in self.iter() {
            node.accept(visitor);
        }
    }
}

impl<'ast, V: ?Sized, T: Visit<'ast, V>, P> Visit<'ast, V>
    for Punctuated<T, P>
{
    fn accept(&'ast self, visitor: &mut V) {
        for node in self.iter() {
            node.accept(visitor);
        }
    }
}
