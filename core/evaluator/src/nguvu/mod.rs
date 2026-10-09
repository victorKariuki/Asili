//! Nguvu: Asili's own native code generator. Compiles register bytecode straight to machine
//! code — no external compiler, assembler or linker — the only way bytecode runs, through
//! [`crate::aot::NativeLibrary`] and its host (`host.rs`).
//!
//! Pipeline: [`lower`] (bytecode → [`ir`]) → [`opt`] (with [`range`]) → [`regalloc`] and
//! [`schedule`] (shared by every target) → [`codegen`] (x86-64 via [`x64`]) or
//! [`codegen_a64`] (AArch64 via [`a64`]) → [`mem`] (executable mapping). Supported on x86-64
//! and AArch64 Unix (Linux, macOS, BSD) and x86-64 Windows.

pub mod a64;
pub mod codegen;
pub mod codegen_a64;
pub mod features;
pub mod ir;
pub mod lower;
pub mod mem;
pub mod opt;
pub mod range;
pub mod regalloc;
pub mod schedule;
pub mod wasm;
pub mod x64;

use crate::bytecode::BytecodeProgram;

/// Whether this build can generate native code for the host: machine code, or a wasm module
/// in the browser build (a page instantiates it; WASI cannot).
pub fn supported() -> bool {
    cfg!(any(
        all(any(target_arch = "x86_64", target_arch = "aarch64"), unix),
        all(target_arch = "x86_64", windows),
        all(
            target_arch = "wasm32",
            feature = "wasm-browser",
            not(feature = "wasm-wasi")
        )
    ))
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

/// One function's machine code and its direct calls to other functions' direct entries, to be
/// linked once every function's place in the image is known.
pub struct Code {
    pub bytes: Vec<u8>,
    /// `(offset, callee)`: on x86-64 the offset of a `call`'s 32-bit displacement, on AArch64
    /// the offset of a `bl` instruction.
    pub calls: Vec<(usize, u32)>,
}

/// Functions that get a direct entry: numeric signature, and a body that lowers without calls
/// into the host given that exactly these functions are callable directly (an optimistic
/// fixpoint, so mutually recursive functions qualify together).
fn direct_entries(program: &BytecodeProgram) -> asili_parser::FxHashSet<usize> {
    let mut direct: asili_parser::FxHashSet<usize> = program
        .functions
        .iter()
        .enumerate()
        .filter(|(_, f)| lower::direct_signature(f))
        .map(|(i, _)| i)
        .collect();
    loop {
        let mut candidates: Vec<usize> = direct.iter().copied().collect();
        candidates.sort_unstable();
        let lowers = par_map(&candidates, |&i| {
            let ctx = lower::Ctx {
                program,
                direct: &direct,
                entry_direct: true,
            };
            lower::lower(i, &program.functions[i], &ctx).is_some()
        });
        let failing: Vec<usize> = candidates
            .iter()
            .zip(lowers)
            .filter(|(_, ok)| !ok)
            .map(|(&i, _)| i)
            .collect();
        if failing.is_empty() {
            return direct;
        }
        for i in failing {
            direct.remove(&i);
        }
    }
}

/// `f` applied to every item, on up to one thread per core, results in the items' order. Threads
/// claim the next item as they finish one, so one large function does not hold up the rest.
fn par_map<T: Sync, R: Send>(items: &[T], f: impl Fn(&T) -> R + Sync) -> Vec<R> {
    let threads = std::thread::available_parallelism()
        .map_or(1, |n| n.get())
        .min(items.len());
    if threads <= 1 {
        return items.iter().map(f).collect();
    }
    let next = std::sync::atomic::AtomicUsize::new(0);
    let mut done: Vec<(usize, R)> = std::thread::scope(|scope| {
        let workers: Vec<_> = (0..threads)
            .map(|_| {
                scope.spawn(|| {
                    let mut out = Vec::new();
                    loop {
                        let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        let Some(item) = items.get(i) else {
                            return out;
                        };
                        out.push((i, f(item)));
                    }
                })
            })
            .collect();
        workers
            .into_iter()
            .flat_map(|w| w.join().unwrap_or_else(|e| std::panic::resume_unwind(e)))
            .collect()
    });
    done.sort_unstable_by_key(|(i, _)| *i);
    done.into_iter().map(|(_, r)| r).collect()
}

fn compile_function(func: &mut ir::Func) -> Result<Code, String> {
    opt::optimize(func);
    #[cfg(target_arch = "aarch64")]
    return codegen_a64::generate(func);
    #[cfg(not(target_arch = "aarch64"))]
    Ok(codegen::generate(func, codegen::HOST))
}

/// Generate machine code for every function of `program`.
pub fn generate(program: &BytecodeProgram) -> Result<Image, String> {
    if !supported() {
        return Err("nguvu: mfumo huu bado hauungwi mkono".into());
    }
    let direct = direct_entries(program);
    // Ordinary entries first (function `i` at `offsets[i]`), then the direct entries. Each is
    // lowered and compiled on its own, in parallel; the image is laid out in this order.
    let mut jobs: Vec<(bool, usize)> = (0..program.functions.len()).map(|i| (false, i)).collect();
    jobs.extend(
        (0..program.functions.len())
            .filter(|i| direct.contains(i))
            .map(|i| (true, i)),
    );
    let compiled = par_map(&jobs, |&(entry_direct, index)| {
        let function = &program.functions[index];
        let ctx = lower::Ctx {
            program,
            direct: &direct,
            entry_direct,
        };
        let mut func = lower::lower(index, function, &ctx)
            .ok_or_else(|| format!("nguvu: kazi '{}' haikuweza kutafsiriwa", function.name))?;
        compile_function(&mut func)
    });
    let mut units: Vec<(Option<usize>, Code)> = Vec::with_capacity(jobs.len()); // (direct entry of, code)
    for (&(entry_direct, index), code) in jobs.iter().zip(compiled) {
        units.push((entry_direct.then_some(index), code?));
    }
    let mut code = Vec::new();
    let mut offsets = Vec::with_capacity(program.functions.len());
    let mut direct_at: asili_parser::FxHashMap<usize, usize> = Default::default();
    let mut placed = Vec::with_capacity(units.len());
    for (of, unit) in &units {
        while code.len() % regalloc::LOOP_ALIGN != 0 {
            // Trap padding between functions: `int3` on x86-64, `udf #0` words on AArch64.
            code.push(if cfg!(target_arch = "aarch64") {
                0x00
            } else {
                0xCC
            });
        }
        match of {
            Some(i) => {
                direct_at.insert(*i, code.len());
            }
            None => offsets.push(code.len()),
        }
        placed.push(code.len());
        code.extend_from_slice(&unit.bytes);
    }
    // Link direct calls (position independent: relative to the call site).
    for ((_, unit), base) in units.iter().zip(placed) {
        for &(at, callee) in &unit.calls {
            let site = base + at;
            let target = direct_at[&(callee as usize)] as i64;
            if cfg!(target_arch = "aarch64") {
                let delta = (target - site as i64) / 4;
                if !(-(1 << 25)..(1 << 25)).contains(&delta) {
                    return Err("nguvu: picha ni kubwa mno kwa mwito wa moja kwa moja".into());
                }
                let mut w = u32::from_le_bytes(code[site..site + 4].try_into().unwrap());
                w |= (delta as u32) & 0x03FF_FFFF;
                code[site..site + 4].copy_from_slice(&w.to_le_bytes());
            } else {
                let rel = target - (site as i64 + 4);
                code[site..site + 4].copy_from_slice(&(rel as i32).to_le_bytes());
            }
        }
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
const IMAGE_VERSION: u32 = 8;
/// Instruction set and calling convention of the image (1 = x86-64 System V, 2 = AArch64
/// AAPCS64, 3 = x86-64 Microsoft x64).
const ARCH: u32 = if cfg!(target_arch = "aarch64") {
    2
} else if cfg!(windows) {
    3
} else {
    1
};

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
    load_image_bytes(&bytes, program)
}

/// [`load_image`] from an image already in memory (one carried inside an executable).
pub fn load_image_bytes(
    bytes: &[u8],
    program: &BytecodeProgram,
) -> Result<crate::aot::NativeLibrary, String> {
    Image::from_bytes(bytes, program)?.load()
}

/// Compile every function of `program` to machine code in executable memory.
#[cfg(not(target_arch = "wasm32"))]
pub fn compile(program: &BytecodeProgram) -> Result<crate::aot::NativeLibrary, String> {
    generate(program)?.load()
}

/// Compile every function of `program` to one wasm module and instantiate it beside this one
/// ([`wasm`]); there are no direct entries.
#[cfg(all(
    target_arch = "wasm32",
    feature = "wasm-browser",
    not(feature = "wasm-wasi")
))]
pub fn compile(program: &BytecodeProgram) -> Result<crate::aot::NativeLibrary, String> {
    let funcs = wasm_functions(program)?;
    let bytes = wasm::module(&funcs)?;
    let base = crate::platform::load_native_module(&bytes, funcs.len() as u32)?;
    let entries = (0..funcs.len())
        .map(|i| {
            // SAFETY: table slot `base + i` holds function `i` of the module just
            // instantiated, of exactly the `NativeFn` type; a wasm function pointer is its
            // table index.
            unsafe { std::mem::transmute::<usize, crate::native::NativeFn>(base + i) }
        })
        .collect();
    Ok(crate::aot::NativeLibrary::from_parts(Box::new(()), entries))
}

#[cfg(all(
    target_arch = "wasm32",
    any(not(feature = "wasm-browser"), feature = "wasm-wasi")
))]
pub fn compile(_program: &BytecodeProgram) -> Result<crate::aot::NativeLibrary, String> {
    Err("nguvu: mfumo huu bado hauungwi mkono".into())
}

/// Every function of `program` lowered and optimized, without direct entries (the wasm
/// target's input).
pub fn wasm_functions(program: &BytecodeProgram) -> Result<Vec<ir::Func>, String> {
    let direct = Default::default();
    program
        .functions
        .iter()
        .enumerate()
        .map(|(i, function)| {
            let ctx = lower::Ctx {
                program,
                direct: &direct,
                entry_direct: false,
            };
            let mut func = lower::lower(i, function, &ctx)
                .ok_or_else(|| format!("nguvu: kazi '{}' haikuweza kutafsiriwa", function.name))?;
            opt::optimize(&mut func);
            Ok(func)
        })
        .collect()
}
