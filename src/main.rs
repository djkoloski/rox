//! Rox language interpreter

#![deny(unsafe_op_in_unsafe_fn)]

mod ast;
mod diagnostic;
mod interpreter;
mod parser;
mod scanner;
mod span;

use std::{
    env::args_os,
    fs,
    io::{stdin, stdout, Error, Write as _},
    path::Path,
    process::exit,
};

use crate::{
    diagnostic::{Context, Diagnostic},
    interpreter::Interpreter,
    parser::Parser,
    scanner::Scanner,
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
    Ok(run(&source))
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
        run(line.trim_end());
        println!();
    }

    Ok(())
}

pub fn run(source: &str) -> Status {
    let mut scanner = Scanner::new(source);
    let tokens = scanner.scan_tokens();

    if !scanner.errors.is_empty() {
        for error in &scanner.errors {
            emit(source, error);
        }
    }

    let mut parser = Parser::new(tokens);
    let program = parser.parse();
    if !parser.errors.is_empty() {
        for error in &parser.errors {
            emit(source, error);
        }
    }

    if !scanner.errors.is_empty()
        || !parser.errors.is_empty()
        || program.is_none()
    {
        return Status::CompilerError;
    }

    let mut interpreter = Interpreter;
    if let Err(error) = interpreter.interpret(&program.unwrap()) {
        emit(source, &error);
        Status::RuntimeError
    } else {
        Status::Ok
    }
}

fn emit<T: Diagnostic>(source: &str, diagnostic: &T) {
    use core::fmt;

    struct Diag<'s, T> {
        pub source: &'s str,
        pub inner: &'s T,
    }

    impl<T: Diagnostic> fmt::Display for Diag<'_, T> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            self.inner.fmt(&mut Context::new(self.source), f)
        }
    }

    eprintln!(
        "{}",
        Diag {
            source,
            inner: diagnostic
        }
    );
}
