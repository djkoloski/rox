use rox_parse::{
    Ast, Dec, FunctionLabel, Visit as _,
    ast::{FunDeclStmt, Visitor, visit},
};

pub struct CollectFunDeclsPass<'ast> {
    functions: Dec<&'ast FunDeclStmt, FunctionLabel>,
}

impl<'ast> CollectFunDeclsPass<'ast> {
    pub fn compile(ast: &'ast Ast) -> Vec<&'ast FunDeclStmt> {
        let mut pass = Self {
            functions: Dec::new(&ast.decorator),
        };

        ast.program.accept(&mut pass);

        pass.functions.unwrap()
    }
}

impl<'ast> Visitor<'ast> for CollectFunDeclsPass<'ast> {
    fn visit_fun_decl_stmt(&mut self, node: &'ast FunDeclStmt) {
        visit::visit_fun_decl_stmt(self, node);

        self.functions.insert(&node.function_label, node);
    }
}
