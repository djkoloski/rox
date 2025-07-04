pub struct Printer<'t> {
    text: &'t str,
    result: String,
}

impl Visitor for Printer<'_> {
    fn visit_expr_stmt(&mut self, stmt: &ExprStmt) {
        stmt.expr.accept(self);
        self.result = format!("(expr {})", self.result);
    }

    fn visit_print_stmt(&mut self, stmt: &PrintStmt) {
        stmt.expr.accept(self);
        self.result = format!("(print {})", self.result);
    }

    fn visit_literal_expr(&mut self, expr: &LiteralExpr) {
        self.result = match &expr.token.kind {
            TokenKind::True => "true".to_string(),
            TokenKind::False => "false".to_string(),
            TokenKind::Nil => "nil".to_string(),
            TokenKind::String(s) => format!("\"{s}\""),
            TokenKind::Number(x) => format!("{x}"),
            _ => panic!("invalid literal expression"),
        };
    }

    fn visit_unary_expr(&mut self, expr: &UnaryExpr) {
        expr.inner.accept(self);
        self.result = format!(
            "({} {})",
            expr.operator.span.get(self.text),
            self.result,
        );
    }

    fn visit_binary_expr(&mut self, expr: &BinaryExpr) {
        use core::mem::take;

        expr.left.accept(self);
        let left = take(&mut self.result);
        expr.right.accept(self);
        let right = take(&mut self.result);

        self.result = format!(
            "({} {} {})",
            expr.operator.span.get(self.text),
            left,
            right,
        );
    }

    fn visit_grouping_expr(&mut self, expr: &GroupingExpr) {
        expr.inner.accept(self);
        self.result = format!("(group {})", self.result);
    }
}
