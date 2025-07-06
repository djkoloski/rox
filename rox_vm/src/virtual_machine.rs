use crate::{Chunk, Codec, Op, RuntimeDiagnostic, RuntimeError, Value};

const MAX_STACK_LEN: usize = 255;

pub struct VirtualMachine<'chunk> {
    chunk: &'chunk Chunk,
    last_ip: usize,
    ip: usize,
    stack: Vec<Value>,
}

impl<'chunk> VirtualMachine<'chunk> {
    pub fn new(chunk: &'chunk Chunk) -> Self {
        Self {
            chunk,
            last_ip: 0,
            ip: 0,
            stack: Vec::new(),
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
        loop {
            #[cfg(feature = "trace")]
            self.trace();

            match self.read_op()? {
                Op::Return => {
                    println!("{}", self.pop()?);
                    break;
                }
                Op::Constant { constant } => {
                    self.push(*self.get_constant(constant as usize)?)?;
                }
                Op::ConstantLong { constant } => {
                    self.push(*self.get_constant(constant as usize)?)?;
                }
                Op::Not => {
                    let target = self.pop()?;
                    self.push(Value::Boolean(!target.truthiness()))?;
                }
                Op::Negate => self.unary_float(|n| -n)?,
                Op::Add => self.binary_float(|a, b| a + b)?,
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
            }
        }
        Ok(())
    }

    #[allow(unused)]
    fn trace(&self) {
        print!("          ");
        for value in &self.stack {
            print!("[{value}]");
        }
        println!();

        let mut ip = self.ip;
        self.chunk.disassemble_instruction(&mut ip);
    }

    fn read_op(&mut self) -> Result<Op, RuntimeError> {
        self.last_ip = self.ip;
        Ok(Op::decode(self.chunk.bytes(), &mut self.ip)?)
    }

    fn get_constant(
        &self,
        constant: usize,
    ) -> Result<&'chunk Value, RuntimeError> {
        self.chunk
            .constants()
            .get(constant)
            .ok_or(RuntimeError::ConstantOutOfBounds)
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
