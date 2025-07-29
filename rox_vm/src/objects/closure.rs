use core::{alloc::Layout, ptr::addr_of_mut};

use crate::{Emplace, Handle, Pointee, Upvalue};

#[derive(Debug)]
pub struct Closure {
    pub function_index: usize,
    pub upvalues: [Handle<Upvalue>],
}

unsafe impl Pointee for Closure {
    type Metadata = <[Handle<Upvalue>] as Pointee>::Metadata;

    fn layout_from_metadata(metadata: Self::Metadata) -> std::alloc::Layout {
        Layout::new::<usize>()
            .extend(<[Handle<Upvalue>] as Pointee>::layout_from_metadata(
                metadata,
            ))
            .unwrap()
            .0
    }

    fn ptr_from_raw_parts(
        address: *mut (),
        metadata: Self::Metadata,
    ) -> *mut Self {
        <[Handle<Upvalue>] as Pointee>::ptr_from_raw_parts(address, metadata)
            as *mut Self
    }
}

pub struct NewClosure {
    pub function_index: usize,
    pub upvalues: Vec<Handle<Upvalue>>,
}

unsafe impl Emplace<Closure> for NewClosure {
    fn emplaced_metadata(&self) -> <Closure as Pointee>::Metadata {
        self.upvalues.len()
    }

    unsafe fn emplace(self, out: *mut Closure) {
        unsafe {
            let function_index_ptr = addr_of_mut!((*out).function_index);
            function_index_ptr.write(self.function_index);
        }
        unsafe {
            let upvalues_ptr = addr_of_mut!((*out).upvalues);
            for (i, upvalue) in self.upvalues.into_iter().enumerate() {
                upvalues_ptr.cast::<Handle<Upvalue>>().add(i).write(upvalue);
            }
        }
    }
}

// TODO: impl emplace for closure
