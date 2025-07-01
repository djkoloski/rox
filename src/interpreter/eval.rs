use core::fmt;

use crate::{
    ast::expr::Expr,
    interpreter::{InterpretError, Interpreter},
    span::Spanned,
};

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Uninitialized,
    Nil,
    Bool(bool),
    Number(f64),
    String(String),
    Function(Function),
    // Object(???),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Uninitialized => write!(f, "uninitialized"),
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
            Self::Function(n) => write!(f, "{n}"),
        }
    }
}

impl Value {
    pub fn truthiness(&self) -> bool {
        match self {
            Self::Uninitialized | Self::Nil => false,
            Self::Bool(b) => *b,
            Self::Number(_) | Self::String(_) | Self::Function(_) => true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Function {
    Clock,
}

impl fmt::Display for Function {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<function>")
    }
}

impl Interpreter {
    #[allow(dead_code)]
    pub fn eval_boolean(
        &mut self,
        expr: &Expr,
    ) -> Result<bool, InterpretError> {
        match self.eval(expr)? {
            Value::Bool(b) => Ok(b),
            actual => Err(InterpretError::ExpectedBoolean {
                span: expr.span(),
                actual,
            }),
        }
    }

    pub fn eval_number(&mut self, expr: &Expr) -> Result<f64, InterpretError> {
        match self.eval(expr)? {
            Value::Number(n) => Ok(n),
            actual => Err(InterpretError::ExpectedNumber {
                span: expr.span(),
                actual,
            }),
        }
    }

    pub fn eval_function(
        &mut self,
        expr: &Expr,
    ) -> Result<Function, InterpretError> {
        match self.eval(expr)? {
            Value::Function(f) => Ok(f),
            actual => Err(InterpretError::ExpectedFunction {
                span: expr.span(),
                actual,
            }),
        }
    }
}
