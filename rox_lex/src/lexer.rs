use rox_diag::Span;

use crate::{LexError, Token, token_kind::*};

pub struct Lexer<'a> {
    source: &'a str,
    start: usize,
    current: usize,
    pub errors: Vec<LexError>,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            start: 0,
            current: 0,
            errors: Vec::new(),
        }
    }

    pub fn lex_tokens(&mut self) -> Vec<Token> {
        let mut result = Vec::new();

        loop {
            self.start = self.current;
            let Some(c) = self.advance() else {
                break;
            };

            if let Some(token) = self.lex_token(c) {
                result.push(token);
            }
        }

        result.push(self.token::<Eof>());

        result
    }

    fn lex_token(&mut self, c: u8) -> Option<Token> {
        Some(match c {
            b'(' => self.token::<LeftParen>(),
            b')' => self.token::<RightParen>(),
            b'{' => self.token::<LeftBrace>(),
            b'}' => self.token::<RightBrace>(),
            b',' => self.token::<Comma>(),
            b'.' => self.token::<Dot>(),
            b'-' => self.token::<Minus>(),
            b'+' => self.token::<Plus>(),
            b';' => self.token::<Semicolon>(),
            b'*' => self.token::<Star>(),
            b'!' => {
                if self.expect(b'=') {
                    self.token::<BangEqual>()
                } else {
                    self.token::<Bang>()
                }
            }
            b'=' => {
                if self.expect(b'=') {
                    self.token::<EqualEqual>()
                } else {
                    self.token::<Equal>()
                }
            }
            b'<' => {
                if self.expect(b'=') {
                    self.token::<LessEqual>()
                } else {
                    self.token::<Less>()
                }
            }
            b'>' => {
                if self.expect(b'=') {
                    self.token::<GreaterEqual>()
                } else {
                    self.token::<Greater>()
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
                _ => self.token::<Slash>(),
            },
            b'"' => self.string()?,
            b'0'..=b'9' => self.number()?,
            b'_' | b'a'..=b'z' | b'A'..=b'Z' => self.identifier(),
            // Whitespace
            b' ' | b'\r' | b'\t' | b'\n' => return None,
            char => {
                self.error(LexError::UnexpectedCharacter {
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

        self.error(LexError::UnterminatedBlockComment { span: self.span() });
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

        self.error(LexError::UnterminatedString { span: self.span() });
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
                self.error(LexError::InvalidNumber {
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
            "and" => self.token::<And>(),
            "class" => self.token::<Class>(),
            "else" => self.token::<Else>(),
            "false" => self.token::<False>(),
            "for" => self.token::<For>(),
            "fun" => self.token::<Fun>(),
            "if" => self.token::<If>(),
            "nil" => self.token::<Nil>(),
            "or" => self.token::<Or>(),
            "print" => self.token::<Print>(),
            "return" => self.token::<Return>(),
            "super" => self.token::<Super>(),
            "this" => self.token::<This>(),
            "true" => self.token::<True>(),
            "var" => self.token::<Var>(),
            "while" => self.token::<While>(),
            ident => Ident {
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
        Span::new(self.start, self.current)
    }

    fn token<T>(&self) -> Token
    where
        T: From<Span>,
        Token: From<T>,
    {
        T::from(self.span()).into()
    }

    fn error(&mut self, e: LexError) {
        self.errors.push(e);
    }
}
