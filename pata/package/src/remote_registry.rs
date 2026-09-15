//! Static-file/hosted registry index over HTTP — a real remote alternative to `LocalRegistry`'s
//! on-disk-directory index, matching Cargo's alternative-registry RFC minimum surface (a JSON
//! index + tarball download, no API server required): a git repository or any static file host
//! serving
//! ```text
//! index/<package-name>/index.json   # [{"version": "1.2.3", "checksum": "<sha256>", "url": "https://.../pkg-1.2.3.tar.gz"}, ...]
//! ```
//! No publish command, no auth, no dynamic API — publishing to the index is a manual commit to
//! whatever host serves it. See `docs/design/pata-implementation-spec.md` Section 16 and
//! `docs/design/pata-production-readiness.md`.
//!
//! Deliberately separate from `LocalRegistry`/`RegistryEntry` (`registry.rs`): those types
//! already have real callers throughout `resolver.rs` with a Cargo/crates.io-shaped schema
//! (`name`, `vers`, `deps`, `source`) — reusing that name for this index's much flatter
//! `{version, checksum, url}` row would collide. `RemoteIndexEntry` here is that flatter shape;
//! `fetch_and_verify` converges back onto `registry.rs::RegistrySource::Http` once a version is
//! chosen, so the rest of the resolver's dispatch (`fetch_from_registry_source`) doesn't need to
//! know the entry came from a remote index at all.

use anyhow::{Context, Result};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::Path;

/// A placeholder — no real hosted index exists for this project yet. Callers must pass a real
/// `index_base` explicitly; this constant exists only so the shape of a real deployment is
/// documented in code, matching how the original design spec flagged this exact value as the one
/// genuinely unresolvable-by-this-doc residue (a hosting/ops decision, not a code-shape one).
pub const DEFAULT_INDEX_BASE_URL: &str = "https://REPLACE-ME.example/pata-index";

/// One row of a remote index's `index/<name>/index.json` — the flat `{version, checksum, url}`
/// shape Cargo's alternative-registry RFC uses, distinct from `registry::RegistryEntry`.
#[derive(Debug, Clone, Deserialize)]
pub struct RemoteIndexEntry {
    pub version: String,
    pub checksum: String,
    pub url: String,
}

/// Fetch and parse `<index_base>/index/<name>/index.json` — the full list of published versions
/// for `name` on a remote static-file index. A real HTTP GET (via `ureq`), so this needs network
/// access; callers resolving against a fully local project should never reach this path.
pub fn fetch_index(index_base: &str, name: &str) -> Result<Vec<RemoteIndexEntry>> {
    let url = format!("{}/index/{}/index.json", index_base.trim_end_matches('/'), name);
    let body = ureq::get(&url)
        .call()
        .with_context(|| format!("imeshindwa kupata faharasa ya rejista kutoka {url}"))?
        .into_string()
        .with_context(|| format!("jibu batili kutoka {url}"))?;
    serde_json::from_str(&body).with_context(|| format!("muundo batili wa JSON: {url}"))
}

/// Download `entry.url`'s tarball, verify its SHA-256 matches `entry.checksum` (defense in
/// depth — a tampered/compromised index host could otherwise serve mismatched content
/// undetected; combined with `LockFile::verify_content_integrity`'s independent re-check of the
/// *extracted* content at every subsequent build, this catches both "wrong bytes downloaded" and
/// "vendored directory tampered with after the fact"), then extract it into `dest_dir`. Returns
/// the real post-extraction `hash_dir` checksum over the extracted files — not `entry.checksum`
/// itself — so it drops straight into the same `LockedDependency.checksum` convention every other
/// source (`fetch_git`/`Path`) already produces, without the caller needing a separate hashing
/// step.
pub fn fetch_and_verify(entry: &RemoteIndexEntry, dest_dir: &Path) -> Result<String> {
    let response = ureq::get(&entry.url)
        .call()
        .with_context(|| format!("imeshindwa kupakua {}", entry.url))?;
    let mut bytes = Vec::new();
    response
        .into_reader()
        .read_to_end(&mut bytes)
        .with_context(|| format!("imeshindwa kusoma jibu kutoka {}", entry.url))?;

    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let digest = hasher.finalize();
    let actual = digest.iter().map(|b| format!("{b:02x}")).collect::<String>();
    if !actual.eq_ignore_ascii_case(&entry.checksum) {
        anyhow::bail!(
            "hundi ya usalama imeshindwa kwa {}: tarajiwa {}, halisi {actual}",
            entry.url,
            entry.checksum
        );
    }

    if dest_dir.exists() {
        std::fs::remove_dir_all(dest_dir)
            .with_context(|| format!("imeshindwa kuondoa {}", dest_dir.display()))?;
    }
    std::fs::create_dir_all(dest_dir)
        .with_context(|| format!("imeshindwa kuunda {}", dest_dir.display()))?;

    let decoder = flate2::read::GzDecoder::new(bytes.as_slice());
    let mut archive = tar::Archive::new(decoder);
    archive
        .unpack(dest_dir)
        .with_context(|| format!("imeshindwa kufungua tarball kwenda {}", dest_dir.display()))?;

    crate::fetch::hash_dir(dest_dir).map_err(|e| anyhow::anyhow!("{e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;
    use std::net::TcpListener;

    /// Build a real `.tar.gz` in memory containing one file, for a fake registry response.
    fn build_tarball(file_name: &str, content: &[u8]) -> Vec<u8> {
        let mut tar_bytes = Vec::new();
        {
            let mut builder = tar::Builder::new(&mut tar_bytes);
            let mut header = tar::Header::new_gnu();
            header.set_size(content.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder.append_data(&mut header, file_name, content).expect("append tar entry");
            builder.finish().expect("finish tar");
        }
        let mut gz = GzEncoder::new(Vec::new(), Compression::default());
        gz.write_all(&tar_bytes).expect("gzip tar bytes");
        gz.finish().expect("finish gzip")
    }

    fn sha256_hex(bytes: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        hasher.finalize().iter().map(|b| format!("{b:02x}")).collect::<String>()
    }

    /// Spin up a real local HTTP server (`tiny_http`, dev-only) serving a fixed `index.json` and
    /// a fixed tarball, and confirm `fetch_index`/`fetch_and_verify` work against it for real —
    /// no mocking of `ureq` itself, an actual socket round-trip on `127.0.0.1`.
    #[test]
    fn fetch_index_and_fetch_and_verify_work_against_a_real_local_http_server() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
        let addr = listener.local_addr().expect("local addr");
        drop(listener); // tiny_http binds its own listener; just needed a free port number

        let server = tiny_http::Server::http(addr).expect("start tiny_http server");
        let base_url = format!("http://{addr}");

        let tarball = build_tarball("greeter.as", b"umma kazi salamu() -> Neno { rejesha \"hi\" }");
        let checksum = sha256_hex(&tarball);
        let index_json = format!(
            r#"[{{"version":"1.0.0","checksum":"{checksum}","url":"{base_url}/pkg-1.0.0.tar.gz"}}]"#
        );

        let handle = std::thread::spawn(move || {
            // Two requests expected: GET /index/greeter/index.json, then GET /pkg-1.0.0.tar.gz
            for _ in 0..2 {
                let request = server.recv().expect("recv request");
                let url = request.url().to_string();
                if url.contains("index.json") {
                    let response = tiny_http::Response::from_string(index_json.clone());
                    request.respond(response).expect("respond index");
                } else {
                    let response = tiny_http::Response::from_data(tarball.clone());
                    request.respond(response).expect("respond tarball");
                }
            }
        });

        let entries = fetch_index(&base_url, "greeter").expect("fetch_index ok");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].version, "1.0.0");
        assert_eq!(entries[0].checksum, checksum);

        let dest = std::env::temp_dir().join(format!("pata-remote-registry-test-{}", std::process::id()));
        let real_checksum = fetch_and_verify(&entries[0], &dest).expect("fetch_and_verify ok");
        assert_eq!(real_checksum.len(), 64, "expected a real 64-hex-char SHA-256 digest");
        assert!(dest.join("greeter.as").is_file(), "tarball must actually be extracted");
        assert_eq!(
            std::fs::read_to_string(dest.join("greeter.as")).unwrap(),
            "umma kazi salamu() -> Neno { rejesha \"hi\" }"
        );

        handle.join().expect("server thread");
        std::fs::remove_dir_all(&dest).ok();
    }

    /// The actual security property: a tarball whose bytes don't match the index's recorded
    /// checksum (a tampered/compromised host, or index/content drift) must be rejected, not
    /// silently extracted.
    #[test]
    fn fetch_and_verify_rejects_checksum_mismatch() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
        let addr = listener.local_addr().expect("local addr");
        drop(listener);

        let server = tiny_http::Server::http(addr).expect("start tiny_http server");
        let base_url = format!("http://{addr}");
        let tarball = build_tarball("f.as", b"real content");

        let handle = std::thread::spawn(move || {
            let request = server.recv().expect("recv request");
            let response = tiny_http::Response::from_data(tarball);
            request.respond(response).expect("respond tarball");
        });

        let entry = RemoteIndexEntry {
            version: "1.0.0".to_string(),
            checksum: "0".repeat(64), // deliberately wrong
            url: format!("{base_url}/pkg.tar.gz"),
        };
        let dest = std::env::temp_dir().join(format!("pata-remote-registry-mismatch-{}", std::process::id()));
        let result = fetch_and_verify(&entry, &dest);
        assert!(result.is_err(), "a checksum mismatch must fail, not silently extract");

        handle.join().expect("server thread");
        std::fs::remove_dir_all(&dest).ok();
    }
}
