use core::fmt;

use crate::{
    ast::{
        BinaryExpr, GroupingExpr, LiteralExpr, Spanned, UnaryExpr, Visit,
        Visitor,
    },
    diagnostic::{Context, Diagnostic},
    scanner::TokenKind,
    span::Span,
};

#[derive(Debug)]
pub enum InterpretError {
    #[allow(dead_code)]
    ExpectedBoolean {
        span: Span,
        actual: Value,
    },
    ExpectedNumber {
        span: Span,
        actual: Value,
    },
    ExpectedString {
        span: Span,
        actual: Value,
    },
    ExpectedNumberOrString {
        span: Span,
        actual: Value,
    },
    DivideByZero(Span),
}

impl Diagnostic for InterpretError {
    fn fmt(
        &self,
        c: &mut Context<'_>,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            Self::ExpectedBoolean { span, actual } => {
                c.error(f, format_args!("unexpected non-boolean value"))?;
                c.span(
                    *span,
                    f,
                    format_args!(
                        "expected this to be a boolean, but it was '{actual}'"
                    ),
                )?;
            }
            Self::ExpectedNumber { span, actual } => {
                c.error(f, format_args!("unexpected non-number value"))?;
                c.span(
                    *span,
                    f,
                    format_args!(
                        "expected this to be a number, but it was '{actual}'"
                    ),
                )?;
            }
            Self::ExpectedString { span, actual } => {
                c.error(f, format_args!("unexpected non-string value"))?;
                c.span(
                    *span,
                    f,
                    format_args!(
                        "expected this to be a string, but it was '{actual}'"
                    ),
                )?;
            }
            Self::ExpectedNumberOrString { span, actual } => {
                c.error(
                    f,
                    format_args!("unexpected non-number, non-string value"),
                )?;
                c.span(
                    *span,
                    f,
                    format_args!(
                        "expected this to be a number or string, but it was \
                         '{actual}'"
                    ),
                )?;
            }
            Self::DivideByZero(span) => {
                c.error(f, format_args!("attempted to divide by zero"))?;
                c.span(
                    *span,
                    f,
                    format_args!("this denominator evaluated to zero"),
                )?;
            }
        }
        Ok(())
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
    fn eval<V>(&mut self, v: &V) -> Result<Value, InterpretError>
    where
        V: Visit<Self>,
    {
        v.accept(self)
    }

    #[allow(dead_code)]
    pub fn eval_boolean<V>(&mut self, v: &V) -> Result<bool, InterpretError>
    where
        V: Visit<Self> + Spanned,
    {
        match self.eval(v)? {
            Value::Bool(b) => Ok(b),
            actual => Err(InterpretError::ExpectedBoolean {
                span: v.span(),
                actual,
            }),
        }
    }

    fn eval_number<V>(&mut self, v: &V) -> Result<f64, InterpretError>
    where
        V: Visit<Self> + Spanned,
    {
        match self.eval(v)? {
            Value::Number(n) => Ok(n),
            actual => Err(InterpretError::ExpectedNumber {
                span: v.span(),
                actual,
            }),
        }
    }
}

impl Visitor for Interpreter {
    type Output = Result<Value, InterpretError>;

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
            TokenKind::Bang => {
                Value::Bool(!self.eval(&unary.expr)?.truthiness())
            }
            TokenKind::Minus => Value::Number(-self.eval_number(&unary.expr)?),
            _ => unreachable!(),
        })
    }

    fn visit_binary_expr(&mut self, binary: &BinaryExpr) -> Self::Output {
        Ok(match binary.operator.kind {
            TokenKind::Minus => Value::Number(
                self.eval_number(&binary.left)?
                    - self.eval_number(&binary.right)?,
            ),
            TokenKind::Star => Value::Number(
                self.eval_number(&binary.left)?
                    * self.eval_number(&binary.right)?,
            ),
            TokenKind::Slash => {
                let num = self.eval_number(&binary.left)?;
                let den = self.eval_number(&binary.right)?;
                if den == 0.0 {
                    return Err(InterpretError::DivideByZero(
                        binary.right.span(),
                    ));
                }
                Value::Number(num / den)
            }
            TokenKind::Plus => {
                match (self.eval(&binary.left)?, self.eval(&binary.right)?) {
                    (Value::Number(l), Value::Number(r)) => {
                        Value::Number(l + r)
                    }
                    (Value::String(l), Value::String(r)) => {
                        Value::String(format!("{l}{r}"))
                    }
                    (Value::Number(_), actual) => {
                        return Err(InterpretError::ExpectedNumber {
                            span: binary.right.span(),
                            actual,
                        });
                    }
                    (Value::String(_), actual) => {
                        return Err(InterpretError::ExpectedString {
                            span: binary.right.span(),
                            actual,
                        });
                    }
                    (actual, _) => {
                        return Err(InterpretError::ExpectedNumberOrString {
                            span: binary.left.span(),
                            actual,
                        });
                    }
                }
            }
            TokenKind::Greater => Value::Bool(
                self.eval_number(&binary.left)?
                    > self.eval_number(&binary.right)?,
            ),
            TokenKind::GreaterEqual => Value::Bool(
                self.eval_number(&binary.left)?
                    >= self.eval_number(&binary.right)?,
            ),
            TokenKind::Less => Value::Bool(
                self.eval_number(&binary.left)?
                    < self.eval_number(&binary.right)?,
            ),
            TokenKind::LessEqual => Value::Bool(
                self.eval_number(&binary.left)?
                    <= self.eval_number(&binary.right)?,
            ),
            TokenKind::EqualEqual => Value::Bool(
                self.eval(&binary.left)? == self.eval(&binary.right)?,
            ),
            TokenKind::BangEqual => Value::Bool(
                self.eval(&binary.left)? != self.eval(&binary.right)?,
            ),
            _ => unreachable!(),
        })
    }

    fn visit_grouping_expr(&mut self, grouping: &GroupingExpr) -> Self::Output {
        grouping.expr.accept(self)
    }
}
