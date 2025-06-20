use crate::{
    ast::expr::{Expr, GroupingExpr},
    scanner::Token,
};

pub trait StmtVisitor {
    type Output;

    fn visit_decl_stmt(&mut self, stmt: &DeclStmt) -> Self::Output;
    fn visit_expr_stmt(&mut self, stmt: &ExprStmt) -> Self::Output;
    fn visit_print_stmt(&mut self, stmt: &PrintStmt) -> Self::Output;
    fn visit_block_stmt(&mut self, stmt: &BlockStmt) -> Self::Output;
    fn visit_if_stmt(&mut self, stmt: &IfStmt) -> Self::Output;
}

pub trait VisitStmt<V: StmtVisitor> {
    fn accept(&self, visitor: &mut V) -> V::Output;
}

ast_node! {
    pub struct Program {
        pub stmts: Vec<Stmt>,
        pub eof: Token,
    }

    pub enum Repl {
        Stmt(Stmt),
        Expr(Expr),
    }

    #[visit(VisitStmt, StmtVisitor)]
    pub enum Stmt {
        Decl(DeclStmt),
        Expr(ExprStmt),
        Print(PrintStmt),
        Block(BlockStmt),
        If(IfStmt),
    }

    #[visit(VisitStmt, StmtVisitor::visit_decl_stmt)]
    pub struct DeclStmt {
        pub var: Token,
        pub ident: Token,
        pub assignment: Option<(Token, Expr)>,
        pub semi: Token,
    }

    #[visit(VisitStmt, StmtVisitor::visit_expr_stmt)]
    pub struct ExprStmt {
        pub expr: Expr,
        pub semi: Token,
    }

    #[visit(VisitStmt, StmtVisitor::visit_print_stmt)]
    pub struct PrintStmt {
        pub print: Token,
        pub expr: Expr,
        pub semi: Token,
    }

    #[visit(VisitStmt, StmtVisitor::visit_block_stmt)]
    pub struct BlockStmt {
        pub lbrace: Token,
        pub stmts: Vec<Stmt>,
        pub rbrace: Token,
    }

    #[visit(VisitStmt, StmtVisitor::visit_if_stmt)]
    pub struct IfStmt {
        pub if_: Token,
        pub group: GroupingExpr,
        pub then: Box<Stmt>,
        pub else_: Option<(Token, Box<Stmt>)>,
    }
}
