//! Rox language interpreter

#![deny(unsafe_op_in_unsafe_fn)]

mod ast;
mod interpreter;
mod parser;
mod scanner;
mod span;

use core::fmt;
use std::{
    env::args_os,
    fs,
    io::{stdin, stdout, Error, Write as _},
    path::Path,
    process::exit,
};

use crate::{
    ast::Visit, interpreter::Interpreter, parser::Parser, scanner::Scanner,
    span::Spanned,
};

fn main() -> Result<(), Error> {
    let mut rox = Rox::new();

    let mut args = args_os();
    let status = match args.len() {
        1 => {
            rox.run_prompt()?;
            Status::Ok
        }
        2 => rox.run_file(Path::new(&args.nth(1).unwrap()))?,
        _ => {
            eprintln!("Usage: rox [script]");
            Status::CompilerError
        }
    };

    exit(status as i32);
}

pub enum Status {
    Ok = 0,
    CompilerError = 64,
    RuntimeError = 65,
}

pub struct Rox {}

impl Rox {
    pub fn new() -> Self {
        Self {}
    }

    pub fn run_file(&mut self, path: &Path) -> Result<Status, Error> {
        let source = fs::read_to_string(path)?;
        Ok(self.run(&source))
    }

    pub fn run_prompt(&mut self) -> Result<(), Error> {
        loop {
            print!("> ");
            stdout().flush()?;

            let mut line = String::new();
            let eof = stdin().read_line(&mut line)? == 0;
            if eof || line.contains('\u{4}') {
                break;
            }
            self.run(&line);
        }

        Ok(())
    }

    pub fn run(&mut self, source: &str) -> Status {
        let mut status = Status::Ok;

        let mut scanner = Scanner::new(source);
        let tokens = scanner.scan_tokens();

        if !scanner.errors.is_empty() {
            status = Status::CompilerError;
            for error in &scanner.errors {
                self.error(error);
            }
        }

        let mut parser = Parser::new(tokens);
        let ast = parser.parse();
        if !parser.errors.is_empty() {
            status = Status::CompilerError;
            for error in &parser.errors {
                self.error(error);
            }
        }
        let Some(ast) = ast else {
            eprintln!("Failed to parse code");
            return Status::CompilerError;
        };

        if !matches!(status, Status::Ok) {
            eprintln!("Aborting due to previous errors");
            return status;
        }

        match ast.accept(&mut Interpreter) {
            Ok(value) => println!("{value}"),
            Err(error) => {
                status = Status::RuntimeError;
                self.error(&error);
            }
        }

        status
    }

    fn error<E: fmt::Display>(&mut self, spanned: &Spanned<E>) {
        eprintln!("[{}] Error: {}", spanned.span, spanned.inner);
    }
}

impl Default for Rox {
    fn default() -> Self {
        Self::new()
    }
}
