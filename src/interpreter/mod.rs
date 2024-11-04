mod error;
mod eval;

use std::collections::HashMap;

pub use self::error::InterpretError;
use crate::{
    ast::{
        expr::{Expr, VisitExpr},
        stmt::{
            BlockStmt, DeclStmt, ExprStmt, PrintStmt, Program, StmtVisitor,
            VisitStmt as _,
        },
    },
    interpreter::eval::Value,
    scanner::TokenKind,
};

struct Environment {
    enclosing: Option<Box<Environment>>,
    values: HashMap<String, Value>,
}

impl Environment {
    fn new() -> Self {
        Self {
            enclosing: None,
            values: HashMap::new(),
        }
    }
}

impl Environment {
    fn define(&mut self, name: String, value: Value) {
        self.values.insert(name, value);
    }

    fn get(&self, name: &str) -> Option<&Value> {
        self.values
            .get(name)
            .or_else(|| self.enclosing.as_ref()?.get(name))
    }

    fn set(&mut self, name: &str, value: Value) {
        *self
            .values
            .get_mut(name)
            .or_else(|| self.enclosing.as_mut()?.values.get_mut(name))
            .unwrap() = value;
    }

    fn push(&mut self) {
        use core::mem::replace;

        let parent = replace(self, Environment::new());
        self.enclosing = Some(Box::new(parent));
    }

    fn pop(&mut self) {
        let parent = self.enclosing.take();
        *self = *parent.unwrap();
    }
}

pub struct Interpreter {
    environment: Environment,
}

impl Interpreter {
    pub fn new() -> Self {
        Self {
            environment: Environment::new(),
        }
    }

    pub fn interpret(
        &mut self,
        program: &Program,
    ) -> Result<(), InterpretError> {
        for stmt in &program.stmts {
            stmt.accept(self)?;
        }

        Ok(())
    }

    pub fn eval(&mut self, expr: &Expr) -> Result<Value, InterpretError> {
        expr.accept(self)
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

impl StmtVisitor for Interpreter {
    type Output = Result<(), InterpretError>;

    fn visit_decl_stmt(&mut self, stmt: &DeclStmt) -> Self::Output {
        let value = if let Some(assignment) = &stmt.assignment {
            self.eval(&assignment.1)?
        } else {
            Value::Uninitialized
        };
        let TokenKind::Identifier(ident) = &stmt.ident.kind else {
            unreachable!()
        };
        self.environment.define(ident.clone(), value);
        Ok(())
    }

    fn visit_expr_stmt(&mut self, stmt: &ExprStmt) -> Self::Output {
        self.eval(&stmt.expr)?;
        Ok(())
    }

    fn visit_print_stmt(&mut self, stmt: &PrintStmt) -> Self::Output {
        let value = self.eval(&stmt.expr)?;
        println!("{value}");
        Ok(())
    }

    fn visit_block_stmt(&mut self, stmt: &BlockStmt) -> Self::Output {
        let mut result = Ok(());
        self.environment.push();

        for stmt in &stmt.stmts {
            if let Err(e) = stmt.accept(self) {
                result = Err(e);
                break;
            }
        }

        self.environment.pop();
        result
    }
}
