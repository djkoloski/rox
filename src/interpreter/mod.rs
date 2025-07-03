pub mod environment;
mod error;
pub mod value;

use core::ops::ControlFlow;
use std::{sync::Arc, time::SystemTime};

use crate::{
    ast::{
        decoration::Decoration,
        expr::{
            AssignExpr, BinaryExpr, BinaryOperator, CallExpr, Expr,
            ExprVisitor, GetExpr, GroupingExpr, Literal, LiteralExpr, SetExpr,
            ThisExpr, UnaryExpr, UnaryOperator, VariableExpr, VisitExpr as _,
        },
        stmt::{
            BlockStmt, ClassDeclStmt, ExprStmt, FunDeclStmt, Function, IfStmt,
            PrintStmt, Program, Repl, ReturnStmt, StmtVisitor, VarDeclStmt,
            VisitStmt as _, WhileStmt,
        },
    },
    compiler::Compiler,
    interpreter::{
        environment::Environment,
        error::InterpretError,
        value::{Callable, CallableKind, Instance, Value},
    },
    span::Spanned as _,
};

pub struct Interpreter<'a> {
    compiler: &'a Compiler,
    environment: Arc<Environment>,
}

impl<'a> Interpreter<'a> {
    pub fn new(
        interpreter: &'a Compiler,
        environment: Arc<Environment>,
    ) -> Self {
        Self {
            compiler: interpreter,
            environment,
        }
    }

    pub fn execute(&mut self, program: &Program) -> Result<(), InterpretError> {
        for stmt in &program.stmts {
            if let Some(result) = stmt.accept(self).break_value() {
                return Err(result.unwrap_err());
            }
        }

        Ok(())
    }

    pub fn repl(
        &mut self,
        repl: &Repl,
    ) -> Result<Option<Value>, InterpretError> {
        match repl {
            Repl::Stmt(stmt) => {
                if let Some(result) = stmt.accept(self).break_value() {
                    Err(result.unwrap_err())
                } else {
                    Ok(None)
                }
            }
            Repl::Expr(expr) => expr.accept(self).map(Some),
        }
    }

    pub fn eval(&mut self, expr: &Expr) -> Result<Value, InterpretError> {
        expr.accept(self)
    }

    fn eval_or_break(
        &mut self,
        expr: &Expr,
    ) -> ControlFlow<Result<Value, InterpretError>, Value> {
        match expr.accept(self) {
            Ok(value) => ControlFlow::Continue(value),
            Err(error) => ControlFlow::Break(Err(error)),
        }
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

    fn eval_callable(
        &mut self,
        expr: &Expr,
    ) -> Result<Callable, InterpretError> {
        match self.eval(expr)? {
            Value::Callable(f) => Ok(f),
            actual => Err(InterpretError::ExpectedCallable {
                span: expr.span(),
                actual,
            }),
        }
    }

    fn push(&mut self) {
        self.environment = Environment::with_parent(self.environment.clone());
    }

    fn pop(&mut self) {
        self.environment = self.environment.parent().unwrap().clone();
    }

    fn find_initializer(&mut self, class: Decoration) -> Option<&'a Function> {
        self.compiler
            .decls
            .get_class(class)
            .unwrap()
            .methods
            .iter()
            .find(|m| m.name.value == "init")
    }

    fn call_function(
        &mut self,
        function: &Function,
        environment: Arc<Environment>,
        arguments: Vec<Value>,
    ) -> Result<Value, InterpretError> {
        let mut interpreter = Interpreter::new(self.compiler, environment);

        interpreter.push();

        for (name, argument) in
            function.params.iter().zip(arguments.into_iter())
        {
            interpreter.environment.define(name.value.clone(), argument);
        }

        for stmt in &function.body.stmts {
            if let Some(break_value) =
                stmt.accept(&mut interpreter).break_value()
            {
                return break_value;
            }
        }

        Ok(Value::Nil)
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
        let value = self.environment.get(&expr.ident.value, depth).unwrap();
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
        self.environment
            .set(&expr.ident.value, depth, value.clone());

        Ok(value)
    }

    fn visit_call_expr(&mut self, expr: &CallExpr) -> Self::Output {
        let callee = self.eval_callable(&expr.function)?;

        let arity = match callee.kind {
            CallableKind::Clock => 0,
            CallableKind::Function(decoration)
            | CallableKind::Method { decoration, .. } => self
                .compiler
                .decls
                .get_fun(decoration)
                .unwrap()
                .params
                .len(),
            CallableKind::Class(decoration) => self
                .find_initializer(decoration)
                .map(|f| f.params.len())
                .unwrap_or(0),
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

        match callee.kind {
            CallableKind::Clock => Ok(Value::Number(
                SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_secs_f64(),
            )),
            CallableKind::Function(decoration) => {
                let function = self.compiler.decls.get_fun(decoration).unwrap();
                self.call_function(
                    function,
                    callee.environment.clone(),
                    arguments,
                )
            }
            CallableKind::Method {
                decoration,
                is_initializer,
            } => {
                let function = self.compiler.decls.get_fun(decoration).unwrap();
                let result = self.call_function(
                    function,
                    callee.environment.clone(),
                    arguments,
                )?;
                if is_initializer {
                    Ok(callee.environment.get("this", 0).unwrap())
                } else {
                    Ok(result)
                }
            }
            CallableKind::Class(decoration) => {
                let instance = Value::Instance(Instance::new(
                    decoration,
                    callee.environment.clone(),
                ));
                if let Some(initializer) = self.find_initializer(decoration) {
                    let environment =
                        Environment::with_parent(callee.environment.clone());
                    environment.define("this".to_string(), instance.clone());
                    self.call_function(initializer, environment, arguments)?;
                }
                Ok(instance)
            }
        }
    }

    fn visit_get_expr(&mut self, expr: &GetExpr) -> Self::Output {
        match self.eval(&expr.instance)? {
            Value::Instance(instance) => {
                if let Some(field) = instance.get(&expr.name.value) {
                    return Ok(field.clone());
                }

                let class =
                    self.compiler.decls.get_class(instance.class()).unwrap();

                let Some(method) = class
                    .methods
                    .iter()
                    .find(|m| m.name.value == expr.name.value)
                else {
                    return Err(InterpretError::UndefinedProperty {
                        span: expr.name.span(),
                        actual: Value::Instance(instance),
                    });
                };

                let environment =
                    Environment::with_parent(instance.environment().clone());
                environment.define(
                    "this".to_string(),
                    Value::Instance(instance.clone()),
                );

                Ok(Value::Callable(Callable {
                    kind: CallableKind::Method {
                        decoration: method.decoration,
                        is_initializer: expr.name.value == "init",
                    },
                    environment,
                }))
            }
            actual => Err(InterpretError::ExpectedInstance {
                span: expr.instance.span(),
                actual,
            }),
        }
    }

    fn visit_set_expr(&mut self, expr: &SetExpr) -> Self::Output {
        match self.eval(&expr.instance)? {
            Value::Instance(instance) => {
                let value = self.eval(&expr.expr)?;
                instance.set(expr.name.value.clone(), value.clone());
                Ok(value)
            }
            actual => Err(InterpretError::ExpectedInstance {
                span: expr.instance.span(),
                actual,
            }),
        }
    }

    fn visit_this_expr(&mut self, expr: &ThisExpr) -> Self::Output {
        let depth = self.compiler.name_resolution.get(expr.decoration).unwrap();
        let value = self.environment.get("this", depth).unwrap();
        if matches!(value, Value::Uninitialized) {
            return Err(InterpretError::UninitializedVariable(expr.span()));
        }

        Ok(value.clone())
    }
}

impl StmtVisitor for Interpreter<'_> {
    type Output = ControlFlow<Result<Value, InterpretError>>;

    fn visit_var_decl_stmt(&mut self, stmt: &VarDeclStmt) -> Self::Output {
        let value = if let Some(assignment) = &stmt.assignment {
            self.eval_or_break(&assignment.1)?
        } else {
            Value::Uninitialized
        };
        self.environment.define(stmt.ident.value.clone(), value);

        ControlFlow::Continue(())
    }

    fn visit_fun_decl_stmt(&mut self, stmt: &FunDeclStmt) -> Self::Output {
        self.environment.define(
            stmt.function.name.value.clone(),
            Value::Callable(Callable {
                kind: CallableKind::Function(stmt.function.decoration),
                environment: self.environment.clone(),
            }),
        );

        ControlFlow::Continue(())
    }

    fn visit_class_decl_stmt(&mut self, stmt: &ClassDeclStmt) -> Self::Output {
        self.environment.define(
            stmt.name.value.clone(),
            Value::Callable(Callable {
                kind: CallableKind::Class(stmt.decoration),
                environment: self.environment.clone(),
            }),
        );

        ControlFlow::Continue(())
    }

    fn visit_expr_stmt(&mut self, stmt: &ExprStmt) -> Self::Output {
        self.eval_or_break(&stmt.expr)?;

        ControlFlow::Continue(())
    }

    fn visit_print_stmt(&mut self, stmt: &PrintStmt) -> Self::Output {
        let value = self.eval_or_break(&stmt.expr)?;
        println!("{value}");

        ControlFlow::Continue(())
    }

    fn visit_block_stmt(&mut self, stmt: &BlockStmt) -> Self::Output {
        self.push();
        for stmt in &stmt.stmts {
            stmt.accept(self)?;
        }
        self.pop();

        ControlFlow::Continue(())
    }

    fn visit_if_stmt(&mut self, stmt: &IfStmt) -> Self::Output {
        let value = self.eval_or_break(&stmt.group.inner)?;
        if value.truthiness() {
            stmt.then.accept(self)?;
        } else if let Some((_, else_)) = &stmt.else_ {
            else_.accept(self)?;
        }

        ControlFlow::Continue(())
    }

    fn visit_while_stmt(&mut self, stmt: &WhileStmt) -> Self::Output {
        while self.eval_or_break(&stmt.expr)?.truthiness() {
            stmt.body.accept(self)?;
        }

        ControlFlow::Continue(())
    }

    fn visit_return_stmt(&mut self, stmt: &ReturnStmt) -> Self::Output {
        let mut result = Ok(Value::Nil);
        if let Some(expr) = &stmt.expr {
            result = Ok(self.eval_or_break(expr)?);
        }
        ControlFlow::Break(result)
    }
}
