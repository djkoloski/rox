use core::ptr::addr_of_mut;

use crate::{Emplace, Pointee};

#[derive(Debug)]
pub struct Class {
    pub class_index: usize,
}

pub struct NewClass {
    pub class_index: usize,
}

unsafe impl Emplace<Class> for NewClass {
    fn emplaced_metadata(&self) -> <Class as Pointee>::Metadata {}

    unsafe fn emplace(self, out: *mut Class) {
        unsafe {
            let class_index_ptr = addr_of_mut!((*out).class_index);
            class_index_ptr.write(self.class_index);
        }
    }
}
