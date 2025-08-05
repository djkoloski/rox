use std::collections::HashMap;

use rox_diag::{Span, Spanned};
use rox_lex::token_kind::Identifier;
use rox_parse::{
    Decoration, NameDecoration, Visit,
    ast::{
        AssignExpr, BinaryExpr, BinaryOperator, BlockStmt, CallExpr,
        ClassDeclStmt, ExprStmt, FunDeclStmt, Function, GetExpr, IfStmt,
        Literal, LiteralExpr, PrintStmt, Program, ReturnStmt, SetExpr,
        ThisExpr, UnaryExpr, UnaryOperator, VarDeclStmt, VariableExpr, Visitor,
        WhileStmt, visit,
    },
};
use rox_vm::{Chunk, Constant, Op};

use crate::{LocalDeclaration, Resolution};

pub struct AssemblyPass<'ast> {
    resolutions: &'ast Vec<Resolution>,
    block_locals: &'ast Vec<Vec<LocalDeclaration>>,

    strings: HashMap<String, usize>,
    is_at_global_scope: bool,

    chunk: &'ast mut Chunk,
}

impl<'ast> AssemblyPass<'ast> {
    pub fn new(
        resolutions: &'ast Vec<Resolution>,
        block_locals: &'ast Vec<Vec<LocalDeclaration>>,
        chunk: &'ast mut Chunk,
    ) -> Self {
        Self {
            resolutions,
            block_locals,

            strings: HashMap::new(),
            is_at_global_scope: true,

            chunk,
        }
    }

    pub fn compile_program(mut self, program: &'ast Program) {
        self.is_at_global_scope = true;

        program.accept(&mut self);

        self.chunk.encode(Op::Nil, program.eof.span());
        self.chunk.encode(Op::Return, program.eof.span());
    }

    pub fn compile_function(mut self, function: &'ast Function) {
        self.is_at_global_scope = false;

        visit::visit_block_stmt(&mut self, &function.body);

        self.chunk.encode(Op::Nil, function.body.rbrace.span());
        self.chunk.encode(Op::Return, function.body.rbrace.span());
    }

    pub fn compile_initializer(mut self, function: &'ast Function) {
        self.is_at_global_scope = false;

        visit::visit_block_stmt(&mut self, &function.body);

        self.chunk
            .encode(Op::get_local(0), function.body.rbrace.span());
        self.chunk.encode(Op::Return, function.body.rbrace.span());
    }

    fn add_float(&mut self, float: f64) -> usize {
        self.chunk.add_constant(Constant::Float(float))
    }

    fn add_string(&mut self, string: String) -> usize {
        if let Some(constant_index) = self.strings.get(&string) {
            *constant_index
        } else {
            let constant_index =
                self.chunk.add_constant(Constant::String(string.clone()));
            self.strings.insert(string, constant_index);
            constant_index
        }
    }

    fn define_if_global(&mut self, identifier: &Identifier) {
        if self.is_at_global_scope {
            let constant_index = self.add_string(identifier.value.clone());
            self.chunk
                .encode(Op::define_global(constant_index), identifier.span());
        }
    }

    fn resolve(
        &mut self,
        decoration: Decoration<NameDecoration>,
        span: Span,
        identifier: Option<&'ast Identifier>,
    ) {
        match self.resolutions[decoration.index()] {
            Resolution::Local { local_index } => {
                self.chunk.encode(Op::get_local(local_index), span);
            }
            Resolution::Upvalue { upvalue_index } => {
                self.chunk.encode(Op::get_upvalue(upvalue_index), span);
            }
            Resolution::Global => {
                let constant_index =
                    self.add_string(identifier.unwrap().value.clone());
                self.chunk.encode(Op::get_global(constant_index), span);
            }
        }
    }
}

impl<'ast> Visitor<'ast> for AssemblyPass<'ast> {
    fn visit_literal_expr(&mut self, node: &'ast LiteralExpr) {
        visit::visit_literal_expr(self, node);

        match &node.literal {
            Literal::Float(n) => {
                let constant_index = self.add_float(n.value);
                self.chunk.encode(Op::constant(constant_index), node.span());
            }
            Literal::String(s) => {
                let constant_index = self.add_string(s.value.clone());
                self.chunk.encode(Op::constant(constant_index), node.span());
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
        self.resolve(
            node.name.decoration,
            node.span(),
            Some(&node.name.identifier),
        );
    }

    fn visit_assign_expr(&mut self, node: &'ast AssignExpr) {
        visit::visit_assign_expr(self, node);

        match self.resolutions[node.name.decoration.index()] {
            Resolution::Local { local_index } => {
                self.chunk
                    .encode(Op::set_local(local_index), node.name.span());
            }
            Resolution::Upvalue { upvalue_index } => {
                self.chunk
                    .encode(Op::set_upvalue(upvalue_index), node.name.span());
            }
            Resolution::Global => {
                let constant_index =
                    self.add_string(node.name.identifier.value.clone());
                self.chunk
                    .encode(Op::set_global(constant_index), node.equal.span());
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

        self.define_if_global(&node.identifier);
    }

    fn visit_block_stmt(&mut self, node: &'ast BlockStmt) {
        let was_at_global_scope = self.is_at_global_scope;
        self.is_at_global_scope = false;

        visit::visit_block_stmt(self, node);

        for local in self.block_locals[node.decoration.index()].iter().rev() {
            match local {
                LocalDeclaration::Local => {
                    self.chunk.encode(Op::Pop, node.rbrace.span())
                }
                LocalDeclaration::Capture => {
                    self.chunk.encode(Op::CloseLocal, node.rbrace.span())
                }
            }
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

            self.chunk.encode(Op::Pop, node.if_.span());
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

    fn visit_fun_decl_stmt(&mut self, node: &'ast FunDeclStmt) {
        self.chunk.encode(
            Op::close_function(node.function.decoration.index()),
            node.fun.span(),
        );
        self.define_if_global(&node.identifier);
    }

    fn visit_call_expr(&mut self, node: &'ast CallExpr) {
        self.chunk.encode(Op::PushFrame, node.span());

        visit::visit_call_expr(self, node);

        self.chunk.encode(
            Op::Call {
                arity: node.arguments.len(),
            },
            node.span(),
        );
    }

    fn visit_class_decl_stmt(&mut self, node: &'ast ClassDeclStmt) {
        self.chunk
            .encode(Op::class(node.decoration.index()), node.span());
        self.define_if_global(&node.identifier);
    }

    fn visit_get_expr(&mut self, node: &'ast GetExpr) {
        node.target.accept(self);

        let field_name = self.add_string(node.field.value.clone());
        self.chunk
            .encode(Op::get_field(field_name), node.field.span());
    }

    fn visit_set_expr(&mut self, node: &'ast SetExpr) {
        node.expr.accept(self);
        node.target.accept(self);

        let field_name = self.add_string(node.field.value.clone());
        self.chunk
            .encode(Op::set_field(field_name), node.field.span());
    }

    fn visit_this_expr(&mut self, node: &'ast ThisExpr) {
        self.resolve(node.decoration, node.span(), None);
    }
}
