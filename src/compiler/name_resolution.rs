use std::collections::{HashMap, HashSet};

use crate::{
    ast::{
        decoration::Decoration,
        expr::{
            AssignExpr, BinaryExpr, CallExpr, ExprVisitor, GetExpr,
            GroupingExpr, LiteralExpr, SetExpr, SuperExpr, ThisExpr, UnaryExpr,
            VariableExpr, VisitExpr as _,
        },
        stmt::{
            BlockStmt, ClassDeclStmt, ExprStmt, FunDeclStmt, IfStmt, PrintStmt,
            ReturnStmt, StmtVisitor, VarDeclStmt, VisitStmt as _, WhileStmt,
        },
    },
    compiler::CompileError,
    span::{Span, Spanned as _},
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
            function_kind: FunctionKind::Free,
            is_subclass: false,
        }
    }
}

impl Default for NameResolution {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy)]
enum FunctionKind {
    Free,
    Method,
    Initializer { method: Span, class: Span },
}

pub struct NameResolutionPass<'a> {
    resolution: &'a mut NameResolution,
    globals: &'a mut HashSet<String>,
    locals: Vec<HashSet<String>>,
    errors: Vec<CompileError>,
    function_kind: FunctionKind,
    is_subclass: bool,
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
        self.define(stmt.function.name.value.clone());

        let prev_function_kind = self.function_kind;
        self.function_kind = FunctionKind::Free;

        self.locals.push(
            stmt.function
                .params
                .iter()
                .map(|p| p.value.clone())
                .collect(),
        );
        for stmt in &stmt.function.body.stmts {
            stmt.accept(self);
        }
        self.locals.pop();

        self.function_kind = prev_function_kind;
    }

    fn visit_class_decl_stmt(&mut self, stmt: &ClassDeclStmt) -> Self::Output {
        let was_subclass = self.is_subclass;

        if let Some(inheritance) = &stmt.inheritance {
            let Some(depth) = self.resolve(&inheritance.superclass.value)
            else {
                self.errors.push(CompileError::UndefinedVariable {
                    span: inheritance.superclass.span(),
                });
                return;
            };

            self.resolution
                .resolutions
                .insert(inheritance.decoration, depth);

            self.is_subclass = true;
        }

        self.define(stmt.name.value.clone());

        let prev_function_kind = self.function_kind;

        if stmt.inheritance.is_some() {
            let mut super_locals = HashSet::new();
            super_locals.insert("super".to_string());
            self.locals.push(super_locals);
        }

        let mut this_locals = HashSet::new();
        this_locals.insert("this".to_string());
        self.locals.push(this_locals);

        for method in &stmt.methods {
            self.function_kind = if method.name.value == "init" {
                FunctionKind::Initializer {
                    method: method.name.span,
                    class: stmt.name.span,
                }
            } else {
                FunctionKind::Method
            };

            self.locals
                .push(method.params.iter().map(|p| p.value.clone()).collect());
            for stmt in &method.body.stmts {
                stmt.accept(self);
            }
            self.locals.pop();
        }

        self.locals.pop();

        if stmt.inheritance.is_some() {
            self.locals.pop();
        }

        self.function_kind = prev_function_kind;
        self.is_subclass = was_subclass;
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
        if let FunctionKind::Initializer { method, class } = &self.function_kind
        {
            self.errors.push(CompileError::ReturnInInitializer {
                method: *method,
                span: stmt.return_.span(),
                class: *class,
            });
        }

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

    fn visit_call_expr(&mut self, expr: &CallExpr) -> Self::Output {
        expr.function.accept(self);
        for argument in expr.arguments.iter() {
            argument.accept(self);
        }
    }

    fn visit_get_expr(&mut self, expr: &GetExpr) -> Self::Output {
        expr.instance.accept(self);
    }

    fn visit_set_expr(&mut self, expr: &SetExpr) -> Self::Output {
        expr.instance.accept(self);
        expr.expr.accept(self);
    }

    fn visit_this_expr(&mut self, expr: &ThisExpr) -> Self::Output {
        if !matches!(self.function_kind, FunctionKind::Method) {
            self.errors
                .push(CompileError::ThisOutsideClass { span: expr.span() });
            return;
        }

        let depth = self.resolve("this").unwrap();
        self.resolution.resolutions.insert(expr.decoration, depth);
    }

    fn visit_super_expr(&mut self, expr: &SuperExpr) -> Self::Output {
        if matches!(self.function_kind, FunctionKind::Free) || !self.is_subclass
        {
            self.errors.push(CompileError::SuperOutsideSubclass {
                span: expr.super_.span(),
            });
            return;
        }

        let depth = self.resolve("super").unwrap();
        self.resolution.resolutions.insert(expr.decoration, depth);
    }
}
