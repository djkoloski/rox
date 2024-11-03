mod error;
mod eval;

pub use self::error::InterpretError;
use crate::{
    ast::{
        expr::{Expr, VisitExpr},
        stmt::{ExprStmt, PrintStmt, Program, StmtVisitor, VisitStmt as _},
    },
    interpreter::eval::Value,
};

pub struct Interpreter;

impl Interpreter {
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

impl StmtVisitor for Interpreter {
    type Output = Result<(), InterpretError>;

    fn visit_expr_stmt(&mut self, stmt: &ExprStmt) -> Self::Output {
        self.eval(&stmt.expr)?;
        Ok(())
    }

    fn visit_print_stmt(&mut self, stmt: &PrintStmt) -> Self::Output {
        let value = self.eval(&stmt.expr)?;
        println!("{value}");
        Ok(())
    }
}
