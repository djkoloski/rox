use crate::{
    ast::expr::{Expr, GroupingExpr},
    scanner::Token,
    span::Spanned,
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

pub struct Program {
    pub stmts: Vec<Stmt>,
    pub eof: Token,
}

impl Spanned for Program {
    fn span_start(&self) -> usize {
        if let Some(stmt) = self.stmts.first() {
            stmt.span_start()
        } else {
            self.eof.span_start()
        }
    }

    fn span_end(&self) -> usize {
        self.eof.span_end()
    }
}

pub enum Repl {
    Stmt(Stmt),
    Expr(Expr),
}

impl Spanned for Repl {
    fn span_start(&self) -> usize {
        match self {
            Self::Stmt(stmt) => stmt.span_start(),
            Self::Expr(expr) => expr.span_start(),
        }
    }

    fn span_end(&self) -> usize {
        match self {
            Self::Stmt(stmt) => stmt.span_end(),
            Self::Expr(expr) => expr.span_end(),
        }
    }
}

pub enum Stmt {
    Decl(DeclStmt),
    Expr(ExprStmt),
    Print(PrintStmt),
    Block(BlockStmt),
    If(IfStmt),
}

impl Spanned for Stmt {
    fn span_start(&self) -> usize {
        match self {
            Self::Decl(stmt) => stmt.span_start(),
            Self::Expr(stmt) => stmt.span_start(),
            Self::Print(stmt) => stmt.span_start(),
            Self::Block(stmt) => stmt.span_start(),
            Self::If(stmt) => stmt.span_start(),
        }
    }

    fn span_end(&self) -> usize {
        match self {
            Self::Decl(stmt) => stmt.span_end(),
            Self::Expr(stmt) => stmt.span_end(),
            Self::Print(stmt) => stmt.span_end(),
            Self::Block(stmt) => stmt.span_end(),
            Self::If(stmt) => stmt.span_end(),
        }
    }
}

impl<V: StmtVisitor> VisitStmt<V> for Stmt {
    fn accept(&self, visitor: &mut V) -> V::Output {
        match self {
            Self::Decl(stmt) => stmt.accept(visitor),
            Self::Expr(stmt) => stmt.accept(visitor),
            Self::Print(stmt) => stmt.accept(visitor),
            Self::Block(stmt) => stmt.accept(visitor),
            Self::If(stmt) => stmt.accept(visitor),
        }
    }
}

pub struct DeclStmt {
    pub var: Token,
    pub ident: Token,
    pub assignment: Option<(Token, Expr)>,
    pub semi: Token,
}

impl Spanned for DeclStmt {
    fn span_start(&self) -> usize {
        self.var.span_start()
    }

    fn span_end(&self) -> usize {
        self.semi.span_end()
    }
}

impl<V: StmtVisitor> VisitStmt<V> for DeclStmt {
    fn accept(&self, visitor: &mut V) -> <V as StmtVisitor>::Output {
        visitor.visit_decl_stmt(self)
    }
}

pub struct ExprStmt {
    pub expr: Expr,
    pub semi: Token,
}

impl Spanned for ExprStmt {
    fn span_start(&self) -> usize {
        self.expr.span_start()
    }

    fn span_end(&self) -> usize {
        self.semi.span_end()
    }
}

impl<V: StmtVisitor> VisitStmt<V> for ExprStmt {
    fn accept(&self, visitor: &mut V) -> V::Output {
        visitor.visit_expr_stmt(self)
    }
}

pub struct PrintStmt {
    pub print: Token,
    pub expr: Expr,
    pub semi: Token,
}

impl Spanned for PrintStmt {
    fn span_start(&self) -> usize {
        self.print.span_start()
    }

    fn span_end(&self) -> usize {
        self.semi.span_end()
    }
}

impl<V: StmtVisitor> VisitStmt<V> for PrintStmt {
    fn accept(&self, visitor: &mut V) -> V::Output {
        visitor.visit_print_stmt(self)
    }
}

pub struct BlockStmt {
    pub lbrace: Token,
    pub stmts: Vec<Stmt>,
    pub rbrace: Token,
}

impl Spanned for BlockStmt {
    fn span_start(&self) -> usize {
        self.lbrace.span_start()
    }

    fn span_end(&self) -> usize {
        self.rbrace.span_end()
    }
}

impl<V: StmtVisitor> VisitStmt<V> for BlockStmt {
    fn accept(&self, visitor: &mut V) -> V::Output {
        visitor.visit_block_stmt(self)
    }
}

pub struct IfStmt {
    pub if_: Token,
    pub group: GroupingExpr,
    pub then: Box<Stmt>,
    pub else_: Option<(Token, Box<Stmt>)>,
}

impl Spanned for IfStmt {
    fn span_start(&self) -> usize {
        self.if_.span_start()
    }

    fn span_end(&self) -> usize {
        if let Some((_, stmt)) = &self.else_ {
            stmt.span_end()
        } else {
            self.then.span_end()
        }
    }
}

impl<V: StmtVisitor> VisitStmt<V> for IfStmt {
    fn accept(&self, visitor: &mut V) -> V::Output {
        visitor.visit_if_stmt(self)
    }
}
