use std::collections::VecDeque;

use rox_diag::{Span, Spanned as _};
use rox_lex::{Token, TokenKind, token_kind::*};

use crate::{
    Decorator, ParseError, Punctuated,
    ast::{
        AssignExpr, Assignment, BinaryExpr, BinaryOperator, BlockStmt,
        CallExpr, ClassDeclStmt, ElseClause, Expr, ExprStmt, FunDeclStmt,
        Function, GetExpr, GroupingExpr, IfStmt, Inheritance, Literal,
        LiteralExpr, Method, Name, PrintStmt, Program, ReturnStmt, SetExpr,
        Stmt, SuperExpr, ThisExpr, UnaryExpr, VarDeclStmt, VariableExpr,
        WhileStmt,
    },
};

pub struct Ast {
    pub program: Program,
    pub decorator: Decorator,
}

pub struct ParseOutput {
    pub ast: Option<Ast>,
    pub errors: Vec<ParseError>,
}

pub struct Parser {
    tokens: VecDeque<Token>,
    decorator: Decorator,
    errors: Vec<ParseError>,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens: VecDeque::from(tokens),
            decorator: Decorator::new(),
            errors: Vec::new(),
        }
    }

    pub fn parse(mut self) -> ParseOutput {
        let program = self.program();
        self.into_output(program)
    }

    pub fn parse_repl(mut self) -> ParseOutput {
        let program = self.repl();
        self.into_output(program)
    }

    fn into_output(self, program: Option<Program>) -> ParseOutput {
        ParseOutput {
            ast: program.map(|program| Ast {
                program,
                decorator: self.decorator,
            }),
            errors: self.errors,
        }
    }

    fn peek(&self) -> &Token {
        self.tokens.front().unwrap()
    }

    fn advance(&mut self) -> Token {
        self.tokens.pop_front().unwrap()
    }

    fn expect<T: TokenKind>(&mut self) -> Option<T> {
        if T::matches_token(self.peek()) {
            Some(T::from_token(self.advance()))
        } else {
            None
        }
    }

    fn assume<T: TokenKind>(&mut self) -> T {
        T::from_token(self.advance())
    }

    fn name(&mut self, identifier: Identifier) -> Name {
        Name {
            decoration: self.decorator.decorate(),
            identifier,
        }
    }

    fn program(&mut self) -> Option<Program> {
        let mut stmts = Vec::new();

        loop {
            if let Some(eof) = self.expect() {
                break Some(Program { stmts, eof });
            } else if let Some(stmt) = self.declaration() {
                stmts.push(stmt);
            } else {
                self.synchronize::<Semicolon>();
            }
        }
    }

    fn repl(&mut self) -> Option<Program> {
        let stmt = match self.peek() {
            Token::Var(_)
            | Token::Fun(_)
            | Token::Class(_)
            | Token::If(_)
            | Token::Print(_)
            | Token::While(_)
            | Token::For(_)
            | Token::Return(_)
            | Token::LeftBrace(_) => self.declaration()?,
            _ => Stmt::Print(PrintStmt {
                print: Print { span: Span::null() },
                expr: self.expression()?,
                semi: Semicolon { span: Span::null() },
            }),
        };

        Some(Program {
            stmts: vec![stmt],
            eof: Eof { span: Span::null() },
        })
    }

    fn declaration(&mut self) -> Option<Stmt> {
        match self.peek() {
            Token::Var(_) => Some(self.var_decl_stmt()?.into()),
            Token::Fun(_) => Some(self.fun_decl_stmt()?.into()),
            Token::Class(_) => Some(self.class_decl_stmt()?.into()),
            _ => self.statement(),
        }
    }

    fn var_decl_stmt(&mut self) -> Option<VarDeclStmt> {
        let var = self.expect::<Var>()?;
        let Some(identifier) = self.expect() else {
            self.errors
                .push(ParseError::ExpectedIdent(self.peek().span()));
            return None;
        };

        let assignment = if let Some(equal) = self.expect() {
            let expr = self.expression()?;
            Some(Assignment { equal, expr })
        } else {
            None
        };

        let Some(semi) = self.expect() else {
            let stmt = if let Some(Assignment { expr, .. }) = &assignment {
                Span::across(&var, expr)
            } else {
                Span::across(&var, &identifier)
            };

            self.errors.push(ParseError::UnterminatedStatement {
                stmt,
                next: self.peek().span(),
            });
            return None;
        };

        Some(VarDeclStmt {
            var,
            identifier,
            assignment,
            semi,
        })
    }

    fn fun_decl_stmt(&mut self) -> Option<FunDeclStmt> {
        let fun = self.expect()?;

        let Some(identifier) = self.expect() else {
            self.errors
                .push(ParseError::ExpectedIdent(self.peek().span()));
            return None;
        };

        let function = self.function()?;

        Some(FunDeclStmt {
            fun,
            identifier,
            function,
        })
    }

    fn class_decl_stmt(&mut self) -> Option<ClassDeclStmt> {
        let class = self.expect()?;
        let Some(identifier) = self.expect() else {
            self.errors
                .push(ParseError::ExpectedIdent(self.peek().span()));
            return None;
        };

        let mut inheritance = None;
        if let Some(less) = self.expect() {
            let Some(superclass) = self.expect() else {
                self.errors
                    .push(ParseError::ExpectedIdent(self.peek().span()));
                return None;
            };
            inheritance = Some(Inheritance {
                less,
                superclass: self.name(superclass),
            });
        }

        let Some(lbrace) = self.expect::<LeftBrace>() else {
            self.errors
                .push(ParseError::ExpectedLeftBrace(self.peek().span()));
            return None;
        };

        let mut methods = Vec::new();
        while !matches!(self.peek(), Token::RightBrace(_)) {
            let Some(identifier) = self.expect() else {
                self.errors
                    .push(ParseError::ExpectedIdent(self.peek().span()));
                return None;
            };
            let function = self.function()?;

            methods.push(Method {
                identifier,
                function,
            });
        }

        let Some(rbrace) = self.expect() else {
            self.errors.push(ParseError::UnterminatedBlock {
                start: lbrace.span(),
                end: self.peek().span(),
            });
            return None;
        };

        Some(ClassDeclStmt {
            class_decoration: self.decorator.decorate(),
            class,
            identifier,
            inheritance,
            block_decoration: self.decorator.decorate(),
            lbrace,
            methods,
            rbrace,
        })
    }

    fn function(&mut self) -> Option<Function> {
        let Some(lparen) = self.expect::<LeftParen>() else {
            self.errors
                .push(ParseError::ExpectedLeftParen(self.peek().span()));
            return None;
        };

        let mut params = Punctuated::new();
        while !matches!(self.peek(), Token::RightParen(_)) {
            let Some(ident) = self.expect() else {
                self.errors
                    .push(ParseError::ExpectedIdent(self.peek().span()));
                return None;
            };
            params.push(ident);
            if let Some(comma) = self.expect() {
                params.push_punct(comma);
            } else {
                break;
            }
        }

        let Some(rparen) = self.expect() else {
            self.errors.push(ParseError::UnterminatedGroup {
                start: lparen.span(),
                end: self.peek().span(),
            });
            self.synchronize::<RightParen>();
            return None;
        };

        let body = self.block_stmt()?;

        Some(Function {
            decoration: self.decorator.decorate(),
            lparen,
            params,
            rparen,
            body,
        })
    }

    fn statement(&mut self) -> Option<Stmt> {
        Some(match self.peek() {
            Token::If(_) => self.if_stmt()?.into(),
            Token::Print(_) => self.print_stmt()?.into(),
            Token::While(_) => self.while_stmt()?.into(),
            Token::For(_) => self.for_stmt()?,
            Token::Return(_) => self.return_stmt()?.into(),
            Token::LeftBrace(_) => self.block_stmt()?.into(),
            _ => self.expr_stmt()?.into(),
        })
    }

    fn if_stmt(&mut self) -> Option<IfStmt> {
        let if_ = self.expect()?;

        let group = self.grouped()?;
        let then = self.statement()?;

        let mut else_clause = None;
        if let Some(else_) = self.expect() {
            else_clause = Some(ElseClause {
                else_,
                stmt: Box::new(self.statement()?),
            });
        }

        Some(IfStmt {
            if_,
            condition: group,
            stmt: Box::new(then),
            else_clause,
        })
    }

    fn print_stmt(&mut self) -> Option<PrintStmt> {
        let print = self.expect()?;
        let expr = self.expression()?;
        let Some(semi) = self.expect() else {
            self.errors.push(ParseError::UnterminatedStatement {
                stmt: Span::across(&print, &expr),
                next: self.peek().span(),
            });
            return None;
        };

        Some(PrintStmt { print, expr, semi })
    }

    fn while_stmt(&mut self) -> Option<WhileStmt> {
        let while_ = self.expect()?;
        let Some(lparen) = self.expect::<LeftParen>() else {
            self.errors
                .push(ParseError::ExpectedLeftParen(self.peek().span()));
            return None;
        };
        let expr = self.expression()?;
        let Some(rparen) = self.expect() else {
            self.errors.push(ParseError::UnterminatedGroup {
                start: lparen.span(),
                end: self.peek().span(),
            });
            return None;
        };
        let body = self.statement()?;

        Some(WhileStmt {
            while_,
            lparen,
            expr,
            rparen,
            body: Box::new(body),
        })
    }

    fn for_stmt(&mut self) -> Option<Stmt> {
        let for_ = self.expect::<For>()?;
        let Some(lparen) = self.expect::<LeftParen>() else {
            self.errors
                .push(ParseError::ExpectedLeftParen(self.peek().span()));
            return None;
        };

        let initializer = match self.peek() {
            Token::Semicolon(_) => {
                self.advance();
                None
            }
            Token::Var(_) => Some(self.var_decl_stmt()?.into()),
            _ => Some(self.expr_stmt()?.into()),
        };

        let expr = match self.peek() {
            Token::Semicolon(_) => Expr::Literal(LiteralExpr {
                literal: Literal::True(True { span: for_.span() }),
            }),
            _ => {
                let expr = self.expression()?;
                let Some(_) = self.expect::<Semicolon>() else {
                    self.errors.push(ParseError::ExpectedSemicolon(
                        self.peek().span(),
                    ));
                    return None;
                };
                expr
            }
        };

        let increment = match self.peek() {
            Token::RightParen(_) => None,
            _ => Some(self.expression()?),
        };

        let Some(rparen) = self.expect() else {
            self.errors.push(ParseError::UnterminatedGroup {
                start: lparen.span(),
                end: self.peek().span(),
            });
            return None;
        };

        let mut body = self.statement()?;
        if let Some(increment) = increment {
            body = Stmt::Block(BlockStmt {
                decoration: self.decorator.decorate(),
                lbrace: LeftBrace { span: for_.span() },
                stmts: vec![
                    body,
                    Stmt::Expr(ExprStmt {
                        expr: increment,
                        semi: Semicolon { span: for_.span() },
                    }),
                ],
                rbrace: RightBrace { span: for_.span() },
            });
        }

        let mut stmts = Vec::new();
        if let Some(initializer) = initializer {
            stmts.push(initializer);
        }
        stmts.push(
            WhileStmt {
                while_: While { span: for_.span() },
                lparen,
                expr,
                rparen,
                body: Box::new(body),
            }
            .into(),
        );

        Some(Stmt::Block(BlockStmt {
            decoration: self.decorator.decorate(),
            lbrace: LeftBrace { span: for_.span() },
            stmts,
            rbrace: RightBrace { span: for_.span() },
        }))
    }

    fn return_stmt(&mut self) -> Option<ReturnStmt> {
        let return_ = self.expect()?;
        let mut expr = None;
        if !matches!(self.peek(), Token::Semicolon(_)) {
            expr = Some(self.expression()?);
        }
        let Some(semi) = self.expect() else {
            self.errors
                .push(ParseError::ExpectedSemicolon(self.peek().span()));
            return None;
        };
        Some(ReturnStmt {
            return_,
            expr,
            semi,
        })
    }

    fn block_stmt(&mut self) -> Option<BlockStmt> {
        let lbrace = self.expect::<LeftBrace>()?;
        let mut stmts = Vec::new();

        loop {
            match self.peek() {
                Token::RightBrace(_) => break,
                Token::Eof(_) => {
                    self.errors.push(ParseError::UnterminatedBlock {
                        start: lbrace.span(),
                        end: self.peek().span(),
                    });
                    return None;
                }
                _ => stmts.push(self.declaration()?),
            }
        }

        let rbrace = self.expect()?;

        Some(BlockStmt {
            decoration: self.decorator.decorate(),
            lbrace,
            stmts,
            rbrace,
        })
    }

    fn expr_stmt(&mut self) -> Option<ExprStmt> {
        let expr = self.expression()?;
        let Some(semi) = self.expect() else {
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
        let expr = self.logic_or()?;

        if let Some(equal) = self.expect() {
            let value = self.assignment()?;

            match expr {
                Expr::Get(GetExpr { target, dot, field }) => {
                    Some(Expr::Set(SetExpr {
                        target,
                        dot,
                        field,
                        equal,
                        expr: Box::new(value),
                    }))
                }
                Expr::Variable(VariableExpr { name }) => {
                    Some(Expr::Assign(AssignExpr {
                        name,
                        equal,
                        expr: Box::new(value),
                    }))
                }
                _ => {
                    self.errors
                        .push(ParseError::InvalidAssignmentTarget(expr.span()));
                    None
                }
            }
        } else {
            Some(expr)
        }
    }

    fn logic_or(&mut self) -> Option<Expr> {
        let mut expr = self.logic_and()?;
        while let Some(or) = self.expect() {
            let rhs = self.logic_and()?;
            expr = Expr::Binary(BinaryExpr {
                left: Box::new(expr),
                operator: BinaryOperator::Or(or),
                right: Box::new(rhs),
            });
        }
        Some(expr)
    }

    fn logic_and(&mut self) -> Option<Expr> {
        let mut expr = self.equality()?;
        while let Some(and) = self.expect() {
            let rhs = self.equality()?;
            expr = Expr::Binary(BinaryExpr {
                left: Box::new(expr),
                operator: BinaryOperator::And(and),
                right: Box::new(rhs),
            });
        }
        Some(expr)
    }

    fn equality(&mut self) -> Option<Expr> {
        self.parse_left_binary(
            |this| match this.peek() {
                Token::BangEqual(_) | Token::EqualEqual(_) => {
                    Some(this.assume())
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
                | Token::LessEqual(_) => Some(this.assume()),
                _ => None,
            },
            |this| this.term(),
        )
    }

    fn term(&mut self) -> Option<Expr> {
        self.parse_left_binary(
            |this| match this.peek() {
                Token::Minus(_) | Token::Plus(_) => Some(this.assume()),
                _ => None,
            },
            |this| this.factor(),
        )
    }

    fn factor(&mut self) -> Option<Expr> {
        self.parse_left_binary(
            |this| match this.peek() {
                Token::Slash(_) | Token::Star(_) => Some(this.assume()),
                _ => None,
            },
            |this| this.unary(),
        )
    }

    fn unary(&mut self) -> Option<Expr> {
        if let Some(operator) = self.expect() {
            Some(Expr::Unary(UnaryExpr {
                operator,
                inner: Box::new(self.unary()?),
            }))
        } else {
            self.call()
        }
    }

    fn call(&mut self) -> Option<Expr> {
        let mut expr = self.primary()?;

        loop {
            match self.peek() {
                Token::LeftParen(_) => {
                    let lparen = self.assume::<LeftParen>();
                    let mut arguments = Punctuated::new();
                    while !matches!(self.peek(), Token::RightParen(_)) {
                        arguments.push(self.expression()?);
                        if let Some(comma) = self.expect() {
                            arguments.push_punct(comma);
                        } else {
                            break;
                        }
                    }

                    let Some(rparen) = self.expect() else {
                        self.errors.push(ParseError::UnterminatedGroup {
                            start: lparen.span(),
                            end: self.peek().span(),
                        });
                        self.synchronize::<RightParen>();
                        return None;
                    };

                    expr = Expr::Call(CallExpr {
                        target: Box::new(expr),
                        lparen,
                        arguments,
                        rparen,
                    });
                }
                Token::Dot(_) => {
                    let dot = self.assume::<Dot>();
                    let Some(field) = self.expect() else {
                        self.errors.push(ParseError::ExpectedIdent(
                            self.peek().span(),
                        ));
                        return None;
                    };

                    expr = Expr::Get(GetExpr {
                        target: Box::new(expr),
                        dot,
                        field,
                    });
                }
                _ => break,
            }
        }

        Some(expr)
    }

    fn primary(&mut self) -> Option<Expr> {
        match self.peek() {
            Token::FloatLiteral(_)
            | Token::StringLiteral(_)
            | Token::True(_)
            | Token::False(_)
            | Token::Nil(_) => Some(Expr::Literal(LiteralExpr {
                literal: self.assume(),
            })),
            Token::LeftParen(_) => self.grouped().map(Expr::Grouping),
            Token::Identifier(_) => {
                let identifier = self.assume();
                Some(Expr::Variable(VariableExpr {
                    name: self.name(identifier),
                }))
            }
            Token::This(_) => Some(Expr::This(ThisExpr {
                decoration: self.decorator.decorate(),
                this: self.assume(),
            })),
            Token::Super(_) => {
                let super_ = self.assume();
                let Some(dot) = self.expect() else {
                    self.errors
                        .push(ParseError::ExpectedDot(self.peek().span()));
                    return None;
                };
                let Some(field) = self.expect() else {
                    self.errors
                        .push(ParseError::ExpectedIdent(self.peek().span()));
                    return None;
                };
                Some(Expr::Super(SuperExpr {
                    decoration: self.decorator.decorate(),
                    super_,
                    dot,
                    field,
                }))
            }
            _ => {
                self.errors
                    .push(ParseError::ExpectedExpression(self.peek().span()));
                None
            }
        }
    }

    fn grouped(&mut self) -> Option<GroupingExpr> {
        let Some(lparen) = self.expect() else {
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
                rparen: self.assume(),
            }),
            _ => {
                self.errors.push(ParseError::UnterminatedGroup {
                    start: lparen.span(),
                    end: self.peek().span(),
                });

                self.synchronize::<RightParen>();

                None
            }
        }
    }

    fn synchronize<T: TokenKind>(&mut self) {
        while !matches!(self.peek(), Token::Eof(_)) {
            if T::matches_token(&self.advance()) {
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
