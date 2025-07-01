use core::{fmt, num::ParseFloatError};

use crate::{
    diagnostic::{Context, Diagnostic},
    span::{Span, Spanned},
};

#[derive(Debug)]
pub enum ScanError {
    UnexpectedCharacter { span: Span, char: u8 },
    UnterminatedBlockComment(Span),
    UnterminatedString(Span),
    InvalidNumber { span: Span, error: ParseFloatError },
}

impl Diagnostic for ScanError {
    fn fmt(
        &self,
        c: &mut Context<'_>,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            Self::UnexpectedCharacter { span, char } => {
                c.error(f, format_args!("unexpected character"))?;
                c.span(
                    *span,
                    f,
                    format_args!(
                        "'{}' (0x{char:x}) is not valid syntax",
                        *char as char
                    ),
                )?;
            }
            Self::UnterminatedBlockComment(span) => {
                c.error(f, format_args!("unterminated block comment"))?;
                c.span(
                    *span,
                    f,
                    format_args!(
                        "this block comment is missing a closing tag (*/)"
                    ),
                )?;
            }
            Self::UnterminatedString(span) => {
                c.error(f, format_args!("unterminated string"))?;
                c.span(
                    *span,
                    f,
                    format_args!("this string is missing a closing quote (\")"),
                )?;
            }
            Self::InvalidNumber { span, error } => {
                c.error(f, format_args!("invalid number"))?;
                c.span(
                    *span,
                    f,
                    format_args!(
                        "failed to parse '{}' as a number: {error}",
                        span.get(c.source())
                    ),
                )?;
            }
        }
        Ok(())
    }
}

pub struct Scanner<'a> {
    source: &'a str,
    start: usize,
    current: usize,
    pub errors: Vec<ScanError>,
}

impl<'a> Scanner<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            start: 0,
            current: 0,
            errors: Vec::new(),
        }
    }

    pub fn scan_tokens(&mut self) -> Vec<Token> {
        let mut result = Vec::new();

        loop {
            self.start = self.current;
            let Some(c) = self.advance() else {
                break;
            };

            if let Some(token) = self.scan_token(c) {
                result.push(token);
            }
        }

        result.push(Eof { span: self.span() }.into());

        result
    }

    fn scan_token(&mut self, c: u8) -> Option<Token> {
        Some(match c {
            b'(' => LeftParen { span: self.span() }.into(),
            b')' => RightParen { span: self.span() }.into(),
            b'{' => LeftBrace { span: self.span() }.into(),
            b'}' => RightBrace { span: self.span() }.into(),
            b',' => Comma { span: self.span() }.into(),
            b'.' => Dot { span: self.span() }.into(),
            b'-' => Minus { span: self.span() }.into(),
            b'+' => Plus { span: self.span() }.into(),
            b';' => Semicolon { span: self.span() }.into(),
            b'*' => Star { span: self.span() }.into(),
            b'!' => {
                if self.expect(b'=') {
                    BangEqual { span: self.span() }.into()
                } else {
                    Bang { span: self.span() }.into()
                }
            }
            b'=' => {
                if self.expect(b'=') {
                    EqualEqual { span: self.span() }.into()
                } else {
                    Equal { span: self.span() }.into()
                }
            }
            b'<' => {
                if self.expect(b'=') {
                    LessEqual { span: self.span() }.into()
                } else {
                    Less { span: self.span() }.into()
                }
            }
            b'>' => {
                if self.expect(b'=') {
                    GreaterEqual { span: self.span() }.into()
                } else {
                    Greater { span: self.span() }.into()
                }
            }
            b'/' => match self.peek()? {
                b'/' => {
                    self.line_comment();
                    return None;
                }
                b'*' => {
                    self.block_comment();
                    return None;
                }
                _ => Slash { span: self.span() }.into(),
            },
            b'"' => self.string()?,
            b'0'..=b'9' => self.number()?,
            b'_' | b'a'..=b'z' | b'A'..=b'Z' => self.identifier(),
            // Whitespace
            b' ' | b'\r' | b'\t' | b'\n' => return None,
            char => {
                self.error(ScanError::UnexpectedCharacter {
                    span: self.span(),
                    char,
                });
                return None;
            }
        })
    }

    fn line_comment(&mut self) {
        self.advance();
        while let Some(next) = self.advance() {
            if next == b'\n' {
                break;
            }
        }
    }

    fn block_comment(&mut self) {
        let mut depth = 1;

        self.advance();
        while let Some(c) = self.advance() {
            match c {
                b'/' => {
                    if self.peek() == Some(b'*') {
                        self.advance();
                        depth += 1;
                    }
                }
                b'*' => {
                    if self.peek() == Some(b'/') {
                        self.advance();
                        depth -= 1;
                        if depth == 0 {
                            return;
                        }
                    }
                }
                _ => (),
            }
        }

        self.error(ScanError::UnterminatedBlockComment(self.span()));
    }

    fn string(&mut self) -> Option<Token> {
        while let Some(c) = self.advance() {
            if c == b'"' {
                let value =
                    self.source[self.start + 1..self.current - 1].to_string();
                return Some(
                    String {
                        span: self.span(),
                        value,
                    }
                    .into(),
                );
            }
        }

        self.error(ScanError::UnterminatedString(self.span()));
        None
    }

    fn number(&mut self) -> Option<Token> {
        while self.peek().is_some_and(|c| c.is_ascii_digit()) {
            self.advance();
        }

        if self.peek() == Some(b'.')
            && self.peek_next().is_some_and(|c| c.is_ascii_digit())
        {
            self.advance();

            while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                self.advance();
            }
        }

        match self.source[self.start..self.current].parse::<f64>() {
            Ok(value) => Some(
                Number {
                    span: self.span(),
                    value,
                }
                .into(),
            ),
            Err(error) => {
                println!(
                    "attempted to parse {}..{} (`{}`) as a float",
                    self.start,
                    self.current,
                    &self.source[self.start..self.current]
                );
                self.error(ScanError::InvalidNumber {
                    span: self.span(),
                    error,
                });
                None
            }
        }
    }

    fn identifier(&mut self) -> Token {
        while self.peek().is_some_and(
            |c| matches!(c, b'_' | b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9'),
        ) {
            self.advance();
        }

        match &self.source[self.start..self.current] {
            "and" => And { span: self.span() }.into(),
            "class" => Class { span: self.span() }.into(),
            "else" => Else { span: self.span() }.into(),
            "false" => False { span: self.span() }.into(),
            "for" => For { span: self.span() }.into(),
            "fun" => Fun { span: self.span() }.into(),
            "if" => If { span: self.span() }.into(),
            "nil" => Nil { span: self.span() }.into(),
            "or" => Or { span: self.span() }.into(),
            "print" => Print { span: self.span() }.into(),
            "return" => Return { span: self.span() }.into(),
            "super" => Super { span: self.span() }.into(),
            "this" => This { span: self.span() }.into(),
            "true" => True { span: self.span() }.into(),
            "var" => Var { span: self.span() }.into(),
            "while" => While { span: self.span() }.into(),
            ident => Identifier {
                span: self.span(),
                value: ident.to_string(),
            }
            .into(),
        }
    }

    fn peek(&self) -> Option<u8> {
        self.source.as_bytes().get(self.current).cloned()
    }

    fn peek_next(&self) -> Option<u8> {
        self.source.as_bytes().get(self.current + 1).cloned()
    }

    fn advance(&mut self) -> Option<u8> {
        let c = self.peek()?;
        self.current += 1;
        Some(c)
    }

    fn expect(&mut self, next: u8) -> bool {
        let c = self.peek();
        if c == Some(next) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn span(&self) -> Span {
        Span::scan(self.start, self.current)
    }

    fn error(&mut self, e: ScanError) {
        self.errors.push(e);
    }
}

pub trait TokenKind {
    fn matches_token(token: &Token) -> bool;
    fn from_token(token: Token) -> Self;
}

macro_rules! token {
    (
        pub enum $name:ident {
            $($variant:ident $(($value:ty))?),* $(,)?
        }
    ) => {
        $(
            #[derive(Debug)]
            pub struct $variant {
                pub span: Span,
                $(pub value: $value,)*
            }

            impl Spanned for $variant {
                fn span_start(&self) -> usize {
                    self.span.start()
                }

                fn span_end(&self) -> usize {
                    self.span.end()
                }
            }

            impl $crate::scanner::TokenKind for $variant {
                fn matches_token(token: &$crate::scanner::Token) -> bool {
                    ::core::matches!(token, $crate::scanner::Token::$variant(_))
                }

                fn from_token(token: $crate::scanner::Token) -> Self {
                    let $crate::scanner::Token::$variant(this) = token else {
                        ::core::unreachable!()
                    };
                    this
                }
            }
        )*

        #[derive(Debug)]
        pub enum $name {
            $($variant($variant),)*
        }

        $(
            impl From<$variant> for $name {
                fn from(token: $variant) -> Self {
                    Self::$variant(token)
                }
            }
        )*

        impl Spanned for $name {
            fn span_start(&self) -> usize {
                match self {
                    $(Self::$variant(token) => token.span_start(),)*
                }
            }

            fn span_end(&self) -> usize {
                match self {
                    $(Self::$variant(token) => token.span_end(),)*
                }
            }
        }
    }
}

token! {
    pub enum Token {
        LeftParen,
        RightParen,
        LeftBrace,
        RightBrace,
        Comma,
        Dot,
        Minus,
        Plus,
        Semicolon,
        Slash,
        Star,
        Bang,
        BangEqual,
        Equal,
        EqualEqual,
        Greater,
        GreaterEqual,
        Less,
        LessEqual,
        Identifier(std::string::String),
        String(std::string::String),
        Number(f64),
        And,
        Class,
        Else,
        False,
        Fun,
        For,
        If,
        Nil,
        Or,
        Print,
        Return,
        Super,
        This,
        True,
        Var,
        While,
        Eof,
    }
}
