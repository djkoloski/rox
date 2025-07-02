use std::collections::HashMap;

use crate::ast::{
    decoration::Decoration,
    stmt::{
        BlockStmt, ExprStmt, FunDeclStmt, IfStmt, PrintStmt, ReturnStmt,
        StmtVisitor, VarDeclStmt, VisitStmt as _, WhileStmt,
    },
};

pub struct Decls {
    functions: HashMap<Decoration, FunDeclStmt>,
}

impl Decls {
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
        }
    }

    pub fn get_fun(&self, id: Decoration) -> Option<&FunDeclStmt> {
        self.functions.get(&id)
    }
}

impl Default for Decls {
    fn default() -> Self {
        Self::new()
    }
}

impl StmtVisitor for Decls {
    type Output = ();

    fn visit_var_decl_stmt(&mut self, _: &VarDeclStmt) -> Self::Output {}

    fn visit_fun_decl_stmt(&mut self, stmt: &FunDeclStmt) -> Self::Output {
        self.functions.insert(stmt.decoration, stmt.clone());

        stmt.body.accept(self);
    }

    fn visit_expr_stmt(&mut self, _: &ExprStmt) -> Self::Output {}

    fn visit_print_stmt(&mut self, _: &PrintStmt) -> Self::Output {}

    fn visit_block_stmt(&mut self, stmt: &BlockStmt) -> Self::Output {
        for stmt in &stmt.stmts {
            stmt.accept(self);
        }
    }

    fn visit_if_stmt(&mut self, stmt: &IfStmt) -> Self::Output {
        stmt.then.accept(self);
        if let Some((_, else_stmt)) = &stmt.else_ {
            else_stmt.accept(self);
        }
    }

    fn visit_while_stmt(&mut self, stmt: &WhileStmt) -> Self::Output {
        stmt.body.accept(self);
    }

    fn visit_return_stmt(&mut self, _: &ReturnStmt) -> Self::Output {}
}
