use std::{
    env::args_os,
    fs,
    io::{self, Write as _},
    path::Path,
    process::exit,
};

use rox_compile::CompilePass;
use rox_diag::{Diagnostic, Formatter};
use rox_lex::Lexer;
use rox_parse::{ParseOutput, Parser};
use rox_vm::{Chunk, VirtualMachine};

enum Error {
    Usage,
    Compile,
    Execute,
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
            Self::Usage => 64,
            Self::Compile => 65,
            Self::Execute => 66,
            Self::Io(_) => 67,
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
        exit(e.status());
    } else {
        exit(0);
    }
}

fn cli() -> Result<(), Error> {
    let mut args = args_os();
    match args.len() {
        1 => run_repl()?,
        2 => run_file(Path::new(&args.nth(1).unwrap()))?,
        _ => {
            eprintln!("Usage: rox [script]");
            return Err(Error::Usage);
        }
    }

    Ok(())
}

fn run_file(path: &Path) -> Result<(), Error> {
    let source = fs::read_to_string(path)?;

    let chunk = compile(&source, Parser::parse)?;

    #[cfg(feature = "trace")]
    {
        println!("== {} ==", path.display());
        chunk.disassemble();
    }

    let mut vm = VirtualMachine::new(&chunk);
    if let Err(e) = vm.execute() {
        emit(&source, &e);
        return Err(Error::Execute);
    }

    Ok(())
}

fn run_repl() -> Result<(), Error> {
    loop {
        print!("> ");
        io::stdout().flush()?;

        let mut line = String::new();
        let eof = io::stdin().read_line(&mut line)? == 0;
        if eof || line.contains('\u{4}') {
            break;
        }
        line = line.trim_end().to_string();

        if let Ok(chunk) = compile(&line, Parser::parse_repl) {
            let mut vm = VirtualMachine::new(&chunk);
            if let Err(e) = vm.execute() {
                emit(&line, &e);
            }
        }

        println!();
    }

    Ok(())
}

fn compile(
    source: &str,
    parse: impl FnOnce(Parser) -> ParseOutput,
) -> Result<Chunk, Error> {
    let lex_output = Lexer::new(source).lex();

    for error in &lex_output.errors {
        emit(source, error);
    }

    let parse_output = parse(Parser::new(lex_output.tokens));

    for error in &parse_output.errors {
        emit(source, error);
    }

    let program = parse_output.ast.ok_or(Error::Compile)?;

    let compile_output = CompilePass::new(&program).compile();

    for error in &compile_output.errors {
        emit(source, error);
    }

    if !lex_output.errors.is_empty()
        || !parse_output.errors.is_empty()
        || !compile_output.errors.is_empty()
    {
        return Err(Error::Compile);
    }

    Ok(compile_output.chunk)
}

fn emit<T: Diagnostic>(source: &str, diagnostic: &T) {
    use core::fmt;

    struct Diag<'s, T> {
        pub source: &'s str,
        pub inner: &'s T,
    }

    impl<T: Diagnostic> fmt::Display for Diag<'_, T> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let mut f = Formatter::new(self.source, f);
            self.inner.fmt(&mut f)
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
