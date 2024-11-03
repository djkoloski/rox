use crate::{ast::expr::Expr, scanner::Token, span::Spanned};

pub trait StmtVisitor {
    type Output;

    fn visit_expr_stmt(&mut self, stmt: &ExprStmt) -> Self::Output;
    fn visit_print_stmt(&mut self, stmt: &PrintStmt) -> Self::Output;
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

pub enum Stmt {
    Expr(ExprStmt),
    Print(PrintStmt),
}

impl Spanned for Stmt {
    fn span_start(&self) -> usize {
        match self {
            Self::Expr(stmt) => stmt.span_start(),
            Self::Print(stmt) => stmt.span_start(),
        }
    }

    fn span_end(&self) -> usize {
        match self {
            Self::Expr(stmt) => stmt.span_end(),
            Self::Print(stmt) => stmt.span_end(),
        }
    }
}

impl<V: StmtVisitor> VisitStmt<V> for Stmt {
    fn accept(&self, visitor: &mut V) -> V::Output {
        match self {
            Self::Expr(stmt) => stmt.accept(visitor),
            Self::Print(stmt) => stmt.accept(visitor),
        }
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
