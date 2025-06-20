use crate::scanner::Token;

pub trait ExprVisitor {
    type Output;

    fn visit_literal_expr(&mut self, expr: &LiteralExpr) -> Self::Output;
    fn visit_unary_expr(&mut self, expr: &UnaryExpr) -> Self::Output;
    fn visit_binary_expr(&mut self, expr: &BinaryExpr) -> Self::Output;
    fn visit_grouping_expr(&mut self, expr: &GroupingExpr) -> Self::Output;
    fn visit_variable_expr(&mut self, expr: &VariableExpr) -> Self::Output;
    fn visit_assign_expr(&mut self, expr: &AssignExpr) -> Self::Output;
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
    }

    #[visit(VisitExpr, ExprVisitor::visit_literal_expr)]
    pub struct LiteralExpr {
        pub token: Token,
    }

    #[visit(VisitExpr, ExprVisitor::visit_unary_expr)]
    pub struct UnaryExpr {
        pub operator: Token,
        pub inner: Box<Expr>,
    }

    #[visit(VisitExpr, ExprVisitor::visit_binary_expr)]
    pub struct BinaryExpr {
        pub left: Box<Expr>,
        pub operator: Token,
        pub right: Box<Expr>,
    }

    #[visit(
        VisitExpr,
        ExprVisitor,
        fn accept(self, visitor) { self.inner.accept(visitor) },
    )]
    pub struct GroupingExpr {
        pub lparen: Token,
        pub inner: Box<Expr>,
        pub rparen: Token,
    }

    #[visit(VisitExpr, ExprVisitor::visit_variable_expr)]
    pub struct VariableExpr {
        pub ident: Token,
    }

    #[visit(VisitExpr, ExprVisitor::visit_assign_expr)]
    pub struct AssignExpr {
        pub ident: Token,
        pub equal: Token,
        pub expr: Box<Expr>,
    }
}
