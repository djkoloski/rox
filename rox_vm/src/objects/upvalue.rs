use core::{cell::Cell, ptr::addr_of_mut};

use crate::{Emplace, Value};

#[derive(Debug)]
pub struct Upvalue {
    location: Cell<*const Cell<Value>>,
    value: Cell<Value>,
}

impl Upvalue {
    pub fn read(&self) -> Value {
        unsafe { (*self.location.get()).get() }
    }

    pub fn write(&self, value: Value) {
        unsafe { (*self.location.get()).set(value) }
    }

    pub fn close(&self) {
        let value = self.read();
        self.value.set(value);
        self.location.set(&self.value);
    }
}

unsafe impl Emplace<Upvalue> for *const Cell<Value> {
    fn emplaced_metadata(
        &self,
    ) -> <*const Cell<Value> as super::Pointee>::Metadata {
    }

    unsafe fn emplace(self, out: *mut Upvalue) {
        unsafe {
            let location_ptr = addr_of_mut!((*out).location);
            location_ptr.write(Cell::new(self));
        }
        unsafe {
            let value_ptr = addr_of_mut!((*out).value);
            value_ptr.write(Cell::new(Value::nil()));
        }
    }
}
