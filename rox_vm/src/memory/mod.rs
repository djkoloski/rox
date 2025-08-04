mod object;

use core::{borrow::Borrow, cell::Cell, hash};
use std::collections::{BTreeMap, HashMap, HashSet};

pub use self::object::Handle;
use self::object::{HandleOperation, ObjectHandle, ObjectKind};
use crate::{
    BoundMethod, Class, Closure, Emplace, Instance, NewBoundMethod, NewClass,
    NewClosure, NewInstance, RuntimeError, String, Upvalue, Value,
    global_values,
};

const MAX_STACK_LEN: usize = 255;

struct InternedString {
    handle: Handle<String>,
}

impl hash::Hash for InternedString {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.handle.get().as_str().hash(state)
    }
}

impl PartialEq for InternedString {
    fn eq(&self, other: &Self) -> bool {
        self.handle.as_str() == other.handle.as_str()
    }
}

impl Eq for InternedString {}

impl Borrow<str> for InternedString {
    fn borrow(&self) -> &str {
        self.handle.as_str()
    }
}

pub struct Memory {
    globals: HashMap<std::string::String, Value>,
    stack: Box<[Cell<Value>; MAX_STACK_LEN]>,
    stack_len: usize,
    handles: Option<ObjectHandle>,
    open_upvalues: BTreeMap<usize, Handle<Upvalue>>,

    strings: HashSet<InternedString>,
    bytes_allocated: usize,
    next_gc: usize,
}

impl Drop for Memory {
    fn drop(&mut self) {
        self.strings.clear();

        while let Some(handle) = self.handles {
            self.handles = handle.gc_next();
            handle.operate(DestroyOp { memory: self });
        }
    }
}

impl Memory {
    /// # Safety
    ///
    /// The handles created by this object are managed via garbage collection.
    /// Special care must be taken to prevent them from leaking.
    pub unsafe fn new() -> Self {
        Self {
            globals: global_values(),
            stack: Box::new([const { Cell::new(Value::nil()) }; MAX_STACK_LEN]),
            stack_len: 0,
            handles: None,
            open_upvalues: BTreeMap::new(),

            strings: HashSet::new(),
            bytes_allocated: 0,
            next_gc: 1024 * 1024,
        }
    }

    pub fn insert_global(
        &mut self,
        name: &str,
        value: Value,
    ) -> Result<(), RuntimeError> {
        if let Some(prev) = self.globals.insert(name.to_string(), value) {
            return Err(RuntimeError::GlobalAlreadyDefined {
                name: name.to_string(),
                value: prev.unpack(),
            });
        }
        Ok(())
    }

    pub fn read_global(&self, name: &str) -> Result<Value, RuntimeError> {
        let Some(value) = self.globals.get(name) else {
            return Err(RuntimeError::UndefinedGlobal {
                name: name.to_string(),
            });
        };

        Ok(*value)
    }

    pub fn write_global(
        &mut self,
        name: &str,
        value: Value,
    ) -> Result<(), RuntimeError> {
        let Some(target) = self.globals.get_mut(name) else {
            return Err(RuntimeError::UndefinedGlobal {
                name: name.to_string(),
            });
        };
        *target = value;

        Ok(())
    }

    pub fn stack_len(&self) -> usize {
        self.stack_len
    }

    pub fn read_stack(
        &self,
        stack_index: usize,
    ) -> Result<Value, RuntimeError> {
        if stack_index >= self.stack_len {
            return Err(RuntimeError::StackVariableOutOfBounds { stack_index });
        }

        Ok(self.stack[stack_index].get())
    }

    pub fn write_stack(
        &mut self,
        stack_index: usize,
        value: Value,
    ) -> Result<(), RuntimeError> {
        if stack_index >= self.stack_len {
            return Err(RuntimeError::StackVariableOutOfBounds { stack_index });
        }
        self.stack[stack_index].set(value);

        Ok(())
    }

    pub fn push(&mut self, value: Value) -> Result<(), RuntimeError> {
        if self.stack_len == MAX_STACK_LEN {
            return Err(RuntimeError::StackOverflow);
        }
        self.stack[self.stack_len].set(value);
        self.stack_len += 1;
        Ok(())
    }

    pub fn pop(&mut self) -> Result<Value, RuntimeError> {
        if self.stack_len == 0 {
            return Err(RuntimeError::StackUnderflow);
        }
        self.stack_len -= 1;
        let result = self.stack[self.stack_len].replace(Value::nil());
        Ok(result)
    }

    pub fn top(&self) -> Result<Value, RuntimeError> {
        if self.stack_len == 0 {
            return Err(RuntimeError::StackUnderflow);
        }
        Ok(self.stack[self.stack_len - 1].get())
    }

    pub fn intern_string(&mut self, s: &str) -> Handle<String> {
        if let Some(interned) = self.strings.get(s) {
            interned.handle
        } else {
            let handle = self.create_object(s);
            self.strings.insert(InternedString { handle });
            handle
        }
    }

    pub fn create_closure(
        &mut self,
        function_index: usize,
        upvalues: Vec<Handle<Upvalue>>,
    ) -> Handle<Closure> {
        self.create_object(NewClosure {
            function_index,
            upvalues,
        })
    }

    pub fn capture_upvalue(&mut self, stack_index: usize) -> Handle<Upvalue> {
        if let Some(open_upvalue) = self.open_upvalues.get(&stack_index) {
            *open_upvalue
        } else {
            let location = &self.stack[stack_index] as *const _;
            let upvalue = self.create_object(location);
            self.open_upvalues.insert(stack_index, upvalue);
            upvalue
        }
    }

    pub fn create_class(&mut self, class_index: usize) -> Handle<Class> {
        self.create_object(NewClass { class_index })
    }

    pub fn create_instance(
        &mut self,
        class: Handle<Class>,
    ) -> Handle<Instance> {
        self.create_object(NewInstance { class })
    }

    pub fn create_bound_method(
        &mut self,
        receiver: Handle<Instance>,
        method: Handle<Closure>,
    ) -> Handle<BoundMethod> {
        self.create_object(NewBoundMethod { receiver, method })
    }

    pub fn close_upvalues_ge(&mut self, stack_index: usize) {
        while let Some(last) = self.open_upvalues.last_entry()
            && *last.key() >= stack_index
        {
            last.get().close();
            last.remove();
        }
    }

    fn collect_garbage(&mut self, temporary: ObjectHandle) {
        #[cfg(feature = "debug_gc")]
        println!("-- gc begin");

        let mut frontier = Vec::new();

        temporary.gc_mark(&mut frontier);

        for global in self.globals.values() {
            if let Some(handle) = value_to_object_handle(*global) {
                handle.gc_mark(&mut frontier);
            }
        }

        for i in 0..self.stack_len {
            if let Some(handle) = value_to_object_handle(self.stack[i].get()) {
                handle.gc_mark(&mut frontier);
            }
        }

        for upvalue in self.open_upvalues.values() {
            Handle::erase(*upvalue).gc_mark(&mut frontier);
        }

        while let Some(last) = frontier.pop() {
            last.gc_mark(&mut frontier);
        }

        let mut previous = None;
        let mut current = self.handles;
        while let Some(current_handle) = current.take() {
            if current_handle.gc_is_marked() {
                current_handle.gc_unmark();
                previous = Some(current_handle);
                current = current_handle.gc_next();
            } else {
                current = current_handle.gc_next();

                if let Some(string_handle) = current_handle.downcast::<String>()
                {
                    self.strings.remove(&InternedString {
                        handle: string_handle,
                    });
                }

                current_handle.operate(DestroyOp { memory: self });

                if let Some(previous_handle) = previous.as_mut() {
                    previous_handle.gc_set_next(current);
                } else {
                    self.handles = current;
                }
            }
        }

        self.next_gc = self.bytes_allocated * 2;

        #[cfg(feature = "debug_gc")]
        println!("-- gc end");
    }

    fn create_object<T: ObjectKind + ?Sized>(
        &mut self,
        emplacer: impl Emplace<T>,
    ) -> Handle<T> {
        let handle = Handle::create(emplacer, self.handles);

        self.bytes_allocated += Handle::object_layout(handle).size();
        let erased = Handle::erase(handle);
        self.handles = Some(erased);

        if self.bytes_allocated > self.next_gc || cfg!(feature = "debug_gc") {
            self.collect_garbage(erased);
        }

        handle
    }

    unsafe fn destroy_object<T: ObjectKind + ?Sized>(
        &mut self,
        handle: Handle<T>,
    ) {
        self.bytes_allocated -= Handle::object_layout(handle).size();

        unsafe {
            Handle::destroy(handle);
        }
    }
}

struct DestroyOp<'a> {
    memory: &'a mut Memory,
}

impl<T: ObjectKind + ?Sized> HandleOperation<T> for DestroyOp<'_> {
    fn operate(self, handle: Handle<T>) {
        unsafe {
            self.memory.destroy_object(handle);
        }
    }
}

fn value_to_object_handle(value: Value) -> Option<ObjectHandle> {
    match value.unpack() {
        crate::UnpackedValue::Float(_)
        | crate::UnpackedValue::Register(_)
        | crate::UnpackedValue::Nil
        | crate::UnpackedValue::False
        | crate::UnpackedValue::True
        | crate::UnpackedValue::NativeFunction(_) => None,
        crate::UnpackedValue::String(handle) => Some(Handle::erase(handle)),
        crate::UnpackedValue::Closure(handle) => Some(Handle::erase(handle)),
        crate::UnpackedValue::Class(handle) => Some(Handle::erase(handle)),
        crate::UnpackedValue::Instance(handle) => Some(Handle::erase(handle)),
        crate::UnpackedValue::BoundMethod(handle) => {
            Some(Handle::erase(handle))
        }
    }
}
