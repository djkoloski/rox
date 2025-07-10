use core::{hash, mem::replace};
use std::collections::{HashMap, hash_map::Entry};

use rox_diag::{Span, Spanned as _};
use rox_lex::token_kind::Identifier;
use rox_parse::{
    Ast, Dec, LocalsCount, NameResolution, Visit as _,
    ast::{
        AssignExpr, BlockStmt, FunDeclStmt, VarDeclStmt, VariableExpr, Visitor,
        visit,
    },
};
use rox_vm::global_names;

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

enum Name<'ast> {
    Ident(&'ast Identifier),
    String(&'ast str),
}

impl<'ast> Name<'ast> {
    fn as_str(&self) -> &'ast str {
        match self {
            Self::Ident(ident) => &ident.value,
            Self::String(string) => string,
        }
    }

    fn span(&self) -> Span {
        match self {
            Self::Ident(ident) => ident.span(),
            Self::String(_) => Span::null(),
        }
    }
}

impl hash::Hash for Name<'_> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}

impl PartialEq for Name<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl Eq for Name<'_> {}

pub struct NameResolutionPass<'ast> {
    globals: HashMap<Name<'ast>, GlobalResolution>,
    scopes: Vec<HashMap<Name<'ast>, usize>>,
    len: usize,

    resolutions: Dec<Resolution, NameResolution>,
    locals_counts: Dec<usize, LocalsCount>,
    errors: Vec<CompileError>,
}

impl<'ast> NameResolutionPass<'ast> {
    pub fn compile(ast: &'ast Ast) -> NameResolutionOutput {
        let mut pass = Self {
            globals: global_names()
                .map(|string| {
                    (
                        Name::String(string),
                        GlobalResolution::Resolved(Span::null()),
                    )
                })
                .collect(),
            scopes: Vec::new(),
            len: 0,

            resolutions: Dec::new(&ast.decorator),
            locals_counts: Dec::new(&ast.decorator),
            errors: Vec::new(),
        };

        ast.program.accept(&mut pass);

        for (name, resolution) in pass.globals {
            if matches!(resolution, GlobalResolution::Pending) {
                pass.errors
                    .push(CompileError::UndefinedItem { span: name.span() });
            }
        }

        NameResolutionOutput {
            resolutions: pass.resolutions.unwrap(),
            locals_counts: pass.locals_counts.unwrap(),
            errors: pass.errors,
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
        let name = Name::Ident(ident);
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

    fn define_local(&mut self, name: Name<'ast>) {
        let scope = self.scopes.last_mut().unwrap();
        if let Some((original, _)) = scope.get_key_value(&name) {
            self.errors.push(CompileError::ItemRedefined {
                original: original.span(),
                redefinition: name.span(),
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
            self.define_local(Name::Ident(ident));
        }
    }

    fn resolve(&mut self, name: Name<'ast>) -> Resolution {
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

impl<'ast> Visitor<'ast> for NameResolutionPass<'ast> {
    fn visit_variable_expr(&mut self, node: &'ast VariableExpr) {
        let resolution = self.resolve(Name::Ident(&node.ident));
        self.resolutions.insert(&node.name_resolution, resolution);
    }

    fn visit_assign_expr(&mut self, node: &'ast AssignExpr) {
        visit::visit_assign_expr(self, node);

        let resolution = self.resolve(Name::Ident(&node.ident));
        self.resolutions.insert(&node.name_resolution, resolution);
    }

    fn visit_var_decl_stmt(&mut self, node: &'ast VarDeclStmt) {
        visit::visit_var_decl_stmt(self, node);

        self.define(&node.ident);
    }

    fn visit_block_stmt(&mut self, node: &'ast BlockStmt) {
        self.push_scope();

        visit::visit_block_stmt(self, node);

        self.locals_counts
            .insert(&node.locals_count, self.scopes.last().unwrap().len());

        self.pop_scope();
    }

    fn visit_fun_decl_stmt(&mut self, node: &'ast FunDeclStmt) {
        self.define(&node.function.name);

        self.push_scope();

        self.define_local(Name::String("this"));

        for param in node.function.params.iter() {
            self.define(param);
        }

        visit::visit_block_stmt(self, &node.function.body);

        self.locals_counts.insert(
            &node.function.body.locals_count,
            self.scopes.last().unwrap().len(),
        );

        self.pop_scope();
    }
}
