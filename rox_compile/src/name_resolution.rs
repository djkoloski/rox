use core::{hash, mem::replace};
use std::collections::{HashMap, hash_map::Entry};

use rox_diag::{Span, Spanned as _};
use rox_lex::token_kind::Identifier;
use rox_parse::{
    Ast, LocalsCount, NameResolution, Visit as _,
    ast::{AssignExpr, BlockStmt, VarDeclStmt, VariableExpr, Visitor, visit},
};

use crate::error::CompileError;

pub struct NameResolutionOutput {
    pub resolutions: Vec<Resolution>,
    pub locals_counts: Vec<usize>,
    pub errors: Vec<CompileError>,
}

#[derive(Clone)]
pub enum Resolution {
    Local(usize),
    Global,
}

enum GlobalResolution {
    Pending,
    Resolved(Span),
}

struct Name<'a> {
    ident: &'a Identifier,
}

impl hash::Hash for Name<'_> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.ident.value.hash(state);
    }
}

impl PartialEq for Name<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.ident.value == other.ident.value
    }
}

impl Eq for Name<'_> {}

pub struct NameResolutionPass<'ast> {
    globals: HashMap<Name<'ast>, GlobalResolution>,
    scopes: Vec<HashMap<Name<'ast>, usize>>,
    len: usize,

    resolutions: Vec<Resolution>,
    locals_counts: Vec<usize>,
    errors: Vec<CompileError>,
}

impl<'ast> NameResolutionPass<'ast> {
    pub fn new() -> Self {
        Self {
            globals: HashMap::new(),
            scopes: Vec::new(),
            len: 0,

            resolutions: Vec::new(),
            locals_counts: Vec::new(),
            errors: Vec::new(),
        }
    }

    pub fn compile(mut self, ast: &'ast Ast) -> NameResolutionOutput {
        self.resolutions.resize(
            ast.decorator.count::<NameResolution>(),
            Resolution::Global,
        );
        self.locals_counts
            .resize(ast.decorator.count::<LocalsCount>(), 0);

        ast.program.accept(&mut self);

        for (name, resolution) in self.globals {
            if matches!(resolution, GlobalResolution::Pending) {
                self.errors.push(CompileError::UndefinedItem {
                    span: name.ident.span(),
                });
            }
        }

        NameResolutionOutput {
            resolutions: self.resolutions,
            locals_counts: self.locals_counts,
            errors: self.errors,
        }
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        let scope = self.scopes.pop().unwrap();
        self.len -= scope.len();
    }

    fn define_global(&mut self, ident: &'ast Identifier) {
        let name = Name { ident };
        let resolved = GlobalResolution::Resolved(ident.span());

        match self.globals.entry(name) {
            Entry::Vacant(vacant) => {
                vacant.insert(resolved);
            }
            Entry::Occupied(mut occupied) => {
                match replace(occupied.get_mut(), resolved) {
                    GlobalResolution::Pending => (),
                    GlobalResolution::Resolved(original) => {
                        self.errors.push(CompileError::ItemRedefined {
                            original,
                            redefinition: ident.span(),
                        });
                    }
                }
            }
        }
    }

    fn define_local(&mut self, ident: &'ast Identifier) {
        let name = Name { ident };

        let scope = self.scopes.last_mut().unwrap();
        if let Some((original, _)) = scope.get_key_value(&name) {
            self.errors.push(CompileError::ItemRedefined {
                original: original.ident.span(),
                redefinition: ident.span(),
            });
        } else {
            scope.insert(name, self.len);
            self.len += 1;
        }
    }

    fn define(&mut self, ident: &'ast Identifier) {
        if self.scopes.is_empty() {
            self.define_global(ident);
        } else {
            self.define_local(ident);
        }
    }

    fn resolve(&mut self, ident: &'ast Identifier) -> Resolution {
        let name = Name { ident };

        for scope in self.scopes.iter().rev() {
            if let Some(index) = scope.get(&name) {
                return Resolution::Local(*index);
            }
        }

        if let Entry::Vacant(vacant) = self.globals.entry(name) {
            vacant.insert(GlobalResolution::Pending);
        }

        Resolution::Global
    }
}

impl Default for NameResolutionPass<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'ast> Visitor<'ast> for NameResolutionPass<'ast> {
    fn visit_variable_expr(&mut self, node: &'ast VariableExpr) {
        self.resolutions[node.name_resolution.index()] =
            self.resolve(&node.ident);
    }

    fn visit_assign_expr(&mut self, node: &'ast AssignExpr) {
        visit::visit_assign_expr(self, node);

        self.resolutions[node.name_resolution.index()] =
            self.resolve(&node.ident);
    }

    fn visit_var_decl_stmt(&mut self, node: &'ast VarDeclStmt) {
        visit::visit_var_decl_stmt(self, node);

        self.define(&node.ident);
    }

    fn visit_block_stmt(&mut self, node: &'ast BlockStmt) {
        self.push_scope();

        visit::visit_block_stmt(self, node);

        self.locals_counts[node.locals_count.index()] =
            self.scopes.last().unwrap().len();

        self.pop_scope();
    }
}
