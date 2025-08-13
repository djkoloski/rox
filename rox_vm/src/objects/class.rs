use core::cell::RefCell;
use std::collections::HashMap;

use crate::{Closure, Emplace, Handle, Pointee};

#[derive(Debug)]
pub struct Class {
    pub class_index: usize,
    pub methods: RefCell<HashMap<String, Handle<Closure>>>,
    pub superclass: Option<Handle<Class>>,
}

pub struct NewClass {
    pub class_index: usize,
    pub superclass: Option<Handle<Class>>,
}

unsafe impl Emplace<Class> for NewClass {
    fn emplaced_metadata(&self) -> <Class as Pointee>::Metadata {}

    unsafe fn emplace(self, out: *mut Class) {
        let methods = if let Some(superclass) = self.superclass {
            superclass.methods.borrow().clone()
        } else {
            HashMap::new()
        };

        unsafe {
            out.write(Class {
                class_index: self.class_index,
                methods: RefCell::new(methods),
                superclass: self.superclass,
            });
        }
    }
}
