macro_rules! ast {
    ($($tt:tt)*) => {
        pub mod visit {
            use super::*;

            define_visitor_fns! { $($tt)* }
        }

        pub trait Visitor<'ast> {
            define_visitor_trait_fns! { $($tt)* }
        }

        impl_visits! { $($tt)* }

        define_nodes! { $($tt)* }
    };
}

macro_rules! define_visitor_fns {
    (
        #[accept = $fn:ident]
        pub struct $name:ident { $($tt:tt)* }
        $($rest:tt)*
    ) => {
        #[allow(unused_variables)]
        pub fn $fn<'ast, V>(visitor: &mut V, node: &'ast $name)
        where
            V: Visitor<'ast> + ?Sized,
        {
            visit_fields! { node visitor => $($tt)* }
        }

        define_visitor_fns! { $($rest)* }
    };
    (
        pub struct $name:ident { $($tt:tt)* }
        $($rest:tt)*
    ) => {
        // Skip unnamed structs
        define_visitor_fns! { $($rest)* }
    };
    (
        pub enum $name:ident { $($tt:tt)* }
        $($rest:tt)*
    ) => {
        // Skip unnamed enums
        define_visitor_fns! { $($rest)* }
    };
    () => {};
}

macro_rules! define_visitor_trait_fns {
    (
        #[accept = $fn:ident]
        pub struct $name:ident { $($tt:tt)* }
        $($rest:tt)*
    ) => {
        #[allow(unused_variables)]
        fn $fn(&mut self, node: &'ast $name) {
            visit::$fn(self, node);
        }

        define_visitor_trait_fns! { $($rest)* }
    };
    (
        pub struct $name:ident { $($tt:tt)* }
        $($rest:tt)*
    ) => {
        // Skip unnamed structs
        define_visitor_trait_fns! { $($rest)* }
    };
    (
        pub enum $name:ident { $($tt:tt)* }
        $($rest:tt)*
    ) => {
        // Skip unnamed enums
        define_visitor_trait_fns! { $($rest)* }
    };
    () => {};
}

macro_rules! impl_visits {
    () => {};
    (
        #[accept = $fn:ident]
        pub struct $name:ident { $($tt:tt)* }
        $($rest:tt)*
    ) => {
        impl<'ast, V: Visitor<'ast> + ?Sized> $crate::Visit<'ast, V> for $name {
            fn accept(&'ast self, visitor: &mut V) {
                Visitor::$fn(visitor, self)
            }
        }

        impl_visits! { $($rest)* }
    };
    (
        pub struct $name:ident { $($tt:tt)* }
        $($rest:tt)*
    ) => {
        impl<'ast, V: Visitor<'ast> + ?Sized> $crate::Visit<'ast, V> for $name {
            #[allow(unused_variables)]
            fn accept(&'ast self, visitor: &mut V) {
                visit_fields! { self visitor => $($tt)* }
            }
        }

        impl_visits! { $($rest)* }
    };
    (
        pub enum $name:ident { $($variant:ident($ty:ty),)* }
        $($rest:tt)*
    ) => {
        impl<'ast, V: Visitor<'ast> + ?Sized> $crate::Visit<'ast, V> for $name {
            fn accept(&'ast self, visitor: &mut V) {
                match self {
                    $(
                        Self::$variant(value) =>
                            $crate::Visit::accept(value, visitor),
                    )*
                }
            }
        }

        impl_visits! { $($rest)* }
    }
}

macro_rules! visit_fields {
    ($node:ident $visitor:ident =>) => {};
    (
        $node:ident $visitor:ident =>
        pub $field:ident: $ty:ty
        $(, $($rest:tt)*)?
    ) => {
        $(visit_fields! { $node $visitor => $($rest)* })?
    };
    (
        $node:ident $visitor:ident =>
        #[visit]
        pub $field:ident: $ty:ty
        $(, $($rest:tt)*)?
    ) => {
        $crate::Visit::accept(&$node.$field, $visitor);
        $(visit_fields! { $node $visitor => $($rest)* })?
    };
}

macro_rules! define_nodes {
    () => {};
    (
        pub enum $name:ident {
            $($variant:ident($ty:ident)),* $(,)?
        }
        $($rest:tt)*
    ) => {
        define_enum! { pub enum $name { $($variant($ty),)* } }

        define_nodes! { $($rest)* }
    };
    (
        $(#[accept = $fn:ident])?
        pub struct $name:ident {
            $(
                $(#[visit])?
                pub $field:ident: $ty:ident $(<$($args:tt),* $(,)?>)?
            ),* $(,)?
        }
        $($rest:tt)*
    ) => {
        #[derive(Debug)]
        pub struct $name {
            $(pub $field: $ty $(<$($args),*>)?,)*
        }

        impl ::rox_diag::Spanned for $name {
            #[allow(unreachable_code)]
            fn span_start(&self) -> usize {
                struct_span_start!(self $($field: $ty,)*);
            }

            #[allow(unreachable_code)]
            fn span_end(&self) -> usize {
                struct_span_end!(self $($field: $ty,)*);
            }
        }

        define_nodes! { $($rest)* }
    };
}

macro_rules! define_enum {
    (
        pub enum $name:ident {
            $($variant:ident($ty:ty)),* $(,)?
        }
    ) => {
        #[derive(Debug)]
        pub enum $name {
            $($variant($ty),)*
        }

        impl ::rox_diag::Spanned for $name {
            fn span_start(&self) -> usize {
                match self {
                    $(Self::$variant(value) => value.span_start(),)*
                }
            }

            fn span_end(&self) -> usize {
                match self {
                    $(Self::$variant(value) => value.span_end(),)*
                }
            }
        }

        $(
            impl From<$ty> for $name {
                fn from(value: $ty) -> Self {
                    Self::$variant(value)
                }
            }
        )*
    }
}

macro_rules! struct_span_start {
    ($self:ident) => {};
    (
        $self:ident
        $first:ident: Decoration,
        $($rest:ident: $rest_ty:ident,)*
    ) => {
        struct_span_start!($self $($rest: $rest_ty,)*);
    };
    (
        $self:ident
        $first:ident: Option,
        $($rest:ident: $rest_ty:ident,)*
    ) => {
        if let Some(value) = &$self.$first {
            return ::rox_diag::Spanned::span_start(value);
        }
        struct_span_start!($self $($rest: $rest_ty,)*);
    };
    (
        $self:ident
        $first:ident: Vec,
        $($rest:ident: $rest_ty:ident,)*
    ) => {
        if let Some(value) = $self.$first.first() {
            return ::rox_diag::Spanned::span_start(value);
        }
        struct_span_start!($self $($rest: $rest_ty,)*);
    };
    (
        $self:ident
        $first:ident: Punctuated,
        $($rest:ident: $rest_ty:ident,)*
    ) => {
        if let Some(span_start) = $self.$first.span_start() {
            return span_start;
        }
        struct_span_start!($self $($rest: $rest_ty,)*);
    };
    (
        $self:ident
        $first:ident: $first_ty:ident,
        $($rest:ident: $rest_ty:ident,)*
    ) => {
        return ::rox_diag::Spanned::span_start(&$self.$first);
    };
}

macro_rules! struct_span_end {
    ($self:ident) => {};
    (
        $self:ident
        $first:ident: Decoration,
        $($rest:ident: $rest_ty:ident,)*
    ) => {
        struct_span_end!($self $($rest: $rest_ty,)*);
    };
    (
        $self:ident
        $first:ident: Option,
        $($rest:ident: $rest_ty:ident,)*
    ) => {
        struct_span_end!($self $($rest: $rest_ty,)*);
        if let Some(value) = &$self.$first {
            return ::rox_diag::Spanned::span_end(value);
        }
    };
    (
        $self:ident
        $first:ident: Vec,
        $($rest:ident: $rest_ty:ident,)*
    ) => {
        struct_span_end!($self $($rest: $rest_ty,)*);
        if let Some(value) = $self.$first.first() {
            return ::rox_diag::Spanned::span_end(value);
        }
    };
    (
        $self:ident
        $first:ident: Punctuated,
        $($rest:ident: $rest_ty:ident,)*
    ) => {
        struct_span_end!($self $($rest: $rest_ty,)*);
        if let Some(span_end) = $self.$first.span_end() {
            return span_end;
        }
    };
    (
        $self:ident
        $first:ident: $first_ty:ident,
        $($rest:ident: $rest_ty:ident,)*
    ) => {
        struct_span_end!($self $($rest: $rest_ty,)*);
        return ::rox_diag::Spanned::span_end(&$self.$first);
    };
}

macro_rules! token_group {
    (
        $(
            pub enum $name:ident {
                $($variant:ident($ty:ident)),* $(,)?
            }
        )*
    ) => {
        $(
            define_enum! { pub enum $name { $($variant($ty)),* } }

            impl ::rox_lex::TokenKind for $name {
                fn matches_token(token: &::rox_lex::Token) -> bool {
                    ::core::matches!(
                        token,
                        $(::rox_lex::Token::$ty(_))|*,
                    )
                }

                fn from_token(token: ::rox_lex::Token) -> Self {
                    match token {
                        $(
                            ::rox_lex::Token::$ty(value) =>
                                Self::$variant(value),
                        )*
                        _ => ::core::unreachable!(),
                    }
                }
            }
        )*
    };
}

pub(crate) use ast;
pub(crate) use define_enum;
pub(crate) use define_nodes;
pub(crate) use define_visitor_fns;
pub(crate) use define_visitor_trait_fns;
pub(crate) use impl_visits;
pub(crate) use struct_span_end;
pub(crate) use struct_span_start;
pub(crate) use token_group;
pub(crate) use visit_fields;
