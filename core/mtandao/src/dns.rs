//! Name lookup.

/// The addresses `host` resolves to, in the resolver's order, without repeats.
pub async fn resolve(host: &str) -> Result<Vec<String>, String> {
    let found = tokio::net::lookup_host((host, 0))
        .await
        .map_err(|e| format!("kutafuta {host}: {e}"))?;
    let mut out: Vec<String> = Vec::new();
    for a in found {
        let ip = a.ip().to_string();
        if !out.contains(&ip) {
            out.push(ip);
        }
    }
    Ok(out)
}
