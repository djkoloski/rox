use core::{fmt, num::ParseFloatError};

use crate::{
    diagnostic::{Context, Diagnostic},
    span::Span,
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
            let Some(c) = self.advance() else { break };

            if let Some(token) = self.scan_token(c) {
                result.push(token);
            }
        }

        result
    }

    fn scan_token(&mut self, c: u8) -> Option<Token> {
        Some(match c {
            b'(' => self.token(TokenKind::LeftParen),
            b')' => self.token(TokenKind::RightParen),
            b'{' => self.token(TokenKind::LeftBrace),
            b'}' => self.token(TokenKind::RightBrace),
            b',' => self.token(TokenKind::Comma),
            b'.' => self.token(TokenKind::Dot),
            b'-' => self.token(TokenKind::Minus),
            b'+' => self.token(TokenKind::Plus),
            b';' => self.token(TokenKind::Semicolon),
            b'*' => self.token(TokenKind::Star),
            b'!' => {
                if self.expect(b'=') {
                    self.token(TokenKind::BangEqual)
                } else {
                    self.token(TokenKind::Bang)
                }
            }
            b'=' => {
                if self.expect(b'=') {
                    self.token(TokenKind::EqualEqual)
                } else {
                    self.token(TokenKind::Equal)
                }
            }
            b'<' => {
                if self.expect(b'=') {
                    self.token(TokenKind::LessEqual)
                } else {
                    self.token(TokenKind::Less)
                }
            }
            b'>' => {
                if self.expect(b'=') {
                    self.token(TokenKind::GreaterEqual)
                } else {
                    self.token(TokenKind::Greater)
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
                _ => self.token(TokenKind::Slash),
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
        while let Some(next) = self.peek() {
            if next != b'\n' {
                self.advance();
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
                return Some(self.token(TokenKind::String(value)));
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
            Ok(value) => Some(self.token(TokenKind::Number(value))),
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

        self.token(match &self.source[self.start..self.current] {
            "and" => TokenKind::And,
            "class" => TokenKind::Class,
            "else" => TokenKind::Else,
            "false" => TokenKind::False,
            "for" => TokenKind::For,
            "fun" => TokenKind::Fun,
            "if" => TokenKind::If,
            "nil" => TokenKind::Nil,
            "or" => TokenKind::Or,
            "print" => TokenKind::Print,
            "return" => TokenKind::Return,
            "super" => TokenKind::Super,
            "this" => TokenKind::This,
            "true" => TokenKind::True,
            "var" => TokenKind::Var,
            "while" => TokenKind::While,
            _ => TokenKind::Identifier,
        })
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

    fn token(&mut self, kind: TokenKind) -> Token {
        Token {
            kind,
            span: self.span(),
        }
    }

    fn error(&mut self, e: ScanError) {
        self.errors.push(e);
    }
}

#[derive(Debug)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Debug)]
pub enum TokenKind {
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
    Identifier,
    String(String),
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
}
