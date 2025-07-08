use std::collections::HashMap;

use rox_diag::Spanned;
use rox_parse::{
    Ast, Visit as _,
    ast::{
        AssignExpr, BinaryExpr, BinaryOperator, BlockStmt, ExprStmt, IfStmt,
        Literal, LiteralExpr, PrintStmt, ReturnStmt, UnaryExpr, UnaryOperator,
        VarDeclStmt, VariableExpr, Visitor, WhileStmt, visit,
    },
};
use rox_vm::{Chunk, Constant, Op};

use crate::Resolution;

pub struct AssemblyPass<'ast> {
    resolutions: &'ast Vec<Resolution>,
    locals_counts: &'ast Vec<usize>,

    strings: HashMap<String, usize>,
    is_at_global_scope: bool,

    chunk: Chunk,
}

impl<'ast> AssemblyPass<'ast> {
    pub fn new(
        resolutions: &'ast Vec<Resolution>,
        locals_counts: &'ast Vec<usize>,
    ) -> Self {
        Self {
            resolutions,
            locals_counts,

            strings: HashMap::new(),
            is_at_global_scope: true,

            chunk: Chunk::new(),
        }
    }

    pub fn compile(mut self, ast: &'ast Ast) -> Chunk {
        ast.program.accept(&mut self);

        self.chunk
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

impl<'ast> Visitor<'ast> for AssemblyPass<'ast> {
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
        match &node.operator {
            BinaryOperator::And(and) => {
                node.left.accept(self);
                let to_end = self
                    .chunk
                    .encode_jump(Op::JumpIfFalse { distance: 0 }, and.span());
                self.chunk.encode(Op::Pop, and.span());
                node.right.accept(self);
                self.chunk.patch_jump(to_end);
            }
            BinaryOperator::Or(or) => {
                node.left.accept(self);
                let to_rhs = self
                    .chunk
                    .encode_jump(Op::JumpIfFalse { distance: 0 }, or.span());
                let to_end =
                    self.chunk.encode_jump(Op::Jump { distance: 0 }, or.span());
                self.chunk.patch_jump(to_rhs);
                node.right.accept(self);
                self.chunk.patch_jump(to_end);
            }
            BinaryOperator::Greater(greater) => {
                visit::visit_binary_expr(self, node);
                self.chunk.encode(Op::Greater, greater.span())
            }
            BinaryOperator::GreaterEqual(greater_equal) => {
                visit::visit_binary_expr(self, node);
                self.chunk.encode(Op::Less, greater_equal.span());
                self.chunk.encode(Op::Not, greater_equal.span());
            }
            BinaryOperator::Less(less) => {
                visit::visit_binary_expr(self, node);
                self.chunk.encode(Op::Less, less.span())
            }
            BinaryOperator::LessEqual(less_equal) => {
                visit::visit_binary_expr(self, node);
                self.chunk.encode(Op::Greater, less_equal.span());
                self.chunk.encode(Op::Not, less_equal.span());
            }
            BinaryOperator::NotEqual(bang_equal) => {
                visit::visit_binary_expr(self, node);
                self.chunk.encode(Op::Equal, bang_equal.span());
                self.chunk.encode(Op::Not, bang_equal.span());
            }
            BinaryOperator::Equal(equal_equal) => {
                visit::visit_binary_expr(self, node);
                self.chunk.encode(Op::Equal, equal_equal.span())
            }
            BinaryOperator::Subtract(minus) => {
                visit::visit_binary_expr(self, node);
                self.chunk.encode(Op::Subtract, minus.span())
            }
            BinaryOperator::Add(plus) => {
                visit::visit_binary_expr(self, node);
                self.chunk.encode(Op::Add, plus.span())
            }
            BinaryOperator::Divide(slash) => {
                visit::visit_binary_expr(self, node);
                self.chunk.encode(Op::Divide, slash.span())
            }
            BinaryOperator::Multiply(star) => {
                visit::visit_binary_expr(self, node);
                self.chunk.encode(Op::Multiply, star.span())
            }
        }
    }

    fn visit_variable_expr(&mut self, node: &'ast VariableExpr) {
        match self.resolutions[node.name_resolution.index()] {
            Resolution::Local(index) => {
                self.chunk.encode(Op::get_local(index), node.ident.span());
            }
            Resolution::Global => {
                let index = self.add_string(node.ident.value.clone());
                self.chunk.encode(Op::get_global(index), node.ident.span());
            }
        }
    }

    fn visit_assign_expr(&mut self, node: &'ast AssignExpr) {
        visit::visit_assign_expr(self, node);

        match self.resolutions[node.name_resolution.index()] {
            Resolution::Local(index) => {
                self.chunk.encode(Op::set_local(index), node.ident.span());
            }
            Resolution::Global => {
                let index = self.add_string(node.ident.value.clone());
                self.chunk.encode(Op::set_global(index), node.equal.span());
            }
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

        if self.is_at_global_scope {
            let index = self.add_string(node.ident.value.clone());
            self.chunk
                .encode(Op::define_global(index), node.ident.span());
        }
    }

    fn visit_block_stmt(&mut self, node: &'ast BlockStmt) {
        let was_at_global_scope = self.is_at_global_scope;
        self.is_at_global_scope = false;

        visit::visit_block_stmt(self, node);

        for _ in 0..self.locals_counts[node.locals_count.index()] {
            self.chunk.encode(Op::Pop, node.rbrace.span());
        }

        self.is_at_global_scope = was_at_global_scope;
    }

    fn visit_if_stmt(&mut self, node: &'ast IfStmt) {
        node.condition.accept(self);

        let to_false = self
            .chunk
            .encode_jump(Op::JumpIfFalse { distance: 0 }, node.if_.span());

        self.chunk.encode(Op::Pop, node.if_.span());
        node.stmt.accept(self);

        if let Some(else_clause) = &node.else_clause {
            let to_end = self.chunk.encode_jump(
                Op::Jump { distance: 0 },
                else_clause.else_.span(),
            );

            self.chunk.patch_jump(to_false);

            self.chunk.encode(Op::Pop, else_clause.else_.span());
            else_clause.stmt.accept(self);

            self.chunk.patch_jump(to_end);
        } else {
            self.chunk.patch_jump(to_false);
        }
    }

    fn visit_while_stmt(&mut self, node: &'ast WhileStmt) {
        let start = self.chunk.current();

        node.expr.accept(self);

        let to_end = self
            .chunk
            .encode_jump(Op::JumpIfFalse { distance: 0 }, node.while_.span());
        self.chunk.encode(Op::Pop, node.while_.span());

        node.body.accept(self);

        self.chunk.encode_loop(start, node.while_.span());

        self.chunk.patch_jump(to_end);
        self.chunk.encode(Op::Pop, node.while_.span());
    }
}
