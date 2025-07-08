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

        match op {
            Op::Return
            | Op::Nil
            | Op::True
            | Op::False
            | Op::Not
            | Op::Negate
            | Op::Add
            | Op::Subtract
            | Op::Multiply
            | Op::Divide
            | Op::Equal
            | Op::Greater
            | Op::Less
            | Op::Print
            | Op::Pop => (),
            Op::Constant { index }
            | Op::ConstantLong { index }
            | Op::DefineGlobal { index }
            | Op::DefineGlobalLong { index }
            | Op::GetGlobal { index }
            | Op::GetGlobalLong { index }
            | Op::SetGlobal { index }
            | Op::SetGlobalLong { index }
            | Op::GetLocal { index }
            | Op::GetLocalLong { index }
            | Op::SetLocal { index }
            | Op::SetLocalLong { index } => self.debug_constant(index),
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
