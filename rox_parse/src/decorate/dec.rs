use core::{
    alloc::Layout, marker::PhantomData, mem::ManuallyDrop, ptr::NonNull,
};
use std::alloc::{alloc, alloc_zeroed, dealloc, handle_alloc_error};

use crate::{Decoration, DecorationKind, Decorator};

type Presence = usize;

pub struct Dec<T, D> {
    len: usize,
    cap: usize,
    presence: NonNull<Presence>,
    elements: NonNull<T>,
    _phantom: PhantomData<D>,
}

impl<T, D> Drop for Dec<T, D> {
    fn drop(&mut self) {
        if self.cap == 0 {
            return;
        }

        for i in 0..self.cap {
            let presence_i = i / Presence::BITS as usize;
            let presence_b = i % Presence::BITS as usize;
            let presence =
                unsafe { self.presence.as_ptr().add(presence_i).read() };
            let presence_mask = 1 << presence_b;

            if presence & presence_mask != 0 {
                unsafe {
                    self.elements.as_ptr().add(i).drop_in_place();
                }
            }
        }

        unsafe {
            dealloc(
                self.presence.as_ptr().cast(),
                Self::presence_layout(self.cap),
            );
        }
        unsafe {
            dealloc(
                self.elements.as_ptr().cast(),
                Self::elements_layout(self.cap),
            );
        }
    }
}

impl<T, D> Dec<T, D> {
    fn presence_layout(cap: usize) -> Layout {
        let presence_cap = cap.div_ceil(Presence::BITS as usize);
        Layout::array::<Presence>(presence_cap).unwrap()
    }

    fn elements_layout(cap: usize) -> Layout {
        Layout::array::<T>(cap).unwrap()
    }

    pub fn new(decorator: &Decorator) -> Self
    where
        D: DecorationKind,
    {
        let cap = decorator.count::<D>();

        if cap == 0 {
            return Self {
                len: 0,
                cap,
                presence: NonNull::dangling(),
                elements: NonNull::dangling(),
                _phantom: PhantomData,
            };
        }

        let presence_layout = Self::presence_layout(cap);
        let presence =
            unsafe { alloc_zeroed(presence_layout).cast::<Presence>() };
        let Some(presence) = NonNull::new(presence) else {
            handle_alloc_error(presence_layout);
        };

        let elements_layout = Self::elements_layout(cap);
        let elements = unsafe { alloc(elements_layout).cast::<T>() };
        let Some(elements) = NonNull::new(elements) else {
            handle_alloc_error(elements_layout);
        };

        Self {
            len: 0,
            cap,
            presence,
            elements,
            _phantom: PhantomData,
        }
    }

    pub fn insert(&mut self, decoration: &Decoration<D>, value: T) {
        let index = decoration.index();

        assert!(index < self.cap, "decoration index out-of-bounds");

        let presence_i = index.div_ceil(usize::BITS as usize);
        let presence_b = index % usize::BITS as usize;

        let presence_ptr = unsafe { self.presence.as_ptr().add(presence_i) };
        let presence_mask = 1 << presence_b;

        let presence = unsafe { presence_ptr.read() };

        assert_eq!(
            presence & presence_mask,
            0,
            "a value with the decoration index {index} was already inserted",
        );

        unsafe {
            presence_ptr.write(presence | presence_mask);
        }
        unsafe {
            self.elements.as_ptr().add(index).write(value);
        }
        self.len += 1;
    }

    pub fn unwrap(self) -> Vec<T> {
        if self.cap == 0 {
            return Vec::new();
        }

        assert_eq!(
            self.len, self.cap,
            "not all decoration elements were inserted"
        );

        let this = ManuallyDrop::new(self);

        unsafe {
            dealloc(
                this.presence.as_ptr().cast(),
                Self::presence_layout(this.cap),
            );
        }

        unsafe {
            Vec::from_raw_parts(this.elements.as_ptr(), this.len, this.cap)
        }
    }
}
