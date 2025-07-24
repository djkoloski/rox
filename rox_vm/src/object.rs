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
use std::alloc::{alloc, dealloc, handle_alloc_error};

use crate::Value;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Tag {
    String,
    Closure,
    Upvalue,
}

pub trait ObjectKind: fmt::Debug {
    const TAG: Tag;
    type Metadata: Copy;

    fn layout_from_metadata(metadata: Self::Metadata) -> Layout;
    fn object_ptr_from_raw_parts(
        address: *mut (),
        metadata: Self::Metadata,
    ) -> *mut Object<Self>;
    fn gc_add_children_to_frontier(&self, frontier: &mut Vec<ErasedHandle>);
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

    fn layout_from_metadata(metadata: Self::Metadata) -> Layout {
        Layout::array::<u8>(metadata).unwrap()
    }

    fn object_ptr_from_raw_parts(
        address: *mut (),
        metadata: Self::Metadata,
    ) -> *mut Object<Self> {
        slice_from_raw_parts_mut(address, metadata) as _
    }

    fn gc_add_children_to_frontier(&self, _: &mut Vec<ErasedHandle>) {}
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

    fn layout_from_metadata(metadata: Self::Metadata) -> Layout {
        Layout::new::<usize>()
            .extend(Layout::array::<Handle<Upvalue>>(metadata).unwrap())
            .unwrap()
            .0
    }

    fn object_ptr_from_raw_parts(
        address: *mut (),
        metadata: Self::Metadata,
    ) -> *mut Object<Self> {
        slice_from_raw_parts_mut(address, metadata) as _
    }

    fn gc_add_children_to_frontier(&self, frontier: &mut Vec<ErasedHandle>) {
        for upvalue in self.upvalues.iter() {
            upvalue.gc_mark(frontier);
        }
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

    fn layout_from_metadata(_: Self::Metadata) -> Layout {
        Layout::new::<Self>()
    }

    fn object_ptr_from_raw_parts(
        address: *mut (),
        _: Self::Metadata,
    ) -> *mut Object<Self> {
        address.cast()
    }

    fn gc_add_children_to_frontier(&self, frontier: &mut Vec<ErasedHandle>) {
        self.read().gc_mark(frontier);
    }
}

#[repr(C)]
struct Header {
    tag: Tag,
    next: Option<ErasedHandle>,
    is_marked: bool,
}

#[repr(C)]
struct MetaHeader<T: ObjectKind + ?Sized> {
    header: Header,
    metadata: T::Metadata,
}

#[repr(C)]
pub struct Object<T: ObjectKind + ?Sized> {
    meta_header: MetaHeader<T>,
    data: T,
}

#[derive(Clone)]
pub struct ErasedHandle {
    ptr: NonNull<Header>,
}

impl ErasedHandle {
    pub fn downcast<T: ObjectKind + ?Sized>(self) -> Option<Handle<T>> {
        let header = unsafe { self.ptr.as_ref() };
        if header.tag == T::TAG {
            unsafe { Some(Handle::from_ptr(self.ptr)) }
        } else {
            None
        }
    }

    pub fn gc_layout(&self) -> Layout {
        let header = unsafe { self.ptr.as_ref() };
        unsafe {
            match header.tag {
                Tag::String => Layout::for_value(
                    &*Handle::<String>::from_ptr(self.ptr).as_ptr(),
                ),
                Tag::Closure => Layout::for_value(
                    &*Handle::<Closure>::from_ptr(self.ptr).as_ptr(),
                ),
                Tag::Upvalue => Layout::for_value(
                    &*Handle::<Upvalue>::from_ptr(self.ptr).as_ptr(),
                ),
            }
        }
    }

    pub fn gc_mark(&self, frontier: &mut Vec<ErasedHandle>) {
        let header = unsafe { self.ptr.as_ref() };
        unsafe {
            match header.tag {
                Tag::String => {
                    Handle::<String>::from_ptr(self.ptr).gc_mark(frontier)
                }
                Tag::Closure => {
                    Handle::<Closure>::from_ptr(self.ptr).gc_mark(frontier)
                }
                Tag::Upvalue => {
                    Handle::<Upvalue>::from_ptr(self.ptr).gc_mark(frontier)
                }
            }
        }
    }

    pub fn gc_unmark(&self) {
        unsafe {
            (*self.ptr.as_ptr()).is_marked = false;
        }
    }

    pub fn gc_is_marked(&self) -> bool {
        unsafe { self.ptr.as_ref().is_marked }
    }

    pub fn gc_next(&self) -> Option<ErasedHandle> {
        unsafe { self.ptr.as_ref().next.clone() }
    }

    pub fn gc_set_next(&self, next: Option<ErasedHandle>) {
        unsafe {
            (*self.ptr.as_ptr()).next = next;
        }
    }

    /// # Safety
    ///
    /// This handle must not be referenced by any other live handles.
    pub unsafe fn gc_sweep(self) -> Option<ErasedHandle> {
        let header = unsafe { self.ptr.as_ref() };
        let next = header.next.clone();
        unsafe {
            match header.tag {
                Tag::String => {
                    Handle::<String>::destroy_object(Handle::from_ptr(self.ptr))
                }
                Tag::Closure => Handle::<Closure>::destroy_object(
                    Handle::from_ptr(self.ptr),
                ),
                Tag::Upvalue => Handle::<Upvalue>::destroy_object(
                    Handle::from_ptr(self.ptr),
                ),
            }
        }
        next
    }
}

pub struct Handle<T: ObjectKind + ?Sized> {
    ptr: NonNull<Header>,
    _phantom: PhantomData<T>,
}

impl<T: ObjectKind + ?Sized> Handle<T> {
    fn as_ptr(&self) -> *mut Object<T> {
        let header = self.ptr.cast::<MetaHeader<T>>();
        let metadata = unsafe { (*header.as_ptr()).metadata };
        T::object_ptr_from_raw_parts(self.ptr.as_ptr().cast(), metadata)
    }

    pub fn get(&self) -> &T {
        let ptr = self.as_ptr();
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

    pub fn erase(this: Self) -> ErasedHandle {
        ErasedHandle { ptr: this.ptr }
    }

    unsafe fn from_ptr(ptr: NonNull<Header>) -> Self {
        Self {
            ptr,
            _phantom: PhantomData,
        }
    }

    pub fn gc_mark(&self, frontier: &mut Vec<ErasedHandle>) {
        let is_marked = unsafe { &mut (*self.ptr.as_ptr()).is_marked };
        if *is_marked {
            return;
        }

        #[cfg(feature = "debug_gc")]
        println!("{:?} mark: {:?}", self.ptr.as_ptr(), self.get());

        *is_marked = true;

        self.get().gc_add_children_to_frontier(frontier);
    }

    unsafe fn create_object(
        next: Option<ErasedHandle>,
        metadata: T::Metadata,
        init: impl FnOnce(*mut T),
    ) -> Handle<T> {
        let layout = Layout::new::<MetaHeader<T>>()
            .extend(T::layout_from_metadata(metadata))
            .unwrap()
            .0
            .pad_to_align();
        let ptr = unsafe {
            T::object_ptr_from_raw_parts(alloc(layout).cast(), metadata)
        };
        let Some(ptr) = NonNull::new(ptr) else {
            handle_alloc_error(layout);
        };

        let header_ptr = unsafe { addr_of_mut!((*ptr.as_ptr()).meta_header) };
        unsafe {
            header_ptr.write(MetaHeader {
                header: Header {
                    tag: T::TAG,
                    next,
                    is_marked: false,
                },
                metadata,
            });
        }

        let data_ptr = unsafe { addr_of_mut!((*ptr.as_ptr()).data) };
        init(data_ptr);

        #[cfg(feature = "debug_gc")]
        println!(
            "{:?} allocate: {} for {:?}",
            ptr.as_ptr().cast::<u8>(),
            layout.size(),
            T::TAG
        );

        Handle {
            ptr: ptr.cast(),
            _phantom: PhantomData,
        }
    }

    unsafe fn destroy_object(handle: Handle<T>) {
        let ptr = handle.as_ptr();

        #[cfg(feature = "debug_gc")]
        println!("{:?} deallocate: {:?}", ptr.cast::<u8>(), T::TAG);

        let metadata = unsafe { (&*ptr).meta_header.metadata };
        let layout = Layout::new::<MetaHeader<T>>()
            .extend(T::layout_from_metadata(metadata))
            .unwrap()
            .0
            .pad_to_align();

        unsafe {
            ptr.drop_in_place();
        }
        unsafe {
            dealloc(ptr.cast(), layout);
        }
    }
}

impl Handle<String> {
    pub fn create_string(next: Option<ErasedHandle>, src: &str) -> Self {
        unsafe {
            Self::create_object(next, src.len(), |ptr| {
                let dst = addr_of_mut!((*ptr).str);
                copy_nonoverlapping(src.as_ptr(), dst.cast::<u8>(), src.len());
            })
        }
    }
}

impl Handle<Closure> {
    pub fn create_closure(
        next: Option<ErasedHandle>,
        function_index: usize,
        mut upvalues: Vec<Handle<Upvalue>>,
    ) -> Handle<Closure> {
        unsafe {
            Self::create_object(next, upvalues.len(), |ptr| {
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
}

impl Handle<Upvalue> {
    /// # Safety
    ///
    /// `location` must point to a valid `Cell` and remain valid for the
    /// lifetime of the returned handle.
    pub unsafe fn create_upvalue(
        next: Option<ErasedHandle>,
        location: *const Cell<Value>,
    ) -> Handle<Upvalue> {
        unsafe {
            Self::create_object(next, (), |ptr| {
                ptr.write(Upvalue {
                    location: Cell::new(location),
                    value: Cell::new(Value::nil()),
                });
            })
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
