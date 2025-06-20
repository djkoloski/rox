macro_rules! ast_node {
    () => {};
    (
        $(
            #[visit($visit_trait:ident, $visitor_trait:ident)]
        )?
        pub enum $name:ident {
            $($variant:ident($ty:ty)),*
            $(,)?
        }
        $($rest:tt)*
    ) => {
        pub enum $name {
            $($variant($ty),)*
        }

        impl $crate::span::Spanned for $name {
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

        ast_node!(
            @enum_visit
            $(#[$visit_trait, $visitor_trait])?
            $name { $($variant)* }
        );

        ast_node! {
            $($rest)*
        }
    };
    (@enum_visit $name:ident { $($variant:ident)* }) => {};
    (@enum_visit
        #[$visit_trait:ident, $visitor_trait:ident]
        $name:ident { $($variant:ident)* }
    ) => {
        impl<V: $visitor_trait> $visit_trait<V> for $name {
            fn accept(&self, visitor: &mut V) -> V::Output {
                match self {
                    $(
                        Self::$variant(value) =>
                            $visit_trait::accept(value, visitor),
                    )*
                }
            }
        }
    };
    (
        $(
            #[visit(
                $visit_trait:ident,
                $visitor_trait:ident
                $(::$fn:ident)?
                $(
                    ,
                    fn accept($self:ident, $visitor:ident) { $($accept:tt)* }
                    $(,)?
                )?
            )]
        )?
        pub struct $name:ident {
            $(pub $field:ident: $ty:ident $(<$($args:tt),* $(,)?>)?),*
            $(,)?
        }
        $($rest:tt)*
    ) => {
        pub struct $name {
            $(pub $field: $ty $(<$($args),*>)?,)*
        }

        impl $crate::span::Spanned for $name {
            #[allow(unreachable_code)]
            fn span_start(&self) -> usize {
                ast_node!(@span_start self $($field: $ty,)*);
            }

            #[allow(unreachable_code)]
            fn span_end(&self) -> usize {
                ast_node!(@span_end self $($field: $ty,)*);
            }
        }

        $(
            impl<V: $visitor_trait> $visit_trait<V> for $name {
                $(
                    fn accept(&self, visitor: &mut V) -> V::Output {
                        visitor.$fn(self)
                    }
                )?
                $(
                    fn accept(&$self, $visitor: &mut V) -> V::Output {
                        $($accept)*
                    }
                )?
            }
        )?

        ast_node! {
            $($rest)*
        }
    };
    (@span_start $self:ident) => {};
    (@span_start
        $self:ident
        $first:ident: Option,
        $($rest:ident: $rest_ty:ident,)*
    ) => {
        if let Some(value) = &$self.$first {
            return $crate::span::Spanned::span_start(value);
        }
        ast_node!(@span_start $self $($rest: $rest_ty,)*);
    };
    (@span_start
        $self:ident
        $first:ident: Vec,
        $($rest:ident: $rest_ty:ident,)*
    ) => {
        if let Some(value) = $self.$first.first() {
            return $crate::span::Spanned::span_start(value);
        }
        ast_node!(@span_start $self $($rest: $rest_ty,)*);
    };
    (@span_start
        $self:ident
        $first:ident: $first_ty:ident,
        $($rest:ident: $rest_ty:ident,)*
    ) => {
        return $crate::span::Spanned::span_start(&$self.$first);
    };
    (@span_end $self:ident) => {};
    (@span_end
        $self:ident
        $first:ident: Option,
        $($rest:ident: $rest_ty:ident,)*
    ) => {
        ast_node!(@span_end $self $($rest: $rest_ty,)*);
        if let Some(value) = &$self.$first {
            return $crate::span::Spanned::span_end(value);
        }
    };
    (@span_end
        $self:ident
        $first:ident: Vec,
        $($rest:ident: $rest_ty:ident,)*
    ) => {
        ast_node!(@span_end $self $($rest: $rest_ty,)*);
        if let Some(value) = $self.$first.first() {
            return $crate::span::Spanned::span_end(value);
        }
    };
    (@span_end
        $self:ident
        $first:ident: $first_ty:ident,
        $($rest:ident: $rest_ty:ident,)*
    ) => {
        ast_node!(@span_end $self $($rest: $rest_ty,)*);
        return $crate::span::Spanned::span_end(&$self.$first);
    };
}

pub mod expr;
pub mod stmt;
