use core::fmt;

use crate::{
    ast::{
        expr::{BinaryExpr, Expr, GroupingExpr, LiteralExpr, UnaryExpr},
        stmt::{ExprStmt, PrintStmt, Program, Stmt},
    },
    diagnostic::{Context, Diagnostic},
    scanner::{Token, TokenKind},
    span::{Span, Spanned as _},
};

#[derive(Debug)]
pub enum ParseError {
    ExpectedExpression(Span),
    UnterminatedGroup(Span),
    UnterminatedStatement { stmt: Span, next: Span },
}

impl Diagnostic for ParseError {
    fn fmt(
        &self,
        c: &mut Context<'_>,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            Self::ExpectedExpression(span) => {
                c.error(f, format_args!("unexpected token"))?;
                c.span(
                    *span,
                    f,
                    format_args!(
                        "expected expression, found '{}'",
                        span.get(c.source())
                    ),
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
            Self::UnterminatedStatement { stmt, next } => {
                c.error(f, format_args!("unterminated statement"))?;
                c.span(
                    *stmt,
                    f,
                    format_args!(
                        "this statement was followed by '{}' instead of ';'",
                        next.get(c.source())
                    ),
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

    pub fn parse(&mut self) -> Option<Program> {
        self.program()
    }

    fn peek(&self) -> &Token {
        self.tokens.last().unwrap()
    }

    fn next(&mut self) -> Token {
        self.tokens.pop().unwrap()
    }

    fn program(&mut self) -> Option<Program> {
        let mut stmts = Vec::new();

        loop {
            match self.peek().kind {
                TokenKind::Eof => break,
                _ => {
                    if let Some(stmt) = self.statement() {
                        stmts.push(stmt);
                    } else {
                        self.synchronize(|kind| {
                            matches!(kind, TokenKind::Semicolon)
                        });
                    }
                }
            }
        }

        Some(Program {
            stmts,
            eof: self.next(),
        })
    }

    fn statement(&mut self) -> Option<Stmt> {
        match self.peek().kind {
            TokenKind::Print => self.print_stmt(),
            _ => self.expr_stmt(),
        }
    }

    fn print_stmt(&mut self) -> Option<Stmt> {
        let print = self.next();
        let expr = self.expression()?;
        if !matches!(self.peek().kind, TokenKind::Semicolon) {
            self.errors.push(ParseError::UnterminatedStatement {
                stmt: Span::across(&print, &expr),
                next: self.peek().span,
            });
            return None;
        }

        Some(Stmt::Print(PrintStmt {
            print,
            expr,
            semi: self.next(),
        }))
    }

    fn expr_stmt(&mut self) -> Option<Stmt> {
        let expr = self.expression()?;
        if !matches!(self.peek().kind, TokenKind::Semicolon) {
            self.errors.push(ParseError::UnterminatedStatement {
                stmt: expr.span(),
                next: self.peek().span,
            });
            return None;
        }

        Some(Stmt::Expr(ExprStmt {
            expr,
            semi: self.next(),
        }))
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
        if matches!(self.peek().kind, TokenKind::Bang | TokenKind::Minus) {
            let operator = self.next();
            let inner = self.unary()?;
            Some(Expr::Unary(UnaryExpr {
                operator,
                inner: Box::new(inner),
            }))
        } else {
            self.primary()
        }
    }

    fn primary(&mut self) -> Option<Expr> {
        match self.peek().kind {
            TokenKind::Number(_)
            | TokenKind::String(_)
            | TokenKind::True
            | TokenKind::False
            | TokenKind::Nil => {
                Some(Expr::Literal(LiteralExpr { token: self.next() }))
            }
            TokenKind::LeftParen => {
                let lparen = self.next();

                let Some(inner) = self.expression() else {
                    self.synchronize(|kind| {
                        matches!(kind, TokenKind::RightParen)
                    });
                    return None;
                };

                match self.peek().kind {
                    TokenKind::RightParen => {
                        Some(Expr::Grouping(GroupingExpr {
                            lparen,
                            inner: Box::new(inner),
                            rparen: self.next(),
                        }))
                    }
                    _ => {
                        self.errors.push(ParseError::UnterminatedGroup(
                            Span::across(&lparen, &inner),
                        ));

                        self.synchronize(|kind| {
                            matches!(kind, TokenKind::RightParen)
                        });

                        None
                    }
                }
            }
            _ => {
                self.errors
                    .push(ParseError::ExpectedExpression(self.peek().span));
                None
            }
        }
    }

    fn synchronize(&mut self, matches: impl Fn(&TokenKind) -> bool) {
        while !matches!(self.peek().kind, TokenKind::Eof) {
            if matches(&self.next().kind) {
                break;
            }
        }
    }

    fn parse_left_binary(
        &mut self,
        matches: impl Fn(&TokenKind) -> bool,
        mut operand: impl FnMut(&mut Self) -> Option<Expr>,
    ) -> Option<Expr> {
        let mut expr = operand(self)?;

        while matches(&self.peek().kind) {
            let operator = self.next();
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
