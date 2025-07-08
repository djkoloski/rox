use core::{fmt, hash::BuildHasher};
use std::collections::HashMap;

use hashbrown::{DefaultHashBuilder, HashTable};

use crate::{
    Chunk, Codec, Constant, Op, RuntimeDiagnostic, RuntimeError, Value,
};

const MAX_STACK_LEN: usize = 255;

pub struct VirtualMachine<'chunk> {
    chunk: &'chunk Chunk,
    last_ip: usize,
    ip: usize,
    stack: Vec<Value>,
    hasher: DefaultHashBuilder,
    string_index: HashTable<usize>,
    strings: Vec<String>,
    globals: HashMap<String, Value>,
}

impl<'chunk> VirtualMachine<'chunk> {
    pub fn new(chunk: &'chunk Chunk) -> Self {
        Self {
            chunk,
            last_ip: 0,
            ip: 0,
            stack: Vec::new(),
            hasher: DefaultHashBuilder::default(),
            string_index: HashTable::new(),
            strings: Vec::new(),
            globals: HashMap::new(),
        }
    }

    fn get_constant(
        &mut self,
        index: usize,
    ) -> Result<&'chunk Constant, RuntimeError> {
        self.chunk
            .constants()
            .get(index)
            .ok_or(RuntimeError::ConstantOutOfBounds)
    }

    fn get_variable_name(
        &mut self,
        index: usize,
    ) -> Result<&'chunk String, RuntimeError> {
        let Constant::String(name) = self.get_constant(index)? else {
            return Err(RuntimeError::ExpectedVariableName { index });
        };

        Ok(name)
    }

    fn intern_string<S: AsRef<str> + Into<String>>(&mut self, s: S) -> usize {
        use hashbrown::hash_table::Entry;

        let hash = self.hasher.hash_one(s.as_ref());
        let entry = self.string_index.entry(
            hash,
            |&i| self.strings[i] == s.as_ref(),
            |&i| self.hasher.hash_one(&self.strings[i]),
        );
        match entry {
            Entry::Occupied(occupied) => *occupied.get(),
            Entry::Vacant(vacant) => {
                let result = self.strings.len();
                vacant.insert(result);
                self.strings.push(s.into());
                result
            }
        }
    }

    fn reify_constant(&mut self, index: usize) -> Result<Value, RuntimeError> {
        match self.get_constant(index)? {
            Constant::Float(f) => Ok(Value::Float(*f)),
            Constant::String(s) => Ok(Value::String(self.intern_string(s))),
        }
    }

    pub fn execute(&mut self) -> Result<(), RuntimeDiagnostic> {
        if let Err(error) = self.execute_inner() {
            return Err(RuntimeDiagnostic::new(
                error,
                self.chunk.span(self.last_ip),
            ));
        }
        Ok(())
    }

    fn execute_inner(&mut self) -> Result<(), RuntimeError> {
        while self.ip < self.chunk.bytes().len() {
            #[cfg(feature = "trace")]
            self.trace();

            match self.read_op()? {
                Op::Return => {
                    let value = self.pop()?;
                    println!("{}", self.display(&value));
                    break;
                }
                Op::Nil => self.push(Value::Nil)?,
                Op::True => self.push(Value::Boolean(true))?,
                Op::False => self.push(Value::Boolean(false))?,
                Op::Constant { index } | Op::ConstantLong { index } => {
                    let value = self.reify_constant(index)?;
                    self.push(value)?;
                }
                Op::Not => {
                    let target = self.pop()?;
                    self.push(Value::Boolean(!target.truthiness()))?;
                }
                Op::Negate => self.unary_float(|n| -n)?,
                Op::Add => {
                    let rhs = self.pop()?;
                    let lhs = self.pop()?;
                    let result = match (lhs, rhs) {
                        (Value::Float(lhs), Value::Float(rhs)) => {
                            Value::Float(lhs + rhs)
                        }
                        (Value::String(lhs), Value::String(rhs)) => {
                            Value::String(self.intern_string(format!(
                                "{}{}",
                                self.strings[lhs], self.strings[rhs]
                            )))
                        }
                        (Value::Float(_), actual) => {
                            return Err(RuntimeError::ExpectedFloat { actual });
                        }
                        (Value::String(_), actual) => {
                            return Err(RuntimeError::ExpectedString {
                                actual,
                            });
                        }
                        (actual, _) => {
                            return Err(RuntimeError::ExpectedFloatOrString {
                                actual,
                            });
                        }
                    };
                    self.push(result)?;
                }
                Op::Subtract => self.binary_float(|a, b| a - b)?,
                Op::Multiply => self.binary_float(|a, b| a * b)?,
                Op::Divide => self.binary_float(|a, b| a / b)?,
                Op::Equal => {
                    let rhs = self.pop()?;
                    let lhs = self.pop()?;
                    self.push(Value::Boolean(rhs == lhs))?;
                }
                Op::Greater => {
                    let rhs = self.pop()?.float()?;
                    let lhs = self.pop()?.float()?;
                    self.push(Value::Boolean(lhs > rhs))?;
                }
                Op::Less => {
                    let rhs = self.pop()?.float()?;
                    let lhs = self.pop()?.float()?;
                    self.push(Value::Boolean(lhs < rhs))?;
                }
                Op::Print => {
                    let value = self.pop()?;
                    println!("{}", self.display(&value));
                }
                Op::Pop => {
                    self.pop()?;
                }
                Op::DefineGlobal { index } | Op::DefineGlobalLong { index } => {
                    let name = self.get_variable_name(index)?;
                    let value = self.pop()?;

                    if let Some(prev) = self.globals.insert(name.clone(), value)
                    {
                        return Err(RuntimeError::GlobalAlreadyDefined {
                            name: name.clone(),
                            value: prev,
                        });
                    }
                }
                Op::GetGlobal { index } | Op::GetGlobalLong { index } => {
                    let name = self.get_variable_name(index)?;
                    let Some(global) = self.globals.get(name) else {
                        return Err(RuntimeError::UndefinedGlobal {
                            name: name.clone(),
                        });
                    };
                    self.push(global.clone())?;
                }
                Op::SetGlobal { index } | Op::SetGlobalLong { index } => {
                    let name = self.get_variable_name(index)?;
                    let value = self.pop()?;
                    let Some(target) = self.globals.get_mut(name) else {
                        return Err(RuntimeError::UndefinedGlobal {
                            name: name.clone(),
                        });
                    };
                    *target = value;
                }
                Op::GetLocal { index } | Op::GetLocalLong { index } => {
                    let Some(value) = self.stack.get(index) else {
                        return Err(RuntimeError::LocalVariableOutOfBounds {
                            index,
                        });
                    };
                    self.push(value.clone())?;
                }
                Op::SetLocal { index } | Op::SetLocalLong { index } => {
                    let value = self.pop()?;
                    let Some(target) = self.stack.get_mut(index) else {
                        return Err(RuntimeError::LocalVariableOutOfBounds {
                            index,
                        });
                    };
                    *target = value;
                }
            }
        }
        Ok(())
    }

    fn display<'a>(&'a self, value: &'a Value) -> &'a dyn fmt::Display {
        match value {
            Value::Float(f) => f,
            Value::Boolean(b) => b,
            Value::Nil => &"<nil>",
            Value::String(i) => &self.strings[*i],
        }
    }

    #[allow(unused)]
    fn trace(&self) {
        print!("          ");
        for value in &self.stack {
            print!("[{}]", self.display(value));
        }
        println!();

        let mut ip = self.ip;
        self.chunk.disassemble_instruction(&mut ip);
    }

    fn read_op(&mut self) -> Result<Op, RuntimeError> {
        self.last_ip = self.ip;
        Ok(Op::decode(self.chunk.bytes(), &mut self.ip)?)
    }

    fn push(&mut self, value: Value) -> Result<(), RuntimeError> {
        if self.stack.len() == MAX_STACK_LEN {
            return Err(RuntimeError::StackOverflow);
        }
        self.stack.push(value);
        Ok(())
    }

    fn pop(&mut self) -> Result<Value, RuntimeError> {
        self.stack.pop().ok_or(RuntimeError::StackUnderflow)
    }

    fn unary_float(
        &mut self,
        f: impl FnOnce(f64) -> f64,
    ) -> Result<(), RuntimeError> {
        let target = self.pop()?.float()?;
        self.push(Value::Float(f(target)))?;
        Ok(())
    }

    fn binary_float(
        &mut self,
        f: impl FnOnce(f64, f64) -> f64,
    ) -> Result<(), RuntimeError> {
        let rhs = self.pop()?.float()?;
        let lhs = self.pop()?.float()?;
        self.push(Value::Float(f(lhs, rhs)))?;
        Ok(())
    }
}
