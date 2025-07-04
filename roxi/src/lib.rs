//! Rox language interpreter

#![deny(unsafe_op_in_unsafe_fn)]

pub mod ast;
pub mod compiler;
pub mod diagnostic;
pub mod interpreter;
pub mod parser;
pub mod scanner;
pub mod span;
