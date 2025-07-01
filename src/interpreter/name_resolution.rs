use core::mem::take;
use std::collections::HashMap;

use crate::{
    ast::{
        decoration::Decoration,
        expr::{
            AssignExpr, BinaryExpr, ExprVisitor, GroupingExpr, LiteralExpr,
            UnaryExpr, VariableExpr, VisitExpr as _,
        },
        stmt::{
            BlockStmt, DeclStmt, ExprStmt, IfStmt, PrintStmt, StmtVisitor,
            VisitStmt as _,
        },
    },
    interpreter::{eval::Value, Environment, InterpretError, Scope},
    span::Spanned as _,
};

pub struct NameResolution {
    names: Environment<()>,
    resolutions: HashMap<Decoration, usize>,
    errors: Vec<InterpretError>,
}

impl NameResolution {
    pub fn new(global_scope: &Scope<Value>) -> Self {
        let mut names = Environment::new();

        for name in global_scope.names.keys() {
            names.define(name.clone(), ());
        }

        Self {
            names,
            resolutions: HashMap::new(),
            errors: Vec::new(),
        }
    }

    pub fn take_errors(&mut self) -> Result<(), Vec<InterpretError>> {
        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(take(&mut self.errors))
        }
    }

    pub fn get(&self, decoration: Decoration) -> Option<usize> {
        self.resolutions.get(&decoration).cloned()
    }
}

impl StmtVisitor for NameResolution {
    type Output = ();

    fn visit_block_stmt(&mut self, stmt: &BlockStmt) -> Self::Output {
        self.names.push();
        for stmt in stmt.stmts.iter() {
            stmt.accept(self);
        }
        self.names.pop();
    }

    fn visit_decl_stmt(&mut self, stmt: &DeclStmt) -> Self::Output {
        if let Some((_, expr)) = &stmt.assignment {
            expr.accept(self);
        }
        self.names.define(stmt.ident.value.clone(), ());
    }

    fn visit_expr_stmt(&mut self, stmt: &ExprStmt) -> Self::Output {
        stmt.expr.accept(self);
    }

    fn visit_print_stmt(&mut self, stmt: &PrintStmt) -> Self::Output {
        stmt.expr.accept(self);
    }

    fn visit_if_stmt(&mut self, stmt: &IfStmt) -> Self::Output {
        stmt.group.accept(self);
        stmt.then.accept(self);
        if let Some((_, group)) = &stmt.else_ {
            group.accept(self);
        }
    }
}

impl ExprVisitor for NameResolution {
    type Output = ();

    fn visit_literal_expr(&mut self, _: &LiteralExpr) -> Self::Output {}

    fn visit_unary_expr(&mut self, expr: &UnaryExpr) -> Self::Output {
        expr.inner.accept(self);
    }

    fn visit_binary_expr(&mut self, expr: &BinaryExpr) -> Self::Output {
        expr.left.accept(self);
        expr.right.accept(self);
    }

    fn visit_grouping_expr(&mut self, expr: &GroupingExpr) -> Self::Output {
        expr.inner.accept(self);
    }

    fn visit_variable_expr(&mut self, expr: &VariableExpr) -> Self::Output {
        let Some(depth) = self.names.resolve(&expr.ident.value) else {
            self.errors.push(InterpretError::UndefinedVariable {
                span: expr.ident.span(),
            });
            return;
        };

        self.resolutions.insert(expr.decoration, depth);
    }

    fn visit_assign_expr(&mut self, expr: &AssignExpr) -> Self::Output {
        expr.expr.accept(self);
    }

    fn visit_call_expr(
        &mut self,
        expr: &crate::ast::expr::CallExpr,
    ) -> Self::Output {
        expr.function.accept(self);
        for argument in expr.arguments.iter() {
            argument.accept(self);
        }
    }
}
