use core::fmt;

use crate::{
    ast::{BinaryExpr, Expr, GroupingExpr, LiteralExpr, UnaryExpr},
    diagnostic::{Context, Diagnostic},
    scanner::{Token, TokenKind},
    span::Span,
};

#[derive(Debug)]
pub enum ParseError {
    ExpectedExpression { span: Span, kind: TokenKind },
    UnterminatedGroup(Span),
    EofDuringExpression(Span),
}

impl Diagnostic for ParseError {
    fn fmt(
        &self,
        c: &mut Context<'_>,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            Self::ExpectedExpression { span, kind } => {
                c.error(f, format_args!("unexpected token"))?;
                c.span(
                    *span,
                    f,
                    format_args!("expressions may not start with {:?}", kind),
                )?;
            }
            Self::UnterminatedGroup(span) => {
                c.error(f, format_args!("unterminated group"))?;
                c.span(
                    *span,
                    f,
                    format_args!("this group is missing a closing ')'"),
                )?;
            }
            Self::EofDuringExpression(span) => {
                c.error(f, format_args!("unexpected eof"))?;
                c.span(
                    *span,
                    f,
                    format_args!("expected an expression to begin here"),
                )?;
            }
        }
        Ok(())
    }
}

pub struct Parser {
    tokens: Vec<Token>,
    pub errors: Vec<ParseError>,
}

impl Parser {
    pub fn new(mut tokens: Vec<Token>) -> Self {
        tokens.reverse();
        Self {
            tokens,
            errors: Vec::new(),
        }
    }

    pub fn parse(&mut self) -> Option<Expr> {
        self.expression()
    }

    fn peek(&self) -> Option<&TokenKind> {
        self.tokens.last().map(|t| &t.kind)
    }

    fn next(&mut self) -> Option<Token> {
        self.tokens.pop()
    }

    fn expression(&mut self) -> Option<Expr> {
        self.equality()
    }

    fn equality(&mut self) -> Option<Expr> {
        self.parse_left_binary(
            |kind| matches!(kind, TokenKind::BangEqual | TokenKind::EqualEqual),
            |this| this.comparison(),
        )
    }

    fn comparison(&mut self) -> Option<Expr> {
        self.parse_left_binary(
            |kind| {
                matches!(
                    kind,
                    TokenKind::Greater
                        | TokenKind::GreaterEqual
                        | TokenKind::Less
                        | TokenKind::LessEqual
                )
            },
            |this| this.term(),
        )
    }

    fn term(&mut self) -> Option<Expr> {
        self.parse_left_binary(
            |kind| matches!(kind, TokenKind::Minus | TokenKind::Plus),
            |this| this.factor(),
        )
    }

    fn factor(&mut self) -> Option<Expr> {
        self.parse_left_binary(
            |kind| matches!(kind, TokenKind::Slash | TokenKind::Star),
            |this| this.unary(),
        )
    }

    fn unary(&mut self) -> Option<Expr> {
        if matches!(self.peek(), Some(TokenKind::Bang | TokenKind::Minus)) {
            let operator = self.next().unwrap();
            let expr = self.unary()?;
            Some(Expr::Unary(UnaryExpr {
                operator,
                expr: Box::new(expr),
            }))
        } else {
            self.primary()
        }
    }

    fn primary(&mut self) -> Option<Expr> {
        match self.next() {
            Some(
                token @ Token {
                    kind:
                        TokenKind::Number(_)
                        | TokenKind::String(_)
                        | TokenKind::True
                        | TokenKind::False
                        | TokenKind::Nil,
                    ..
                },
            ) => Some(Expr::Literal(LiteralExpr { token })),
            Some(
                token @ Token {
                    kind: TokenKind::LeftParen,
                    ..
                },
            ) => {
                let Some(expr) = self.expression() else {
                    self.synchronize(|kind| {
                        matches!(kind, TokenKind::RightParen)
                    });
                    return None;
                };

                match self.next() {
                    Some(
                        rparen @ Token {
                            kind: TokenKind::RightParen,
                            ..
                        },
                    ) => Some(Expr::Grouping(GroupingExpr {
                        lparen: token,
                        expr: Box::new(expr),
                        rparen,
                    })),
                    Some(token) => {
                        self.errors
                            .push(ParseError::UnterminatedGroup(token.span));

                        self.synchronize(|kind| {
                            matches!(kind, TokenKind::RightParen)
                        });

                        None
                    }
                    None => {
                        self.errors
                            .push(ParseError::EofDuringExpression(Span::eof()));
                        None
                    }
                }
            }
            Some(token) => {
                self.errors.push(ParseError::ExpectedExpression {
                    kind: token.kind,
                    span: token.span,
                });
                None
            }
            None => {
                self.errors
                    .push(ParseError::EofDuringExpression(Span::eof()));
                None
            }
        }
    }

    fn synchronize(&mut self, matches: impl Fn(&TokenKind) -> bool) {
        while let Some(kind) = &self.peek() {
            if !matches(kind) {
                self.next();
            }
        }
    }

    fn parse_left_binary(
        &mut self,
        matches: impl Fn(&TokenKind) -> bool,
        mut operand: impl FnMut(&mut Self) -> Option<Expr>,
    ) -> Option<Expr> {
        let mut expr = operand(self)?;

        while self.peek().is_some_and(&matches) {
            let operator = self.next().unwrap();
            let right = operand(self)?;
            expr = Expr::Binary(BinaryExpr {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
            });
        }

        Some(expr)
    }
}
