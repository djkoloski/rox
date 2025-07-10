use core::fmt;

use crate::Chunk;

pub struct Function {
    pub name: String,
    pub arity: usize,
    pub chunk: Chunk,
}

impl fmt::Display for Function {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<fun {}>", self.name)
    }
}

pub struct Executable {
    pub main: Chunk,
    pub functions: Vec<Function>,
}

impl Executable {
    pub fn disassemble(&self) {
        println!("=== <main> ===");
        self.main.disassemble();

        for function in &self.functions {
            println!("=== {}({}) ===", function.name, function.arity);
            function.chunk.disassemble();
        }
    }
}
