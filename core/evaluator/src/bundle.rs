//! Standalone executables: a program's `.asb` artifact and native image appended to a copy of
//! the runner. The runner checks its own file for this payload when it starts, so the result runs
//! directly (`./jina`) — no `pata`, no separate artifact files, and no external linker.
//!
//! Layout: `runner bytes | asb | image | asb_len: u64 LE | image_len: u64 LE | MAGIC`. Trailing
//! data is ignored by ELF, Mach-O and PE loaders alike.

use std::io::{Read, Seek, SeekFrom};

const MAGIC: &[u8; 8] = b"ASILIEXE";
const TRAILER: u64 = 8 + 8 + 8;

/// A program carried inside an executable.
pub struct Bundle {
    pub asb: Vec<u8>,
    /// The native image `pata jenga` built, when there is one.
    pub image: Option<Vec<u8>>,
}

/// `runner` with `asb` (and `image`) appended, ready to write out as an executable.
pub fn assemble(runner: &[u8], asb: &[u8], image: Option<&[u8]>) -> Vec<u8> {
    let image = image.unwrap_or(&[]);
    let mut out = Vec::with_capacity(runner.len() + asb.len() + image.len() + TRAILER as usize);
    out.extend_from_slice(runner);
    out.extend_from_slice(asb);
    out.extend_from_slice(image);
    out.extend_from_slice(&(asb.len() as u64).to_le_bytes());
    out.extend_from_slice(&(image.len() as u64).to_le_bytes());
    out.extend_from_slice(MAGIC);
    out
}

/// The program appended to the running executable, if any. Reads only the trailer unless the
/// magic is there.
pub fn embedded() -> Option<Bundle> {
    let path = std::env::current_exe().ok()?;
    let mut file = std::fs::File::open(path).ok()?;
    let size = file.metadata().ok()?.len();
    if size < TRAILER {
        return None;
    }
    let mut trailer = [0u8; TRAILER as usize];
    file.seek(SeekFrom::Start(size - TRAILER)).ok()?;
    file.read_exact(&mut trailer).ok()?;
    if &trailer[16..] != MAGIC {
        return None;
    }
    let asb_len = u64::from_le_bytes(trailer[..8].try_into().ok()?);
    let image_len = u64::from_le_bytes(trailer[8..16].try_into().ok()?);
    let start = size
        .checked_sub(TRAILER)?
        .checked_sub(asb_len)?
        .checked_sub(image_len)?;
    file.seek(SeekFrom::Start(start)).ok()?;
    let mut asb = vec![0u8; asb_len as usize];
    file.read_exact(&mut asb).ok()?;
    let mut image = vec![0u8; image_len as usize];
    file.read_exact(&mut image).ok()?;
    Some(Bundle {
        asb,
        image: (image_len > 0).then_some(image),
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn trailer_round_trips() {
        let exe = super::assemble(b"RUNNER", b"asb-bytes", Some(b"img"));
        let n = exe.len();
        assert_eq!(&exe[n - 8..], super::MAGIC);
        assert_eq!(&exe[6..15], b"asb-bytes");
        assert_eq!(&exe[15..18], b"img");
        assert_eq!(
            u64::from_le_bytes(exe[n - 24..n - 16].try_into().unwrap()),
            9
        );
        assert_eq!(
            u64::from_le_bytes(exe[n - 16..n - 8].try_into().unwrap()),
            3
        );
    }
}
