mod assembly;
mod error;
mod name_resolution;

use rox_parse::Ast;
use rox_vm::{Chunk, Executable, Function};

pub use self::{assembly::*, error::*, name_resolution::*};

pub struct CompileOutput {
    pub executable: Option<Executable>,
    pub errors: Vec<CompileError>,
}

pub fn compile(
    ast: &Ast,
    should_assemble: bool,
) -> Result<Executable, Vec<CompileError>> {
    let NameResolutionOutput {
        resolutions,
        block_locals,
        function_infos,
        errors,
    } = NameResolutionPass::compile(ast);

    if !should_assemble || !errors.is_empty() {
        return Err(errors);
    }

    let mut chunk = Chunk::new();
    AssemblyPass::new(&resolutions, &block_locals, &mut chunk)
        .compile_program(&ast.program);

    let mut functions = Vec::new();
    for function_info in function_infos {
        functions.push(Function {
            name: function_info.identifier.value.clone(),
            arity: function_info.function.params.len(),
            ip: chunk.bytes().len(),
            captures: function_info.captures,
        });
        AssemblyPass::new(&resolutions, &block_locals, &mut chunk)
            .compile_function(function_info.function);
    }

    Ok(Executable { chunk, functions })
}
