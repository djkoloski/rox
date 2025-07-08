use core::hash;
use std::collections::HashMap;

use rox_diag::Spanned as _;
use rox_lex::token_kind::Identifier;

use crate::error::CompileError;

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

pub struct NameResolution<'a> {
    scopes: Vec<HashMap<Name<'a>, usize>>,
    len: usize,
}

impl<'a> NameResolution<'a> {
    pub fn new() -> Self {
        Self {
            scopes: Vec::new(),
            len: 0,
        }
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn pop_scope(&mut self) {
        let scope = self.scopes.pop().unwrap();
        self.len -= scope.len();
    }

    pub fn is_global(&self) -> bool {
        self.scopes.is_empty()
    }

    pub fn define(
        &mut self,
        ident: &'a Identifier,
    ) -> Result<(), CompileError> {
        if self.scopes.is_empty() {
            return Ok(());
        }

        let name = Name { ident };

        let scope = self.scopes.last_mut().unwrap();
        if let Some((original, _)) = scope.get_key_value(&name) {
            return Err(CompileError::LocalRedefined {
                original: original.ident.span(),
                redefinition: ident.span(),
            });
        }
        scope.insert(name, self.len);
        self.len += 1;

        Ok(())
    }

    pub fn resolve(&mut self, ident: &'a Identifier) -> Option<usize> {
        let name = Name { ident };

        for scope in self.scopes.iter().rev() {
            if let Some(index) = scope.get(&name) {
                return Some(*index);
            }
        }

        None
    }

    pub fn local_scope_len(&self) -> usize {
        self.scopes.last().map(|scope| scope.len()).unwrap_or(0)
    }
}
