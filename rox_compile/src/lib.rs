mod error;

use rox_diag::Spanned;
use rox_parse::{
    Visit as _,
    ast::{
        AssignExpr, BinaryExpr, BinaryOperator, ExprStmt, Literal, LiteralExpr,
        PrintStmt, Program, ReturnStmt, UnaryExpr, UnaryOperator, VarDeclStmt,
        VariableExpr, Visitor, visit,
    },
};
use rox_vm::{Chunk, Constant, Op};

use self::error::CompileError;

pub struct CompileOutput {
    pub chunk: Chunk,
    pub errors: Vec<CompileError>,
}

pub struct CompilePass<'a> {
    ast: &'a Program,
    chunk: Chunk,
    // TODO: intern strings
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

        match &node.literal {
            Literal::Float(n) => {
                let constant = Constant::Float(n.value);
                let index = self.chunk.add_constant(constant);
                self.chunk.encode(Op::constant(index), node.span());
            }
            Literal::String(s) => {
                let constant = Constant::String(s.value.clone());
                let index = self.chunk.add_constant(constant);
                self.chunk.encode(Op::constant(index), node.span());
            }
            Literal::Nil(_) => self.chunk.encode(Op::Nil, node.span()),
            Literal::True(_) => self.chunk.encode(Op::True, node.span()),
            Literal::False(_) => self.chunk.encode(Op::False, node.span()),
        }
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

    fn visit_variable_expr(&mut self, node: &VariableExpr) {
        let index = self
            .chunk
            .add_constant(Constant::String(node.ident.value.clone()));
        self.chunk.encode(Op::get_global(index), node.ident.span());
    }

    fn visit_assign_expr(&mut self, node: &AssignExpr) {
        visit::visit_assign_expr(self, node);

        let index = self
            .chunk
            .add_constant(Constant::String(node.ident.value.clone()));
        self.chunk.encode(Op::set_global(index), node.equal.span());
    }

    fn visit_print_stmt(&mut self, node: &PrintStmt) {
        visit::visit_print_stmt(self, node);

        self.chunk.encode(Op::Print, node.print.span());
    }

    fn visit_expr_stmt(&mut self, node: &ExprStmt) {
        visit::visit_expr_stmt(self, node);

        self.chunk.encode(Op::Pop, node.semi.span());
    }

    fn visit_var_decl_stmt(&mut self, node: &VarDeclStmt) {
        if let Some(assignment) = &node.assignment {
            assignment.expr.accept(self);
        } else {
            self.chunk.encode(Op::Nil, node.var.span());
        }

        let index = self
            .chunk
            .add_constant(Constant::String(node.ident.value.clone()));
        self.chunk
            .encode(Op::define_global(index), node.ident.span());
    }
}
