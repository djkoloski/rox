mod bound_method;
mod class;
mod closure;
mod instance;
mod string;
mod upvalue;

use core::{alloc::Layout, ptr::slice_from_raw_parts_mut};

pub use self::{
    bound_method::*, class::*, closure::*, instance::*, string::*, upvalue::*,
};

/// # Safety
///
/// - `layout_from_metadata` must return the layout of a value of this type with
///   the given `metadata`.
/// - `ptr_from_raw_parts` must return the pointer with the given `address` and
///   `metadata`.
pub unsafe trait Pointee {
    type Metadata: Copy;

    fn layout_from_metadata(metadata: Self::Metadata) -> Layout;
    fn ptr_from_raw_parts(
        address: *mut (),
        metadata: Self::Metadata,
    ) -> *mut Self;
}

unsafe impl<T> Pointee for T {
    type Metadata = ();

    fn layout_from_metadata(_: Self::Metadata) -> Layout {
        Layout::new::<Self>()
    }

    fn ptr_from_raw_parts(address: *mut (), _: Self::Metadata) -> *mut Self {
        address.cast()
    }
}

unsafe impl<T> Pointee for [T] {
    type Metadata = usize;

    fn layout_from_metadata(metadata: Self::Metadata) -> Layout {
        Layout::array::<T>(metadata).unwrap()
    }

    fn ptr_from_raw_parts(
        address: *mut (),
        metadata: Self::Metadata,
    ) -> *mut Self {
        slice_from_raw_parts_mut(address.cast::<T>(), metadata)
    }
}

unsafe impl Pointee for str {
    type Metadata = <[u8] as Pointee>::Metadata;

    fn layout_from_metadata(metadata: Self::Metadata) -> Layout {
        <[u8] as Pointee>::layout_from_metadata(metadata)
    }

    fn ptr_from_raw_parts(
        address: *mut (),
        metadata: Self::Metadata,
    ) -> *mut Self {
        <[u8] as Pointee>::ptr_from_raw_parts(address, metadata) as *mut Self
    }
}

/// # Safety
///
/// - `emplaced_metadata` must return the metadata of the value which it
///   emplaces.
/// - `emplace` must fully initialize the value at `out`.
pub unsafe trait Emplace<T: Pointee + ?Sized> {
    fn emplaced_metadata(&self) -> T::Metadata;

    /// # Safety
    ///
    /// `out` must point to a place that is valid for reads and writes, and have
    /// the metadata returned by `emplaced_metadata`.
    unsafe fn emplace(self, out: *mut T);
}
