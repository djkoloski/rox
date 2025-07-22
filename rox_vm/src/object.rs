use core::{
    alloc::Layout,
    borrow::Borrow,
    cell::Cell,
    fmt,
    hash::{Hash, Hasher},
    marker::PhantomData,
    ops::Deref,
    ptr::{
        NonNull, addr_of_mut, copy_nonoverlapping, slice_from_raw_parts_mut,
        with_exposed_provenance_mut,
    },
};
use std::alloc::{alloc, handle_alloc_error};

use crate::Value;

#[derive(Clone, Copy, PartialEq)]
pub enum Tag {
    String,
    Closure,
    Upvalue,
}

pub trait ObjectKind {
    const TAG: Tag;
    type Metadata: Copy;

    fn object_layout_from_metadata(metadata: Self::Metadata) -> Layout;
    fn object_ptr_from_raw_parts(
        address: *mut (),
        metadata: Self::Metadata,
    ) -> *mut Object<Self>;
}

#[derive(Debug)]
pub struct String {
    str: str,
}

impl fmt::Display for Handle<String> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        str::fmt(&self.str, f)
    }
}

impl Hash for Handle<String> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        str::hash(&self.str, state)
    }
}

impl PartialEq for Handle<String> {
    fn eq(&self, other: &Self) -> bool {
        str::eq(&self.str, &other.str)
    }
}

impl Eq for Handle<String> {}

impl Borrow<str> for Handle<String> {
    fn borrow(&self) -> &str {
        &self.str
    }
}

impl ObjectKind for String {
    const TAG: Tag = Tag::String;
    type Metadata = usize;

    fn object_layout_from_metadata(metadata: Self::Metadata) -> Layout {
        Layout::new::<Header<Self>>()
            .extend(Layout::array::<u8>(metadata).unwrap())
            .unwrap()
            .0
    }

    fn object_ptr_from_raw_parts(
        address: *mut (),
        metadata: Self::Metadata,
    ) -> *mut Object<Self> {
        slice_from_raw_parts_mut(address, metadata) as _
    }
}

#[derive(Debug)]
pub struct Closure {
    pub function_index: usize,
    pub upvalues: [Handle<Upvalue>],
}

impl PartialEq for Handle<Closure> {
    fn eq(&self, other: &Self) -> bool {
        self.ptr == other.ptr
    }
}

impl ObjectKind for Closure {
    const TAG: Tag = Tag::Closure;
    type Metadata = usize;

    fn object_layout_from_metadata(metadata: Self::Metadata) -> Layout {
        Layout::new::<Header<Self>>()
            .extend(Layout::new::<usize>())
            .unwrap()
            .0
            .extend(Layout::array::<usize>(metadata).unwrap())
            .unwrap()
            .0
    }

    fn object_ptr_from_raw_parts(
        address: *mut (),
        metadata: Self::Metadata,
    ) -> *mut Object<Self> {
        slice_from_raw_parts_mut(address, metadata) as _
    }
}

#[derive(Debug)]
pub struct Upvalue {
    location: Cell<*const Cell<Value>>,
    value: Cell<Value>,
}

impl Upvalue {
    pub fn close(&self) {
        self.value.set(self.read());
        self.location.set(&self.value);
    }

    fn location(&self) -> &Cell<Value> {
        unsafe { &*self.location.get() }
    }

    pub fn read(&self) -> Value {
        self.location().get()
    }

    pub fn write(&self, value: Value) {
        self.location().set(value)
    }
}

impl ObjectKind for Upvalue {
    const TAG: Tag = Tag::Upvalue;
    type Metadata = ();

    fn object_layout_from_metadata(_: Self::Metadata) -> Layout {
        Layout::new::<Object<Self>>()
    }

    fn object_ptr_from_raw_parts(
        address: *mut (),
        _: Self::Metadata,
    ) -> *mut Object<Self> {
        address.cast()
    }
}

#[repr(C)]
struct Header<T: ObjectKind + ?Sized> {
    tag: Tag,
    metadata: T::Metadata,
}

#[repr(C)]
pub struct Object<T: ObjectKind + ?Sized> {
    header: Header<T>,
    data: T,
}

#[derive(Debug)]
pub enum HandleKind {
    String(Handle<String>),
    Closure(Handle<Closure>),
    Upvalue(Handle<Upvalue>),
}

pub struct Handle<T: ObjectKind + ?Sized> {
    ptr: NonNull<Tag>,
    _phantom: PhantomData<T>,
}

impl<T: ObjectKind + ?Sized> Handle<T> {
    pub fn get(&self) -> &T {
        let header = self.ptr.cast::<Header<T>>();
        let metadata = unsafe { (*header.as_ptr()).metadata };
        let ptr =
            T::object_ptr_from_raw_parts(self.ptr.as_ptr().cast(), metadata);
        unsafe { &(*ptr).data }
    }

    pub fn address(&self) -> usize {
        self.ptr.as_ptr().expose_provenance()
    }

    /// # Safety
    ///
    /// `address` must be an address of a handle which is still valid and was
    /// obtained by calling [`address`].
    pub unsafe fn from_address(address: usize) -> Self {
        let ptr = with_exposed_provenance_mut(address);
        Self {
            ptr: unsafe { NonNull::new_unchecked(ptr) },
            _phantom: PhantomData,
        }
    }
}

impl<T: ObjectKind + ?Sized> Clone for Handle<T> {
    fn clone(&self) -> Self {
        Self {
            ptr: self.ptr,
            _phantom: PhantomData,
        }
    }
}

impl<T: ObjectKind + fmt::Debug + ?Sized> fmt::Debug for Handle<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        T::fmt(self, f)
    }
}

impl<T: ObjectKind + ?Sized> Deref for Handle<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

pub struct Heap {}

impl Heap {
    pub fn new() -> Self {
        Self {}
    }

    unsafe fn create_object<T: ObjectKind + ?Sized>(
        metadata: T::Metadata,
        init: impl FnOnce(*mut T),
    ) -> Handle<T> {
        let layout = T::object_layout_from_metadata(metadata);
        let ptr = unsafe {
            T::object_ptr_from_raw_parts(alloc(layout).cast(), metadata)
        };
        let Some(ptr) = NonNull::new(ptr) else {
            handle_alloc_error(layout);
        };

        let header_ptr = unsafe { addr_of_mut!((*ptr.as_ptr()).header) };
        unsafe {
            header_ptr.write(Header {
                tag: T::TAG,
                metadata,
            });
        }

        let data_ptr = unsafe { addr_of_mut!((*ptr.as_ptr()).data) };
        init(data_ptr);

        Handle {
            ptr: ptr.cast(),
            _phantom: PhantomData,
        }
    }

    pub fn create_string(&mut self, src: &str) -> Handle<String> {
        unsafe {
            Self::create_object::<String>(src.len(), |ptr| {
                let dst = addr_of_mut!((*ptr).str);
                copy_nonoverlapping(src.as_ptr(), dst.cast::<u8>(), src.len());
            })
        }
    }

    pub fn create_closure(
        &mut self,
        function_index: usize,
        mut upvalues: Vec<Handle<Upvalue>>,
    ) -> Handle<Closure> {
        unsafe {
            Self::create_object::<Closure>(upvalues.len(), |ptr| {
                let dst = addr_of_mut!((*ptr).function_index);
                dst.write(function_index);

                let dst = addr_of_mut!((*ptr).upvalues);
                copy_nonoverlapping(
                    upvalues.as_ptr(),
                    dst.cast::<Handle<Upvalue>>(),
                    upvalues.len(),
                );
                upvalues.set_len(0);
            })
        }
    }

    /// # Safety
    ///
    /// `location` must point to a valid `Cell` and remain valid for the
    /// lifetime of the returned handle.
    pub unsafe fn create_upvalue(
        &mut self,
        location: *const Cell<Value>,
    ) -> Handle<Upvalue> {
        unsafe {
            Self::create_object::<Upvalue>((), |ptr| {
                ptr.write(Upvalue {
                    location: Cell::new(location),
                    value: Cell::new(Value::nil()),
                });
            })
        }
    }
}

impl Default for Heap {
    fn default() -> Self {
        Self::new()
    }
}
