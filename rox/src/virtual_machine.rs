use crate::{Chunk, Codec, DecodeError, Op};

#[derive(Debug)]
pub enum RuntimeError {
    BytecodeOutOfBounds,
    ConstantOutOfBounds,
    Decode(DecodeError),
}

impl From<DecodeError> for RuntimeError {
    fn from(value: DecodeError) -> Self {
        Self::Decode(value)
    }
}

pub struct VirtualMachine<'chunk> {
    chunk: &'chunk Chunk,
    ip: usize,
}

impl<'chunk> VirtualMachine<'chunk> {
    pub fn new(chunk: &'chunk Chunk) -> Self {
        Self { chunk, ip: 0 }
    }

    pub fn interpret(&mut self) -> Result<(), RuntimeError> {
        loop {
            #[cfg(feature = "trace")]
            self.chunk.disassemble_instruction(self.ip);

            match self.read_op()? {
                Op::Return => break,
                Op::Constant { constant } => {
                    println!("{constant}");
                }
                Op::ConstantLong { constant } => {
                    println!("{constant}");
                }
            }
        }
        Ok(())
    }

    fn read_op(&mut self) -> Result<Op, RuntimeError> {
        Ok(Op::decode(self.chunk.bytes(), &mut self.ip)?)
    }
}
