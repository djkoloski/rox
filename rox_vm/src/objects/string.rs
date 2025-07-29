use core::{
    fmt,
    ptr::{addr_of_mut, copy_nonoverlapping},
};

use crate::{Emplace, Pointee};

#[derive(Debug)]
pub struct String {
    str: str,
}

impl String {
    pub fn as_str(&self) -> &str {
        &self.str
    }
}

unsafe impl Pointee for String {
    type Metadata = <str as Pointee>::Metadata;

    fn layout_from_metadata(metadata: Self::Metadata) -> std::alloc::Layout {
        <str as Pointee>::layout_from_metadata(metadata)
    }

    fn ptr_from_raw_parts(
        address: *mut (),
        metadata: Self::Metadata,
    ) -> *mut Self {
        <str as Pointee>::ptr_from_raw_parts(address, metadata) as *mut Self
    }
}

unsafe impl Emplace<String> for &str {
    fn emplaced_metadata(&self) -> <String as Pointee>::Metadata {
        self.len()
    }

    unsafe fn emplace(self, out: *mut String) {
        let src_ptr = self.as_ptr();
        unsafe {
            let dst_ptr = addr_of_mut!((*out).str);
            copy_nonoverlapping(
                src_ptr.cast::<u8>(),
                dst_ptr.cast::<u8>(),
                self.len(),
            );
        }
    }
}

impl fmt::Display for String {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_str().fmt(f)
    }
}
