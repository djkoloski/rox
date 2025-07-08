mod error;
mod name_resolution;

use std::collections::HashMap;

use rox_diag::Spanned;
use rox_parse::{
    Visit as _,
    ast::{
        AssignExpr, BinaryExpr, BinaryOperator, BlockStmt, ExprStmt, Literal,
        LiteralExpr, PrintStmt, Program, ReturnStmt, UnaryExpr, UnaryOperator,
        VarDeclStmt, VariableExpr, Visitor, visit,
    },
};
use rox_vm::{Chunk, Constant, Op};

use self::{error::CompileError, name_resolution::NameResolution};

pub struct CompileOutput {
    pub chunk: Chunk,
    pub errors: Vec<CompileError>,
}

pub struct CompilePass<'ast> {
    ast: &'ast Program,
    chunk: Chunk,
    strings: HashMap<String, usize>,
    errors: Vec<CompileError>,
    name_resolution: NameResolution<'ast>,
}

impl<'ast> CompilePass<'ast> {
    pub fn new(ast: &'ast Program) -> Self {
        Self {
            ast,
            chunk: Chunk::new(),
            strings: HashMap::new(),
            errors: Vec::new(),
            name_resolution: NameResolution::new(),
        }
    }

    pub fn compile(mut self) -> CompileOutput {
        self.ast.accept(&mut self);
        CompileOutput {
            chunk: self.chunk,
            errors: self.errors,
        }
    }

    fn add_float(&mut self, float: f64) -> usize {
        self.chunk.add_constant(Constant::Float(float))
    }

    fn add_string(&mut self, string: String) -> usize {
        if let Some(index) = self.strings.get(&string) {
            *index
        } else {
            let index =
                self.chunk.add_constant(Constant::String(string.clone()));
            self.strings.insert(string, index);
            index
        }
    }
}

impl<'ast> Visitor<'ast> for CompilePass<'ast> {
    fn visit_literal_expr(&mut self, node: &'ast LiteralExpr) {
        visit::visit_literal_expr(self, node);

        match &node.literal {
            Literal::Float(n) => {
                let index = self.add_float(n.value);
                self.chunk.encode(Op::constant(index), node.span());
            }
            Literal::String(s) => {
                let index = self.add_string(s.value.clone());
                self.chunk.encode(Op::constant(index), node.span());
            }
            Literal::Nil(_) => self.chunk.encode(Op::Nil, node.span()),
            Literal::True(_) => self.chunk.encode(Op::True, node.span()),
            Literal::False(_) => self.chunk.encode(Op::False, node.span()),
        }
    }

    fn visit_return_stmt(&mut self, node: &'ast ReturnStmt) {
        visit::visit_return_stmt(self, node);

        self.chunk.encode(Op::Return, node.return_.span());
    }

    fn visit_unary_expr(&mut self, node: &'ast UnaryExpr) {
        visit::visit_unary_expr(self, node);

        match &node.operator {
            UnaryOperator::Not(bang) => self.chunk.encode(Op::Not, bang.span()),
            UnaryOperator::Negate(minus) => {
                self.chunk.encode(Op::Negate, minus.span())
            }
        }
    }

    fn visit_binary_expr(&mut self, node: &'ast BinaryExpr) {
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

    fn visit_variable_expr(&mut self, node: &'ast VariableExpr) {
        if let Some(index) = self.name_resolution.resolve(&node.ident) {
            self.chunk.encode(Op::get_local(index), node.ident.span());
        } else {
            let index = self.add_string(node.ident.value.clone());
            self.chunk.encode(Op::get_global(index), node.ident.span());
        }
    }

    fn visit_assign_expr(&mut self, node: &'ast AssignExpr) {
        visit::visit_assign_expr(self, node);

        if let Some(index) = self.name_resolution.resolve(&node.ident) {
            self.chunk.encode(Op::set_local(index), node.ident.span());
        } else {
            let index = self.add_string(node.ident.value.clone());
            self.chunk.encode(Op::set_global(index), node.equal.span());
        }
    }

    fn visit_print_stmt(&mut self, node: &'ast PrintStmt) {
        visit::visit_print_stmt(self, node);

        self.chunk.encode(Op::Print, node.print.span());
    }

    fn visit_expr_stmt(&mut self, node: &'ast ExprStmt) {
        visit::visit_expr_stmt(self, node);

        self.chunk.encode(Op::Pop, node.semi.span());
    }

    fn visit_var_decl_stmt(&mut self, node: &'ast VarDeclStmt) {
        if let Some(assignment) = &node.assignment {
            assignment.expr.accept(self);
        } else {
            self.chunk.encode(Op::Nil, node.var.span());
        }

        if self.name_resolution.is_global() {
            let index = self.add_string(node.ident.value.clone());
            self.chunk
                .encode(Op::define_global(index), node.ident.span());
        } else if let Err(error) = self.name_resolution.define(&node.ident) {
            self.errors.push(error);
        }
    }

    fn visit_block_stmt(&mut self, node: &'ast BlockStmt) {
        self.name_resolution.push_scope();

        visit::visit_block_stmt(self, node);

        for _ in 0..self.name_resolution.local_scope_len() {
            self.chunk.encode(Op::Pop, node.rbrace.span());
        }

        self.name_resolution.pop_scope();
    }
}
