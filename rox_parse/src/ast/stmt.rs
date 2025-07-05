use rox_lex::token_kind::*;

use crate::ast::{
    ast_node,
    decoration::Decoration,
    expr::{Expr, GroupingExpr},
    punctuated::Punctuated,
};

pub trait StmtVisitor {
    type Output;

    fn visit_var_decl_stmt(&mut self, stmt: &VarDeclStmt) -> Self::Output;
    fn visit_fun_decl_stmt(&mut self, stmt: &FunDeclStmt) -> Self::Output;
    fn visit_class_decl_stmt(&mut self, stmt: &ClassDeclStmt) -> Self::Output;
    fn visit_expr_stmt(&mut self, stmt: &ExprStmt) -> Self::Output;
    fn visit_print_stmt(&mut self, stmt: &PrintStmt) -> Self::Output;
    fn visit_block_stmt(&mut self, stmt: &BlockStmt) -> Self::Output;
    fn visit_if_stmt(&mut self, stmt: &IfStmt) -> Self::Output;
    fn visit_while_stmt(&mut self, stmt: &WhileStmt) -> Self::Output;
    fn visit_return_stmt(&mut self, stmt: &ReturnStmt) -> Self::Output;
}

pub trait VisitStmt<V: StmtVisitor> {
    fn accept(&self, visitor: &mut V) -> V::Output;
}

ast_node! {
    pub struct Program {
        pub stmts: Vec<Stmt>,
        pub eof: Eof,
    }

    pub enum Repl {
        Stmt(Stmt),
        Expr(Expr),
    }

    #[visit(VisitStmt, StmtVisitor)]
    pub enum Stmt {
        VarDecl(VarDeclStmt),
        FunDecl(FunDeclStmt),
        ClassDecl(ClassDeclStmt),
        Expr(ExprStmt),
        Print(PrintStmt),
        Block(BlockStmt),
        If(IfStmt),
        While(WhileStmt),
        Return(ReturnStmt),
    }

    #[visit(VisitStmt, StmtVisitor::visit_var_decl_stmt)]
    pub struct VarDeclStmt {
        pub var: Var,
        pub ident: Identifier,
        pub assignment: Option<(Equal, Expr)>,
        pub semi: Semicolon,
    }

    #[visit(VisitStmt, StmtVisitor::visit_fun_decl_stmt)]
    pub struct FunDeclStmt {
        pub fun: Fun,
        pub function: Function,
    }

    #[visit(VisitStmt, StmtVisitor::visit_class_decl_stmt)]
    pub struct ClassDeclStmt {
        pub decoration: Decoration,
        pub class: Class,
        pub name: Identifier,
        pub inheritance: Option<Inheritance>,
        pub lbrace: LeftBrace,
        pub methods: Vec<Function>,
        pub rbrace: RightBrace,
    }

    pub struct Inheritance {
        pub decoration: Decoration,
        pub less: Less,
        pub superclass: Identifier,
    }

    pub struct Function {
        pub decoration: Decoration,
        pub name: Identifier,
        pub lparen: LeftParen,
        pub params: Punctuated<Identifier, Comma>,
        pub rparen: RightParen,
        pub body: BlockStmt,
    }

    #[visit(VisitStmt, StmtVisitor::visit_expr_stmt)]
    pub struct ExprStmt {
        pub expr: Expr,
        pub semi: Semicolon,
    }

    #[visit(VisitStmt, StmtVisitor::visit_print_stmt)]
    pub struct PrintStmt {
        pub print: Print,
        pub expr: Expr,
        pub semi: Semicolon,
    }

    #[visit(VisitStmt, StmtVisitor::visit_block_stmt)]
    pub struct BlockStmt {
        pub lbrace: LeftBrace,
        pub stmts: Vec<Stmt>,
        pub rbrace: RightBrace,
    }

    #[visit(VisitStmt, StmtVisitor::visit_if_stmt)]
    pub struct IfStmt {
        pub if_: If,
        pub group: GroupingExpr,
        pub then: Box<Stmt>,
        pub else_: Option<(Else, Box<Stmt>)>,
    }

    #[visit(VisitStmt, StmtVisitor::visit_while_stmt)]
    pub struct WhileStmt {
        pub while_: While,
        pub lparen: LeftParen,
        pub expr: Expr,
        pub rparen: RightParen,
        pub body: Box<Stmt>,
    }

    #[visit(VisitStmt, StmtVisitor::visit_return_stmt)]
    pub struct ReturnStmt {
        pub return_: Return,
        pub expr: Option<Expr>,
        pub semi: Semicolon,
    }
}
