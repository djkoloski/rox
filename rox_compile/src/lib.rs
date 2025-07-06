mod error;

use rox_diag::Spanned;
use rox_parse::{
    Visit as _,
    ast::{
        BinaryExpr, BinaryOperator, Literal, LiteralExpr, Program, ReturnStmt,
        UnaryExpr, UnaryOperator, Visitor, visit,
    },
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
        visit::visit_literal_expr(self, node);

        let value = match &node.literal {
            Literal::Float(n) => Value::Float(n.value),
            Literal::False(_) => Value::Boolean(false),
            Literal::True(_) => Value::Boolean(true),
            Literal::Nil(_) => Value::Nil,
            Literal::String(_) => todo!(),
        };
        let constant = self.chunk.add_constant(value);
        self.chunk.encode_constant(constant, node.span());
    }

    fn visit_return_stmt(&mut self, node: &ReturnStmt) {
        visit::visit_return_stmt(self, node);

        self.chunk.encode(Op::Return, node.return_.span());
    }

    fn visit_unary_expr(&mut self, node: &UnaryExpr) {
        visit::visit_unary_expr(self, node);

        match &node.operator {
            UnaryOperator::Not(bang) => self.chunk.encode(Op::Not, bang.span()),
            UnaryOperator::Negate(minus) => {
                self.chunk.encode(Op::Negate, minus.span())
            }
        }
    }

    fn visit_binary_expr(&mut self, node: &BinaryExpr) {
        visit::visit_binary_expr(self, node);

        match &node.operator {
            BinaryOperator::And(_and) => todo!(),
            BinaryOperator::Or(_or) => todo!(),
            BinaryOperator::Greater(greater) => {
                self.chunk.encode(Op::Greater, greater.span())
            }
            BinaryOperator::GreaterEqual(greater_equal) => {
                self.chunk.encode(Op::Less, greater_equal.span());
                self.chunk.encode(Op::Not, greater_equal.span());
            }
            BinaryOperator::Less(less) => {
                self.chunk.encode(Op::Less, less.span())
            }
            BinaryOperator::LessEqual(less_equal) => {
                self.chunk.encode(Op::Greater, less_equal.span());
                self.chunk.encode(Op::Not, less_equal.span());
            }
            BinaryOperator::NotEqual(bang_equal) => {
                self.chunk.encode(Op::Equal, bang_equal.span());
                self.chunk.encode(Op::Not, bang_equal.span());
            }
            BinaryOperator::Equal(equal_equal) => {
                self.chunk.encode(Op::Equal, equal_equal.span())
            }
            BinaryOperator::Subtract(minus) => {
                self.chunk.encode(Op::Subtract, minus.span())
            }
            BinaryOperator::Add(plus) => {
                self.chunk.encode(Op::Add, plus.span())
            }
            BinaryOperator::Divide(slash) => {
                self.chunk.encode(Op::Divide, slash.span())
            }
            BinaryOperator::Multiply(star) => {
                self.chunk.encode(Op::Multiply, star.span())
            }
        }
    }
}
