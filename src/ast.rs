use crate::scanner::{Token, TokenKind, Value};

pub enum Expr {
    Literal(LiteralExpr),
    Unary(UnaryExpr),
    Binary(BinaryExpr),
    Grouping(GroupingExpr),
}

pub struct LiteralExpr {
    pub token: Token,
}

pub struct UnaryExpr {
    pub operator: Token,
    pub expr: Box<Expr>,
}

pub struct BinaryExpr {
    pub left: Box<Expr>,
    pub operator: Token,
    pub right: Box<Expr>,
}

pub struct GroupingExpr {
    pub lparen: Token,
    pub expr: Box<Expr>,
    pub rparen: Token,
}

pub trait Visitor {
    type Output;

    fn visit_literal_expr(&self, literal: &LiteralExpr) -> Self::Output;
    fn visit_unary_expr(&self, unary: &UnaryExpr) -> Self::Output;
    fn visit_binary_expr(&self, binary: &BinaryExpr) -> Self::Output;
    fn visit_grouping_expr(&self, grouping: &GroupingExpr) -> Self::Output;
}

pub trait Visit<V: Visitor> {
    fn accept(&self, visitor: &V) -> V::Output;
}

impl<V: Visitor> Visit<V> for Expr {
    fn accept(&self, visitor: &V) -> V::Output {
        match self {
            Self::Literal(literal) => visitor.visit_literal_expr(literal),
            Self::Unary(unary) => visitor.visit_unary_expr(unary),
            Self::Binary(binary) => visitor.visit_binary_expr(binary),
            Self::Grouping(grouping) => visitor.visit_grouping_expr(grouping),
        }
    }
}

pub struct AstPrinter;

impl Visitor for AstPrinter {
    type Output = String;

    fn visit_literal_expr(&self, literal: &LiteralExpr) -> Self::Output {
        match literal.token.kind {
            TokenKind::True => "true".to_string(),
            TokenKind::False => "false".to_string(),
            TokenKind::Nil => "nil".to_string(),
            TokenKind::String | TokenKind::Number => {
                match literal.token.value.as_ref().unwrap() {
                    Value::Number(number) => format!("{}", number),
                    Value::String(string) => format!("\"{}\"", string),
                }
            }
            _ => panic!("invalid literal expression"),
        }
    }

    fn visit_unary_expr(&self, unary: &UnaryExpr) -> Self::Output {
        format!("({} {})", unary.operator.lexeme, unary.expr.accept(self))
    }

    fn visit_binary_expr(&self, binary: &BinaryExpr) -> Self::Output {
        format!(
            "({} {} {})",
            binary.operator.lexeme,
            binary.left.accept(self),
            binary.right.accept(self),
        )
    }

    fn visit_grouping_expr(&self, grouping: &GroupingExpr) -> Self::Output {
        format!("(group {})", grouping.expr.accept(self))
    }
}
