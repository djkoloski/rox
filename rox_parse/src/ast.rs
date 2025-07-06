use rox_lex::token_kind::*;

use crate::{Punctuated, ast_macro::*};

token_group! {
    pub enum Literal {
        Number(FloatLiteral),
        String(StringLiteral),
        True(True),
        False(False),
        Nil(Nil),
    }

    pub enum UnaryOperator {
        Bang(Bang),
        Minus(Minus),
    }

    pub enum BinaryOperator {
        And(And),
        Or(Or),
        Greater(Greater),
        GreaterEqual(GreaterEqual),
        Less(Less),
        LessEqual(LessEqual),
        BangEqual(BangEqual),
        EqualEqual(EqualEqual),
        Minus(Minus),
        Plus(Plus),
        Slash(Slash),
        Star(Star),
    }
}

ast! {
    // Expr

    pub enum Expr {
        Literal(LiteralExpr),
        Unary(UnaryExpr),
        Binary(BinaryExpr),
        Grouping(GroupingExpr),
        Variable(VariableExpr),
        Assign(AssignExpr),
        Call(CallExpr),
        Get(GetExpr),
        Set(SetExpr),
        This(ThisExpr),
        Super(SuperExpr),
    }

    #[accept = visit_literal_expr]
    pub struct LiteralExpr {
        pub literal: Literal,
    }

    #[accept = visit_unary_expr]
    pub struct UnaryExpr {
        pub operator: UnaryOperator,
        #[visit]
        pub inner: Box<Expr>,
    }

    #[accept = visit_binary_expr]
    pub struct BinaryExpr {
        #[visit]
        pub left: Box<Expr>,
        pub operator: BinaryOperator,
        #[visit]
        pub right: Box<Expr>,
    }

    #[accept = visit_grouping_expr]
    pub struct GroupingExpr {
        pub lparen: LeftParen,
        #[visit]
        pub inner: Box<Expr>,
        pub rparen: RightParen,
    }

    #[accept = visit_variable_expr]
    pub struct VariableExpr {
        pub ident: Identifier,
    }

    #[accept = visit_assign_expr]
    pub struct AssignExpr {
        pub ident: Identifier,
        pub equal: Equal,
        #[visit]
        pub expr: Box<Expr>,
    }

    #[accept = visit_call_expr]
    pub struct CallExpr {
        #[visit]
        pub function: Box<Expr>,
        pub lparen: LeftParen,
        #[visit]
        pub arguments: Punctuated<Expr, Comma>,
        pub rparen: RightParen,
    }

    #[accept = visit_get_expr]
    pub struct GetExpr {
        #[visit]
        pub instance: Box<Expr>,
        pub dot: Dot,
        pub name: Identifier,
    }

    #[accept = visit_set_expr]
    pub struct SetExpr {
        #[visit]
        pub instance: Box<Expr>,
        pub dot: Dot,
        pub name: Identifier,
        pub equal: Equal,
        #[visit]
        pub expr: Box<Expr>,
    }

    #[accept = visit_this_expr]
    pub struct ThisExpr {
        pub this: This,
    }

    #[accept = visit_super_expr]
    pub struct SuperExpr {
        pub super_: Super,
        pub dot: Dot,
        pub name: Identifier,
    }

    // Stmt

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

    #[accept = visit_var_decl_stmt]
    pub struct VarDeclStmt {
        pub var: Var,
        pub ident: Identifier,
        #[visit]
        pub assignment: Option<Assignment>,
        pub semi: Semicolon,
    }

    pub struct Assignment {
        pub equal: Equal,
        #[visit]
        pub expr: Expr,
    }

    #[accept = visit_fun_decl_stmt]
    pub struct FunDeclStmt {
        pub fun: Fun,
        #[visit]
        pub function: Function,
    }

    #[accept = visit_class_decl_stmt]
    pub struct ClassDeclStmt {
        pub class: Class,
        pub name: Identifier,
        pub inheritance: Option<Inheritance>,
        pub lbrace: LeftBrace,
        #[visit]
        pub methods: Vec<Function>,
        pub rbrace: RightBrace,
    }

    pub struct Inheritance {
        pub less: Less,
        pub superclass: Identifier,
    }

    pub struct Function {
        pub name: Identifier,
        pub lparen: LeftParen,
        pub params: Punctuated<Identifier, Comma>,
        pub rparen: RightParen,
        #[visit]
        pub body: BlockStmt,
    }

    #[accept = visit_expr_stmt]
    pub struct ExprStmt {
        #[visit]
        pub expr: Expr,
        pub semi: Semicolon,
    }

    #[accept = visit_print_stmt]
    pub struct PrintStmt {
        pub print: Print,
        #[visit]
        pub expr: Expr,
        pub semi: Semicolon,
    }

    #[accept = visit_block_stmt]
    pub struct BlockStmt {
        pub lbrace: LeftBrace,
        #[visit]
        pub stmts: Vec<Stmt>,
        pub rbrace: RightBrace,
    }

    #[accept = visit_if_stmt]
    pub struct IfStmt {
        pub if_: If,
        #[visit]
        pub condition: GroupingExpr,
        #[visit]
        pub stmt: Box<Stmt>,
        #[visit]
        pub else_clause: Option<ElseClause>,
    }

    pub struct ElseClause {
        pub else_: Else,
        #[visit]
        pub stmt: Box<Stmt>,
    }

    #[accept = visit_while_stmt]
    pub struct WhileStmt {
        pub while_: While,
        pub lparen: LeftParen,
        #[visit]
        pub expr: Expr,
        pub rparen: RightParen,
        #[visit]
        pub body: Box<Stmt>,
    }

    #[accept = visit_return_stmt]
    pub struct ReturnStmt {
        pub return_: Return,
        #[visit]
        pub expr: Option<Expr>,
        pub semi: Semicolon,
    }

    // Top-level

    pub struct Program {
        #[visit]
        pub stmts: Vec<Stmt>,
        pub eof: Eof,
    }

    pub enum Repl {
        Stmt(Stmt),
        Expr(Expr),
    }
}
