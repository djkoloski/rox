use core::hash;
use std::collections::{HashMap, hash_map::Entry};

use rox_diag::{Span, Spanned as _};
use rox_lex::token_kind::Identifier;
use rox_parse::{
    Ast, BlockDecoration, ClassDecoration, Dec, Decoration, FunctionDecoration,
    NameDecoration, Visit as _,
    ast::{
        AssignExpr, BlockStmt, ClassDeclStmt, FunDeclStmt, Function,
        ReturnStmt, ThisExpr, VarDeclStmt, VariableExpr, Visitor, visit,
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
    pub kind: FrameKind,
}

pub struct ClassInfo<'ast> {
    pub identifier: &'ast Identifier,
    pub methods: HashMap<String, usize>,
}

pub struct NameResolutionOutput<'ast> {
    pub resolutions: Vec<Resolution>,
    pub block_locals: Vec<Vec<LocalDeclaration>>,
    pub function_infos: Vec<FunctionInfo<'ast>>,
    pub class_infos: Vec<ClassInfo<'ast>>,
    pub errors: Vec<CompileError>,
}

#[derive(Clone, Copy)]
enum Name<'ast> {
    Ident(&'ast Identifier),
    This(Span),
}

impl<'ast> Name<'ast> {
    fn as_str(&self) -> &'ast str {
        match self {
            Self::Ident(identifier) => &identifier.value,
            Self::This(_) => "this",
        }
    }

    fn span(&self) -> Span {
        match self {
            Self::Ident(identifier) => identifier.span(),
            Self::This(span) => *span,
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

struct Scope<'ast> {
    block: &'ast BlockStmt,
    name_to_local_decl: HashMap<Name<'ast>, usize>,
    local_decls: Vec<LocalDeclaration>,
}

impl<'ast> Scope<'ast> {
    fn new(block: &'ast BlockStmt) -> Self {
        Self {
            block,
            name_to_local_decl: HashMap::new(),
            local_decls: Vec::new(),
        }
    }

    fn resolve(
        &mut self,
        name: Name<'ast>,
        and_capture: bool,
    ) -> Option<usize> {
        if let Some(&base_index) = self.name_to_local_decl.get(&name) {
            if and_capture {
                self.local_decls[base_index] = LocalDeclaration::Capture;
            }
            Some(base_index)
        } else {
            None
        }
    }

    fn declare(&mut self, name: Name<'ast>) -> Result<usize, CompileError> {
        let span = name.span();
        match self.name_to_local_decl.entry(name) {
            Entry::Occupied(occupied) => Err(CompileError::ItemRedefined {
                original: occupied.key().span(),
                redefinition: span,
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
        name: Name<'ast>,
        and_capture: bool,
    ) -> Option<usize> {
        let mut base_index = self.local_count;
        for scope in self.scopes.iter_mut().rev() {
            base_index -= scope.local_decls.len();

            if let Some(local_index) = scope.resolve(name, and_capture) {
                return Some(base_index + local_index);
            }
        }

        None
    }

    fn declare(
        &mut self,
        name: Name<'ast>,
    ) -> Option<Result<(), CompileError>> {
        if let Some(scope) = self.scopes.last_mut() {
            if let Err(error) = scope.declare(name) {
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
    name_to_capture_index: HashMap<Name<'ast>, usize>,
    captures: Vec<Place>,
    kind: FrameKind,
}

impl<'ast> Frame<'ast> {
    fn new(function: &'ast Function, kind: FrameKind) -> Self {
        Self {
            function,
            context: Context::with_reserved(kind.unused_reserved_names()),
            name_to_capture_index: HashMap::new(),
            captures: Vec::new(),
            kind,
        }
    }

    fn resolve(
        &mut self,
        name: Name<'ast>,
        and_capture: bool,
    ) -> Option<Place> {
        if let Some(local_index) = self.context.resolve(name, and_capture) {
            Some(Place::Local { local_index })
        } else if let Some(&upvalue_index) =
            self.name_to_capture_index.get(&name)
        {
            Some(Place::Upvalue { upvalue_index })
        } else {
            None
        }
    }

    fn add_capture(&mut self, name: Name<'ast>, place: Place) -> usize {
        let upvalue_index = self.captures.len();
        self.name_to_capture_index.insert(name, upvalue_index);
        self.captures.push(place);
        upvalue_index
    }

    fn declare(&mut self, name: Name<'ast>) -> Result<(), CompileError> {
        self.context.declare(name).unwrap()
    }
}

#[derive(Clone, Copy)]
pub enum FrameKind {
    Function,
    Method,
    Initializer,
}

impl FrameKind {
    fn unused_reserved_names(&self) -> usize {
        match self {
            Self::Function => 1,
            Self::Method => 0,
            Self::Initializer => 0,
        }
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
    class_infos: Dec<ClassDecoration, ClassInfo<'ast>>,
    errors: Vec<CompileError>,
}

impl<'ast> NameResolutionPass<'ast> {
    fn propagate_capture(
        &mut self,
        frame: usize,
        name: Name<'ast>,
        mut place: Place,
    ) -> Resolution {
        for frame in &mut self.frames[frame..] {
            place = Place::Upvalue {
                upvalue_index: frame.add_capture(name, place),
            };
        }

        Resolution::from(place)
    }

    fn resolve_identifier(&mut self, name: Name<'ast>) -> Resolution {
        let mut frame_index = self.frames.len();
        let mut and_capture = false;

        while frame_index > 0 {
            if let Some(capture) =
                self.frames[frame_index - 1].resolve(name, and_capture)
            {
                return self.propagate_capture(frame_index, name, capture);
            }

            frame_index -= 1;
            and_capture = true;
        }

        if let Some(local_index) =
            self.global_context.resolve(name, and_capture)
        {
            return self.propagate_capture(
                0,
                name,
                Place::Local { local_index },
            );
        }

        Resolution::Global
    }

    fn resolve(
        &mut self,
        name: Name<'ast>,
        decoration: Decoration<NameDecoration>,
    ) {
        let resolution = self.resolve_identifier(name);

        if matches!(resolution, Resolution::Global) {
            match name {
                Name::Ident(identifier) => {
                    if let Entry::Vacant(vacant) =
                        self.globals.entry(GlobalIdent::Identifier(identifier))
                    {
                        vacant.insert(GlobalResolution::Pending);
                    }
                }
                Name::This(span) => {
                    self.errors.push(CompileError::UndefinedItem { span })
                }
            }
        }

        self.resolutions.insert(decoration, resolution);
    }

    fn declare(&mut self, name: Name<'ast>) {
        if let Some(last_frame) = self.frames.last_mut() {
            if let Err(error) = last_frame.declare(name) {
                self.errors.push(error);
            }
        } else if let Some(result) = self.global_context.declare(name) {
            if let Err(error) = result {
                self.errors.push(error);
            }
        } else if let Name::Ident(identifier) = name
            && let Some(GlobalResolution::Resolved(original)) =
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

    fn push_frame(&mut self, function: &'ast Function, kind: FrameKind) {
        let mut frame = Frame::new(function, kind);
        frame.context.push_scope(&function.body);
        if matches!(kind, FrameKind::Method | FrameKind::Initializer)
            && let Err(error) = frame.declare(Name::This(Span::null()))
        {
            self.errors.push(error);
        }
        for param in function.params.iter() {
            if let Err(error) = frame.declare(Name::Ident(param)) {
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
                kind: frame.kind,
            },
        );
    }

    fn frame_kind(&self) -> Option<FrameKind> {
        Some(self.frames.last()?.kind)
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
            class_infos: Dec::new(&ast.decorator),
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
            class_infos: pass.class_infos.unwrap(),
            errors: pass.errors,
        }
    }
}

impl<'ast> Visitor<'ast> for NameResolutionPass<'ast> {
    fn visit_variable_expr(&mut self, node: &'ast VariableExpr) {
        self.resolve(Name::Ident(&node.name.identifier), node.name.decoration);
    }

    fn visit_assign_expr(&mut self, node: &'ast AssignExpr) {
        visit::visit_assign_expr(self, node);

        self.resolve(Name::Ident(&node.name.identifier), node.name.decoration);
    }

    fn visit_var_decl_stmt(&mut self, node: &'ast VarDeclStmt) {
        visit::visit_var_decl_stmt(self, node);

        self.declare(Name::Ident(&node.identifier));
    }

    fn visit_block_stmt(&mut self, node: &'ast BlockStmt) {
        self.push_scope(node);

        visit::visit_block_stmt(self, node);

        self.pop_scope();
    }

    fn visit_fun_decl_stmt(&mut self, node: &'ast FunDeclStmt) {
        self.declare(Name::Ident(&node.identifier));

        self.push_frame(&node.function, FrameKind::Function);

        visit::visit_block_stmt(self, &node.function.body);

        self.pop_frame(&node.identifier);
    }

    fn visit_class_decl_stmt(&mut self, node: &'ast ClassDeclStmt) {
        self.declare(Name::Ident(&node.identifier));

        let mut methods = HashMap::new();
        for method in &node.methods {
            let kind = if method.identifier.value == "init" {
                FrameKind::Initializer
            } else {
                FrameKind::Method
            };
            self.push_frame(&method.function, kind);

            visit::visit_block_stmt(self, &method.function.body);

            self.pop_frame(&method.identifier);

            methods.insert(
                method.identifier.value.clone(),
                method.function.decoration.index(),
            );
        }

        self.class_infos.insert(
            node.decoration,
            ClassInfo {
                identifier: &node.identifier,
                methods,
            },
        );
    }

    fn visit_this_expr(&mut self, node: &'ast ThisExpr) {
        self.resolve(Name::This(node.span()), node.decoration);
    }

    fn visit_return_stmt(&mut self, node: &'ast ReturnStmt) {
        if let Some(FrameKind::Initializer) = self.frame_kind() {
            self.errors
                .push(CompileError::ReturnInInitializer { span: node.span() });
        }
    }
}
