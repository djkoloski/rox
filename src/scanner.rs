use core::{fmt, num::ParseFloatError};

use crate::span::{Span, Spanned};

#[derive(Debug)]
pub enum ScanError {
    UnexpectedCharacter(u8),
    UnterminatedBlockComment,
    UnterminatedString,
    InvalidNumber(ParseFloatError),
}

impl fmt::Display for ScanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedCharacter(c) => {
                write!(f, "unexpected character: {}", *c as char)
            }
            Self::UnterminatedBlockComment => {
                write!(f, "unterminated block comment")
            }
            Self::UnterminatedString => write!(f, "unterminated string"),
            Self::InvalidNumber(e) => write!(f, "invalid number: {e}"),
        }
    }
}

pub struct Scanner<'a> {
    source: &'a str,
    start: usize,
    current: usize,
    line: usize,
    pub errors: Vec<Spanned<ScanError>>,
}

impl<'a> Scanner<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            start: 0,
            current: 0,
            line: 1,
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
            b' ' | b'\r' | b'\t' => return None,
            b'\n' => {
                self.line += 1;
                return None;
            }
            c => {
                self.error(ScanError::UnexpectedCharacter(c));
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

        self.error(ScanError::UnterminatedBlockComment);
    }

    fn string(&mut self) -> Option<Token> {
        while let Some(c) = self.peek() {
            match c {
                b'"' => {
                    // closing quote
                    self.advance();

                    let value = self.source[self.start + 1..self.current - 1]
                        .to_string();
                    return Some(self.token(TokenKind::String(value)));
                }
                b'\n' => self.line += 1,
                _ => (),
            }
            self.advance();
        }

        self.error(ScanError::UnterminatedString);
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
            Err(e) => {
                println!(
                    "attempted to parse {}..{} (`{}`) as a float",
                    self.start,
                    self.current,
                    &self.source[self.start..self.current]
                );
                self.error(ScanError::InvalidNumber(e));
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
        let c = self.peek();
        self.current += 1;
        c
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

    fn token(&mut self, kind: TokenKind) -> Token {
        Token {
            kind,
            lexeme: self.source[self.start..self.current].to_string(),
            span: Span::for_line(self.line),
        }
    }

    fn error(&mut self, error: ScanError) {
        self.errors.push(Spanned {
            inner: error,
            span: Span::for_line(self.line),
        });
    }
}

#[derive(Debug)]
pub struct Token {
    pub kind: TokenKind,
    pub lexeme: String,
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
