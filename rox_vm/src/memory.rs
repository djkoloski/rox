use core::cell::Cell;
use std::collections::{HashMap, HashSet};

use crate::{
    Closure, Handle, Heap, RuntimeError, String, Upvalue, Value, global_values,
};

const MAX_STACK_LEN: usize = 255;

pub struct Memory {
    globals: HashMap<std::string::String, Value>,
    stack: Box<[Cell<Value>; MAX_STACK_LEN]>,
    stack_len: usize,
    heap: Heap,

    strings: HashSet<Handle<String>>,
}

impl Memory {
    pub fn new() -> Self {
        Self {
            globals: global_values(),
            stack: Box::new([const { Cell::new(Value::nil()) }; MAX_STACK_LEN]),
            stack_len: 0,
            heap: Heap::new(),

            strings: HashSet::new(),
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
                value: prev.unpack()?,
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
            let handle = self.heap.create_string(s);
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
        self.heap.create_closure(function_index, upvalues)
    }

    pub fn create_upvalue(&mut self, stack_index: usize) -> Handle<Upvalue> {
        unsafe { self.heap.create_upvalue(&self.stack[stack_index]) }
    }
}

impl Default for Memory {
    fn default() -> Self {
        Self::new()
    }
}
