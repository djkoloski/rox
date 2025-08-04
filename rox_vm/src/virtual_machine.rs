use std::time::{SystemTime, UNIX_EPOCH};

use crate::{
    Closure, Codec, Constant, Executable, Handle, Memory, NativeFunction, Op,
    Place, RuntimeDiagnostic, RuntimeError, UnpackedValue, Upvalue, Value,
};

pub struct VirtualMachine<'exe> {
    executable: &'exe Executable,
    ip: usize,
    fp: usize,

    memory: Memory,
}

impl<'exe> VirtualMachine<'exe> {
    pub fn new(executable: &'exe Executable) -> Self {
        Self {
            executable,
            ip: 0,
            fp: 0,

            memory: unsafe { Memory::new() },
        }
    }

    fn get_constant(
        &mut self,
        constant_index: usize,
    ) -> Result<&'exe Constant, RuntimeError> {
        self.executable
            .chunk
            .constants()
            .get(constant_index)
            .ok_or(RuntimeError::ConstantOutOfBounds)
    }

    fn get_name(
        &mut self,
        constant_index: usize,
    ) -> Result<&'exe String, RuntimeError> {
        let name = match self.get_constant(constant_index)? {
            Constant::String(name) => name,
            Constant::Float(actual) => {
                return Err(RuntimeError::ExpectedName { actual: *actual });
            }
        };

        Ok(name)
    }

    fn reify_constant(
        &mut self,
        constant_index: usize,
    ) -> Result<Value, RuntimeError> {
        match self.get_constant(constant_index)? {
            Constant::Float(f) => Ok(Value::float(*f)),
            Constant::String(s) => {
                Ok(Value::string(self.memory.intern_string(s)))
            }
        }
    }

    fn pop_frame(&mut self) -> Result<usize, RuntimeError> {
        self.memory.close_upvalues_ge(self.fp);

        while self.memory.stack_len() > self.fp {
            self.memory.pop()?;
        }

        let _callee = self.memory.pop()?;
        let return_ip = self.memory.pop()?.as_register();
        let return_fp = self.memory.pop()?.as_register();

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

    fn read_local(
        &mut self,
        local_index: usize,
    ) -> Result<Value, RuntimeError> {
        self.memory.read_stack(self.fp + local_index)
    }

    fn write_local(
        &mut self,
        local_index: usize,
        value: Value,
    ) -> Result<(), RuntimeError> {
        self.memory.write_stack(self.fp + local_index, value)
    }

    fn get_upvalue(
        &mut self,
        upvalue_index: usize,
    ) -> Result<Handle<Upvalue>, RuntimeError> {
        let UnpackedValue::Closure(closure) =
            self.memory.read_stack(self.fp)?.unpack()
        else {
            return Err(RuntimeError::UpvalueAtGlobalScope);
        };

        let Some(upvalue) = closure.upvalues.get(upvalue_index) else {
            return Err(RuntimeError::UpvalueOutOfBounds { upvalue_index });
        };

        Ok(*upvalue)
    }

    fn capture(
        &mut self,
        place: &Place,
    ) -> Result<Handle<Upvalue>, RuntimeError> {
        Ok(match place {
            Place::Local { local_index } => {
                let stack_index = self.fp + local_index;
                self.memory.capture_upvalue(stack_index)
            }
            Place::Upvalue { upvalue_index } => {
                self.get_upvalue(*upvalue_index)?
            }
        })
    }

    fn close_function(
        &mut self,
        function_index: usize,
    ) -> Result<Handle<Closure>, RuntimeError> {
        let function = &self.executable.functions[function_index];

        let mut upvalues = Vec::new();
        for place in &function.captures {
            upvalues.push(self.capture(place)?);
        }

        Ok(self.memory.create_closure(function_index, upvalues))
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

                    let value = self.memory.pop()?;
                    next_ip = self.pop_frame()?;

                    self.memory.push(value)?;
                }
                Op::Nil => self.memory.push(Value::nil())?,
                Op::True => self.memory.push(Value::boolean(true))?,
                Op::False => self.memory.push(Value::boolean(false))?,
                Op::Constant { constant_index }
                | Op::ConstantLong { constant_index } => {
                    let value = self.reify_constant(constant_index)?;
                    self.memory.push(value)?;
                }
                Op::Not => {
                    let target = self.memory.pop()?;
                    self.memory.push(Value::boolean(!target.truthiness()))?;
                }
                Op::Negate => self.unary_float(|n| -n)?,
                Op::Add => {
                    let rhs = self.memory.pop()?;
                    let lhs = self.memory.pop()?;
                    let result = match (lhs.unpack(), rhs.unpack()) {
                        (
                            UnpackedValue::Float(lhs),
                            UnpackedValue::Float(rhs),
                        ) => Value::float(lhs + rhs),
                        (
                            UnpackedValue::String(lhs),
                            UnpackedValue::String(rhs),
                        ) => Value::string(
                            self.memory.intern_string(&format!("{lhs}{rhs}")),
                        ),
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
                    self.memory.push(result)?;
                }
                Op::Subtract => self.binary_float(|a, b| a - b)?,
                Op::Multiply => self.binary_float(|a, b| a * b)?,
                Op::Divide => self.binary_float(|a, b| a / b)?,
                Op::Equal => {
                    let rhs = self.memory.pop()?;
                    let lhs = self.memory.pop()?;
                    self.memory.push(Value::boolean(rhs == lhs))?;
                }
                Op::Greater => {
                    let rhs = self.memory.pop()?.as_float();
                    let lhs = self.memory.pop()?.as_float();
                    self.memory.push(Value::boolean(lhs > rhs))?;
                }
                Op::Less => {
                    let rhs = self.memory.pop()?.as_float();
                    let lhs = self.memory.pop()?.as_float();
                    self.memory.push(Value::boolean(lhs < rhs))?;
                }
                Op::Print => {
                    let value = self.memory.pop()?;
                    self.display_value(&value);
                    println!();
                }
                Op::Pop => {
                    self.memory.pop()?;
                }
                Op::DefineGlobal { constant_index }
                | Op::DefineGlobalLong { constant_index } => {
                    let name = self.get_name(constant_index)?;
                    let value = self.memory.pop()?;

                    self.memory.insert_global(name, value)?
                }
                Op::GetGlobal { constant_index }
                | Op::GetGlobalLong { constant_index } => {
                    let name = self.get_name(constant_index)?;
                    let value = self.memory.read_global(name)?;
                    self.memory.push(value)?;
                }
                Op::SetGlobal { constant_index }
                | Op::SetGlobalLong { constant_index } => {
                    let name = self.get_name(constant_index)?;
                    let value = self.memory.top()?;
                    self.memory.write_global(name, value)?;
                }
                Op::GetLocal { local_index }
                | Op::GetLocalLong { local_index } => {
                    let value = self.read_local(local_index)?;
                    self.memory.push(value)?;
                }
                Op::SetLocal { local_index }
                | Op::SetLocalLong { local_index } => {
                    let value = self.memory.top()?;
                    self.write_local(local_index, value)?;
                }
                Op::JumpIfFalse { distance } => {
                    if !self.memory.top()?.truthiness() {
                        next_ip += distance;
                    }
                }
                Op::Jump { distance } => next_ip += distance,
                Op::Loop { distance } => next_ip -= distance,
                Op::PushFrame => {
                    self.memory.push(Value::register(0))?;
                    self.memory.push(Value::register(0))?;
                    self.memory.push(Value::nil())?;
                }
                Op::Call { arity } => {
                    if self.memory.stack_len() <= self.fp + arity {
                        return Err(RuntimeError::TooFewArguments { arity });
                    }

                    let next_fp = self.memory.stack_len() - arity - 1;
                    let target = self.memory.read_stack(next_fp)?;

                    self.memory.write_stack(
                        next_fp - 3,
                        Value::register(self.fp as u64),
                    )?;
                    self.memory.write_stack(
                        next_fp - 2,
                        Value::register(next_ip as u64),
                    )?;
                    self.memory.write_stack(next_fp - 1, target)?;

                    self.fp = next_fp;

                    match target.unpack() {
                        UnpackedValue::Closure(closure) => {
                            let function = &self.executable.functions
                                [closure.function_index];
                            next_ip = function.ip;
                        }
                        UnpackedValue::NativeFunction(
                            native_function_index,
                        ) => {
                            let native_function = NativeFunction::try_from(
                                native_function_index,
                            )?;
                            let return_value =
                                self.call_native(native_function)?;
                            self.pop_frame()?;
                            self.memory.push(return_value)?;
                        }
                        UnpackedValue::Class(class) => {
                            self.pop_frame()?;
                            let instance = self.memory.create_instance(class);
                            self.memory.push(Value::instance(instance))?;
                        }
                        UnpackedValue::BoundMethod(bound_method) => {
                            self.memory.write_stack(
                                self.fp,
                                Value::instance(bound_method.receiver),
                            )?;
                            let function = &self.executable.functions
                                [bound_method.method.function_index];
                            next_ip = function.ip;
                        }
                        actual => {
                            return Err(RuntimeError::ExpectedCallable {
                                actual,
                            });
                        }
                    }
                }
                Op::CloseFunction { function_index }
                | Op::CloseFunctionLong { function_index } => {
                    let closure = self.close_function(function_index)?;
                    self.memory.push(Value::closure(closure))?;
                }
                Op::CloseLocal => {
                    let stack_len = self.memory.stack_len();
                    if stack_len > 0 {
                        self.memory.close_upvalues_ge(stack_len - 1);
                    }
                    self.memory.pop()?;
                }
                Op::GetUpvalue { upvalue_index }
                | Op::GetUpvalueLong { upvalue_index } => {
                    let value = self.get_upvalue(upvalue_index)?.read();
                    self.memory.push(value)?;
                }
                Op::SetUpvalue { upvalue_index }
                | Op::SetUpvalueLong { upvalue_index } => {
                    let value = self.memory.top()?;
                    self.get_upvalue(upvalue_index)?.write(value);
                }
                Op::Class { class_index } | Op::ClassLong { class_index } => {
                    let class = self.memory.create_class(class_index);
                    self.memory.push(Value::class(class))?;

                    let class_def = &self.executable.classes[class_index];
                    for (name, function_index) in &class_def.methods {
                        class.methods.borrow_mut().insert(
                            name.clone(),
                            self.close_function(*function_index)?,
                        );
                    }
                }
                Op::GetField { constant_index }
                | Op::GetFieldLong { constant_index } => {
                    let field = self.get_name(constant_index)?;

                    let instance = self.memory.pop()?.unpack();
                    let UnpackedValue::Instance(instance) = instance else {
                        return Err(RuntimeError::ExpectedInstance {
                            actual: instance,
                        });
                    };

                    let fields = instance.fields.borrow();
                    let value = if let Some(value) = fields.get(field) {
                        *value
                    } else if let Some(method) =
                        instance.class.methods.borrow().get(field)
                    {
                        Value::bound_method(
                            self.memory.create_bound_method(instance, *method),
                        )
                    } else {
                        return Err(RuntimeError::UndefinedField);
                    };

                    self.memory.push(value)?;
                }
                Op::SetField { constant_index }
                | Op::SetFieldLong { constant_index } => {
                    let field = self.get_name(constant_index)?;

                    let instance = self.memory.pop()?.unpack();
                    let UnpackedValue::Instance(instance) = instance else {
                        return Err(RuntimeError::ExpectedInstance {
                            actual: instance,
                        });
                    };

                    instance
                        .fields
                        .borrow_mut()
                        .insert(field.clone(), self.memory.top()?);
                }
            }
            self.ip = next_ip;
        }
        Ok(())
    }

    fn display_value(&self, value: &Value) {
        match value.unpack() {
            UnpackedValue::Float(f) => print!("{f}"),
            UnpackedValue::Register(r) => print!("<register={r}>"),
            UnpackedValue::Nil => print!("<nil>"),
            UnpackedValue::False => print!("false"),
            UnpackedValue::True => print!("true"),
            UnpackedValue::String(s) => print!("\"{s}\""),
            UnpackedValue::Closure(c) => {
                let function = &self.executable.functions[c.function_index];
                print!("<fun {}>", function.name)
            }
            UnpackedValue::Class(c) => {
                print!(
                    "<class {}>",
                    self.executable.classes[c.class_index].name
                )
            }
            UnpackedValue::Instance(i) => {
                print!(
                    "<instance {}>",
                    self.executable.classes[i.class.class_index].name
                )
            }
            UnpackedValue::BoundMethod(m) => {
                print!(
                    "<bound_method {}.{}>",
                    self.executable.classes[m.receiver.class.class_index].name,
                    self.executable.functions[m.method.function_index].name,
                )
            }
            UnpackedValue::NativeFunction(f) => {
                let name = NativeFunction::try_from(f)
                    .map(|nf| nf.name())
                    .unwrap_or("invalid");
                print!("<nat {name}>");
            }
        }
    }

    fn debug_value(&self, value: &Value) {
        match value.unpack() {
            UnpackedValue::Float(f) => print!("{f}"),
            UnpackedValue::Register(r) => print!("<register={r}>"),
            UnpackedValue::Nil => print!("<nil>"),
            UnpackedValue::False => print!("false"),
            UnpackedValue::True => print!("true"),
            UnpackedValue::String(s) => print!("\"{s}\""),
            UnpackedValue::Closure(c) => {
                let function = &self.executable.functions[c.function_index];
                print!("<fun {}>", function.name)
            }
            UnpackedValue::Class(c) => {
                print!(
                    "<class {}>",
                    self.executable.classes[c.class_index].name
                )
            }
            UnpackedValue::Instance(i) => {
                print!(
                    "<instance {}>",
                    self.executable.classes[i.class.class_index].name
                )
            }
            UnpackedValue::BoundMethod(m) => {
                print!(
                    "<bound_method {}.{}>",
                    self.executable.classes[m.receiver.class.class_index].name,
                    self.executable.functions[m.method.function_index].name,
                )
            }
            UnpackedValue::NativeFunction(f) => {
                let name = NativeFunction::try_from(f)
                    .map(|nf| nf.name())
                    .unwrap_or("invalid");
                print!("<nat {name}>");
            }
        }
    }

    #[allow(unused)]
    fn trace(&self) -> Result<(), RuntimeError> {
        enum StackKind {
            Frame(usize),
            Return(usize),
            Callee(Value),
            Value(Value),
        }

        fn stack_kind(
            vm: &VirtualMachine,
            i: usize,
        ) -> Result<StackKind, RuntimeError> {
            let mut fp = vm.fp;
            while i + 3 < fp {
                fp = vm.memory.read_stack(fp - 3)?.as_register() as usize;
            }

            let value = vm.memory.read_stack(i)?;
            Ok(match i + 3 - fp {
                0 => StackKind::Frame(value.as_register() as usize),
                1 => StackKind::Return(value.as_register() as usize),
                2 => StackKind::Callee(value),
                _ => StackKind::Value(value),
            })
        }

        let mut frame = 0;
        print!("    <main>: ");
        let mut i = 0;
        while i < self.memory.stack_len() {
            match stack_kind(self, i)? {
                StackKind::Frame(fp) => {
                    print!(" .. (fp={fp:04}, ");
                }
                StackKind::Return(ip) => {
                    println!("ip={ip:04x})");
                    print!("        #{frame}: ");
                    frame += 1;
                }
                StackKind::Callee(v) | StackKind::Value(v) => {
                    if v == Value::register(0) {
                        print!(" => ");
                        i += 1;
                    } else {
                        print!("[");
                        self.debug_value(&v);
                        print!("]");
                    }
                }
            }

            i += 1;
        }
        println!();

        let mut ip = self.ip;
        self.executable.disassemble_instruction(&mut ip);

        Ok(())
    }

    fn unary_float(
        &mut self,
        f: impl FnOnce(f64) -> f64,
    ) -> Result<(), RuntimeError> {
        let target = self.memory.pop()?.as_float();
        self.memory.push(Value::float(f(target)))?;
        Ok(())
    }

    fn binary_float(
        &mut self,
        f: impl FnOnce(f64, f64) -> f64,
    ) -> Result<(), RuntimeError> {
        let rhs = self.memory.pop()?.as_float();
        let lhs = self.memory.pop()?.as_float();
        self.memory.push(Value::float(f(lhs, rhs)))?;
        Ok(())
    }
}
