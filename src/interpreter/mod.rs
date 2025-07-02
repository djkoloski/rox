mod error;
pub mod value;

use std::{collections::HashMap, time::SystemTime};

use crate::{
    ast::{
        expr::{
            AssignExpr, BinaryExpr, BinaryOperator, CallExpr, Expr,
            ExprVisitor, GroupingExpr, Literal, LiteralExpr, UnaryExpr,
            UnaryOperator, VariableExpr, VisitExpr as _,
        },
        stmt::{
            BlockStmt, ExprStmt, FunDeclStmt, IfStmt, PrintStmt, Program,
            ReturnStmt, Stmt, StmtVisitor, VarDeclStmt, VisitStmt as _,
        },
    },
    compiler::Compiler,
    interpreter::{
        error::InterpretError,
        value::{Function, Value},
    },
    span::Spanned as _,
};

pub struct Interpreter<'a> {
    compiler: &'a Compiler,
    globals: &'a mut HashMap<String, Value>,
    locals: Vec<HashMap<String, Value>>,
}

impl<'a> Interpreter<'a> {
    pub fn new(
        interpreter: &'a Compiler,
        globals: &'a mut HashMap<String, Value>,
    ) -> Self {
        Self {
            compiler: interpreter,
            globals,
            locals: Vec::new(),
        }
    }

    pub fn execute(&mut self, program: &Program) -> Result<(), InterpretError> {
        for stmt in &program.stmts {
            self.step(stmt)?;
        }

        Ok(())
    }

    pub fn step(&mut self, stmt: &Stmt) -> Result<(), InterpretError> {
        stmt.accept(self)
    }

    pub fn eval(&mut self, expr: &Expr) -> Result<Value, InterpretError> {
        expr.accept(self)
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

    fn eval_function(
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

    fn get(&self, name: &str, depth: usize) -> Option<Value> {
        if depth == 0 {
            self.globals.get(name).cloned()
        } else {
            self.locals[depth - 1].get(name).cloned()
        }
    }

    fn set(&mut self, name: &str, depth: usize, value: Value) {
        if depth == 0 {
            *self.globals.get_mut(name).unwrap() = value;
        } else {
            *self.locals[depth - 1].get_mut(name).unwrap() = value;
        }
    }

    fn define(&mut self, name: String, value: Value) {
        if let Some(local) = self.locals.last_mut() {
            local.insert(name, value);
        } else {
            self.globals.insert(name, value);
        }
    }

    fn push(&mut self) {
        self.locals.push(HashMap::new());
    }

    fn pop(&mut self) {
        self.locals.pop();
    }
}

impl ExprVisitor for Interpreter<'_> {
    type Output = Result<Value, InterpretError>;

    fn visit_literal_expr(&mut self, literal: &LiteralExpr) -> Self::Output {
        Ok(match &literal.literal {
            Literal::Nil(_) => Value::Nil,
            Literal::True(_) => Value::Bool(true),
            Literal::False(_) => Value::Bool(false),
            Literal::Number(x) => Value::Number(x.value),
            Literal::String(s) => Value::String(s.value.clone()),
        })
    }

    fn visit_unary_expr(&mut self, unary: &UnaryExpr) -> Self::Output {
        Ok(match unary.operator {
            UnaryOperator::Bang(_) => {
                Value::Bool(!self.eval(&unary.inner)?.truthiness())
            }
            UnaryOperator::Minus(_) => {
                Value::Number(-self.eval_number(&unary.inner)?)
            }
        })
    }

    fn visit_binary_expr(&mut self, binary: &BinaryExpr) -> Self::Output {
        Ok(match binary.operator {
            BinaryOperator::And(_) => {
                let lhs = self.eval(&binary.left)?;
                if !lhs.truthiness() {
                    lhs
                } else {
                    self.eval(&binary.right)?
                }
            }
            BinaryOperator::Or(_) => {
                let lhs = self.eval(&binary.left)?;
                if lhs.truthiness() {
                    lhs
                } else {
                    self.eval(&binary.right)?
                }
            }
            BinaryOperator::Minus(_) => Value::Number(
                self.eval_number(&binary.left)?
                    - self.eval_number(&binary.right)?,
            ),
            BinaryOperator::Star(_) => Value::Number(
                self.eval_number(&binary.left)?
                    * self.eval_number(&binary.right)?,
            ),
            BinaryOperator::Slash(_) => {
                let num = self.eval_number(&binary.left)?;
                let den = self.eval_number(&binary.right)?;
                if den == 0.0 {
                    return Err(InterpretError::DivideByZero(
                        binary.right.span(),
                    ));
                }
                Value::Number(num / den)
            }
            BinaryOperator::Plus(_) => {
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
            BinaryOperator::Greater(_) => Value::Bool(
                self.eval_number(&binary.left)?
                    > self.eval_number(&binary.right)?,
            ),
            BinaryOperator::GreaterEqual(_) => Value::Bool(
                self.eval_number(&binary.left)?
                    >= self.eval_number(&binary.right)?,
            ),
            BinaryOperator::Less(_) => Value::Bool(
                self.eval_number(&binary.left)?
                    < self.eval_number(&binary.right)?,
            ),
            BinaryOperator::LessEqual(_) => Value::Bool(
                self.eval_number(&binary.left)?
                    <= self.eval_number(&binary.right)?,
            ),
            BinaryOperator::EqualEqual(_) => Value::Bool(
                self.eval(&binary.left)? == self.eval(&binary.right)?,
            ),
            BinaryOperator::BangEqual(_) => Value::Bool(
                self.eval(&binary.left)? != self.eval(&binary.right)?,
            ),
        })
    }

    fn visit_grouping_expr(&mut self, grouping: &GroupingExpr) -> Self::Output {
        grouping.inner.accept(self)
    }

    fn visit_variable_expr(&mut self, expr: &VariableExpr) -> Self::Output {
        let depth = self.compiler.name_resolution.get(expr.decoration).unwrap();
        let value = self.get(&expr.ident.value, depth).unwrap();
        if matches!(value, Value::Uninitialized) {
            return Err(InterpretError::UninitializedVariable(
                expr.ident.span(),
            ));
        }

        Ok(value.clone())
    }

    fn visit_assign_expr(&mut self, expr: &AssignExpr) -> Self::Output {
        let depth = self.compiler.name_resolution.get(expr.decoration).unwrap();
        let value = self.eval(&expr.expr)?;
        self.set(&expr.ident.value, depth, value.clone());

        Ok(value)
    }

    fn visit_call_expr(&mut self, expr: &CallExpr) -> Self::Output {
        let callee = self.eval_function(&expr.function)?;

        let arity = match callee {
            Function::Clock => 0,
            Function::Decl(id) => {
                self.compiler.decls.get_fun(id).unwrap().params.len()
            }
        };
        if arity != expr.arguments.len() {
            return Err(InterpretError::IncorrectFunctionArity {
                span: expr.span(),
                callee,
                expected: arity,
                actual: expr.arguments.len(),
            });
        }

        let mut arguments = Vec::new();
        for argument in expr.arguments.iter() {
            arguments.push(self.eval(argument)?);
        }

        match callee {
            Function::Clock => Ok(Value::Number(
                SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_secs_f64(),
            )),
            Function::Decl(id) => {
                let function = self.compiler.decls.get_fun(id).unwrap();

                let mut interpreter =
                    Interpreter::new(self.compiler, self.globals);

                interpreter.push();

                for (name, argument) in
                    function.params.iter().zip(arguments.into_iter())
                {
                    interpreter.define(name.value.clone(), argument);
                }

                for stmt in &function.body.stmts {
                    stmt.accept(&mut interpreter)?;
                }

                Ok(Value::Nil)
            }
        }
    }
}

impl StmtVisitor for Interpreter<'_> {
    type Output = Result<(), InterpretError>;

    fn visit_var_decl_stmt(&mut self, stmt: &VarDeclStmt) -> Self::Output {
        let value = if let Some(assignment) = &stmt.assignment {
            self.eval(&assignment.1)?
        } else {
            Value::Uninitialized
        };
        self.define(stmt.ident.value.clone(), value);
        Ok(())
    }

    fn visit_fun_decl_stmt(&mut self, stmt: &FunDeclStmt) -> Self::Output {
        // TODO: can't reference functions before they're defined
        self.define(
            stmt.name.value.clone(),
            Value::Function(Function::Decl(stmt.decoration)),
        );
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
        self.push();
        for stmt in &stmt.stmts {
            stmt.accept(self)?;
        }
        self.pop();

        Ok(())
    }

    fn visit_if_stmt(&mut self, stmt: &IfStmt) -> Self::Output {
        let value = self.eval(&stmt.group.inner)?;
        if value.truthiness() {
            stmt.then.accept(self)?;
        } else if let Some((_, else_)) = &stmt.else_ {
            else_.accept(self)?;
        }

        Ok(())
    }

    fn visit_return_stmt(&mut self, _stmt: &ReturnStmt) -> Self::Output {
        todo!()
    }
}
