mod decls;
mod error;
mod name_resolution;

use std::collections::HashSet;

pub use self::error::CompileError;
use crate::{
    ast::{
        expr::VisitExpr,
        stmt::{Program, Repl, VisitStmt as _},
    },
    compiler::{decls::Decls, name_resolution::NameResolution},
};

pub struct Compiler {
    // TODO: private
    pub name_resolution: NameResolution,
    pub decls: Decls,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            name_resolution: NameResolution::new(),
            decls: Decls::new(),
        }
    }

    pub fn compile(
        &mut self,
        program: &Program,
        globals: &mut HashSet<String>,
    ) -> Result<(), Vec<CompileError>> {
        let mut pass = self.name_resolution.pass(globals);
        for stmt in &program.stmts {
            stmt.accept(&mut pass);
        }
        pass.finish()?;

        for stmt in &program.stmts {
            stmt.accept(&mut self.decls);
        }

        Ok(())
    }

    pub fn compile_repl(
        &mut self,
        repl: &Repl,
        globals: &mut HashSet<String>,
    ) -> Result<(), Vec<CompileError>> {
        let mut pass = self.name_resolution.pass(globals);
        match repl {
            Repl::Expr(expr) => expr.accept(&mut pass),
            Repl::Stmt(stmt) => stmt.accept(&mut pass),
        }
        pass.finish()?;

        if let Repl::Stmt(stmt) = &repl {
            stmt.accept(&mut self.decls);
        }

        Ok(())
    }
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new()
    }
}
