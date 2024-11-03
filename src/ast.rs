use crate::{
    scanner::{Token, TokenKind},
    span::Span,
};

pub trait Visitor {
    type Output;

    fn visit_literal_expr(&mut self, literal: &LiteralExpr) -> Self::Output;
    fn visit_unary_expr(&mut self, unary: &UnaryExpr) -> Self::Output;
    fn visit_binary_expr(&mut self, binary: &BinaryExpr) -> Self::Output;
    fn visit_grouping_expr(&mut self, grouping: &GroupingExpr) -> Self::Output;
}

pub trait Visit<V: Visitor> {
    fn accept(&self, visitor: &mut V) -> V::Output;
}

impl<T: Visit<V>, V: Visitor> Visit<V> for Box<T> {
    fn accept(&self, visitor: &mut V) -> <V as Visitor>::Output {
        (**self).accept(visitor)
    }
}

pub enum Expr {
    Literal(LiteralExpr),
    Unary(UnaryExpr),
    Binary(BinaryExpr),
    Grouping(GroupingExpr),
}

impl<V: Visitor> Visit<V> for Expr {
    fn accept(&self, visitor: &mut V) -> V::Output {
        match self {
            Self::Literal(literal) => visitor.visit_literal_expr(literal),
            Self::Unary(unary) => visitor.visit_unary_expr(unary),
            Self::Binary(binary) => visitor.visit_binary_expr(binary),
            Self::Grouping(grouping) => visitor.visit_grouping_expr(grouping),
        }
    }
}

pub struct LiteralExpr {
    pub token: Token,
}

impl<V: Visitor> Visit<V> for LiteralExpr {
    fn accept(&self, visitor: &mut V) -> <V as Visitor>::Output {
        visitor.visit_literal_expr(self)
    }
}

pub struct UnaryExpr {
    pub operator: Token,
    pub expr: Box<Expr>,
}

impl<V: Visitor> Visit<V> for UnaryExpr {
    fn accept(&self, visitor: &mut V) -> <V as Visitor>::Output {
        visitor.visit_unary_expr(self)
    }
}

pub struct BinaryExpr {
    pub left: Box<Expr>,
    pub operator: Token,
    pub right: Box<Expr>,
}

impl<V: Visitor> Visit<V> for BinaryExpr {
    fn accept(&self, visitor: &mut V) -> <V as Visitor>::Output {
        visitor.visit_binary_expr(self)
    }
}

pub struct GroupingExpr {
    pub lparen: Token,
    pub expr: Box<Expr>,
    pub rparen: Token,
}

impl<V: Visitor> Visit<V> for GroupingExpr {
    fn accept(&self, visitor: &mut V) -> <V as Visitor>::Output {
        self.expr.accept(visitor)
    }
}

pub struct SpanStart;

impl Visitor for SpanStart {
    type Output = usize;

    fn visit_literal_expr(&mut self, literal: &LiteralExpr) -> Self::Output {
        literal.token.span.start()
    }

    fn visit_unary_expr(&mut self, unary: &UnaryExpr) -> Self::Output {
        unary.operator.span.start()
    }

    fn visit_binary_expr(&mut self, binary: &BinaryExpr) -> Self::Output {
        binary.left.accept(self)
    }

    fn visit_grouping_expr(&mut self, grouping: &GroupingExpr) -> Self::Output {
        grouping.lparen.span.start()
    }
}

pub struct SpanEnd;

impl Visitor for SpanEnd {
    type Output = usize;

    fn visit_literal_expr(&mut self, literal: &LiteralExpr) -> Self::Output {
        literal.token.span.end()
    }

    fn visit_unary_expr(&mut self, unary: &UnaryExpr) -> Self::Output {
        unary.expr.accept(self)
    }

    fn visit_binary_expr(&mut self, binary: &BinaryExpr) -> Self::Output {
        binary.right.accept(self)
    }

    fn visit_grouping_expr(&mut self, grouping: &GroupingExpr) -> Self::Output {
        grouping.rparen.span.end()
    }
}

pub trait Spanned {
    fn span_start(&self) -> usize;
    fn span_end(&self) -> usize;

    fn span(&self) -> Span {
        Span::scan(self.span_start(), self.span_end())
    }
}

impl<T: Visit<SpanStart> + Visit<SpanEnd>> Spanned for T {
    fn span_start(&self) -> usize {
        self.accept(&mut SpanStart)
    }

    fn span_end(&self) -> usize {
        self.accept(&mut SpanEnd)
    }
}

pub struct Printer<'t> {
    text: &'t str,
}

impl Visitor for Printer<'_> {
    type Output = String;

    fn visit_literal_expr(&mut self, literal: &LiteralExpr) -> Self::Output {
        match &literal.token.kind {
            TokenKind::True => "true".to_string(),
            TokenKind::False => "false".to_string(),
            TokenKind::Nil => "nil".to_string(),
            TokenKind::String(s) => format!("\"{s}\""),
            TokenKind::Number(x) => format!("{x}"),
            _ => panic!("invalid literal expression"),
        }
    }

    fn visit_unary_expr(&mut self, unary: &UnaryExpr) -> Self::Output {
        format!(
            "({} {})",
            unary.operator.span.get(self.text),
            unary.expr.accept(self),
        )
    }

    fn visit_binary_expr(&mut self, binary: &BinaryExpr) -> Self::Output {
        format!(
            "({} {} {})",
            binary.operator.span.get(self.text),
            binary.left.accept(self),
            binary.right.accept(self),
        )
    }

    fn visit_grouping_expr(&mut self, grouping: &GroupingExpr) -> Self::Output {
        format!("(group {})", grouping.expr.accept(self))
    }
}
