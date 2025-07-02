use core::fmt;
use std::sync::Arc;

use crate::{
    ast::decoration::Decoration, interpreter::environment::Environment,
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
            Self::Uninitialized => write!(f, "<uninitialized>"),
            Self::Nil => write!(f, "<nil>"),
            Self::Bool(b) => {
                if *b {
                    write!(f, "true")
                } else {
                    write!(f, "false")
                }
            }
            Self::Number(n) => write!(f, "{n}"),
            Self::String(s) => write!(f, "{s}"),
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

#[derive(Debug, Clone)]
pub struct Function {
    pub kind: FunctionKind,
    pub environment: Arc<Environment>,
}

impl PartialEq for Function {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
    }
}

// TODO: need interpreter context for proper function display
impl fmt::Display for Function {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            FunctionKind::Clock => write!(f, "<builtin fun clock()>")?,
            FunctionKind::Decl(decl) => write!(f, "<decl fun {decl:?}>")?,
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FunctionKind {
    Clock,
    Decl(Decoration),
}
