mod error;
mod eval;

use std::collections::HashMap;

pub use self::error::InterpretError;
use crate::{
    ast::{
        expr::{
            AssignExpr, BinaryExpr, Expr, ExprVisitor, GroupingExpr,
            LiteralExpr, UnaryExpr, VariableExpr, VisitExpr,
        },
        stmt::{
            BlockStmt, DeclStmt, ExprStmt, IfStmt, PrintStmt, Program,
            StmtVisitor, VisitStmt as _,
        },
    },
    interpreter::eval::Value,
    scanner::TokenKind,
    span::Spanned,
};

struct Scope {
    values: HashMap<String, Value>,
}

impl Scope {
    fn new() -> Self {
        Self {
            values: HashMap::new(),
        }
    }
}

struct Environment {
    scopes: Vec<Scope>,
}

impl Environment {
    fn new() -> Self {
        Self {
            scopes: vec![Scope::new()],
        }
    }
}

impl Environment {
    fn define(&mut self, name: String, value: Value) {
        self.scopes.last_mut().unwrap().values.insert(name, value);
    }

    fn get(&self, name: &str) -> Option<&Value> {
        for scope in self.scopes.iter().rev() {
            if let Some(v) = scope.values.get(name) {
                return Some(v);
            }
        }
        None
    }

    fn set(&mut self, name: &str, value: Value) {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(v) = scope.values.get_mut(name) {
                *v = value;
                return;
            }
        }
        panic!("failed to locate variable while setting");
    }

    fn push(&mut self) {
        self.scopes.push(Scope::new());
    }

    fn pop(&mut self) {
        self.scopes.pop();
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
        if matches!(value, Value::Uninitialized) {
            return Err(InterpretError::UninitializedVariable(expr.ident.span));
        }

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

    fn visit_if_stmt(&mut self, stmt: &IfStmt) -> Self::Output {
        let cond = self.eval_boolean(&stmt.group.inner)?;
        if cond {
            stmt.then.accept(self)?;
        } else if let Some((_, else_)) = &stmt.else_ {
            else_.accept(self)?;
        }

        Ok(())
    }
}
