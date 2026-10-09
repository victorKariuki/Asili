//! Expected results of the differential tests, one file per test binary under `tests/golden/`:
//! `name::function<TAB>result` lines, the language's reference semantics (recorded from the
//! tree-walking evaluator before it was removed). Native code must reproduce every one bit for
//! bit. `ASILI_GOLDEN=write` records the results the tests compute instead — only for a new
//! snippet, after checking by hand that its results are right.

use std::collections::BTreeMap;
use std::sync::Mutex;

/// Keys recorded by this run, so two snippets cannot share one.
static WRITTEN: Mutex<Vec<String>> = Mutex::new(Vec::new());

fn path(file: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(format!("{file}.txt"))
}

fn read(file: &str) -> BTreeMap<String, String> {
    std::fs::read_to_string(path(file))
        .unwrap_or_default()
        .lines()
        .filter_map(|l| l.split_once('\t'))
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

/// The expected result for `key`, or — when recording — `actual`, stored as the expectation.
pub fn expected(file: &str, key: &str, actual: &str) -> String {
    assert!(
        !actual.contains('\n') && !key.contains('\t'),
        "{key}: one line per result"
    );
    if std::env::var_os("ASILI_GOLDEN").is_some_and(|v| v == "write") {
        let mut written = WRITTEN.lock().unwrap_or_else(|e| e.into_inner());
        let unique = format!("{file}/{key}");
        assert!(
            !written.contains(&unique),
            "{key}: recorded twice (rename a snippet)"
        );
        written.push(unique);
        let mut all = read(file);
        all.insert(key.to_string(), actual.to_string());
        let text: String = all.iter().map(|(k, v)| format!("{k}\t{v}\n")).collect();
        std::fs::write(path(file), text).expect("write golden file");
        return actual.to_string();
    }
    read(file)
        .remove(key)
        .unwrap_or_else(|| panic!("{key}: no expected result in tests/golden/{file}.txt"))
}
