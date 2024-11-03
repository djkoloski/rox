use core::fmt;

use crate::{
    ast::{
        BinaryExpr, GroupingExpr, LiteralExpr, UnaryExpr, Visit as _, Visitor,
    },
    scanner::TokenKind,
    span::{Span, Spanned},
};

#[derive(Debug)]
pub enum InterpretError {
    #[allow(dead_code)]
    ExpectedBoolean,
    ExpectedNumber,
    ExpectedNumberOrString,
    DivideByZero,
}

impl fmt::Display for InterpretError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ExpectedBoolean => write!(f, "expected a boolean"),
            Self::ExpectedNumber => write!(f, "expected a number"),
            Self::ExpectedNumberOrString => {
                write!(f, "expected a number or a string")
            }
            Self::DivideByZero => write!(f, "attempted to divide by zero"),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum Value {
    Nil,
    Bool(bool),
    Number(f64),
    String(String),
    // Object(???),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Nil => write!(f, "nil"),
            Self::Bool(b) => {
                if *b {
                    write!(f, "true")
                } else {
                    write!(f, "false")
                }
            }
            Self::Number(n) => write!(f, "{n}"),
            Self::String(s) => write!(f, "\"{s}\""),
        }
    }
}

impl Value {
    pub fn truthiness(&self) -> bool {
        match self {
            Self::Nil => false,
            Self::Bool(b) => *b,
            Self::Number(_) | Self::String(_) => true,
        }
    }

    #[allow(dead_code)]
    pub fn expect_bool(
        self,
        span: Span,
    ) -> Result<bool, Spanned<InterpretError>> {
        if let Value::Bool(b) = self {
            Ok(b)
        } else {
            Err(Spanned {
                inner: InterpretError::ExpectedBoolean,
                span,
            })
        }
    }

    pub fn expect_number(
        self,
        span: Span,
    ) -> Result<f64, Spanned<InterpretError>> {
        if let Value::Number(n) = self {
            Ok(n)
        } else {
            Err(Spanned {
                inner: InterpretError::ExpectedNumber,
                span,
            })
        }
    }
}

pub struct Interpreter;

impl Visitor for Interpreter {
    type Output = Result<Value, Spanned<InterpretError>>;

    fn visit_literal_expr(&mut self, literal: &LiteralExpr) -> Self::Output {
        Ok(match &literal.token.kind {
            TokenKind::Nil => Value::Nil,
            TokenKind::True => Value::Bool(true),
            TokenKind::False => Value::Bool(false),
            TokenKind::Number(x) => Value::Number(*x),
            TokenKind::String(s) => Value::String(s.clone()),
            _ => unreachable!(),
        })
    }

    fn visit_unary_expr(&mut self, unary: &UnaryExpr) -> Self::Output {
        let value = unary.expr.accept(self)?;
        let span = unary.operator.span;

        Ok(match unary.operator.kind {
            TokenKind::Bang => Value::Bool(!value.truthiness()),
            TokenKind::Minus => Value::Number(-value.expect_number(span)?),
            _ => unreachable!(),
        })
    }

    fn visit_binary_expr(&mut self, binary: &BinaryExpr) -> Self::Output {
        let left = binary.left.accept(self)?;
        let right = binary.right.accept(self)?;
        let span = binary.operator.span;

        Ok(match binary.operator.kind {
            TokenKind::Minus => Value::Number(
                left.expect_number(span)? - right.expect_number(span)?,
            ),
            TokenKind::Star => Value::Number(
                left.expect_number(span)? * right.expect_number(span)?,
            ),
            TokenKind::Slash => {
                let num = left.expect_number(span)?;
                let den = right.expect_number(span)?;
                if den == 0.0 {
                    return Err(Spanned {
                        inner: InterpretError::DivideByZero,
                        span,
                    });
                }
                Value::Number(num / den)
            }
            TokenKind::Plus => match (left, right) {
                (Value::Number(l), Value::Number(r)) => Value::Number(l + r),
                (Value::String(l), Value::String(r)) => {
                    Value::String(format!("{l}{r}"))
                }
                _ => {
                    return Err(Spanned {
                        inner: InterpretError::ExpectedNumberOrString,
                        span,
                    })
                }
            },
            TokenKind::Greater => Value::Bool(
                left.expect_number(span)? > right.expect_number(span)?,
            ),
            TokenKind::GreaterEqual => Value::Bool(
                left.expect_number(span)? >= right.expect_number(span)?,
            ),
            TokenKind::Less => Value::Bool(
                left.expect_number(span)? < right.expect_number(span)?,
            ),
            TokenKind::LessEqual => Value::Bool(
                left.expect_number(span)? <= right.expect_number(span)?,
            ),
            TokenKind::EqualEqual => Value::Bool(left == right),
            TokenKind::BangEqual => Value::Bool(left != right),
            _ => unreachable!(),
        })
    }

    fn visit_grouping_expr(&mut self, grouping: &GroupingExpr) -> Self::Output {
        grouping.expr.accept(self)
    }
}
