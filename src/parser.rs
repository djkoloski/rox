use core::fmt;

use crate::{
    ast::{
        expr::{
            AssignExpr, BinaryExpr, Expr, GroupingExpr, LiteralExpr, UnaryExpr,
            VariableExpr,
        },
        stmt::{
            BlockStmt, DeclStmt, ExprStmt, IfStmt, PrintStmt, Program, Repl,
            Stmt,
        },
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
    UnterminatedBlock(Span),
    ExpectedIdent { stmt: Span, next: Span },
    ExpectedLeftParen(Span),
    InvalidAssignmentTarget(Span),
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
            Self::UnterminatedBlock(span) => {
                c.error(f, format_args!("unterminated block"))?;
                c.span(
                    *span,
                    f,
                    format_args!("this block is missing a closing '}}'"),
                )?;
            }
            Self::ExpectedIdent { stmt, next } => {
                c.error(f, format_args!("expected identifier"))?;
                c.span(
                    *stmt,
                    f,
                    format_args!(
                        "this variable declaration was followed by '{}' \
                         instead of a valid identifier",
                        next.get(c.source())
                    ),
                )?;
            }
            Self::ExpectedLeftParen(span) => {
                c.error(f, format_args!("expected grouped expression"))?;
                c.span(
                    *span,
                    f,
                    format_args!("expected a left parenthesis here"),
                )?;
            }
            Self::InvalidAssignmentTarget(span) => {
                c.error(f, format_args!("invalid assignment target"))?;
                c.span(
                    *span,
                    f,
                    format_args!(
                        "this is the left-hand side of an assignment \
                         expression, but is not a place"
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

    pub fn parse_repl(&mut self) -> Option<Repl> {
        self.repl()
    }

    fn peek(&self) -> &Token {
        self.tokens.last().unwrap()
    }

    fn expect(
        &mut self,
        matches: impl FnOnce(&TokenKind) -> bool,
    ) -> Option<Token> {
        if matches(&self.peek().kind) {
            Some(self.next())
        } else {
            None
        }
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
                    if let Some(stmt) = self.declaration() {
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

    fn repl(&mut self) -> Option<Repl> {
        match self.peek().kind {
            TokenKind::Var
            | TokenKind::Print
            | TokenKind::LeftBrace
            | TokenKind::If => Some(Repl::Stmt(self.declaration()?)),
            _ => Some(Repl::Expr(self.expression()?)),
        }
    }

    fn declaration(&mut self) -> Option<Stmt> {
        match self.peek().kind {
            TokenKind::Var => self.decl_stmt(),
            _ => self.statement(),
        }
    }

    fn decl_stmt(&mut self) -> Option<Stmt> {
        let var = self.next();
        let Some(ident) =
            self.expect(|k| matches!(k, TokenKind::Identifier(_)))
        else {
            self.errors.push(ParseError::ExpectedIdent {
                stmt: var.span(),
                next: self.peek().span,
            });
            return None;
        };

        let assignment = if matches!(self.peek().kind, TokenKind::Equal) {
            let equal = self.next();
            let expr = self.expression()?;
            Some((equal, expr))
        } else {
            None
        };

        let Some(semi) = self.expect(|k| matches!(k, TokenKind::Semicolon))
        else {
            let stmt = if let Some((_, expr)) = &assignment {
                Span::across(&var, expr)
            } else {
                Span::across(&var, &ident)
            };

            self.errors.push(ParseError::UnterminatedStatement {
                stmt,
                next: self.peek().span,
            });
            return None;
        };

        Some(Stmt::Decl(DeclStmt {
            var,
            ident,
            assignment,
            semi,
        }))
    }

    fn statement(&mut self) -> Option<Stmt> {
        match self.peek().kind {
            TokenKind::If => self.if_stmt(),
            TokenKind::Print => self.print_stmt(),
            TokenKind::LeftBrace => self.block_stmt(),
            _ => self.expr_stmt(),
        }
    }

    fn if_stmt(&mut self) -> Option<Stmt> {
        let if_ = self.next();

        let group = self.grouped()?;
        let then = self.statement()?;

        let mut else_ = None;
        if let Some(else_token) = self.expect(|k| matches!(k, TokenKind::Else))
        {
            else_ = Some((else_token, Box::new(self.statement()?)));
        }

        Some(Stmt::If(IfStmt {
            if_,
            group,
            then: Box::new(then),
            else_,
        }))
    }

    fn print_stmt(&mut self) -> Option<Stmt> {
        let print = self.next();
        let expr = self.expression()?;
        let Some(semi) = self.expect(|k| matches!(k, TokenKind::Semicolon))
        else {
            self.errors.push(ParseError::UnterminatedStatement {
                stmt: Span::across(&print, &expr),
                next: self.peek().span,
            });
            return None;
        };

        Some(Stmt::Print(PrintStmt { print, expr, semi }))
    }

    fn block_stmt(&mut self) -> Option<Stmt> {
        let lbrace = self.next();
        let mut stmts = Vec::new();

        loop {
            match self.peek().kind {
                TokenKind::RightBrace => break,
                TokenKind::Eof => {
                    let span = if stmts.is_empty() {
                        lbrace.span()
                    } else {
                        Span::across(&lbrace, stmts.last().unwrap())
                    };
                    self.errors.push(ParseError::UnterminatedBlock(span));
                    return None;
                }
                _ => stmts.push(self.declaration()?),
            }
        }

        let rbrace = self.next();

        Some(Stmt::Block(BlockStmt {
            lbrace,
            stmts,
            rbrace,
        }))
    }

    fn expr_stmt(&mut self) -> Option<Stmt> {
        let expr = self.expression()?;
        let Some(semi) = self.expect(|k| matches!(k, TokenKind::Semicolon))
        else {
            self.errors.push(ParseError::UnterminatedStatement {
                stmt: expr.span(),
                next: self.peek().span,
            });
            return None;
        };

        Some(Stmt::Expr(ExprStmt { expr, semi }))
    }

    fn expression(&mut self) -> Option<Expr> {
        self.assignment()
    }

    fn assignment(&mut self) -> Option<Expr> {
        let expr = self.equality()?;

        if let Some(equal) = self.expect(|k| matches!(k, TokenKind::Equal)) {
            let value = self.assignment()?;

            if let Expr::Variable(VariableExpr { ident }) = expr {
                Some(Expr::Assign(AssignExpr {
                    ident,
                    equal,
                    expr: Box::new(value),
                }))
            } else {
                self.errors
                    .push(ParseError::InvalidAssignmentTarget(expr.span()));
                None
            }
        } else {
            Some(expr)
        }
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
            TokenKind::LeftParen => self.grouped().map(Expr::Grouping),
            TokenKind::Identifier(_) => {
                Some(Expr::Variable(VariableExpr { ident: self.next() }))
            }
            _ => {
                self.errors
                    .push(ParseError::ExpectedExpression(self.peek().span));
                None
            }
        }
    }

    fn grouped(&mut self) -> Option<GroupingExpr> {
        let Some(lparen) = self.expect(|k| matches!(k, TokenKind::LeftParen))
        else {
            self.errors
                .push(ParseError::ExpectedLeftParen(self.peek().span));
            return None;
        };

        let Some(inner) = self.expression() else {
            self.synchronize(|kind| matches!(kind, TokenKind::RightParen));
            return None;
        };

        match self.peek().kind {
            TokenKind::RightParen => Some(GroupingExpr {
                lparen,
                inner: Box::new(inner),
                rparen: self.next(),
            }),
            _ => {
                self.errors.push(ParseError::UnterminatedGroup(Span::across(
                    &lparen, &inner,
                )));

                self.synchronize(|kind| matches!(kind, TokenKind::RightParen));

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
