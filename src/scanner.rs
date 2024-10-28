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

pub struct ScanResult {
    pub tokens: Vec<Token>,
    pub errors: Vec<ScanError>,
}

pub struct Scanner<'a> {
    source: &'a str,
    start: usize,
    current: usize,
    line: usize,
    pub result: ScanResult,
}

impl<'a> Scanner<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            start: 0,
            current: 0,
            line: 1,
            result: ScanResult {
                tokens: Vec::new(),
                errors: Vec::new(),
            },
        }
    }

    pub fn scan_tokens(&mut self) {
        while !self.is_at_end() {
            self.start = self.current;
            self.scan_token();
        }

        self.result.tokens.push(Token {
            kind: TokenKind::Eof,
            lexeme: String::new(),
            literal: None,
            line: self.line,
        });
    }

    fn scan_token(&mut self) {
        match self.advance() {
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
                    self.non_literal(TokenKind::BangEqual);
                } else {
                    self.non_literal(TokenKind::Bang);
                }
            }
            b'=' => {
                if self.expect(b'=') {
                    self.non_literal(TokenKind::EqualEqual);
                } else {
                    self.non_literal(TokenKind::Equal);
                }
            }
            b'<' => {
                if self.expect(b'=') {
                    self.non_literal(TokenKind::LessEqual);
                } else {
                    self.non_literal(TokenKind::Less);
                }
            }
            b'>' => {
                if self.expect(b'=') {
                    self.non_literal(TokenKind::GreaterEqual);
                } else {
                    self.non_literal(TokenKind::Greater);
                }
            }
            b'/' => match self.peek() {
                b'/' => self.line_comment(),
                b'*' => self.block_comment(),
                _ => self.non_literal(TokenKind::Slash),
            },
            b'"' => self.string(),
            b'0'..=b'9' => self.number(),
            b'_' | b'a'..=b'z' | b'A'..=b'Z' => self.identifier(),
            // Whitespace
            b' ' | b'\r' | b'\t' => (),
            b'\n' => self.line += 1,
            c => self.error(ScanErrorKind::UnexpectedCharacter(c)),
        }
    }

    fn line_comment(&mut self) {
        self.advance();
        while self.peek() != b'\n' && !self.is_at_end() {
            self.advance();
        }
    }

    fn block_comment(&mut self) {
        let mut depth = 1;

        self.advance();
        loop {
            match self.advance() {
                b'\0' => {
                    self.error(ScanErrorKind::UnterminatedBlockComment);
                    break;
                }
                b'/' => {
                    if self.peek() == b'*' {
                        self.advance();
                        depth += 1;
                    }
                }
                b'*' => {
                    if self.peek() == b'/' {
                        self.advance();
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                }
                _ => (),
            }
        }
    }

    fn string(&mut self) {
        loop {
            match self.peek() {
                b'"' => break,
                b'\n' => self.line += 1,
                b'\0' => {
                    self.error(ScanErrorKind::UnterminatedString);
                    break;
                }
                _ => (),
            }
            self.advance();
        }

        // closing quote
        self.advance();

        let value = self.source[self.start + 1..self.current - 1].to_string();
        self.token(TokenKind::String, Some(Literal::String(value)));
    }

    fn number(&mut self) {
        while self.peek().is_ascii_digit() {
            self.advance();
        }

        if self.peek() == b'.' && self.peek_next().is_ascii_digit() {
            self.advance();

            while self.peek().is_ascii_digit() {
                self.advance();
            }
        }

        match self.source[self.start..self.current].parse::<f64>() {
            Ok(value) => {
                self.token(TokenKind::Number, Some(Literal::Number(value)))
            }
            Err(e) => self.error(ScanErrorKind::InvalidNumber(e)),
        }
    }

    fn identifier(&mut self) {
        loop {
            let is_alphanumeric = matches!(
                self.peek(),
                b'_' | b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9',
            );

            if !is_alphanumeric {
                break;
            }

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
        });
    }

    fn peek(&self) -> u8 {
        self.source
            .as_bytes()
            .get(self.current)
            .cloned()
            .unwrap_or(b'\0')
    }

    fn peek_next(&self) -> u8 {
        self.source
            .as_bytes()
            .get(self.current + 1)
            .cloned()
            .unwrap_or(b'\0')
    }

    fn advance(&mut self) -> u8 {
        let c = self.peek();
        self.current += 1;
        c
    }

    fn expect(&mut self, next: u8) -> bool {
        let c = self.peek();
        if c == next {
            self.advance();
            true
        } else {
            false
        }
    }

    fn non_literal(&mut self, kind: TokenKind) {
        self.token(kind, None)
    }

    fn token(&mut self, kind: TokenKind, literal: Option<Literal>) {
        self.result.tokens.push(Token {
            kind,
            lexeme: self.source[self.start..self.current].to_string(),
            literal,
            line: self.line,
        });
    }

    fn error(&mut self, kind: ScanErrorKind) {
        self.result.errors.push(ScanError {
            kind,
            line: self.line,
        });
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.source.as_bytes().len()
    }
}

#[derive(Debug)]
pub enum Literal {
    String(String),
    Number(f64),
}

#[derive(Debug)]
pub struct Token {
    pub kind: TokenKind,
    pub lexeme: String,
    pub literal: Option<Literal>,
    pub line: usize,
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
    Eof,
}
