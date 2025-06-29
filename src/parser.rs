use core::fmt;

use crate::{
    ast::{
        decoration::Decorator,
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
    scanner::Token,
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
    decorator: Decorator,
    pub errors: Vec<ParseError>,
}

impl Parser {
    pub fn new(mut tokens: Vec<Token>) -> Self {
        tokens.reverse();
        Self {
            tokens,
            decorator: Decorator::new(),
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
        matches: impl FnOnce(&Token) -> bool,
    ) -> Option<Token> {
        if matches(self.peek()) {
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
            match self.peek() {
                Token::Eof(_) => break,
                _ => {
                    if let Some(stmt) = self.declaration() {
                        stmts.push(stmt);
                    } else {
                        self.synchronize(|token| {
                            matches!(token, Token::Semicolon(_))
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
        match self.peek() {
            Token::Var(_)
            | Token::Print(_)
            | Token::LeftBrace(_)
            | Token::If(_) => Some(Repl::Stmt(self.declaration()?)),
            _ => Some(Repl::Expr(self.expression()?)),
        }
    }

    fn declaration(&mut self) -> Option<Stmt> {
        match self.peek() {
            Token::Var(_) => self.decl_stmt(),
            _ => self.statement(),
        }
    }

    fn decl_stmt(&mut self) -> Option<Stmt> {
        let var = self.next();
        let Some(ident) = self.expect(|k| matches!(k, Token::Identifier(_)))
        else {
            self.errors.push(ParseError::ExpectedIdent {
                stmt: var.span(),
                next: self.peek().span(),
            });
            return None;
        };

        let assignment = if matches!(self.peek(), Token::Equal(_)) {
            let equal = self.next();
            let expr = self.expression()?;
            Some((equal, expr))
        } else {
            None
        };

        let Some(semi) = self.expect(|k| matches!(k, Token::Semicolon(_)))
        else {
            let stmt = if let Some((_, expr)) = &assignment {
                Span::across(&var, expr)
            } else {
                Span::across(&var, &ident)
            };

            self.errors.push(ParseError::UnterminatedStatement {
                stmt,
                next: self.peek().span(),
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
        match self.peek() {
            Token::If(_) => self.if_stmt(),
            Token::Print(_) => self.print_stmt(),
            Token::LeftBrace(_) => self.block_stmt(),
            _ => self.expr_stmt(),
        }
    }

    fn if_stmt(&mut self) -> Option<Stmt> {
        let if_ = self.next();

        let group = self.grouped()?;
        let then = self.statement()?;

        let mut else_ = None;
        if let Some(else_token) = self.expect(|k| matches!(k, Token::Else(_))) {
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
        let Some(semi) = self.expect(|k| matches!(k, Token::Semicolon(_)))
        else {
            self.errors.push(ParseError::UnterminatedStatement {
                stmt: Span::across(&print, &expr),
                next: self.peek().span(),
            });
            return None;
        };

        Some(Stmt::Print(PrintStmt { print, expr, semi }))
    }

    fn block_stmt(&mut self) -> Option<Stmt> {
        let lbrace = self.next();
        let mut stmts = Vec::new();

        loop {
            match self.peek() {
                Token::RightBrace(_) => break,
                Token::Eof(_) => {
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
        let Some(semi) = self.expect(|k| matches!(k, Token::Semicolon(_)))
        else {
            self.errors.push(ParseError::UnterminatedStatement {
                stmt: expr.span(),
                next: self.peek().span(),
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

        if let Some(equal) = self.expect(|k| matches!(k, Token::Equal(_))) {
            let value = self.assignment()?;

            if let Expr::Variable(VariableExpr { decoration, ident }) = expr {
                Some(Expr::Assign(AssignExpr {
                    decoration,
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
            |kind| matches!(kind, Token::BangEqual(_) | Token::EqualEqual(_)),
            |this| this.comparison(),
        )
    }

    fn comparison(&mut self) -> Option<Expr> {
        self.parse_left_binary(
            |kind| {
                matches!(
                    kind,
                    Token::Greater(_)
                        | Token::GreaterEqual(_)
                        | Token::Less(_)
                        | Token::LessEqual(_)
                )
            },
            |this| this.term(),
        )
    }

    fn term(&mut self) -> Option<Expr> {
        self.parse_left_binary(
            |kind| matches!(kind, Token::Minus(_) | Token::Plus(_)),
            |this| this.factor(),
        )
    }

    fn factor(&mut self) -> Option<Expr> {
        self.parse_left_binary(
            |kind| matches!(kind, Token::Slash(_) | Token::Star(_)),
            |this| this.unary(),
        )
    }

    fn unary(&mut self) -> Option<Expr> {
        if matches!(self.peek(), Token::Bang(_) | Token::Minus(_)) {
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
        match self.peek() {
            Token::Number(_)
            | Token::String(_)
            | Token::True(_)
            | Token::False(_)
            | Token::Nil(_) => {
                Some(Expr::Literal(LiteralExpr { token: self.next() }))
            }
            Token::LeftParen(_) => self.grouped().map(Expr::Grouping),
            Token::Identifier(_) => Some(Expr::Variable(VariableExpr {
                decoration: self.decorator.decorate(),
                ident: self.next(),
            })),
            _ => {
                self.errors
                    .push(ParseError::ExpectedExpression(self.peek().span()));
                None
            }
        }
    }

    fn grouped(&mut self) -> Option<GroupingExpr> {
        let Some(lparen) = self.expect(|k| matches!(k, Token::LeftParen(_)))
        else {
            self.errors
                .push(ParseError::ExpectedLeftParen(self.peek().span()));
            return None;
        };

        let Some(inner) = self.expression() else {
            self.synchronize(|kind| matches!(kind, Token::RightParen(_)));
            return None;
        };

        match self.peek() {
            Token::RightParen(_) => Some(GroupingExpr {
                lparen,
                inner: Box::new(inner),
                rparen: self.next(),
            }),
            _ => {
                self.errors.push(ParseError::UnterminatedGroup(Span::across(
                    &lparen, &inner,
                )));

                self.synchronize(|kind| matches!(kind, Token::RightParen(_)));

                None
            }
        }
    }

    fn synchronize(&mut self, matches: impl Fn(&Token) -> bool) {
        while !matches!(self.peek(), Token::Eof(_)) {
            if matches(&self.next()) {
                break;
            }
        }
    }

    fn parse_left_binary(
        &mut self,
        matches: impl Fn(&Token) -> bool,
        mut operand: impl FnMut(&mut Self) -> Option<Expr>,
    ) -> Option<Expr> {
        let mut expr = operand(self)?;

        while matches(self.peek()) {
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
