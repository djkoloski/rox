use rox_diag::Span;

use crate::{Codec as _, Constant, Op, rle::Rle};

pub struct Chunk {
    bytes: Vec<u8>,
    spans: Rle<Span>,
    constants: Vec<Constant>,
}

impl Chunk {
    pub fn new() -> Self {
        Self {
            bytes: Vec::new(),
            spans: Rle::new(),
            constants: Vec::new(),
        }
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn constants(&self) -> &[Constant] {
        &self.constants
    }

    pub fn span(&self, offset: usize) -> Span {
        self.spans[offset]
    }

    pub fn encode(&mut self, op: Op, span: Span) {
        op.encode(&mut self.bytes);
        self.spans.extend(span, self.bytes.len() - self.spans.len());
    }

    pub fn encode_constant(&mut self, index: usize, span: Span) {
        if index <= u8::MAX as usize {
            self.encode(Op::Constant { index: index as u8 }, span);
        } else {
            self.encode(
                Op::ConstantLong {
                    index: index as u32,
                },
                span,
            );
        }
    }

    pub fn add_constant(&mut self, constant: Constant) -> usize {
        let result = self.constants.len();
        self.constants.push(constant);
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

        let span = self.spans[*offset];
        if *offset > 0 && span == self.spans[*offset - 1] {
            print!("   | ");
        } else {
            print!("{}..{} ", span.start(), span.end());
        }

        let op = Op::decode(self.bytes(), offset).unwrap();
        print!("{op:16}");

        match &op {
            Op::Constant { index } => self.debug_constant(*index as usize),
            Op::ConstantLong { index } => self.debug_constant(*index as usize),
            _ => (),
        }

        println!();
    }

    fn debug_constant(&self, index: usize) {
        let value = &self.constants[index];
        print!(" '{value}'");
    }
}

impl Default for Chunk {
    fn default() -> Self {
        Self::new()
    }
}
