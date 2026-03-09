//! Binary entry point for Mwalimu (Asili LSP). Also runnable via `pata mwalimu`.

#[tokio::main]
async fn main() {
    pata_lsp::run_stdio().await;
}
