use crate::{Closure, Emplace, Handle, Instance, Pointee};

#[derive(Debug)]
pub struct BoundMethod {
    pub receiver: Handle<Instance>,
    pub method: Handle<Closure>,
}

pub struct NewBoundMethod {
    pub receiver: Handle<Instance>,
    pub method: Handle<Closure>,
}

unsafe impl Emplace<BoundMethod> for NewBoundMethod {
    fn emplaced_metadata(&self) -> <BoundMethod as Pointee>::Metadata {}

    unsafe fn emplace(self, out: *mut BoundMethod) {
        unsafe {
            out.write(BoundMethod {
                receiver: self.receiver,
                method: self.method,
            });
        }
    }
}
