use crate::Chunk;

pub struct Function {
    pub name: String,
    pub arity: usize,
    pub ip: usize,
}

pub struct Executable {
    pub chunk: Chunk,
    pub functions: Vec<Function>,
}

impl Executable {
    pub fn disassemble(&self) {
        self.chunk.disassemble();

        for function in &self.functions {
            println!("{}({}): {}", function.name, function.arity, function.ip);
        }
    }
}
