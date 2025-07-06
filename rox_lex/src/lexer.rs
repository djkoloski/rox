use rox_diag::Span;

use crate::{LexError, Token, token_kind::*};

pub struct LexOutput {
    pub tokens: Vec<Token>,
    pub errors: Vec<LexError>,
}

pub struct Lexer<'a> {
    source: &'a str,
    start: usize,
    current: usize,
    tokens: Vec<Token>,
    errors: Vec<LexError>,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            start: 0,
            current: 0,
            tokens: Vec::new(),
            errors: Vec::new(),
        }
    }

    pub fn lex(mut self) -> LexOutput {
        loop {
            self.start = self.current;
            let Some(c) = self.advance() else {
                break;
            };

            self.lex_token(c);
        }

        self.simple::<Eof>();

        LexOutput {
            tokens: self.tokens,
            errors: self.errors,
        }
    }

    fn lex_token(&mut self, c: u8) {
        match c {
            b'(' => self.simple::<LeftParen>(),
            b')' => self.simple::<RightParen>(),
            b'{' => self.simple::<LeftBrace>(),
            b'}' => self.simple::<RightBrace>(),
            b',' => self.simple::<Comma>(),
            b'.' => self.simple::<Dot>(),
            b'-' => self.simple::<Minus>(),
            b'+' => self.simple::<Plus>(),
            b';' => self.simple::<Semicolon>(),
            b'*' => self.simple::<Star>(),
            b'!' => {
                if self.expect(b'=') {
                    self.simple::<BangEqual>()
                } else {
                    self.simple::<Bang>()
                }
            }
            b'=' => {
                if self.expect(b'=') {
                    self.simple::<EqualEqual>()
                } else {
                    self.simple::<Equal>()
                }
            }
            b'<' => {
                if self.expect(b'=') {
                    self.simple::<LessEqual>()
                } else {
                    self.simple::<Less>()
                }
            }
            b'>' => {
                if self.expect(b'=') {
                    self.simple::<GreaterEqual>()
                } else {
                    self.simple::<Greater>()
                }
            }
            b'/' => match self.peek() {
                Some(b'/') => self.line_comment(),
                Some(b'*') => self.block_comment(),
                _ => self.simple::<Slash>(),
            },
            b'"' => self.string(),
            b'0'..=b'9' => self.number(),
            b'_' | b'a'..=b'z' | b'A'..=b'Z' => self.identifier(),
            // Whitespace
            b' ' | b'\r' | b'\t' | b'\n' => (),
            char => {
                self.error(LexError::UnexpectedCharacter {
                    span: self.span(),
                    char,
                });
            }
        }
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
                    if self.expect(b'*') {
                        depth += 1;
                    }
                }
                b'*' => {
                    if self.expect(b'/') {
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

    fn string(&mut self) {
        while let Some(c) = self.advance() {
            if c == b'"' {
                let value =
                    self.source[self.start + 1..self.current - 1].to_string();
                self.tokens.push(
                    StringLiteral {
                        span: self.span(),
                        value,
                    }
                    .into(),
                );
            }
        }

        self.error(LexError::UnterminatedString { span: self.span() });
    }

    fn number(&mut self) {
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
            Ok(value) => self.token(FloatLiteral {
                span: self.span(),
                value,
            }),
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
            }
        }
    }

    fn identifier(&mut self) {
        while self.peek().is_some_and(
            |c| matches!(c, b'_' | b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9'),
        ) {
            self.advance();
        }

        match &self.source[self.start..self.current] {
            "and" => self.simple::<And>(),
            "class" => self.simple::<Class>(),
            "else" => self.simple::<Else>(),
            "false" => self.simple::<False>(),
            "for" => self.simple::<For>(),
            "fun" => self.simple::<Fun>(),
            "if" => self.simple::<If>(),
            "nil" => self.simple::<Nil>(),
            "or" => self.simple::<Or>(),
            "print" => self.simple::<Print>(),
            "return" => self.simple::<Return>(),
            "super" => self.simple::<Super>(),
            "this" => self.simple::<This>(),
            "true" => self.simple::<True>(),
            "var" => self.simple::<Var>(),
            "while" => self.simple::<While>(),
            ident => self.token(Identifier {
                span: self.span(),
                value: ident.to_string(),
            }),
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

    fn simple<T>(&mut self)
    where
        T: From<Span>,
        Token: From<T>,
    {
        self.token(T::from(self.span()))
    }

    fn token<T>(&mut self, t: T)
    where
        Token: From<T>,
    {
        self.tokens.push(t.into());
    }

    fn error(&mut self, e: LexError) {
        self.errors.push(e);
    }
}
