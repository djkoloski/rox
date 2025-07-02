use crate::{
    ast::{decoration::Decoration, punctuated::Punctuated},
    scanner::{
        And, Bang, BangEqual, Comma, Equal, EqualEqual, False, Greater,
        GreaterEqual, Identifier, LeftParen, Less, LessEqual, Minus, Nil,
        Number, Or, Plus, RightParen, Slash, Star, String, True,
    },
};

pub trait ExprVisitor {
    type Output;

    fn visit_literal_expr(&mut self, expr: &LiteralExpr) -> Self::Output;
    fn visit_unary_expr(&mut self, expr: &UnaryExpr) -> Self::Output;
    fn visit_binary_expr(&mut self, expr: &BinaryExpr) -> Self::Output;
    fn visit_grouping_expr(&mut self, expr: &GroupingExpr) -> Self::Output;
    fn visit_variable_expr(&mut self, expr: &VariableExpr) -> Self::Output;
    fn visit_assign_expr(&mut self, expr: &AssignExpr) -> Self::Output;
    fn visit_call_expr(&mut self, expr: &CallExpr) -> Self::Output;
}

pub trait VisitExpr<V: ExprVisitor> {
    fn accept(&self, visitor: &mut V) -> V::Output;
}

ast_node! {
    #[visit(VisitExpr, ExprVisitor)]
    pub enum Expr {
        Literal(LiteralExpr),
        Unary(UnaryExpr),
        Binary(BinaryExpr),
        Grouping(GroupingExpr),
        Variable(VariableExpr),
        Assign(AssignExpr),
        Call(CallExpr),
    }

    #[visit(VisitExpr, ExprVisitor::visit_literal_expr)]
    pub struct LiteralExpr {
        pub literal: Literal,
    }

    #[token]
    pub enum Literal {
        Number(Number),
        String(String),
        True(True),
        False(False),
        Nil(Nil),
    }

    #[visit(VisitExpr, ExprVisitor::visit_unary_expr)]
    pub struct UnaryExpr {
        pub operator: UnaryOperator,
        pub inner: Box<Expr>,
    }

    #[token]
    pub enum UnaryOperator {
        Bang(Bang),
        Minus(Minus),
    }

    #[visit(VisitExpr, ExprVisitor::visit_binary_expr)]
    pub struct BinaryExpr {
        pub left: Box<Expr>,
        pub operator: BinaryOperator,
        pub right: Box<Expr>,
    }

    #[token]
    pub enum BinaryOperator {
        And(And),
        Or(Or),
        Greater(Greater),
        GreaterEqual(GreaterEqual),
        Less(Less),
        LessEqual(LessEqual),
        BangEqual(BangEqual),
        EqualEqual(EqualEqual),
        Minus(Minus),
        Plus(Plus),
        Slash(Slash),
        Star(Star),
    }

    #[visit(
        VisitExpr,
        ExprVisitor,
        fn accept(self, visitor) { self.inner.accept(visitor) },
    )]
    pub struct GroupingExpr {
        pub lparen: LeftParen,
        pub inner: Box<Expr>,
        pub rparen: RightParen,
    }

    #[visit(VisitExpr, ExprVisitor::visit_variable_expr)]
    pub struct VariableExpr {
        pub decoration: Decoration,
        pub ident: Identifier,
    }

    #[visit(VisitExpr, ExprVisitor::visit_assign_expr)]
    pub struct AssignExpr {
        pub decoration: Decoration,
        pub ident: Identifier,
        pub equal: Equal,
        pub expr: Box<Expr>,
    }

    #[visit(VisitExpr, ExprVisitor::visit_call_expr)]
    pub struct CallExpr {
        pub function: Box<Expr>,
        pub lparen: LeftParen,
        pub arguments: Punctuated<Expr, Comma>,
        pub rparen: RightParen,
    }
}
