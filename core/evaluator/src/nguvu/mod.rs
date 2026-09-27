//! Nguvu: Asili's own native code generator. Compiles register bytecode straight to machine
//! code in memory — no external compiler, assembler, linker or file — with the same calling
//! convention, deoptimization protocol and analysis as the LLVM backend (`aot.rs`), so the VM
//! runs either through the same [`crate::aot::NativeLibrary`] interface.
//!
//! Pipeline: [`lower`] (bytecode → [`ir`]) → [`codegen`] (IR → x86-64 via [`x64`]) → [`mem`]
//! (executable mapping). Supported on x86-64 Unix today.

pub mod codegen;
pub mod ir;
pub mod lower;
pub mod mem;
pub mod x64;

use crate::bytecode::BytecodeProgram;

/// Whether this build can generate native code for the host.
pub fn supported() -> bool {
    cfg!(all(target_arch = "x86_64", unix))
}

/// Compile every function of `program` to machine code in executable memory.
pub fn compile(program: &BytecodeProgram) -> Result<crate::aot::NativeLibrary, String> {
    if !supported() {
        return Err("nguvu: mfumo huu bado hauungwi mkono".into());
    }
    let mut code = Vec::new();
    let mut offsets = Vec::with_capacity(program.functions.len());
    for (index, function) in program.functions.iter().enumerate() {
        let func = lower::lower(index, function)
            .ok_or_else(|| format!("nguvu: kazi '{}' haikuweza kutafsiriwa", function.name))?;
        while code.len() % 16 != 0 {
            code.push(0xCC); // int3 padding between functions
        }
        offsets.push(code.len());
        code.extend(codegen::generate(&func));
    }
    let mem = mem::ExecMem::new(&code)?;
    let funcs = offsets
        .iter()
        .map(|&off| {
            // SAFETY: each offset is the start of a function generated with exactly the
            // `NativeFn` signature (System V, four pointer arguments, u64 result).
            unsafe { std::mem::transmute::<*const u8, crate::native::NativeFn>(mem.at(off)) }
        })
        .collect();
    Ok(crate::aot::NativeLibrary::from_parts(Box::new(mem), funcs))
}
