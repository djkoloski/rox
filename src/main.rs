use std::{
    env::args_os,
    fs,
    io::{self, Write as _},
    path::Path,
    process::exit,
};

use rox::{
    diagnostic::{Context, Diagnostic},
    interpreter::Interpreter,
    parser::Parser,
    scanner::Scanner,
};

enum Error {
    Compiler,
    Runtime,
    Io(io::Error),
}

impl Error {
    fn emit(&self) {
        if let Self::Io(e) = self {
            eprintln!("IO error: {e}");
        }
    }

    fn status(&self) -> i32 {
        match self {
            Self::Compiler => 64,
            Self::Runtime => 65,
            Self::Io(_) => 66,
        }
    }
}

impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

fn main() {
    if let Err(e) = cli() {
        e.emit();
        exit(e.status())
    } else {
        exit(0);
    }
}

fn cli() -> Result<(), Error> {
    let mut args = args_os();
    match args.len() {
        1 => run_prompt()?,
        2 => run_file(Path::new(&args.nth(1).unwrap()))?,
        _ => {
            eprintln!("Usage: rox [script]");
            return Err(Error::Compiler);
        }
    }

    Ok(())
}

fn run_file(path: &Path) -> Result<(), Error> {
    let source = fs::read_to_string(path)?;

    let mut scanner = Scanner::new(&source);
    let tokens = scanner.scan_tokens();

    if !scanner.errors.is_empty() {
        for error in &scanner.errors {
            emit(&source, error);
        }
    }

    let mut parser = Parser::new(tokens);
    let program = parser.parse();
    if !parser.errors.is_empty() {
        for error in &parser.errors {
            emit(&source, error);
        }
    }

    if !scanner.errors.is_empty()
        || !parser.errors.is_empty()
        || program.is_none()
    {
        return Err(Error::Compiler);
    }

    let mut interpreter = Interpreter::new();
    if let Err(errors) = interpreter.interpret(&program.unwrap()) {
        for error in errors {
            emit(&source, &error);
        }
        return Err(Error::Runtime);
    }

    Ok(())
}

fn run_prompt() -> Result<(), Error> {
    let mut interpreter = Interpreter::new();

    loop {
        print!("> ");
        io::stdout().flush()?;

        let mut line = String::new();
        let eof = io::stdin().read_line(&mut line)? == 0;
        if eof || line.contains('\u{4}') {
            break;
        }
        line = line.trim_end().to_string();

        let mut scanner = Scanner::new(&line);
        let tokens = scanner.scan_tokens();

        if !scanner.errors.is_empty() {
            for error in &scanner.errors {
                emit(&line, error);
            }
        }

        let mut parser = Parser::new(tokens);
        let repl = parser.parse_repl();
        if !parser.errors.is_empty() {
            for error in &parser.errors {
                emit(&line, error);
            }
        }

        if !scanner.errors.is_empty()
            || !parser.errors.is_empty()
            || repl.is_none()
        {
            continue;
        }

        match interpreter.repl(&repl.unwrap()) {
            Ok(None) => (),
            Ok(Some(value)) => println!("{value}"),
            Err(errors) => {
                for error in errors {
                    emit(&line, &error);
                }
            }
        }

        println!();
    }

    Ok(())
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
