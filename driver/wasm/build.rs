//! The wasm build exports its function table, growable: native code is a second wasm module
//! the page instantiates against this one's table (`asili_evaluator::nguvu::wasm`).

fn main() {
    if std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("wasm32") {
        println!("cargo:rustc-link-arg-cdylib=--export-table");
        println!("cargo:rustc-link-arg-cdylib=--growable-table");
    }
}
