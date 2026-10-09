//! The `.asb` artifact: a text header, then the program's bytecode (`BytecodeProgram`) and the
//! syntax tree it carries, serialized with bincode.

use std::fmt;

const ASB_HEADER_PREFIX: &str = "ASB-STUB\nversion=5\nformat=serialized\n";
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

/// Version of the bytecode payload (its instruction set, and the syntax tree it carries). Artifacts built by an
/// older `pata jenga` carry a different `version=` and must be rebuilt.
const BYTECODE_VERSION: &str = "17";

/// Every artifact format this build reads and writes, for build caches: a cached artifact made
/// under different formats (an older or newer toolchain) must be rebuilt, not loaded.
pub fn artifact_formats() -> String {
    format!(
        "{ASB_HEADER_PREFIX}bytecode={BYTECODE_VERSION};compiler={}",
        env!("CARGO_PKG_VERSION")
    )
}

/// Emit a real bytecode artifact.  The header remains intentionally simple and textual so older
/// runners can reject it cleanly, while the payload is the same deterministic bincode envelope
/// used by the AST fallback.
pub fn emit_bytecode_bytes(program: &crate::bytecode::BytecodeProgram, source: &str) -> Vec<u8> {
    let payload = bincode::serialize(program).expect("BytecodeProgram serialization");
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    source.hash(&mut h);
    let header = format!(
        "ASB-STUB\nversion={BYTECODE_VERSION}\nformat=bytecode\nmodule_hash={:016x}\nPAYLOAD\n",
        h.finish()
    );
    let mut out = header.into_bytes();
    out.extend_from_slice(&payload);
    out
}
fn payload_slice(bytes: &[u8]) -> Result<&[u8], AsbLoadError> {
    let pos = bytes
        .windows(PAYLOAD_MARKER.len())
        .position(|w| w == PAYLOAD_MARKER)
        .ok_or_else(|| {
            AsbLoadError::InvalidFormat(
                "alama ya PAYLOAD haipo; jenga upya ili kupata kilele kinachotendeka".to_string(),
            )
        })?;
    Ok(&bytes[pos + PAYLOAD_MARKER.len()..])
}

/// Load BytecodeProgram from .asb bytes. Expects format=bytecode and a PAYLOAD section.
pub fn load_asb_bytecode(bytes: &[u8]) -> Result<crate::bytecode::BytecodeProgram, AsbLoadError> {
    let payload = payload_slice(bytes)?;
    let header = std::str::from_utf8(&bytes[..bytes.len() - payload.len()]).unwrap_or("");
    let version = header.lines().find_map(|l| l.strip_prefix("version="));
    if version != Some(BYTECODE_VERSION) {
        return Err(AsbLoadError::InvalidFormat(
            "kilele kilijengwa na toleo jingine la bytecode; jenga upya kwa `pata jenga`"
                .to_string(),
        ));
    }
    let program: crate::bytecode::BytecodeProgram =
        bincode::deserialize(payload).map_err(|e| AsbLoadError::Decode(e.to_string()))?;
    crate::bytecode_verify::verify(&program).map_err(|e| {
        AsbLoadError::InvalidFormat(format!(
            "kilele kimeharibika ({e}); jenga upya kwa `pata jenga`"
        ))
    })?;
    Ok(program)
}
