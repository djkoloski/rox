use std::collections::HashMap;

use crate::ast::{
    decoration::Decoration,
    stmt::{FunDeclStmt, StmtVisitor},
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

    fn visit_var_decl_stmt(
        &mut self,
        _: &crate::ast::stmt::VarDeclStmt,
    ) -> Self::Output {
    }

    fn visit_fun_decl_stmt(&mut self, stmt: &FunDeclStmt) -> Self::Output {
        self.functions.insert(stmt.decoration, stmt.clone());
    }

    fn visit_expr_stmt(
        &mut self,
        _: &crate::ast::stmt::ExprStmt,
    ) -> Self::Output {
    }

    fn visit_print_stmt(
        &mut self,
        _: &crate::ast::stmt::PrintStmt,
    ) -> Self::Output {
    }

    fn visit_block_stmt(
        &mut self,
        _: &crate::ast::stmt::BlockStmt,
    ) -> Self::Output {
    }

    fn visit_if_stmt(&mut self, _: &crate::ast::stmt::IfStmt) -> Self::Output {}
}
