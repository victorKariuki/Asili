//! ASB (Asili bytecode) serialized format: header + payload for run-from-.asb.
//
// HACK: The "ASB" format currently serializes the parsed AST (Module) via bincode, not real bytecode.
// Running an .asb file re-interprets the AST through the tree-walk evaluator, not a VM.
// This means .asb files carry the full AST, not a compact instruction stream, and "compilation"
// provides no performance benefit over re-parsing source.
// TODO(Phase II): Replace with a real bytecode format: lower AST -> TIR -> BytecodeProgram,
// serialize BytecodeProgram, and execute with run_bytecode() instead of run_main().

use asili_parser::Module;
use std::fmt;

const ASB_HEADER_PREFIX: &str = "ASB-STUB\nversion=3\nformat=serialized\n";
const PAYLOAD_MARKER: &[u8] = b"\nPAYLOAD\n";

/// Parse format from .asb header (e.g. "serialized" or "bytecode"). Returns None if header missing.
pub fn parse_format(bytes: &[u8]) -> Option<String> {
    let pos = bytes
        .windows(PAYLOAD_MARKER.len())
        .position(|w| w == PAYLOAD_MARKER)?;
    let header = std::str::from_utf8(&bytes[..pos]).ok()?;
    if !header.starts_with("ASB-STUB") {
        return None;
    }
    for line in header.lines() {
        if let Some(v) = line.strip_prefix("format=") {
            return Some(v.trim().to_string());
        }
    }
    None
}

/// Error when loading an .asb file.
#[derive(Debug)]
pub enum AsbLoadError {
    /// No PAYLOAD marker found or format not serialized.
    InvalidFormat(String),
    /// Bincode deserialization failed.
    Decode(String),
}

impl fmt::Display for AsbLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AsbLoadError::InvalidFormat(s) => write!(f, "asb format: {s}"),
            AsbLoadError::Decode(s) => write!(f, "asb decode: {s}"),
        }
    }
}

impl std::error::Error for AsbLoadError {}

/// Emit .asb as bytes: UTF-8 header (with module_hash) then PAYLOAD marker then bincode-serialized Module.
pub fn emit_asb_bytes(module: &Module, source: &str) -> Vec<u8> {
    let payload = bincode::serialize(module).expect("Module serialization");
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    source.hash(&mut h);
    let hash = h.finish();
    let header = format!("{ASB_HEADER_PREFIX}module_hash={hash:016x}\nPAYLOAD\n");
    let mut out = header.into_bytes();
    out.extend_from_slice(&payload);
    out
}

/// Load a Module from .asb bytes. Expects format=serialized and a PAYLOAD section.
pub fn load_asb(bytes: &[u8]) -> Result<Module, AsbLoadError> {
    let payload = payload_slice(bytes)?;
    bincode::deserialize(payload).map_err(|e| AsbLoadError::Decode(e.to_string()))
}

fn payload_slice(bytes: &[u8]) -> Result<&[u8], AsbLoadError> {
    let pos = bytes
        .windows(PAYLOAD_MARKER.len())
        .position(|w| w == PAYLOAD_MARKER)
        .ok_or_else(|| {
            AsbLoadError::InvalidFormat("PAYLOAD marker not found; recompile for runnable artifact".to_string())
        })?;
    Ok(&bytes[pos + PAYLOAD_MARKER.len()..])
}

/// Load BytecodeProgram from .asb bytes. Expects format=bytecode and a PAYLOAD section.
pub fn load_asb_bytecode(bytes: &[u8]) -> Result<crate::bytecode::BytecodeProgram, AsbLoadError> {
    let payload = payload_slice(bytes)?;
    bincode::deserialize(payload).map_err(|e| AsbLoadError::Decode(e.to_string()))
}
