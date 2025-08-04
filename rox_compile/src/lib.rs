mod assembly;
mod error;
mod name_resolution;

use rox_parse::Ast;
use rox_vm::{Chunk, ClassDef, Executable, FunctionDef};

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
        class_infos,
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
        functions.push(FunctionDef {
            name: function_info.identifier.value.clone(),
            arity: function_info.function.params.len(),
            ip: chunk.bytes().len(),
            captures: function_info.captures,
        });
        AssemblyPass::new(&resolutions, &block_locals, &mut chunk)
            .compile_function(function_info.function);
    }

    let mut classes = Vec::new();
    for class_info in class_infos {
        classes.push(ClassDef {
            name: class_info.identifier.value.clone(),
            methods: class_info.methods,
        });
    }

    Ok(Executable {
        chunk,
        functions,
        classes,
    })
}
