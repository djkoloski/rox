use std::{
    env::args_os,
    fs,
    io::{self, Write as _},
    path::Path,
    process::exit,
    sync::Arc,
};

use rox::{
    ast::decoration::Decorator,
    compiler::Compiler,
    diagnostic::{Context, Diagnostic},
    interpreter::{
        environment::Environment,
        value::{Function, FunctionKind, Value},
        Interpreter,
    },
    parser::Parser,
    scanner::Scanner,
};

enum Error {
    Compile,
    Interpret,
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
            Self::Compile => 64,
            Self::Interpret => 65,
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
            return Err(Error::Compile);
        }
    }

    Ok(())
}

fn make_globals() -> Arc<Environment> {
    let environment = Environment::new();

    environment.define(
        "clock".to_string(),
        Value::Function(Function {
            kind: FunctionKind::Clock,
            environment: environment.clone(),
        }),
    );

    environment
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

    let mut decorator = Decorator::new();
    let mut parser = Parser::new(&mut decorator, tokens);
    let program = parser.parse();
    if !parser.errors.is_empty() {
        for error in &parser.errors {
            emit(&source, error);
        }
    }

    if !scanner.errors.is_empty() || !parser.errors.is_empty() {
        return Err(Error::Compile);
    }

    let program = program.unwrap();

    let environment = make_globals();
    let mut names = environment.names();

    let mut compiler = Compiler::new();

    if let Err(errors) = compiler.compile(&program, &mut names) {
        for error in errors {
            emit(&source, &error);
        }
        return Err(Error::Compile);
    }

    let mut interpreter = Interpreter::new(&compiler, environment);

    if let Err(error) = interpreter.execute(&program) {
        emit(&source, &error);
        return Err(Error::Interpret);
    }

    Ok(())
}

fn run_prompt() -> Result<(), Error> {
    let mut decorator = Decorator::new();
    let environment = make_globals();
    let mut names = environment.names();

    let mut compiler = Compiler::new();

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

        // TODO: parser is creating a fresh decorator every time, which is wrong
        let mut parser = Parser::new(&mut decorator, tokens);
        let repl = parser.parse_repl();
        if !parser.errors.is_empty() {
            for error in &parser.errors {
                emit(&line, error);
            }
        }

        if !scanner.errors.is_empty() || !parser.errors.is_empty() {
            continue;
        }

        let repl = repl.unwrap();

        if let Err(errors) = compiler.compile_repl(&repl, &mut names) {
            for error in errors {
                emit(&line, &error);
            }
            continue;
        }

        let mut interpreter = Interpreter::new(&compiler, environment.clone());

        match interpreter.repl(&repl) {
            Err(error) => emit(&line, &error),
            Ok(None) => (),
            Ok(Some(value)) => println!("{value}"),
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
