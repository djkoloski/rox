use core::{
    alloc::Layout, marker::PhantomData, mem::ManuallyDrop, ptr::NonNull,
};
use std::alloc::{alloc, alloc_zeroed, dealloc, handle_alloc_error};

use crate::{Decoration, DecorationKind, Decorator};

type Presence = usize;

pub struct Dec<D, T> {
    len: usize,
    cap: usize,
    presence: NonNull<Presence>,
    elements: NonNull<T>,
    _phantom: PhantomData<D>,
}

impl<D, T> Drop for Dec<D, T> {
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

impl<D, T> Dec<D, T> {
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

    fn get_presence(&self, index: usize) -> (*mut Presence, Presence) {
        assert!(index < self.cap, "decoration index out-of-bounds");

        let word = index / usize::BITS as usize;
        let bit = index % usize::BITS as usize;
        let mask = 1 << bit;
        let ptr = unsafe { self.presence.as_ptr().add(word) };

        (ptr, mask)
    }

    fn is_present(&self, index: usize) -> bool {
        let (ptr, mask) = self.get_presence(index);
        let presence = unsafe { ptr.read() };
        presence & mask != 0
    }

    pub fn insert(&mut self, decoration: Decoration<D>, value: T) {
        let index = decoration.index();

        let (ptr, mask) = self.get_presence(index);
        let presence = unsafe { ptr.read() };
        assert_eq!(
            presence & mask,
            0,
            "a value with the decoration index {index} was already inserted",
        );

        unsafe {
            ptr.write(presence | mask);
        }
        unsafe {
            self.elements.as_ptr().add(index).write(value);
        }
        self.len += 1;
    }

    pub fn get(&self, decoration: Decoration<D>) -> Option<&T> {
        let index = decoration.index();

        assert!(
            self.is_present(index),
            "a value with the decoration index {index} was not already \
             inserted"
        );

        unsafe { Some(self.elements.add(index).as_ref()) }
    }

    pub fn get_mut(&mut self, decoration: Decoration<D>) -> Option<&mut T> {
        let index = decoration.index();

        assert!(
            self.is_present(index),
            "a value with the decoration index {index} was not already \
             inserted"
        );

        unsafe { Some(self.elements.add(index).as_mut()) }
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
