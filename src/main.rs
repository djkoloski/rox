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
    let mut args = args_os();
    let status = match args.len() {
        1 => {
            run_prompt()?;
            Status::Ok
        }
        2 => run_file(Path::new(&args.nth(1).unwrap()))?,
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

pub fn run_file(path: &Path) -> Result<Status, Error> {
    let source = fs::read_to_string(path)?;
    let mut rox = Rox::new(&source);
    Ok(rox.run())
}

pub fn run_prompt() -> Result<(), Error> {
    loop {
        print!("> ");
        stdout().flush()?;

        let mut line = String::new();
        let eof = stdin().read_line(&mut line)? == 0;
        if eof || line.contains('\u{4}') {
            break;
        }
        let mut rox = Rox::new(&line);
        rox.run();
    }

    Ok(())
}

pub struct Rox<'t> {
    source: &'t str,
}

impl<'t> Rox<'t> {
    pub fn new(source: &'t str) -> Self {
        Self {
            source,
        }
    }

    pub fn run(&mut self) -> Status {
        let mut status = Status::Ok;

        let mut scanner = Scanner::new(self.source);
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
        println!("error: {}", spanned.inner);
        spanned.span.indicate(self.source);
    }
}
