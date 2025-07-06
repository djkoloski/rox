use core::fmt;

use rox_diag::{Diagnostic, Formatter};

#[derive(Debug)]
pub enum CompileError {}

impl Diagnostic for CompileError {
    fn fmt(&self, _f: &mut Formatter<'_, '_>) -> fmt::Result {
        todo!()
    }
}
