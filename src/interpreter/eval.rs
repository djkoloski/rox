use core::fmt;

use crate::{
    ast::expr::{
        AssignExpr, BinaryExpr, Expr, ExprVisitor, GroupingExpr, LiteralExpr,
        UnaryExpr, VariableExpr, VisitExpr,
    },
    interpreter::{InterpretError, Interpreter},
    scanner::TokenKind,
    span::Spanned,
};

#[derive(Debug, Clone, PartialEq)]
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

impl Interpreter {
    #[allow(dead_code)]
    fn eval_boolean(&mut self, expr: &Expr) -> Result<bool, InterpretError> {
        match self.eval(expr)? {
            Value::Bool(b) => Ok(b),
            actual => Err(InterpretError::ExpectedBoolean {
                span: expr.span(),
                actual,
            }),
        }
    }

    fn eval_number(&mut self, expr: &Expr) -> Result<f64, InterpretError> {
        match self.eval(expr)? {
            Value::Number(n) => Ok(n),
            actual => Err(InterpretError::ExpectedNumber {
                span: expr.span(),
                actual,
            }),
        }
    }
}

impl ExprVisitor for Interpreter {
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
                Value::Bool(!self.eval(&unary.inner)?.truthiness())
            }
            TokenKind::Minus => Value::Number(-self.eval_number(&unary.inner)?),
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
        grouping.inner.accept(self)
    }

    fn visit_variable_expr(&mut self, expr: &VariableExpr) -> Self::Output {
        let TokenKind::Identifier(ident) = &expr.ident.kind else {
            unreachable!();
        };
        let Some(value) = self.environment.get(ident) else {
            return Err(InterpretError::UndefinedVariable(expr.ident.span));
        };

        Ok(value.clone())
    }

    fn visit_assign_expr(&mut self, expr: &AssignExpr) -> Self::Output {
        let TokenKind::Identifier(ident) = &expr.ident.kind else {
            unreachable!();
        };
        if self.environment.get(ident).is_none() {
            return Err(InterpretError::UndefinedVariable(expr.ident.span));
        }
        let value = self.eval(&expr.expr)?;
        self.environment.set(ident, value.clone());

        Ok(value)
    }
}
