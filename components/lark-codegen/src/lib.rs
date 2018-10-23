use lark_mir::Context;

mod build;
mod codegen_c;
mod codegen_rust;

#[derive(Copy, Clone)]
pub enum CodegenType {
    Rust,
    C,
}

pub fn codegen(context: &Context, codegen_type: CodegenType) -> String {
    match codegen_type {
        CodegenType::Rust => codegen_rust::codegen_rust(context),
        CodegenType::C => codegen_c::codegen_c(context),
    }
}

pub fn build(
    target_filename: &str,
    src: &String,
    codegen_type: CodegenType,
) -> std::io::Result<()> {
    build::build(target_filename, &src, codegen_type)
}
