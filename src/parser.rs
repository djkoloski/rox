use core::fmt;

use crate::{
    ast::{
        decoration::Decorator,
        expr::{
            AssignExpr, BinaryExpr, BinaryOperator, CallExpr, Expr,
            GroupingExpr, LiteralExpr, UnaryExpr, VariableExpr,
        },
        punctuated::Punctuated,
        stmt::{
            BlockStmt, ExprStmt, FunDeclStmt, IfStmt, PrintStmt, Program, Repl,
            Stmt, VarDeclStmt,
        },
    },
    diagnostic::{Context, Diagnostic},
    scanner::{Fun, LeftBrace, RightParen, Semicolon, Token, TokenKind, Var},
    span::{Span, Spanned as _},
};

#[derive(Debug)]
pub enum ParseError {
    ExpectedExpression(Span),
    UnterminatedGroup(Span),
    UnterminatedStatement { stmt: Span, next: Span },
    UnterminatedBlock(Span),
    ExpectedIdent { span: Span, next: Span },
    ExpectedLeftParen(Span),
    ExpectedRightParen(Span),
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
            Self::ExpectedIdent { span: stmt, next } => {
                c.error(f, format_args!("expected identifier"))?;
                c.span(
                    *stmt,
                    f,
                    format_args!(
                        "this declaration was followed by '{}' instead of a \
                         valid identifier",
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
            Self::ExpectedRightParen(span) => {
                c.error(
                    f,
                    format_args!(
                        "expected right parenthesis after function call \
                         arguments"
                    ),
                )?;
                c.span(*span, f, format_args!("expected a ')' here"))?;
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

    fn try_next<T: TokenKind>(&mut self) -> Option<T> {
        if T::matches_token(self.peek()) {
            Some(T::from_token(self.next()))
        } else {
            None
        }
    }

    fn expect<T: TokenKind>(&mut self) -> T {
        T::from_token(self.next())
    }

    fn next(&mut self) -> Token {
        self.tokens.pop().unwrap()
    }

    fn program(&mut self) -> Option<Program> {
        let mut stmts = Vec::new();

        loop {
            if let Some(eof) = self.try_next() {
                break Some(Program { stmts, eof });
            } else if let Some(stmt) = self.declaration() {
                stmts.push(stmt);
            } else {
                self.synchronize::<Semicolon>();
            }
        }
    }

    fn repl(&mut self) -> Option<Repl> {
        match self.peek() {
            Token::Var(_)
            | Token::Fun(_)
            | Token::Print(_)
            | Token::LeftBrace(_)
            | Token::If(_) => Some(Repl::Stmt(self.declaration()?)),
            _ => Some(Repl::Expr(self.expression()?)),
        }
    }

    fn declaration(&mut self) -> Option<Stmt> {
        match self.peek() {
            Token::Var(_) => self.var_decl_stmt(),
            Token::Fun(_) => self.fun_decl_stmt(),
            _ => self.statement(),
        }
    }

    fn var_decl_stmt(&mut self) -> Option<Stmt> {
        let var = self.try_next::<Var>()?;
        let Some(ident) = self.try_next() else {
            self.errors.push(ParseError::ExpectedIdent {
                span: var.span(),
                next: self.peek().span(),
            });
            return None;
        };

        let assignment = if let Some(equal) = self.try_next() {
            let expr = self.expression()?;
            Some((equal, expr))
        } else {
            None
        };

        let Some(semi) = self.try_next() else {
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

        Some(Stmt::VarDecl(VarDeclStmt {
            var,
            ident,
            assignment,
            semi,
        }))
    }

    fn fun_decl_stmt(&mut self) -> Option<Stmt> {
        let fun = self.try_next::<Fun>()?;
        let Some(name) = self.try_next() else {
            self.errors.push(ParseError::ExpectedIdent {
                span: fun.span(),
                next: self.peek().span(),
            });
            return None;
        };

        let Some(lparen) = self.try_next() else {
            self.errors
                .push(ParseError::ExpectedLeftParen(self.peek().span()));
            return None;
        };

        let mut params = Punctuated::new();
        while !matches!(self.peek(), Token::RightParen(_)) {
            let Some(ident) = self.try_next() else {
                self.errors.push(ParseError::ExpectedIdent {
                    span: fun.span(),
                    next: self.peek().span(),
                });
                return None;
            };
            params.push(ident);
            if matches!(self.peek(), Token::Comma(_)) {
                params.push_punct(self.expect());
            } else {
                break;
            }
        }

        let Some(rparen) = self.try_next() else {
            self.errors
                .push(ParseError::ExpectedRightParen(self.peek().span()));
            self.synchronize::<RightParen>();
            return None;
        };

        let body = self.block_stmt()?;

        Some(Stmt::FunDecl(FunDeclStmt {
            decoration: self.decorator.decorate(),
            fun,
            name,
            lparen,
            params,
            rparen,
            body,
        }))
    }

    fn statement(&mut self) -> Option<Stmt> {
        Some(match self.peek() {
            Token::If(_) => self.if_stmt()?.into(),
            Token::Print(_) => self.print_stmt()?.into(),
            Token::LeftBrace(_) => self.block_stmt()?.into(),
            _ => self.expr_stmt()?.into(),
        })
    }

    fn if_stmt(&mut self) -> Option<IfStmt> {
        let if_ = self.try_next()?;

        let group = self.grouped()?;
        let then = self.statement()?;

        let mut else_ = None;
        if let Some(else_token) = self.try_next() {
            else_ = Some((else_token, Box::new(self.statement()?)));
        }

        Some(IfStmt {
            if_,
            group,
            then: Box::new(then),
            else_,
        })
    }

    fn print_stmt(&mut self) -> Option<PrintStmt> {
        let print = self.try_next()?;
        let expr = self.expression()?;
        let Some(semi) = self.try_next() else {
            self.errors.push(ParseError::UnterminatedStatement {
                stmt: Span::across(&print, &expr),
                next: self.peek().span(),
            });
            return None;
        };

        Some(PrintStmt { print, expr, semi })
    }

    fn block_stmt(&mut self) -> Option<BlockStmt> {
        let lbrace = self.try_next::<LeftBrace>()?;
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

        let rbrace = self.try_next()?;

        Some(BlockStmt {
            lbrace,
            stmts,
            rbrace,
        })
    }

    fn expr_stmt(&mut self) -> Option<ExprStmt> {
        let expr = self.expression()?;
        let Some(semi) = self.try_next() else {
            self.errors.push(ParseError::UnterminatedStatement {
                stmt: expr.span(),
                next: self.peek().span(),
            });
            return None;
        };

        Some(ExprStmt { expr, semi })
    }

    fn expression(&mut self) -> Option<Expr> {
        self.assignment()
    }

    fn assignment(&mut self) -> Option<Expr> {
        let expr = self.equality()?;

        if let Some(equal) = self.try_next() {
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
            |this| match this.peek() {
                Token::BangEqual(_) | Token::EqualEqual(_) => {
                    Some(this.expect())
                }
                _ => None,
            },
            |this| this.comparison(),
        )
    }

    fn comparison(&mut self) -> Option<Expr> {
        self.parse_left_binary(
            |this| match this.peek() {
                Token::Greater(_)
                | Token::GreaterEqual(_)
                | Token::Less(_)
                | Token::LessEqual(_) => Some(this.expect()),
                _ => None,
            },
            |this| this.term(),
        )
    }

    fn term(&mut self) -> Option<Expr> {
        self.parse_left_binary(
            |this| match this.peek() {
                Token::Minus(_) | Token::Plus(_) => Some(this.expect()),
                _ => None,
            },
            |this| this.factor(),
        )
    }

    fn factor(&mut self) -> Option<Expr> {
        self.parse_left_binary(
            |this| match this.peek() {
                Token::Slash(_) | Token::Star(_) => Some(this.expect()),
                _ => None,
            },
            |this| this.unary(),
        )
    }

    fn unary(&mut self) -> Option<Expr> {
        if let Some(operator) = self.try_next() {
            Some(Expr::Unary(UnaryExpr {
                operator,
                inner: Box::new(self.unary()?),
            }))
        } else {
            self.call()
        }
    }

    fn call(&mut self) -> Option<Expr> {
        let expr = self.primary()?;
        if let Some(lparen) = self.try_next() {
            let mut arguments = Punctuated::new();
            while !matches!(self.peek(), Token::RightParen(_)) {
                arguments.push(self.expression()?);
                if matches!(self.peek(), Token::Comma(_)) {
                    arguments.push_punct(self.expect());
                } else {
                    break;
                }
            }

            let Some(rparen) = self.try_next() else {
                self.errors
                    .push(ParseError::ExpectedRightParen(self.peek().span()));
                self.synchronize::<RightParen>();
                return None;
            };

            Some(Expr::Call(CallExpr {
                function: Box::new(expr),
                lparen,
                arguments,
                rparen,
            }))
        } else {
            Some(expr)
        }
    }

    fn primary(&mut self) -> Option<Expr> {
        match self.peek() {
            Token::Number(_)
            | Token::String(_)
            | Token::True(_)
            | Token::False(_)
            | Token::Nil(_) => Some(Expr::Literal(LiteralExpr {
                literal: self.expect(),
            })),
            Token::LeftParen(_) => self.grouped().map(Expr::Grouping),
            Token::Identifier(_) => Some(Expr::Variable(VariableExpr {
                decoration: self.decorator.decorate(),
                ident: self.expect(),
            })),
            _ => {
                self.errors
                    .push(ParseError::ExpectedExpression(self.peek().span()));
                None
            }
        }
    }

    fn grouped(&mut self) -> Option<GroupingExpr> {
        let Some(lparen) = self.try_next() else {
            self.errors
                .push(ParseError::ExpectedLeftParen(self.peek().span()));
            return None;
        };

        let Some(inner) = self.expression() else {
            self.synchronize::<RightParen>();
            return None;
        };

        match self.peek() {
            Token::RightParen(_) => Some(GroupingExpr {
                lparen,
                inner: Box::new(inner),
                rparen: self.expect(),
            }),
            _ => {
                self.errors.push(ParseError::UnterminatedGroup(Span::across(
                    &lparen, &inner,
                )));

                self.synchronize::<RightParen>();

                None
            }
        }
    }

    fn synchronize<T: TokenKind>(&mut self) {
        while !matches!(self.peek(), Token::Eof(_)) {
            if T::matches_token(&self.next()) {
                break;
            }
        }
    }

    fn parse_left_binary(
        &mut self,
        mut operator: impl FnMut(&mut Self) -> Option<BinaryOperator>,
        mut operand: impl FnMut(&mut Self) -> Option<Expr>,
    ) -> Option<Expr> {
        let mut expr = operand(self)?;

        while let Some(operator) = operator(self) {
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
