use core::hash::BuildHasher;
use std::{
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH},
};

use hashbrown::{DefaultHashBuilder, HashTable};

use crate::{
    Codec, Constant, Executable, NativeFunction, Op, RuntimeDiagnostic,
    RuntimeError, UnpackedValue, Value, global_values,
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
            Constant::Float(f) => Ok(Value::float(*f)),
            Constant::String(s) => Ok(Value::string(self.intern_string(s))),
            Constant::Function(i) => Ok(Value::function(*i)),
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
            self.trace()?;

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
                Op::Nil => self.push(Value::nil())?,
                Op::True => self.push(Value::boolean(true))?,
                Op::False => self.push(Value::boolean(false))?,
                Op::Constant { index } | Op::ConstantLong { index } => {
                    let value = self.reify_constant(index)?;
                    self.push(value)?;
                }
                Op::Not => {
                    let target = self.pop()?;
                    self.push(Value::boolean(!target.truthiness()))?;
                }
                Op::Negate => self.unary_float(|n| -n)?,
                Op::Add => {
                    let rhs = self.pop()?;
                    let lhs = self.pop()?;
                    let result = match (lhs.unpack()?, rhs.unpack()?) {
                        (
                            UnpackedValue::Float(lhs),
                            UnpackedValue::Float(rhs),
                        ) => Value::float(lhs + rhs),
                        (
                            UnpackedValue::String(lhs),
                            UnpackedValue::String(rhs),
                        ) => Value::string(self.intern_string(format!(
                            "{}{}",
                            self.strings[lhs], self.strings[rhs]
                        ))),
                        (UnpackedValue::Float(_), actual) => {
                            return Err(RuntimeError::ExpectedFloat { actual });
                        }
                        (UnpackedValue::String(_), actual) => {
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
                    self.push(Value::boolean(rhs == lhs))?;
                }
                Op::Greater => {
                    let rhs = self.pop()?.as_float();
                    let lhs = self.pop()?.as_float();
                    self.push(Value::boolean(lhs > rhs))?;
                }
                Op::Less => {
                    let rhs = self.pop()?.as_float();
                    let lhs = self.pop()?.as_float();
                    self.push(Value::boolean(lhs < rhs))?;
                }
                Op::Print => {
                    let value = self.pop()?;
                    self.print_value(&value)?;
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
                            value: prev.unpack()?,
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
                    let value = self.top()?.clone();
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
                    let value = self.top()?.clone();
                    let Some(target) = self.stack.get_mut(self.fp + index)
                    else {
                        return Err(RuntimeError::LocalVariableOutOfBounds {
                            index,
                        });
                    };
                    *target = value;
                }
                Op::JumpIfFalse { distance } => {
                    if !self.top()?.truthiness() {
                        next_ip += distance;
                    }
                }
                Op::Jump { distance } => next_ip += distance,
                Op::Loop { distance } => next_ip -= distance,
                Op::PushFrame => {
                    self.stack.push(Value::integer(self.fp));
                    self.stack.push(Value::integer(0));
                }
                Op::Call { arity } => {
                    if self.stack.len() <= arity {
                        return Err(RuntimeError::TooFewArguments { arity });
                    }

                    self.fp = self.stack.len() - arity - 1;
                    let target = self.stack[self.fp].clone();

                    match target.unpack()? {
                        UnpackedValue::Function(index) => {
                            self.stack[self.fp - 1] = Value::integer(next_ip);
                            next_ip = self.executable.functions[index].ip;
                        }
                        UnpackedValue::NativeFunction(function) => {
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

    fn print_value(&self, value: &Value) -> Result<(), RuntimeError> {
        match value.unpack()? {
            UnpackedValue::Float(f) => print!("{f}"),
            UnpackedValue::Nil => print!("<nil>"),
            UnpackedValue::False => print!("false"),
            UnpackedValue::True => print!("true"),
            UnpackedValue::String(i) => print!("{}", &self.strings[i]),
            UnpackedValue::Function(i) => {
                print!("<fun {}>", self.executable.functions[i].name)
            }
            UnpackedValue::NativeFunction(f) => print!("<nat {}>", f.name()),
            UnpackedValue::Integer(fp) => print!("<int {fp:04x}>"),
        }

        Ok(())
    }

    #[allow(unused)]
    fn trace(&self) -> Result<(), RuntimeError> {
        print!("          ");
        for value in &self.stack {
            print!("[");
            self.print_value(value)?;
            print!("]");
        }
        println!();

        let mut ip = self.ip;
        self.executable.chunk.disassemble_instruction(&mut ip);

        Ok(())
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

    fn top(&self) -> Result<&Value, RuntimeError> {
        self.stack.last().ok_or(RuntimeError::StackUnderflow)
    }

    fn unary_float(
        &mut self,
        f: impl FnOnce(f64) -> f64,
    ) -> Result<(), RuntimeError> {
        let target = self.pop()?.as_float();
        self.push(Value::float(f(target)))?;
        Ok(())
    }

    fn binary_float(
        &mut self,
        f: impl FnOnce(f64, f64) -> f64,
    ) -> Result<(), RuntimeError> {
        let rhs = self.pop()?.as_float();
        let lhs = self.pop()?.as_float();
        self.push(Value::float(f(lhs, rhs)))?;
        Ok(())
    }

    fn pop_frame(&mut self) -> Result<usize, RuntimeError> {
        while self.stack.len() > self.fp {
            self.pop()?;
        }

        let return_ip = self.pop()?.as_integer();
        let return_fp = self.pop()?.as_integer();

        self.fp = return_fp as usize;

        Ok(return_ip as usize)
    }

    fn call_native(
        &mut self,
        native_function: NativeFunction,
    ) -> Result<Value, RuntimeError> {
        Ok(match native_function {
            NativeFunction::Clock => Value::float(
                SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs_f64(),
            ),
        })
    }
}
