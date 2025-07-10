mod dec;
mod decoration;

pub use self::{dec::*, decoration::*};

macro_rules! define_decorations {
    ($($name:ident),* $(,)?) => {
        $(
            pub struct $name;
        )*

        impl_decorations!(0, $($name)*);
    };
}

macro_rules! impl_decorations {
    ($index:expr,) => {
        const DECORATIONS_MAX: usize = 0;
    };
    ($index:expr, $name:ident) => {
        impl DecorationKind for $name {
            const DECORATION_INDEX: usize = $index;
        }

        const DECORATIONS_MAX: usize = $index + 1;
    };
    ($index:expr, $name:ident $($rest:ident)+) => {
        impl DecorationKind for $name {
            const DECORATION_INDEX: usize = $index;
        }

        impl_decorations!($index + 1, $($rest)*);
    };
}

define_decorations! {
    NameResolution,
    LocalsCount,
    FunctionLabel,
}
