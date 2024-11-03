use core::fmt;

use crate::{
    ast::{
        BinaryExpr, GroupingExpr, LiteralExpr, SpanExt as _, Spanner, UnaryExpr, Visit, Visitor
    },
    scanner::TokenKind,
    span::Spanned,
};

#[derive(Debug)]
pub enum InterpretError {
    #[allow(dead_code)]
    ExpectedBoolean(Value),
    ExpectedNumber(Value),
    ExpectedString(Value),
    ExpectedNumberOrString(Value),
    DivideByZero,
}

impl fmt::Display for InterpretError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ExpectedBoolean(v) => write!(f, "expected a boolean, found {v:?}"),
            Self::ExpectedNumber(v) => write!(f, "expected a number, found {v:?}"),
            Self::ExpectedString(v) => write!(f, "expected a string, found {v:?}"),
            Self::ExpectedNumberOrString(v) => {
                write!(f, "expected a number or a string, found {v:?}")
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
}

pub struct Interpreter;

impl Interpreter {
    fn eval<V>(&mut self, v: &V) -> Result<Value, Spanned<InterpretError>>
    where
        V: Visit<Self>,
    {
        v.accept(self)
    }

    fn eval_number<V>(
        &mut self,
        v: &V,
    ) -> Result<f64, Spanned<InterpretError>>
    where
        V: Visit<Self> + Visit<Spanner>,
    {
        match self.eval(v)? {
            Value::Number(n) => Ok(n),
            value => Err(Spanned {
                inner: InterpretError::ExpectedNumber(value),
                span: v.span(),
            }),
        }
    }

    #[allow(dead_code)]
    pub fn eval_boolean<V>(
        &mut self,
        v: &V,
    ) -> Result<bool, Spanned<InterpretError>>
    where
        V: Visit<Self> + Visit<Spanner>,
    {
        match self.eval(v)? {
            Value::Bool(b) => Ok(b),
            value => Err(Spanned {
                inner: InterpretError::ExpectedBoolean(value),
                span: v.span(),
            }),
        }
    }
}

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
        Ok(match unary.operator.kind {
            TokenKind::Bang => Value::Bool(!self.eval(&unary.expr)?.truthiness()),
            TokenKind::Minus => Value::Number(-self.eval_number(&unary.expr)?),
            _ => unreachable!(),
        })
    }

    fn visit_binary_expr(&mut self, binary: &BinaryExpr) -> Self::Output {
        Ok(match binary.operator.kind {
            TokenKind::Minus => Value::Number(
                self.eval_number(&binary.left)? - self.eval_number(&binary.right)?,
            ),
            TokenKind::Star => Value::Number(
                self.eval_number(&binary.left)? * self.eval_number(&binary.right)?,
            ),
            TokenKind::Slash => {
                let num = self.eval_number(&binary.left)?;
                let den = self.eval_number(&binary.right)?;
                if den == 0.0 {
                    return Err(Spanned {
                        inner: InterpretError::DivideByZero,
                        span: binary.right.span(),
                    });
                }
                Value::Number(num / den)
            }
            TokenKind::Plus => match (self.eval(&binary.left)?, self.eval(&binary.right)?) {
                (Value::Number(l), Value::Number(r)) => Value::Number(l + r),
                (Value::String(l), Value::String(r)) => {
                    Value::String(format!("{l}{r}"))
                }
                (Value::Number(_), r) => {
                    return Err(Spanned {
                        inner: InterpretError::ExpectedNumber(r),
                        span: binary.right.span(),
                    });
                }
                (Value::String(_), r) => {
                    return Err(Spanned {
                        inner: InterpretError::ExpectedString(r),
                        span: binary.right.span(),
                    });
                }
                (l, _) => {
                    return Err(Spanned {
                        inner: InterpretError::ExpectedNumberOrString(l),
                        span: binary.left.span(),
                    });
                }
            },
            TokenKind::Greater => Value::Bool(
                self.eval_number(&binary.left)? > self.eval_number(&binary.right)?,
            ),
            TokenKind::GreaterEqual => Value::Bool(
                self.eval_number(&binary.left)? >= self.eval_number(&binary.right)?,
            ),
            TokenKind::Less => Value::Bool(
                self.eval_number(&binary.left)? < self.eval_number(&binary.right)?,
            ),
            TokenKind::LessEqual => Value::Bool(
                self.eval_number(&binary.left)? <= self.eval_number(&binary.right)?,
            ),
            TokenKind::EqualEqual => Value::Bool(self.eval(&binary.left)? == self.eval(&binary.right)?),
            TokenKind::BangEqual => Value::Bool(self.eval(&binary.left)? != self.eval(&binary.right)?),
            _ => unreachable!(),
        })
    }

    fn visit_grouping_expr(&mut self, grouping: &GroupingExpr) -> Self::Output {
        grouping.expr.accept(self)
    }
}
