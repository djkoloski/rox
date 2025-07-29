use core::cell::RefCell;
use std::collections::HashMap;

use crate::{Class, Emplace, Handle, Pointee, Value};

#[derive(Debug)]
pub struct Instance {
    pub class: Handle<Class>,
    pub fields: RefCell<HashMap<usize, Value>>,
}

pub struct NewInstance {
    pub class: Handle<Class>,
}

unsafe impl Emplace<Instance> for NewInstance {
    fn emplaced_metadata(&self) -> <Instance as Pointee>::Metadata {}

    unsafe fn emplace(self, out: *mut Instance) {
        unsafe {
            out.write(Instance {
                class: self.class,
                fields: RefCell::new(HashMap::new()),
            });
        }
    }
}
