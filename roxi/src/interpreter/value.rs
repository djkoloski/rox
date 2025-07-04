use core::fmt;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

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
    Callable(Callable),
    Instance(Instance),
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
            Self::Callable(n) => write!(f, "{n}"),
            Self::Instance(i) => write!(f, "{i}"),
        }
    }
}

impl Value {
    pub fn truthiness(&self) -> bool {
        match self {
            Self::Uninitialized | Self::Nil => false,
            Self::Bool(b) => *b,
            Self::Number(_)
            | Self::String(_)
            | Self::Callable(_)
            | Self::Instance(_) => true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Callable {
    pub kind: CallableKind,
    pub environment: Arc<Environment>,
}

impl PartialEq for Callable {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
    }
}

impl fmt::Display for Callable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            CallableKind::Clock => write!(f, "<builtin clock()>")?,
            CallableKind::Function(decoration) => {
                write!(f, "<fun {decoration:?}>")?
            }
            CallableKind::Class(decoration) => {
                write!(f, "<class {decoration:?}>")?
            }
            CallableKind::Method { decoration, .. } => {
                write!(f, "<method {decoration:?}>")?
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CallableKind {
    Clock,
    Function(Decoration),
    Class(Decoration),
    Method {
        decoration: Decoration,
        is_initializer: bool,
    },
}

#[derive(Debug, Clone)]
pub struct Instance {
    class: Decoration,
    environment: Arc<Environment>,
    fields: Arc<Mutex<HashMap<String, Value>>>,
}

impl Instance {
    pub fn new(class: Decoration, environment: Arc<Environment>) -> Self {
        Self {
            class,
            environment,
            fields: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn class(&self) -> Decoration {
        self.class
    }

    pub fn environment(&self) -> &Arc<Environment> {
        &self.environment
    }

    pub fn get(&self, name: &str) -> Option<Value> {
        self.fields.lock().unwrap().get(name).cloned()
    }

    pub fn set(&self, name: String, value: Value) {
        self.fields.lock().unwrap().insert(name, value);
    }
}

impl PartialEq for Instance {
    fn eq(&self, other: &Self) -> bool {
        self.class == other.class && Arc::ptr_eq(&self.fields, &other.fields)
    }
}

impl fmt::Display for Instance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<instance {:?}>", self.class)
    }
}
