//! TLS: certificates from PEM files, client and server configurations, handshakes. One crypto
//! provider (aws-lc-rs) for the whole program.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use rustls::crypto::CryptoProvider;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName};
use rustls::{ClientConfig, RootCertStore, ServerConfig};
use tokio::net::TcpStream;
use tokio_rustls::TlsStream;

use crate::stream::{ConnectOptions, Stream};

/// The crypto provider every TLS configuration uses.
pub fn provider() -> Arc<CryptoProvider> {
    static PROVIDER: OnceLock<Arc<CryptoProvider>> = OnceLock::new();
    PROVIDER
        .get_or_init(|| Arc::new(rustls::crypto::aws_lc_rs::default_provider()))
        .clone()
}

/// The certificates in the PEM file at `path` (at least one).
pub fn read_certs(path: &str) -> Result<Vec<CertificateDer<'static>>, String> {
    let pem = std::fs::read(path).map_err(|e| format!("{path}: {e}"))?;
    let certs = rustls_pemfile::certs(&mut pem.as_slice())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("{path}: cheti batili: {e}"))?;
    if certs.is_empty() {
        return Err(format!("{path}: hakuna cheti kwenye faili"));
    }
    Ok(certs)
}

/// The private key in the PEM file at `path`.
pub fn read_key(path: &str) -> Result<PrivateKeyDer<'static>, String> {
    let pem = std::fs::read(path).map_err(|e| format!("{path}: {e}"))?;
    rustls_pemfile::private_key(&mut pem.as_slice())
        .map_err(|e| format!("{path}: ufunguo batili: {e}"))?
        .ok_or_else(|| format!("{path}: hakuna ufunguo kwenye faili"))
}

/// A server configuration from a certificate chain and its key (PEM files), offering `alpn`.
pub fn server_config(
    cert_path: &str,
    key_path: &str,
    alpn: &[&[u8]],
) -> Result<Arc<ServerConfig>, String> {
    let certs = read_certs(cert_path)?;
    let key = read_key(key_path)?;
    let mut config = ServerConfig::builder_with_provider(provider())
        .with_safe_default_protocol_versions()
        .map_err(|e| e.to_string())?
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|e| format!("cheti na ufunguo havilingani: {e}"))?;
    config.alpn_protocols = alpn.iter().map(|p| p.to_vec()).collect();
    Ok(Arc::new(config))
}

type ClientKey = (Option<String>, Option<(String, String)>, Vec<Vec<u8>>);

/// A client configuration: trusting `ca_file`'s certificates only (default: the Mozilla roots),
/// presenting `client_cert` (certificate and key PEM files) if given, offering `alpn`. Built
/// once per distinct set of options.
pub fn client_config(
    ca_file: Option<&str>,
    client_cert: Option<(&str, &str)>,
    alpn: &[Vec<u8>],
) -> Result<Arc<ClientConfig>, String> {
    static CONFIGS: OnceLock<Mutex<HashMap<ClientKey, Arc<ClientConfig>>>> = OnceLock::new();
    let key: ClientKey = (
        ca_file.map(str::to_string),
        client_cert.map(|(c, k)| (c.to_string(), k.to_string())),
        alpn.to_vec(),
    );
    let configs = CONFIGS.get_or_init(Default::default);
    if let Some(c) = configs.lock().unwrap_or_else(|e| e.into_inner()).get(&key) {
        return Ok(c.clone());
    }
    let mut roots = RootCertStore::empty();
    match ca_file {
        Some(path) => {
            for c in read_certs(path)? {
                roots
                    .add(c)
                    .map_err(|e| format!("{path}: cheti batili: {e}"))?;
            }
        }
        None => roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned()),
    }
    let builder = ClientConfig::builder_with_provider(provider())
        .with_safe_default_protocol_versions()
        .map_err(|e| e.to_string())?
        .with_root_certificates(roots);
    let mut config = match client_cert {
        Some((cert, key)) => builder
            .with_client_auth_cert(read_certs(cert)?, read_key(key)?)
            .map_err(|e| format!("cheti cha mteja: {e}"))?,
        None => builder.with_no_client_auth(),
    };
    config.alpn_protocols = alpn.to_vec();
    let config = Arc::new(config);
    configs
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(key, config.clone());
    Ok(config)
}

/// The client side of a TLS handshake over `tcp`, for the server called `name`.
pub async fn connect(tcp: TcpStream, name: &str, opts: &ConnectOptions) -> Result<Stream, String> {
    let config = client_config(
        opts.ca_file.as_deref(),
        opts.client_cert
            .as_ref()
            .map(|(c, k)| (c.as_str(), k.as_str())),
        &opts.alpn,
    )?;
    let server_name = ServerName::try_from(name.to_string())
        .map_err(|_| format!("jina la seva batili: {name}"))?;
    let s = tokio_rustls::TlsConnector::from(config)
        .connect(server_name, tcp)
        .await
        .map_err(|e| format!("TLS {name}: {e}"))?;
    Ok(Stream::Tls(Box::new(TlsStream::Client(s))))
}

/// The server side of a TLS handshake over an accepted `stream` (TCP).
pub async fn accept(config: Arc<ServerConfig>, stream: Stream) -> Result<Stream, String> {
    let Stream::Tcp(tcp) = stream else {
        return Err("TLS inahitaji muunganisho wa TCP".to_string());
    };
    let s = tokio_rustls::TlsAcceptor::from(config)
        .accept(tcp)
        .await
        .map_err(|e| format!("TLS: {e}"))?;
    Ok(Stream::Tls(Box::new(TlsStream::Server(s))))
}
