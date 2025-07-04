use core::fmt;

use crate::{
    diagnostic::{Context, Diagnostic},
    span::Span,
};

#[derive(Debug)]
pub enum CompileError {
    UndefinedVariable {
        span: Span,
    },
    TopLevelReturn {
        span: Span,
    },
    ThisOutsideClass {
        span: Span,
    },
    SuperOutsideClass {
        span: Span,
    },
    ReturnInInitializer {
        method: Span,
        span: Span,
        class: Span,
    },
}

impl Diagnostic for CompileError {
    fn fmt(
        &self,
        c: &mut Context<'_>,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            Self::UndefinedVariable { span } => {
                c.error(
                    f,
                    format_args!(
                        "undefined variable '{}'",
                        span.get(c.source())
                    ),
                )?;
                c.span(
                    *span,
                    f,
                    format_args!("this variable has not been defined"),
                )?;
            }
            Self::TopLevelReturn { span } => {
                c.error(f, format_args!("return statement in top-level code"))?;
                c.span(
                    *span,
                    f,
                    format_args!(
                        "this `return` should be contained in a function"
                    ),
                )?;
            }
            Self::ThisOutsideClass { span } => {
                c.error(
                    f,
                    format_args!("`this` may not be used outside of a class"),
                )?;
                c.span(
                    *span,
                    f,
                    format_args!(
                        "`this` always refers to the instance of the \
                         enclosing class"
                    ),
                )?;
            }
            Self::SuperOutsideClass { span } => {
                c.error(
                    f,
                    format_args!("`super` may not be used outside of a class"),
                )?;
                c.span(
                    *span,
                    f,
                    format_args!(
                        "`super` always refers to the parent of the enclosing \
                         class"
                    ),
                )?;
            }
            Self::ReturnInInitializer {
                method,
                span,
                class,
            } => {
                c.error(
                    f,
                    format_args!("return statement in class initializer"),
                )?;
                c.span(
                    *span,
                    f,
                    format_args!(
                        "this `return` is inside of a class initializer"
                    ),
                )?;
                c.span(
                    *method,
                    f,
                    format_args!(
                        "`{}` is a class initializer ...",
                        method.get(c.source())
                    ),
                )?;
                c.span(
                    *class,
                    f,
                    format_args!("... for `{}`", class.get(c.source())),
                )?;
            }
        }
        Ok(())
    }
}
