use core::fmt;

use crate::ast::decoration::Decoration;

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

#[derive(Debug, Clone, PartialEq)]
pub enum Function {
    Clock,
    Decl(Decoration),
}

// TODO: need interpreter context for proper function display
impl fmt::Display for Function {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Clock => write!(f, "<builtin fun clock()>")?,
            Self::Decl(decl) => write!(f, "<decl fun {decl:?}>")?,
        }

        Ok(())
    }
}
