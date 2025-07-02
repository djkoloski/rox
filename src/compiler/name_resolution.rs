use std::collections::{HashMap, HashSet};

use crate::{
    ast::{
        decoration::Decoration,
        expr::{
            AssignExpr, BinaryExpr, ExprVisitor, GroupingExpr, LiteralExpr,
            UnaryExpr, VariableExpr, VisitExpr as _,
        },
        stmt::{
            BlockStmt, ExprStmt, FunDeclStmt, IfStmt, PrintStmt, ReturnStmt,
            StmtVisitor, VarDeclStmt, VisitStmt as _, WhileStmt,
        },
    },
    compiler::CompileError,
    span::Spanned as _,
};

pub struct NameResolution {
    resolutions: HashMap<Decoration, usize>,
}

impl NameResolution {
    pub fn new() -> Self {
        Self {
            resolutions: HashMap::new(),
        }
    }

    pub fn get(&self, decoration: Decoration) -> Option<usize> {
        self.resolutions.get(&decoration).cloned()
    }

    pub fn pass<'a>(
        &'a mut self,
        globals: &'a mut HashSet<String>,
    ) -> NameResolutionPass<'a> {
        NameResolutionPass {
            resolution: self,
            globals,
            locals: Vec::new(),
            errors: Vec::new(),
        }
    }
}

impl Default for NameResolution {
    fn default() -> Self {
        Self::new()
    }
}

pub struct NameResolutionPass<'a> {
    resolution: &'a mut NameResolution,
    globals: &'a mut HashSet<String>,
    locals: Vec<HashSet<String>>,
    errors: Vec<CompileError>,
}

impl NameResolutionPass<'_> {
    pub fn finish(self) -> Result<(), Vec<CompileError>> {
        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors)
        }
    }

    fn define(&mut self, name: String) {
        if let Some(local) = self.locals.last_mut() {
            local.insert(name);
        } else {
            self.globals.insert(name);
        }
    }

    fn resolve(&self, name: &str) -> Option<usize> {
        for (i, scope) in self.locals.iter().rev().enumerate() {
            if scope.contains(name) {
                return Some(i);
            }
        }
        if self.globals.contains(name) {
            return Some(self.locals.len());
        }
        None
    }
}

impl StmtVisitor for NameResolutionPass<'_> {
    type Output = ();

    fn visit_block_stmt(&mut self, stmt: &BlockStmt) -> Self::Output {
        self.locals.push(HashSet::new());
        for stmt in stmt.stmts.iter() {
            stmt.accept(self);
        }
        self.locals.pop();
    }

    fn visit_var_decl_stmt(&mut self, stmt: &VarDeclStmt) -> Self::Output {
        if let Some((_, expr)) = &stmt.assignment {
            expr.accept(self);
        }
        self.define(stmt.ident.value.clone());
    }

    fn visit_fun_decl_stmt(&mut self, stmt: &FunDeclStmt) -> Self::Output {
        self.define(stmt.name.value.clone());
        self.locals
            .push(stmt.params.iter().map(|p| p.value.clone()).collect());
        for stmt in stmt.body.stmts.iter() {
            stmt.accept(self);
        }
        self.locals.pop();
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

    fn visit_while_stmt(&mut self, stmt: &WhileStmt) -> Self::Output {
        stmt.expr.accept(self);
        stmt.body.accept(self);
    }

    fn visit_return_stmt(&mut self, stmt: &ReturnStmt) -> Self::Output {
        if let Some(expr) = &stmt.expr {
            if self.locals.is_empty() {
                self.errors.push(CompileError::TopLevelReturn {
                    span: stmt.return_.span(),
                });
            }
            expr.accept(self);
        }
    }
}

impl ExprVisitor for NameResolutionPass<'_> {
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
        let Some(depth) = self.resolve(&expr.ident.value) else {
            self.errors.push(CompileError::UndefinedVariable {
                span: expr.ident.span(),
            });
            return;
        };

        self.resolution.resolutions.insert(expr.decoration, depth);
    }

    fn visit_assign_expr(&mut self, expr: &AssignExpr) -> Self::Output {
        let Some(depth) = self.resolve(&expr.ident.value) else {
            self.errors.push(CompileError::UndefinedVariable {
                span: expr.ident.span(),
            });
            return;
        };

        self.resolution.resolutions.insert(expr.decoration, depth);

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
