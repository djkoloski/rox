use core::{
    alloc::Layout,
    cell::Cell,
    fmt,
    marker::PhantomData,
    ops::Deref,
    ptr::{NonNull, addr_of_mut, with_exposed_provenance_mut},
};
use std::alloc::{alloc, dealloc, handle_alloc_error};

use crate::{
    BoundMethod, Class, Closure, Emplace, Instance, Pointee, String, Upvalue,
    memory::value_to_object_handle,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Tag {
    String,
    Closure,
    Upvalue,
    Class,
    Instance,
    BoundMethod,
}

pub trait ObjectKind: Pointee {
    const TAG: Tag;

    fn gc_explore(&self, frontier: &mut Vec<ObjectHandle>);
}

impl ObjectKind for Class {
    const TAG: Tag = Tag::Class;

    fn gc_explore(&self, frontier: &mut Vec<ObjectHandle>) {
        for method in self.methods.borrow().values() {
            frontier.push(Handle::erase(*method));
        }
    }
}

impl ObjectKind for String {
    const TAG: Tag = Tag::String;

    fn gc_explore(&self, _: &mut Vec<ObjectHandle>) {}
}

impl ObjectKind for Closure {
    const TAG: Tag = Tag::Closure;

    fn gc_explore(&self, frontier: &mut Vec<ObjectHandle>) {
        for handle in self.upvalues.iter() {
            frontier.push(Handle::erase(*handle));
        }
    }
}

impl ObjectKind for Upvalue {
    const TAG: Tag = Tag::Upvalue;

    fn gc_explore(&self, frontier: &mut Vec<ObjectHandle>) {
        if let Some(object_handle) = value_to_object_handle(self.read()) {
            frontier.push(object_handle);
        }
    }
}

impl ObjectKind for Instance {
    const TAG: Tag = Tag::Instance;

    fn gc_explore(&self, frontier: &mut Vec<ObjectHandle>) {
        frontier.push(Handle::erase(self.class));

        for value in self.fields.borrow().values() {
            if let Some(handle) = value_to_object_handle(*value) {
                frontier.push(handle);
            }
        }
    }
}

impl ObjectKind for BoundMethod {
    const TAG: Tag = Tag::BoundMethod;

    fn gc_explore(&self, frontier: &mut Vec<ObjectHandle>) {
        frontier.push(Handle::erase(self.receiver));
        frontier.push(Handle::erase(self.method));
    }
}

#[repr(C)]
struct Header {
    tag: Tag,
    is_marked: Cell<bool>,
    next: Cell<Option<ObjectHandle>>,
}

#[repr(C)]
struct Prefix<T: ObjectKind + ?Sized> {
    header: Header,
    metadata: T::Metadata,
}

#[repr(C)]
struct Object<T: ObjectKind + ?Sized> {
    prefix: Prefix<T>,
    data: T,
}

pub struct Handle<T: ObjectKind + ?Sized> {
    ptr: NonNull<Prefix<T>>,
    _phantom: PhantomData<T>,
}

impl<T: ObjectKind + ?Sized> Handle<T> {
    pub fn create(
        emplacer: impl Emplace<T>,
        next: Option<ObjectHandle>,
    ) -> Self {
        let metadata = emplacer.emplaced_metadata();
        let layout = Layout::new::<Prefix<T>>()
            .extend(T::layout_from_metadata(metadata))
            .unwrap()
            .0
            .pad_to_align();
        let ptr = unsafe { alloc(layout).cast::<Prefix<T>>() };
        let Some(ptr) = NonNull::new(ptr) else {
            handle_alloc_error(layout);
        };

        let obj_ptr = T::ptr_from_raw_parts(ptr.as_ptr().cast(), metadata)
            as *mut Object<T>;

        unsafe {
            let prefix_ptr = addr_of_mut!((*obj_ptr).prefix);
            prefix_ptr.write(Prefix {
                header: Header {
                    tag: T::TAG,
                    is_marked: Cell::new(false),
                    next: Cell::new(next),
                },
                metadata,
            });
        }
        unsafe {
            let data_ptr = addr_of_mut!((*obj_ptr).data);
            emplacer.emplace(data_ptr);
        }

        #[cfg(feature = "debug_gc")]
        println!(
            "{:?} allocate: {} for {:?}",
            ptr.as_ptr(),
            layout.size(),
            T::TAG
        );

        Self {
            ptr,
            _phantom: PhantomData,
        }
    }

    /// # Safety
    ///
    /// This handle must never be accessed again.
    pub unsafe fn destroy(self) {
        let layout = Self::object_layout(self);

        #[cfg(feature = "debug_gc")]
        println!(
            "{:?} deallocate: {} of {:?}",
            self.as_ptr(),
            layout.size(),
            T::TAG
        );

        unsafe {
            self.as_ptr().drop_in_place();
        }
        unsafe {
            dealloc(self.ptr.as_ptr().cast(), layout);
        }
    }

    fn metadata(self) -> T::Metadata {
        unsafe { self.ptr.as_ref().metadata }
    }

    fn as_ptr(self) -> *mut Object<T> {
        T::ptr_from_raw_parts(self.ptr.as_ptr().cast(), self.metadata())
            as *mut Object<T>
    }

    pub fn object_layout(this: Self) -> Layout {
        unsafe { Layout::for_value(&*this.as_ptr()) }
    }

    pub fn get(&self) -> &T {
        unsafe { &(*self.as_ptr()).data }
    }

    pub fn address(this: Self) -> usize {
        this.ptr.as_ptr().expose_provenance()
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

    pub fn erase(this: Self) -> ObjectHandle {
        ObjectHandle {
            ptr: this.ptr.cast(),
        }
    }
}

impl<T: ObjectKind + ?Sized> Clone for Handle<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ObjectKind + ?Sized> Copy for Handle<T> {}

impl<T: ObjectKind + fmt::Debug + ?Sized> fmt::Debug for Handle<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        T::fmt(self, f)
    }
}

impl<T: ObjectKind + fmt::Display + ?Sized> fmt::Display for Handle<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        T::fmt(self, f)
    }
}

impl<T: ObjectKind + ?Sized> Deref for Handle<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

impl<T: ObjectKind + ?Sized> PartialEq for Handle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.ptr == other.ptr
    }
}

impl<T: ObjectKind + ?Sized> Eq for Handle<T> {}

pub trait HandleOperation<T: ObjectKind + ?Sized> {
    fn operate(self, handle: Handle<T>);
}

struct GcExplore<'a> {
    frontier: &'a mut Vec<ObjectHandle>,
}

impl<T: ObjectKind + fmt::Debug + ?Sized> HandleOperation<T> for GcExplore<'_> {
    fn operate(self, handle: Handle<T>) {
        #[cfg(feature = "debug_gc")]
        println!("{:?} mark: {:?}", handle.ptr.as_ptr(), handle.get());

        handle.get().gc_explore(self.frontier);
    }
}

#[derive(Clone, Copy)]
pub struct ObjectHandle {
    ptr: NonNull<Header>,
}

impl ObjectHandle {
    fn header(&self) -> &Header {
        unsafe { self.ptr.as_ref() }
    }

    pub fn downcast<T: ObjectKind + ?Sized>(&self) -> Option<Handle<T>> {
        if self.header().tag == T::TAG {
            unsafe { Some(self.downcast_unchecked()) }
        } else {
            None
        }
    }

    unsafe fn downcast_unchecked<T: ObjectKind + ?Sized>(&self) -> Handle<T> {
        Handle {
            ptr: self.ptr.cast::<Prefix<T>>(),
            _phantom: PhantomData,
        }
    }

    pub fn operate<O>(self, operation: O)
    where
        O: HandleOperation<String>
            + HandleOperation<Closure>
            + HandleOperation<Upvalue>
            + HandleOperation<Class>
            + HandleOperation<Instance>
            + HandleOperation<BoundMethod>,
    {
        unsafe {
            match self.header().tag {
                Tag::String => {
                    operation.operate(self.downcast_unchecked::<String>())
                }
                Tag::Closure => {
                    operation.operate(self.downcast_unchecked::<Closure>())
                }
                Tag::Upvalue => {
                    operation.operate(self.downcast_unchecked::<Upvalue>())
                }
                Tag::Class => {
                    operation.operate(self.downcast_unchecked::<Class>())
                }
                Tag::Instance => {
                    operation.operate(self.downcast_unchecked::<Instance>())
                }
                Tag::BoundMethod => {
                    operation.operate(self.downcast_unchecked::<BoundMethod>())
                }
            }
        }
    }

    pub fn gc_mark(&self, frontier: &mut Vec<Self>) {
        if self.header().is_marked.get() {
            return;
        }

        self.header().is_marked.set(true);

        self.operate(GcExplore { frontier });
    }

    pub fn gc_unmark(&self) {
        self.header().is_marked.set(false);
    }

    pub fn gc_is_marked(&self) -> bool {
        self.header().is_marked.get()
    }

    pub fn gc_next(&self) -> Option<Self> {
        self.header().next.get()
    }

    pub fn gc_set_next(&self, next: Option<Self>) {
        self.header().next.set(next);
    }
}
