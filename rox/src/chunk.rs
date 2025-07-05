use crate::{Codec as _, Op, Value, rle::Rle};

pub struct Chunk {
    bytes: Vec<u8>,
    lines: Rle<usize>,
    constants: Vec<Value>,
}

impl Chunk {
    pub fn new() -> Self {
        Self {
            bytes: Vec::new(),
            lines: Rle::new(),
            constants: Vec::new(),
        }
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn constants(&self) -> &[Value] {
        &self.constants
    }

    pub fn encode(&mut self, op: Op, line: usize) {
        op.encode(&mut self.bytes);
        self.lines.extend(line, self.bytes.len() - self.lines.len());
    }

    pub fn encode_constant(&mut self, constant: usize, line: usize) {
        if constant <= u8::MAX as usize {
            self.encode(
                Op::Constant {
                    constant: constant as u8,
                },
                line,
            );
        } else {
            self.encode(
                Op::ConstantLong {
                    constant: constant as u32,
                },
                line,
            );
        }
    }

    pub fn add_constant(&mut self, value: Value) -> usize {
        let result = self.constants.len();
        self.constants.push(value);
        result
    }

    pub fn disassemble(&self, name: &str) {
        println!("== {name} ==");
        let mut offset = 0;
        while offset < self.bytes.len() {
            self.disassemble_instruction(&mut offset);
        }
    }

    pub fn disassemble_instruction(&self, offset: &mut usize) {
        print!("{offset:0>4x} ");

        let line = self.lines[*offset];
        if *offset > 0 && line == self.lines[*offset - 1] {
            print!("   | ");
        } else {
            print!("{line:>4} ");
        }

        let op = Op::decode(self.bytes(), offset).unwrap();
        print!("{op:16}");

        match &op {
            Op::Constant { constant } => {
                self.debug_constant(*constant as usize)
            }
            Op::ConstantLong { constant } => {
                self.debug_constant(*constant as usize)
            }
            _ => (),
        }

        println!();
    }

    fn debug_constant(&self, constant: usize) {
        let value = &self.constants[constant];
        print!(" '{value}'");
    }
}

impl Default for Chunk {
    fn default() -> Self {
        Self::new()
    }
}
