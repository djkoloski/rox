mod assembly;
mod error;
mod name_resolution;

use rox_parse::Ast;
use rox_vm::Chunk;

pub use self::{assembly::*, error::*, name_resolution::*};

pub struct CompileOutput {
    pub chunk: Option<Chunk>,
    pub errors: Vec<CompileError>,
}

pub fn compile(
    ast: &Ast,
    should_assemble: bool,
) -> Result<Chunk, Vec<CompileError>> {
    let NameResolutionOutput {
        errors,
        resolutions,
        locals_counts,
    } = NameResolutionPass::new().compile(ast);

    if !should_assemble || !errors.is_empty() {
        return Err(errors);
    }

    let chunk = AssemblyPass::new(&resolutions, &locals_counts).compile(ast);

    Ok(chunk)
}
