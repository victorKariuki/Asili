//! Nguvu: Asili's own native code generator. Compiles register bytecode straight to machine
//! code in memory — no external compiler, assembler, linker or file — with the same calling
//! convention, deoptimization protocol and analysis as the LLVM backend (`aot.rs`), so the VM
//! runs either through the same [`crate::aot::NativeLibrary`] interface.
//!
//! Pipeline: [`lower`] (bytecode → [`ir`]) → [`codegen`] (IR → x86-64 via [`x64`]) → [`mem`]
//! (executable mapping). Supported on x86-64 Unix today.

pub mod codegen;
pub mod features;
pub mod ir;
pub mod lower;
pub mod mem;
pub mod opt;
pub mod range;
pub mod regalloc;
pub mod x64;

use crate::bytecode::BytecodeProgram;

/// Whether this build can generate native code for the host.
pub fn supported() -> bool {
    cfg!(all(target_arch = "x86_64", unix))
}

/// Machine code for a whole program: function `i` starts at `offsets[i]` of `code`. The code is
/// position-independent (runtime entry points are reached through the `Runtime` table), so an
/// image can be written to disk at build time and mapped as-is at run time.
pub struct Image {
    pub offsets: Vec<usize>,
    pub code: Vec<u8>,
    /// Instruction-set extensions the code uses ([`features`]).
    pub features: features::Features,
}

/// Generate machine code for every function of `program`.
pub fn generate(program: &BytecodeProgram) -> Result<Image, String> {
    if !supported() {
        return Err("nguvu: mfumo huu bado hauungwi mkono".into());
    }
    let mut code = Vec::new();
    let mut offsets = Vec::with_capacity(program.functions.len());
    for (index, function) in program.functions.iter().enumerate() {
        let mut func = lower::lower(index, function)
            .ok_or_else(|| format!("nguvu: kazi '{}' haikuweza kutafsiriwa", function.name))?;
        opt::optimize(&mut func);
        while code.len() % 16 != 0 {
            code.push(0xCC); // int3 padding between functions
        }
        offsets.push(code.len());
        code.extend(codegen::generate(&func));
    }
    if let Ok(path) = std::env::var("ASILI_NGUVU_DUMP") {
        // Debugging aid: raw machine code plus function offsets, for `objdump -b binary`.
        let _ = std::fs::write(&path, &code);
        let _ = std::fs::write(format!("{path}.offsets"), format!("{offsets:?}"));
    }
    Ok(Image {
        offsets,
        code,
        features: features::host(),
    })
}

impl Image {
    /// Map the code executable and expose its functions.
    pub fn load(&self) -> Result<crate::aot::NativeLibrary, String> {
        if self.offsets.iter().any(|&o| o >= self.code.len()) {
            return Err("nguvu: picha ya msimbo imeharibika".into());
        }
        let mem = mem::ExecMem::new(&self.code)?;
        if let Ok(path) = std::env::var("ASILI_NGUVU_DUMP") {
            // Where the code was mapped, to match profiler addresses against the dump.
            let _ = std::fs::write(format!("{path}.base"), format!("{}", mem.at(0) as usize));
        }
        let funcs = self
            .offsets
            .iter()
            .map(|&off| {
                // SAFETY: each offset is the start of a function generated with exactly the
                // `NativeFn` signature (System V, four pointer arguments, u64 result).
                unsafe { std::mem::transmute::<*const u8, crate::native::NativeFn>(mem.at(off)) }
            })
            .collect();
        Ok(crate::aot::NativeLibrary::from_parts(Box::new(mem), funcs))
    }

    /// Serialize for `program`: header (magic, format and ABI versions, architecture,
    /// required extensions, program hash), function offsets, then the code.
    pub fn to_bytes(&self, program: &BytecodeProgram) -> Vec<u8> {
        let mut out = Vec::with_capacity(40 + 4 * self.offsets.len() + self.code.len());
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&IMAGE_VERSION.to_le_bytes());
        out.extend_from_slice(&crate::aot::ABI_VERSION.to_le_bytes());
        out.extend_from_slice(&ARCH.to_le_bytes());
        out.extend_from_slice(&self.features.to_le_bytes());
        out.extend_from_slice(&crate::aot::program_hash(program).to_le_bytes());
        out.extend_from_slice(&(self.offsets.len() as u32).to_le_bytes());
        for &o in &self.offsets {
            out.extend_from_slice(&(o as u32).to_le_bytes());
        }
        out.extend_from_slice(&(self.code.len() as u64).to_le_bytes());
        out.extend_from_slice(&self.code);
        out
    }

    /// Parse an image written by [`Image::to_bytes`], rejecting one built by another toolchain
    /// version, for another machine, or from different bytecode.
    pub fn from_bytes(bytes: &[u8], program: &BytecodeProgram) -> Result<Image, String> {
        let stale = || "picha ya msimbo asilia haiendani na kilele; jenga upya".to_string();
        let mut r = Reader { bytes, at: 0 };
        if r.take(MAGIC.len()).ok_or_else(stale)? != MAGIC {
            return Err(stale());
        }
        if r.u32()? != IMAGE_VERSION || r.u32()? != crate::aot::ABI_VERSION || r.u32()? != ARCH {
            return Err(stale());
        }
        let features = r.u32()?;
        if features & !features::host() != 0 {
            return Err("picha ya msimbo asilia inahitaji maagizo ambayo prosesa hii haina".into());
        }
        if r.u64()? != crate::aot::program_hash(program) {
            return Err(stale());
        }
        let n = r.u32()? as usize;
        if n != program.functions.len() {
            return Err(stale());
        }
        let offsets = (0..n)
            .map(|_| r.u32().map(|o| o as usize))
            .collect::<Result<Vec<_>, _>>()?;
        let len = r.u64()? as usize;
        let code = r.take(len).ok_or_else(stale)?.to_vec();
        Ok(Image {
            offsets,
            code,
            features,
        })
    }
}

const MAGIC: &[u8; 8] = b"NGUVU\0\0\0";
/// Bumped whenever generated code changes, so images from an older toolchain are rebuilt
/// rather than run.
const IMAGE_VERSION: u32 = 3;
/// Instruction set of the image (1 = x86-64 System V).
const ARCH: u32 = 1;

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let s = self.bytes.get(self.at..self.at.checked_add(n)?)?;
        self.at += n;
        Some(s)
    }
    fn u32(&mut self) -> Result<u32, String> {
        self.take(4)
            .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
            .ok_or_else(|| "picha ya msimbo asilia imekatika".to_string())
    }
    fn u64(&mut self) -> Result<u64, String> {
        self.take(8)
            .map(|b| u64::from_le_bytes(b.try_into().unwrap()))
            .ok_or_else(|| "picha ya msimbo asilia imekatika".to_string())
    }
}

/// File name of the machine-code image for artifact `name` (e.g. `sudoku.nguvu`).
pub fn image_file_name(name: &str) -> String {
    format!("{name}.nguvu")
}

/// Generate `program`'s machine code and write `dir/<name>.nguvu`.
pub fn write_image(
    program: &BytecodeProgram,
    dir: &std::path::Path,
    name: &str,
) -> Result<std::path::PathBuf, String> {
    let path = dir.join(image_file_name(name));
    let bytes = generate(program)?.to_bytes(program);
    std::fs::write(&path, bytes)
        .map_err(|e| format!("imeshindwa kuandika {}: {e}", path.display()))?;
    Ok(path)
}

/// Load `path` for `program`; fails if it is missing, stale or corrupt.
pub fn load_image(
    path: &std::path::Path,
    program: &BytecodeProgram,
) -> Result<crate::aot::NativeLibrary, String> {
    let bytes =
        std::fs::read(path).map_err(|e| format!("imeshindwa kusoma {}: {e}", path.display()))?;
    Image::from_bytes(&bytes, program)?.load()
}

/// Compile every function of `program` to machine code in executable memory.
pub fn compile(program: &BytecodeProgram) -> Result<crate::aot::NativeLibrary, String> {
    generate(program)?.load()
}
