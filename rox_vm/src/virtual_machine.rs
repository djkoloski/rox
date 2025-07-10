use core::hash::BuildHasher;
use std::{
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH},
};

use hashbrown::{DefaultHashBuilder, HashTable};

use crate::{
    Codec, Constant, Executable, NativeFunction, Op, RuntimeDiagnostic,
    RuntimeError, Value, global_values,
};

const MAX_STACK_LEN: usize = 255;

pub struct VirtualMachine<'exe> {
    executable: &'exe Executable,
    ip: usize,
    fp: usize,
    stack: Vec<Value>,
    hasher: DefaultHashBuilder,
    string_index: HashTable<usize>,
    strings: Vec<String>,
    globals: HashMap<String, Value>,
}

impl<'exe> VirtualMachine<'exe> {
    pub fn new(executable: &'exe Executable) -> Self {
        Self {
            executable,
            ip: 0,
            fp: 0,
            stack: Vec::new(),
            hasher: DefaultHashBuilder::default(),
            string_index: HashTable::new(),
            strings: Vec::new(),
            globals: global_values(),
        }
    }

    fn get_constant(
        &mut self,
        index: usize,
    ) -> Result<&'exe Constant, RuntimeError> {
        self.executable
            .chunk
            .constants()
            .get(index)
            .ok_or(RuntimeError::ConstantOutOfBounds)
    }

    fn get_variable_name(
        &mut self,
        index: usize,
    ) -> Result<&'exe String, RuntimeError> {
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
            Constant::Function(i) => Ok(Value::Function(*i)),
        }
    }

    pub fn execute(&mut self) -> Result<(), RuntimeDiagnostic> {
        if let Err(error) = self.execute_inner() {
            return Err(RuntimeDiagnostic::new(
                error,
                self.executable.chunk.span(self.ip),
            ));
        }
        Ok(())
    }

    fn execute_inner(&mut self) -> Result<(), RuntimeError> {
        while self.ip < self.executable.chunk.bytes().len() {
            #[cfg(feature = "trace")]
            self.trace();

            let mut next_ip = self.ip;
            match Op::decode(self.executable.chunk.bytes(), &mut next_ip)? {
                Op::Return => {
                    if self.fp == 0 {
                        break;
                    }

                    let value = self.pop()?;
                    next_ip = self.pop_frame()?;

                    self.push(value)?;
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
                    self.print_value(&value);
                    println!();
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
                    let Some(value) = self.stack.get(self.fp + index) else {
                        return Err(RuntimeError::LocalVariableOutOfBounds {
                            index,
                        });
                    };
                    self.push(value.clone())?;
                }
                Op::SetLocal { index } | Op::SetLocalLong { index } => {
                    let value = self.last()?.clone();
                    let Some(target) = self.stack.get_mut(self.fp + index)
                    else {
                        return Err(RuntimeError::LocalVariableOutOfBounds {
                            index,
                        });
                    };
                    *target = value;
                }
                Op::JumpIfFalse { distance } => {
                    if !self.last()?.truthiness() {
                        next_ip += distance;
                    }
                }
                Op::Jump { distance } => next_ip += distance,
                Op::Loop { distance } => next_ip -= distance,
                // TODO: what if control flow diverges between PushFrame and
                // Call?
                Op::PushFrame => {
                    self.stack.push(Value::FramePointer(self.fp));
                    self.stack.push(Value::InstructionPointer(0));
                }
                Op::Call { arity } => {
                    if self.stack.len() <= arity {
                        return Err(RuntimeError::TooFewArguments { arity });
                    }

                    self.fp = self.stack.len() - arity - 1;
                    let target = self.stack[self.fp].clone();

                    match target {
                        Value::Function(index) => {
                            self.stack[self.fp - 1] =
                                Value::InstructionPointer(next_ip);
                            next_ip = self.executable.functions[index].ip;
                        }
                        Value::NativeFunction(function) => {
                            let return_value = self.call_native(function)?;
                            self.pop_frame()?;
                            self.push(return_value)?;
                        }
                        actual => {
                            return Err(RuntimeError::ExpectedFunction {
                                actual,
                            });
                        }
                    }
                }
            }
            self.ip = next_ip;
        }
        Ok(())
    }

    fn print_value(&self, value: &Value) {
        match value {
            Value::Float(f) => print!("{f}"),
            Value::Boolean(b) => print!("{b}"),
            Value::Nil => print!("<nil>"),
            Value::String(i) => print!("{}", &self.strings[*i]),
            Value::Function(i) => {
                print!("<fun {}>", self.executable.functions[*i].name)
            }
            Value::NativeFunction(f) => print!("<nat {}>", f.name()),
            Value::FramePointer(fp) => print!("<fp {fp:04x}>"),
            Value::InstructionPointer(ip) => print!("<ip {ip:04x}>"),
        }
    }

    #[allow(unused)]
    fn trace(&self) {
        print!("          ");
        for value in &self.stack {
            print!("[");
            self.print_value(value);
            print!("]");
        }
        println!();

        let mut ip = self.ip;
        self.executable.chunk.disassemble_instruction(&mut ip);
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

    fn last(&self) -> Result<&Value, RuntimeError> {
        self.stack.last().ok_or(RuntimeError::StackUnderflow)
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

    fn pop_frame(&mut self) -> Result<usize, RuntimeError> {
        while self.stack.len() > self.fp {
            self.pop()?;
        }

        let Value::InstructionPointer(return_ip) = self.pop()? else {
            unreachable!();
        };
        let Value::FramePointer(return_fp) = self.pop()? else {
            unreachable!();
        };

        self.fp = return_fp;

        Ok(return_ip)
    }

    fn call_native(
        &mut self,
        native_function: NativeFunction,
    ) -> Result<Value, RuntimeError> {
        Ok(match native_function {
            NativeFunction::Clock => Value::Float(
                SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs_f64(),
            ),
        })
    }
}
