//! Rox language interpreter

#![deny(unsafe_op_in_unsafe_fn)]

mod ast;
mod parser;
mod scanner;

use core::fmt;
use std::{
    env::args_os,
    fs,
    io::{stdin, Result},
    path::Path,
    process::exit,
};

use crate::{
    ast::{AstPrinter, Visit},
    parser::Parser,
    scanner::Scanner,
};

fn main() -> Result<()> {
    let mut rox = Rox::new();

    let mut args = args_os();
    match args.len() {
        1 => rox.run_prompt()?,
        2 => rox.run_file(Path::new(&args.nth(1).unwrap()))?,
        _ => {
            eprintln!("Usage: rox [script]");
            exit(64);
        }
    }

    if rox.had_error {
        exit(65);
    }

    Ok(())
}

pub struct Rox {
    pub had_error: bool,
}

impl Rox {
    pub fn new() -> Self {
        Self { had_error: false }
    }

    pub fn run_file(&mut self, path: &Path) -> Result<()> {
        let source = fs::read_to_string(path)?;
        self.run(&source);

        Ok(())
    }

    pub fn run_prompt(&mut self) -> Result<()> {
        loop {
            print!("> ");
            let mut line = String::new();
            let eof = stdin().read_line(&mut line)? == 0;
            if eof {
                break;
            }
            self.run(&line);
            self.had_error = false;
        }

        Ok(())
    }

    pub fn run(&mut self, source: &str) {
        let mut scanner = Scanner::new(source);
        let tokens = scanner.scan_tokens();

        if !scanner.errors.is_empty() {
            self.had_error = true;
            for error in scanner.errors {
                self.error(error.line, &error.kind);
            }
        }

        let mut parser = Parser::new(tokens);
        if let Some(ast) = parser.parse() {
            println!("{}", ast.accept(&AstPrinter));
        } else {
            for error in parser.errors {
                self.error(error.line, &error.kind);
            }
        }
    }

    fn error(&mut self, line: usize, message: &impl fmt::Display) {
        eprintln!("[line {line}] Error: {message}");
        self.had_error = true;
    }
}

impl Default for Rox {
    fn default() -> Self {
        Self::new()
    }
}
