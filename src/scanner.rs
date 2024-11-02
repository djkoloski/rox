use core::{fmt, num::ParseFloatError};

#[derive(Debug)]
pub struct ScanError {
    pub kind: ScanErrorKind,
    pub line: usize,
}

impl fmt::Display for ScanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "on line {}:\n{}", self.line, self.kind)
    }
}

#[derive(Debug)]
pub enum ScanErrorKind {
    UnexpectedCharacter(u8),
    UnterminatedBlockComment,
    UnterminatedString,
    InvalidNumber(ParseFloatError),
}

impl fmt::Display for ScanErrorKind {
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
    pub errors: Vec<ScanError>,
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
            b'(' => self.non_literal(TokenKind::LeftParen),
            b')' => self.non_literal(TokenKind::RightParen),
            b'{' => self.non_literal(TokenKind::LeftBrace),
            b'}' => self.non_literal(TokenKind::RightBrace),
            b',' => self.non_literal(TokenKind::Comma),
            b'.' => self.non_literal(TokenKind::Dot),
            b'-' => self.non_literal(TokenKind::Minus),
            b'+' => self.non_literal(TokenKind::Plus),
            b';' => self.non_literal(TokenKind::Semicolon),
            b'*' => self.non_literal(TokenKind::Star),
            b'!' => {
                if self.expect(b'=') {
                    self.non_literal(TokenKind::BangEqual)
                } else {
                    self.non_literal(TokenKind::Bang)
                }
            }
            b'=' => {
                if self.expect(b'=') {
                    self.non_literal(TokenKind::EqualEqual)
                } else {
                    self.non_literal(TokenKind::Equal)
                }
            }
            b'<' => {
                if self.expect(b'=') {
                    self.non_literal(TokenKind::LessEqual)
                } else {
                    self.non_literal(TokenKind::Less)
                }
            }
            b'>' => {
                if self.expect(b'=') {
                    self.non_literal(TokenKind::GreaterEqual)
                } else {
                    self.non_literal(TokenKind::Greater)
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
                _ => self.non_literal(TokenKind::Slash),
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
                self.error(ScanErrorKind::UnexpectedCharacter(c));
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

        self.error(ScanErrorKind::UnterminatedBlockComment);
    }

    fn string(&mut self) -> Option<Token> {
        while let Some(c) = self.peek() {
            match c {
                b'"' => {
                    // closing quote
                    self.advance();

                    let value = self.source[self.start + 1..self.current - 1]
                        .to_string();
                    return Some(
                        self.token(
                            TokenKind::String,
                            Some(Value::String(value)),
                        ),
                    );
                }
                b'\n' => self.line += 1,
                _ => (),
            }
            self.advance();
        }

        self.error(ScanErrorKind::UnterminatedString);
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
            Ok(value) => {
                Some(self.token(TokenKind::Number, Some(Value::Number(value))))
            }
            Err(e) => {
                println!(
                    "attempted to parse {}..{} (`{}`) as a float",
                    self.start,
                    self.current,
                    &self.source[self.start..self.current]
                );
                self.error(ScanErrorKind::InvalidNumber(e));
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

        self.non_literal(match &self.source[self.start..self.current] {
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

    fn non_literal(&mut self, kind: TokenKind) -> Token {
        self.token(kind, None)
    }

    fn token(&mut self, kind: TokenKind, literal: Option<Value>) -> Token {
        Token {
            kind,
            lexeme: self.source[self.start..self.current].to_string(),
            value: literal,
            line: self.line,
        }
    }

    fn error(&mut self, kind: ScanErrorKind) {
        self.errors.push(ScanError {
            kind,
            line: self.line,
        });
    }
}

#[derive(Debug)]
pub enum Value {
    String(String),
    Number(f64),
}

#[derive(Debug)]
pub struct Token {
    pub kind: TokenKind,
    pub lexeme: String,
    pub value: Option<Value>,
    pub line: usize,
}

#[derive(Debug, PartialEq)]
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
    String,
    Number,
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
