use crate::{scanner::Token, span::Spanned};

pub trait ExprVisitor {
    type Output;

    fn visit_literal_expr(&mut self, expr: &LiteralExpr) -> Self::Output;
    fn visit_unary_expr(&mut self, expr: &UnaryExpr) -> Self::Output;
    fn visit_binary_expr(&mut self, expr: &BinaryExpr) -> Self::Output;
    fn visit_grouping_expr(&mut self, expr: &GroupingExpr) -> Self::Output;
    fn visit_variable_expr(&mut self, expr: &VariableExpr) -> Self::Output;
    fn visit_assign_expr(&mut self, expr: &AssignExpr) -> Self::Output;
}

pub trait VisitExpr<V: ExprVisitor> {
    fn accept(&self, visitor: &mut V) -> V::Output;
}

pub enum Expr {
    Literal(LiteralExpr),
    Unary(UnaryExpr),
    Binary(BinaryExpr),
    Grouping(GroupingExpr),
    Variable(VariableExpr),
    Assign(AssignExpr),
}

impl Spanned for Expr {
    fn span_start(&self) -> usize {
        match self {
            Self::Literal(expr) => expr.span_start(),
            Self::Unary(expr) => expr.span_start(),
            Self::Binary(expr) => expr.span_start(),
            Self::Grouping(expr) => expr.span_start(),
            Self::Variable(expr) => expr.span_start(),
            Self::Assign(expr) => expr.span_start(),
        }
    }

    fn span_end(&self) -> usize {
        match self {
            Self::Literal(expr) => expr.span_end(),
            Self::Unary(expr) => expr.span_end(),
            Self::Binary(expr) => expr.span_end(),
            Self::Grouping(expr) => expr.span_end(),
            Self::Variable(expr) => expr.span_end(),
            Self::Assign(expr) => expr.span_end(),
        }
    }
}

impl<V: ExprVisitor> VisitExpr<V> for Expr {
    fn accept(&self, visitor: &mut V) -> V::Output {
        match self {
            Self::Literal(expr) => visitor.visit_literal_expr(expr),
            Self::Unary(expr) => visitor.visit_unary_expr(expr),
            Self::Binary(expr) => visitor.visit_binary_expr(expr),
            Self::Grouping(expr) => visitor.visit_grouping_expr(expr),
            Self::Variable(expr) => visitor.visit_variable_expr(expr),
            Self::Assign(expr) => visitor.visit_assign_expr(expr),
        }
    }
}

pub struct LiteralExpr {
    pub token: Token,
}

impl Spanned for LiteralExpr {
    fn span_start(&self) -> usize {
        self.token.span_start()
    }

    fn span_end(&self) -> usize {
        self.token.span_end()
    }
}

impl<V: ExprVisitor> VisitExpr<V> for LiteralExpr {
    fn accept(&self, visitor: &mut V) -> V::Output {
        visitor.visit_literal_expr(self)
    }
}

pub struct UnaryExpr {
    pub operator: Token,
    pub inner: Box<Expr>,
}

impl Spanned for UnaryExpr {
    fn span_start(&self) -> usize {
        self.operator.span_start()
    }

    fn span_end(&self) -> usize {
        self.inner.span_end()
    }
}

impl<V: ExprVisitor> VisitExpr<V> for UnaryExpr {
    fn accept(&self, visitor: &mut V) -> V::Output {
        visitor.visit_unary_expr(self)
    }
}

pub struct BinaryExpr {
    pub left: Box<Expr>,
    pub operator: Token,
    pub right: Box<Expr>,
}

impl Spanned for BinaryExpr {
    fn span_start(&self) -> usize {
        self.left.span_start()
    }

    fn span_end(&self) -> usize {
        self.right.span_end()
    }
}

impl<V: ExprVisitor> VisitExpr<V> for BinaryExpr {
    fn accept(&self, visitor: &mut V) -> V::Output {
        visitor.visit_binary_expr(self)
    }
}

pub struct GroupingExpr {
    pub lparen: Token,
    pub inner: Box<Expr>,
    pub rparen: Token,
}

impl Spanned for GroupingExpr {
    fn span_start(&self) -> usize {
        self.lparen.span_start()
    }

    fn span_end(&self) -> usize {
        self.rparen.span_end()
    }
}

impl<V: ExprVisitor> VisitExpr<V> for GroupingExpr {
    fn accept(&self, visitor: &mut V) -> V::Output {
        self.inner.accept(visitor)
    }
}

pub struct VariableExpr {
    pub ident: Token,
}

impl Spanned for VariableExpr {
    fn span_start(&self) -> usize {
        self.ident.span_start()
    }

    fn span_end(&self) -> usize {
        self.ident.span_end()
    }
}

impl<V: ExprVisitor> VisitExpr<V> for VariableExpr {
    fn accept(&self, visitor: &mut V) -> <V as ExprVisitor>::Output {
        visitor.visit_variable_expr(self)
    }
}

pub struct AssignExpr {
    pub ident: Token,
    #[allow(dead_code)]
    pub equal: Token,
    pub expr: Box<Expr>,
}

impl Spanned for AssignExpr {
    fn span_start(&self) -> usize {
        self.ident.span_start()
    }

    fn span_end(&self) -> usize {
        self.expr.span_end()
    }
}

impl<V: ExprVisitor> VisitExpr<V> for AssignExpr {
    fn accept(&self, visitor: &mut V) -> <V as ExprVisitor>::Output {
        visitor.visit_assign_expr(self)
    }
}
