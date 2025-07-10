mod assembly;
mod collect_fun_decls;
mod error;
mod name_resolution;

use rox_parse::Ast;
use rox_vm::{Executable, Function};

pub use self::{assembly::*, error::*, name_resolution::*};
use crate::collect_fun_decls::CollectFunDeclsPass;

pub struct CompileOutput {
    pub executable: Option<Executable>,
    pub errors: Vec<CompileError>,
}

pub fn compile(
    ast: &Ast,
    should_assemble: bool,
) -> Result<Executable, Vec<CompileError>> {
    let NameResolutionOutput {
        errors,
        resolutions,
        locals_counts,
    } = NameResolutionPass::compile(ast);

    if !should_assemble || !errors.is_empty() {
        return Err(errors);
    }

    let fun_decls = CollectFunDeclsPass::compile(ast);
    let main = AssemblyPass::new(&resolutions, &locals_counts)
        .compile_program(&ast.program);
    let mut functions = Vec::new();
    for fun_decl in fun_decls {
        functions.push(Function {
            name: fun_decl.function.name.value.clone(),
            arity: fun_decl.function.params.len(),
            chunk: AssemblyPass::new(&resolutions, &locals_counts)
                .compile_function(fun_decl),
        });
    }

    Ok(Executable { main, functions })
}
