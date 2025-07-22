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

    pub fn encode_jump(&mut self, op: Op, span: Span) -> usize {
        self.encode(op, span);
        self.current()
    }

    pub fn patch_jump(&mut self, target: usize) {
        let distance = self.current() - target;
        if distance > u16::MAX as usize {
            panic!("jump distance too large");
        }

        let bytes = distance.to_le_bytes();
        self.bytes[target - 2] = bytes[0];
        self.bytes[target - 1] = bytes[1];
    }

    pub fn current(&self) -> usize {
        self.bytes.len()
    }

    pub fn encode_loop(&mut self, target: usize, span: Span) {
        self.encode(
            Op::Loop {
                distance: self.current() - target + 3,
            },
            span,
        );
    }
}

impl Default for Chunk {
    fn default() -> Self {
        Self::new()
    }
}
