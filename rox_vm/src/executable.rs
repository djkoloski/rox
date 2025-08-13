use std::collections::HashMap;

use crate::{Chunk, Codec as _, Op};

pub enum Place {
    Local { local_index: usize },
    Upvalue { upvalue_index: usize },
}

pub struct FunctionDef {
    pub name: String,
    pub arity: usize,
    pub ip: usize,
    pub captures: Vec<Place>,
}

pub struct ClassDef {
    pub name: String,
    pub methods: HashMap<String, usize>,
    pub has_superclass: bool,
}

pub struct Executable {
    pub chunk: Chunk,
    pub functions: Vec<FunctionDef>,
    pub classes: Vec<ClassDef>,
}

impl Executable {
    pub fn disassemble(&self) {
        let mut ip_to_function = HashMap::new();
        for function in &self.functions {
            ip_to_function.insert(function.ip, function);
        }
        let mut ip = 0;
        while ip < self.chunk.bytes().len() {
            if let Some(function) = ip_to_function.get(&ip) {
                println!("{}({}):", function.name, function.arity);
            }
            self.disassemble_instruction(&mut ip);
        }
    }

    pub fn disassemble_instruction(&self, ip: &mut usize) {
        print!("{ip:0>4x} ");

        let span = self.chunk.span(*ip);
        if *ip > 0 && span == self.chunk.span(*ip - 1) {
            print!("   | ");
        } else {
            print!("{}..{} ", span.start(), span.end());
        }

        let op = Op::decode(self.chunk.bytes(), ip).unwrap();
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
            | Op::Pop
            | Op::GetLocal { .. }
            | Op::GetLocalLong { .. }
            | Op::SetLocal { .. }
            | Op::SetLocalLong { .. }
            | Op::JumpIfFalse { .. }
            | Op::Jump { .. }
            | Op::Loop { .. }
            | Op::PushFrame
            | Op::Call { .. }
            | Op::CloseLocal => (),
            Op::Constant { constant_index }
            | Op::ConstantLong { constant_index }
            | Op::DefineGlobal { constant_index }
            | Op::DefineGlobalLong { constant_index }
            | Op::GetGlobal { constant_index }
            | Op::GetGlobalLong { constant_index }
            | Op::SetGlobal { constant_index }
            | Op::SetGlobalLong { constant_index }
            | Op::GetField { constant_index }
            | Op::GetFieldLong { constant_index }
            | Op::SetField { constant_index }
            | Op::SetFieldLong { constant_index }
            | Op::GetSuper { constant_index }
            | Op::GetSuperLong { constant_index } => {
                let value = &self.chunk.constants()[constant_index];
                print!(" {value}");
            }
            Op::CloseFunction { function_index }
            | Op::CloseFunctionLong { function_index } => {
                let value = &self.functions[function_index];
                print!(" {}({})", value.name, value.arity);
            }
            Op::Class { class_index } | Op::ClassLong { class_index } => {
                let value = &self.classes[class_index];
                print!(" {}", value.name);
            }
            Op::GetUpvalue { upvalue_index }
            | Op::GetUpvalueLong { upvalue_index }
            | Op::SetUpvalue { upvalue_index }
            | Op::SetUpvalueLong { upvalue_index } => {
                print!(" {upvalue_index}");
            }
            Op::Invoke {
                constant_index,
                arity,
            }
            | Op::InvokeLong {
                constant_index,
                arity,
            }
            | Op::InvokeSuper {
                constant_index,
                arity,
            }
            | Op::InvokeSuperLong {
                constant_index,
                arity,
            } => {
                let value = &self.chunk.constants()[constant_index];
                print!(" {value}({arity})");
            }
        }

        println!();
    }
}
