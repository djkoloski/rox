use core::cell::Cell;
use std::collections::{BTreeMap, HashMap, HashSet};

use crate::{
    Closure, ErasedHandle, Handle, ObjectKind, RuntimeError, String, Upvalue,
    Value, global_values,
};

const MAX_STACK_LEN: usize = 255;

pub struct Memory {
    globals: HashMap<std::string::String, Value>,
    stack: Box<[Cell<Value>; MAX_STACK_LEN]>,
    stack_len: usize,
    handles: Option<ErasedHandle>,
    open_upvalues: BTreeMap<usize, Handle<Upvalue>>,

    strings: HashSet<Handle<String>>,
    bytes_allocated: usize,
    next_gc: usize,
}

impl Drop for Memory {
    fn drop(&mut self) {
        self.strings.clear();

        while let Some(handle) = self.handles.clone() {
            self.handles = unsafe { handle.gc_sweep() };
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

    pub fn intern_string(&mut self, s: &str) -> Handle<String> {
        if let Some(handle) = self.strings.get(s) {
            handle.clone()
        } else {
            let handle =
                self.track_handle(|next| Handle::create_string(next, s));
            self.strings.insert(handle.clone());
            handle
        }
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

    pub fn create_closure(
        &mut self,
        function_index: usize,
        upvalues: Vec<Handle<Upvalue>>,
    ) -> Handle<Closure> {
        self.track_handle(|next| {
            Handle::create_closure(next, function_index, upvalues)
        })
    }

    pub fn close_upvalues_ge(&mut self, stack_index: usize) {
        while let Some(last) = self.open_upvalues.last_entry()
            && *last.key() >= stack_index
        {
            last.get().close();
            last.remove();
        }
    }

    pub fn capture_upvalue(&mut self, stack_index: usize) -> Handle<Upvalue> {
        if let Some(open_upvalue) = self.open_upvalues.get(&stack_index) {
            open_upvalue.clone()
        } else {
            let location = &self.stack[stack_index] as *const _;
            let upvalue = self.track_handle(|next| unsafe {
                Handle::create_upvalue(next, location)
            });
            self.open_upvalues.insert(stack_index, upvalue.clone());
            upvalue
        }
    }

    fn track_handle<T: ObjectKind + ?Sized>(
        &mut self,
        f: impl FnOnce(Option<ErasedHandle>) -> Handle<T>,
    ) -> Handle<T> {
        if self.bytes_allocated > self.next_gc || cfg!(feature = "debug_gc") {
            self.collect_garbage();
        }

        let handle = f(self.handles.clone());

        let erased = Handle::erase(handle.clone());
        self.bytes_allocated += erased.gc_layout().size();
        self.handles = Some(erased);

        handle
    }

    fn collect_garbage(&mut self) {
        #[cfg(feature = "debug_gc")]
        println!("-- gc begin");

        let mut frontier = Vec::new();

        for global in self.globals.values() {
            global.gc_mark(&mut frontier);
        }

        for i in 0..self.stack_len {
            self.stack[i].get().gc_mark(&mut frontier);
        }

        for upvalue in self.open_upvalues.values() {
            upvalue.gc_mark(&mut frontier);
        }

        while let Some(last) = frontier.pop() {
            last.gc_mark(&mut frontier);
        }

        let mut previous = None;
        let mut current = self.handles.clone();
        while let Some(current_handle) = current.take() {
            if current_handle.gc_is_marked() {
                current_handle.gc_unmark();
                previous = Some(current_handle.clone());
                current = current_handle.gc_next();
            } else {
                if let Some(string_handle) =
                    current_handle.clone().downcast::<String>()
                {
                    self.strings.remove(&string_handle);
                }
                self.bytes_allocated -= current_handle.gc_layout().size();
                current = unsafe { current_handle.gc_sweep() };
                if let Some(previous_handle) = previous.as_mut() {
                    previous_handle.gc_set_next(current.clone());
                } else {
                    self.handles = current.clone();
                }
            }
        }

        self.next_gc = self.bytes_allocated * 2;

        #[cfg(feature = "debug_gc")]
        println!("-- gc end");
    }
}
