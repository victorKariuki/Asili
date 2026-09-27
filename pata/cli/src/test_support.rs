//! Fixtures shared by `pata-cli`'s unit tests.

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// The minimal `pata.toml` every fixture project uses.
pub const MANIFEST: &str = "[jumla]\njina = \"app\"\ntoleo = \"0.1.0\"\nasili = \"1.1\"\n\n[chanzo]\nkuingia = \"src/kuu.as\"\n\n[tegemezi]\n";

/// A fresh, empty, uniquely named directory `pata-<label>-<nanos>` under the system temp dir.
pub fn temp_dir(label: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("pata-{label}-{stamp}"));
    fs::create_dir_all(&dir).expect("mkdir");
    dir
}

/// A fixture project: `MANIFEST` plus `src/kuu.as` holding `main_source`.
pub fn temp_project(label: &str, main_source: &str) -> PathBuf {
    let dir = temp_dir(label);
    fs::create_dir_all(dir.join("src")).expect("mkdir src");
    fs::write(dir.join("pata.toml"), MANIFEST).expect("manifest");
    fs::write(dir.join("src/kuu.as"), main_source).expect("src");
    dir
}
