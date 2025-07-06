mod error;

use rox_diag::Spanned;
use rox_parse::{
    Visit as _,
    ast::{Literal, LiteralExpr, Program, ReturnStmt, Visitor},
};
use rox_vm::{Chunk, Op, Value};

use self::error::CompileError;

pub struct CompileOutput {
    pub chunk: Chunk,
    pub errors: Vec<CompileError>,
}

pub struct CompilePass<'a> {
    ast: &'a Program,
    chunk: Chunk,
    errors: Vec<CompileError>,
}

impl<'a> CompilePass<'a> {
    pub fn new(ast: &'a Program) -> Self {
        Self {
            ast,
            chunk: Chunk::new(),
            errors: Vec::new(),
        }
    }

    pub fn compile(mut self) -> CompileOutput {
        self.ast.accept(&mut self);
        CompileOutput {
            chunk: self.chunk,
            errors: self.errors,
        }
    }
}

impl Visitor for CompilePass<'_> {
    fn visit_literal_expr(&mut self, node: &LiteralExpr) {
        match &node.literal {
            Literal::Number(n) => {
                let constant = self.chunk.add_constant(Value::Float(n.value));
                self.chunk.encode_constant(constant, node.span());
            }
            _ => todo!(),
        }
    }

    fn visit_return_stmt(&mut self, node: &ReturnStmt) {
        node.expr.accept(self);
        self.chunk.encode(Op::Return, node.span());
    }
}
