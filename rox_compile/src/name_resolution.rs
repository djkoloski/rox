use core::hash;
use std::collections::{HashMap, hash_map::Entry};

use rox_diag::{Span, Spanned as _};
use rox_lex::token_kind::Identifier;
use rox_parse::{
    Ast, BlockDecoration, Dec, FunctionDecoration, NameDecoration, Visit as _,
    ast::{
        AssignExpr, BlockStmt, FunDeclStmt, Function, Name, VarDeclStmt,
        VariableExpr, Visitor, visit,
    },
};
use rox_vm::{Place, global_names};

use crate::error::CompileError;

#[derive(Clone)]
pub enum Resolution {
    // A local variable
    Local { local_index: usize },
    // An upvalue from the current closure
    Upvalue { upvalue_index: usize },
    // A global variable
    Global,
}

impl From<Place> for Resolution {
    fn from(value: Place) -> Self {
        match value {
            Place::Local { local_index } => Self::Local { local_index },
            Place::Upvalue { upvalue_index } => Self::Upvalue { upvalue_index },
        }
    }
}

pub enum LocalDeclaration {
    // A local variable
    Local,
    // A captured variable
    Capture,
}

pub struct FunctionInfo<'ast> {
    pub identifier: &'ast Identifier,
    pub function: &'ast Function,
    pub captures: Vec<Place>,
}

pub struct NameResolutionOutput<'ast> {
    pub resolutions: Vec<Resolution>,
    pub block_locals: Vec<Vec<LocalDeclaration>>,
    pub function_infos: Vec<FunctionInfo<'ast>>,
    pub errors: Vec<CompileError>,
}

struct Ident<'ast> {
    identifier: &'ast Identifier,
}

impl<'ast> Ident<'ast> {
    fn as_str(&self) -> &'ast str {
        &self.identifier.value
    }

    fn span(&self) -> Span {
        self.identifier.span()
    }
}

impl hash::Hash for Ident<'_> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}

impl PartialEq for Ident<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl Eq for Ident<'_> {}

struct Scope<'ast> {
    block: &'ast BlockStmt,
    ident_to_local_decl: HashMap<Ident<'ast>, usize>,
    local_decls: Vec<LocalDeclaration>,
}

impl<'ast> Scope<'ast> {
    fn new(block: &'ast BlockStmt) -> Self {
        Self {
            block,
            ident_to_local_decl: HashMap::new(),
            local_decls: Vec::new(),
        }
    }

    fn resolve(
        &mut self,
        identifier: &'ast Identifier,
        and_capture: bool,
    ) -> Option<usize> {
        if let Some(&base_index) =
            self.ident_to_local_decl.get(&Ident { identifier })
        {
            if and_capture {
                self.local_decls[base_index] = LocalDeclaration::Capture;
            }
            Some(base_index)
        } else {
            None
        }
    }

    fn declare(
        &mut self,
        identifier: &'ast Identifier,
    ) -> Result<usize, CompileError> {
        match self.ident_to_local_decl.entry(Ident { identifier }) {
            Entry::Occupied(occupied) => Err(CompileError::ItemRedefined {
                original: occupied.key().span(),
                redefinition: identifier.span(),
            }),
            Entry::Vacant(vacant) => {
                let base_index = self.local_decls.len();
                vacant.insert(base_index);
                self.local_decls.push(LocalDeclaration::Local);
                Ok(base_index)
            }
        }
    }
}

struct Context<'ast> {
    scopes: Vec<Scope<'ast>>,
    local_count: usize,
}

impl<'ast> Context<'ast> {
    fn empty() -> Self {
        Self::with_reserved(0)
    }

    fn with_reserved(local_count: usize) -> Self {
        Self {
            scopes: Vec::new(),
            local_count,
        }
    }

    fn push_scope(&mut self, block: &'ast BlockStmt) {
        self.scopes.push(Scope::new(block));
    }

    fn pop_scope(&mut self) -> Option<Scope<'ast>> {
        let scope = self.scopes.pop()?;
        self.local_count -= scope.local_decls.len();
        Some(scope)
    }

    fn resolve(
        &mut self,
        identifier: &'ast Identifier,
        and_capture: bool,
    ) -> Option<usize> {
        let mut base_index = self.local_count;
        for scope in self.scopes.iter_mut().rev() {
            base_index -= scope.local_decls.len();

            if let Some(local_index) = scope.resolve(identifier, and_capture) {
                return Some(base_index + local_index);
            }
        }

        None
    }

    fn declare(
        &mut self,
        identifier: &'ast Identifier,
    ) -> Option<Result<(), CompileError>> {
        if let Some(scope) = self.scopes.last_mut() {
            if let Err(error) = scope.declare(identifier) {
                Some(Err(error))
            } else {
                self.local_count += 1;
                Some(Ok(()))
            }
        } else {
            None
        }
    }
}

struct Frame<'ast> {
    function: &'ast Function,
    context: Context<'ast>,
    name_to_capture_index: HashMap<Ident<'ast>, usize>,
    captures: Vec<Place>,
}

impl<'ast> Frame<'ast> {
    fn new(function: &'ast Function) -> Self {
        Self {
            function,
            context: Context::with_reserved(1),
            name_to_capture_index: HashMap::new(),
            captures: Vec::new(),
        }
    }

    fn resolve(
        &mut self,
        identifier: &'ast Identifier,
        and_capture: bool,
    ) -> Option<Place> {
        if let Some(local_index) = self.context.resolve(identifier, and_capture)
        {
            Some(Place::Local { local_index })
        } else if let Some(&upvalue_index) =
            self.name_to_capture_index.get(&Ident { identifier })
        {
            Some(Place::Upvalue { upvalue_index })
        } else {
            None
        }
    }

    fn add_capture(
        &mut self,
        identifier: &'ast Identifier,
        place: Place,
    ) -> usize {
        let upvalue_index = self.captures.len();
        self.name_to_capture_index
            .insert(Ident { identifier }, upvalue_index);
        self.captures.push(place);
        upvalue_index
    }

    fn declare(
        &mut self,
        identifier: &'ast Identifier,
    ) -> Result<(), CompileError> {
        self.context.declare(identifier).unwrap()
    }
}

enum GlobalResolution {
    Pending,
    Resolved(Span),
}

enum GlobalIdent<'ast> {
    Identifier(&'ast Identifier),
    String(&'ast str),
}

impl<'ast> GlobalIdent<'ast> {
    fn as_str(&self) -> &'ast str {
        match self {
            Self::Identifier(identifier) => &identifier.value,
            Self::String(string) => string,
        }
    }

    fn span(&self) -> Span {
        match self {
            Self::Identifier(identifier) => identifier.span(),
            Self::String(_) => Span::null(),
        }
    }
}

impl hash::Hash for GlobalIdent<'_> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}

impl PartialEq for GlobalIdent<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl Eq for GlobalIdent<'_> {}

pub struct NameResolutionPass<'ast> {
    globals: HashMap<GlobalIdent<'ast>, GlobalResolution>,
    global_context: Context<'ast>,
    frames: Vec<Frame<'ast>>,
    resolutions: Dec<NameDecoration, Resolution>,
    block_locals: Dec<BlockDecoration, Vec<LocalDeclaration>>,
    function_infos: Dec<FunctionDecoration, FunctionInfo<'ast>>,
    errors: Vec<CompileError>,
}

impl<'ast> NameResolutionPass<'ast> {
    fn propagate_capture(
        &mut self,
        frame: usize,
        identifier: &'ast Identifier,
        mut place: Place,
    ) -> Resolution {
        for frame in &mut self.frames[frame..] {
            place = Place::Upvalue {
                upvalue_index: frame.add_capture(identifier, place),
            };
        }

        Resolution::from(place)
    }

    fn resolve_identifier(
        &mut self,
        identifier: &'ast Identifier,
    ) -> Resolution {
        let mut frame_index = self.frames.len();
        let mut and_capture = false;

        while frame_index > 0 {
            if let Some(capture) =
                self.frames[frame_index - 1].resolve(identifier, and_capture)
            {
                return self.propagate_capture(
                    frame_index,
                    identifier,
                    capture,
                );
            }

            frame_index -= 1;
            and_capture = true;
        }

        if let Some(local_index) =
            self.global_context.resolve(identifier, and_capture)
        {
            return self.propagate_capture(
                0,
                identifier,
                Place::Local { local_index },
            );
        }

        if let Entry::Vacant(vacant) =
            self.globals.entry(GlobalIdent::Identifier(identifier))
        {
            vacant.insert(GlobalResolution::Pending);
        }

        Resolution::Global
    }

    fn resolve(&mut self, name: &'ast Name) {
        let resolution = self.resolve_identifier(&name.identifier);
        self.resolutions.insert(name.decoration, resolution);
    }

    fn declare(&mut self, identifier: &'ast Identifier) {
        if let Some(last_frame) = self.frames.last_mut() {
            if let Err(error) = last_frame.declare(identifier) {
                self.errors.push(error);
            }
        } else if let Some(result) = self.global_context.declare(identifier) {
            if let Err(error) = result {
                self.errors.push(error);
            }
        } else if let Some(GlobalResolution::Resolved(original)) =
            self.globals.insert(
                GlobalIdent::Identifier(identifier),
                GlobalResolution::Resolved(identifier.span()),
            )
        {
            self.errors.push(CompileError::ItemRedefined {
                original,
                redefinition: identifier.span(),
            });
        }
    }

    fn push_scope(&mut self, block: &'ast BlockStmt) {
        if let Some(last_frame) = self.frames.last_mut() {
            last_frame.context.push_scope(block);
        } else {
            self.global_context.push_scope(block);
        }
    }

    fn pop_scope(&mut self) {
        let scope = if let Some(last_frame) = self.frames.last_mut() {
            last_frame.context.pop_scope().unwrap()
        } else {
            self.global_context.pop_scope().unwrap()
        };
        self.block_locals
            .insert(scope.block.decoration, scope.local_decls);
    }

    fn push_frame(&mut self, function: &'ast Function) {
        let mut frame = Frame::new(function);
        frame.context.push_scope(&function.body);
        for param in function.params.iter() {
            if let Err(error) = frame.declare(param) {
                self.errors.push(error);
            }
        }
        self.frames.push(frame);
    }

    fn pop_frame(&mut self, identifier: &'ast Identifier) {
        self.pop_scope();

        let frame = self.frames.pop().unwrap();
        self.function_infos.insert(
            frame.function.decoration,
            FunctionInfo {
                identifier,
                function: frame.function,
                captures: frame.captures,
            },
        );
    }

    pub fn compile(ast: &'ast Ast) -> NameResolutionOutput<'ast> {
        let mut pass = Self {
            globals: global_names()
                .map(|string| {
                    (
                        GlobalIdent::String(string),
                        GlobalResolution::Resolved(Span::null()),
                    )
                })
                .collect(),
            global_context: Context::empty(),
            frames: Vec::new(),
            resolutions: Dec::new(&ast.decorator),
            block_locals: Dec::new(&ast.decorator),
            function_infos: Dec::new(&ast.decorator),
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
            block_locals: pass.block_locals.unwrap(),
            function_infos: pass.function_infos.unwrap(),
            errors: pass.errors,
        }
    }
}

impl<'ast> Visitor<'ast> for NameResolutionPass<'ast> {
    fn visit_variable_expr(&mut self, node: &'ast VariableExpr) {
        self.resolve(&node.name);
    }

    fn visit_assign_expr(&mut self, node: &'ast AssignExpr) {
        visit::visit_assign_expr(self, node);

        self.resolve(&node.name);
    }

    fn visit_var_decl_stmt(&mut self, node: &'ast VarDeclStmt) {
        visit::visit_var_decl_stmt(self, node);

        self.declare(&node.identifier);
    }

    fn visit_block_stmt(&mut self, node: &'ast BlockStmt) {
        self.push_scope(node);

        visit::visit_block_stmt(self, node);

        self.pop_scope();
    }

    fn visit_fun_decl_stmt(&mut self, node: &'ast FunDeclStmt) {
        self.declare(&node.identifier);

        self.push_frame(&node.function);

        visit::visit_block_stmt(self, &node.function.body);

        self.pop_frame(&node.identifier);
    }
}
