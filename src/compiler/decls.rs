use std::collections::HashMap;

use crate::ast::{
    decoration::Decoration,
    stmt::{
        BlockStmt, ExprStmt, FunDeclStmt, IfStmt, PrintStmt, ReturnStmt,
        StmtVisitor, VarDeclStmt,
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
    }

    fn visit_expr_stmt(&mut self, _: &ExprStmt) -> Self::Output {}

    fn visit_print_stmt(&mut self, _: &PrintStmt) -> Self::Output {}

    fn visit_block_stmt(&mut self, _: &BlockStmt) -> Self::Output {}

    fn visit_if_stmt(&mut self, _: &IfStmt) -> Self::Output {}

    fn visit_return_stmt(&mut self, _: &ReturnStmt) -> Self::Output {}
}
