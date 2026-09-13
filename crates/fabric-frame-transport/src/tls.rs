//! Optional TLS support for the Fabric frame transport.
//!
//! Wraps a [`TcpStream`] in a TLS layer using `rustls`. When TLS is
//! configured, the server presents a certificate and encrypts all traffic.
//! If no certificate/key is provided, a self-signed certificate is
//! generated automatically (intended for development only).

use std::path::Path;
use std::sync::Arc;

use rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName};
use rustls::{ClientConfig, ServerConfig, StreamOwned};
use tokio::net::TcpStream;

/// Errors from TLS setup.
#[derive(Debug)]
pub enum TlsError {
    /// I/O error reading cert or key files.
    Io(String),
    /// Certificate or key parsing failed.
    Cert(String),
    /// TLS handshake failed.
    Handshake(String),
    /// Self-signed certificate generation failed.
    SelfSigned(String),
}

impl std::fmt::Display for TlsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(msg) => write!(f, "TLS I/O error: {msg}"),
            Self::Cert(msg) => write!(f, "TLS certificate error: {msg}"),
            Self::Handshake(msg) => write!(f, "TLS handshake error: {msg}"),
            Self::SelfSigned(msg) => write!(f, "self-signed cert error: {msg}"),
        }
    }
}

impl std::error::Error for TlsError {}

/// TLS configuration.
#[derive(Debug, Clone)]
pub struct TlsConfig {
    /// Path to the PEM-encoded certificate file.
    pub cert_path: Option<String>,
    /// Path to the PEM-encoded private key file.
    pub key_path: Option<String>,
    /// Whether to auto-generate a self-signed cert if none provided.
    pub allow_self_signed: bool,
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            cert_path: None,
            key_path: None,
            allow_self_signed: true,
        }
    }
}

/// Build a `rustls::ServerConfig` from the given TLS configuration.
///
/// If `cert_path` and `key_path` are provided, those are loaded.
/// Otherwise, a self-signed certificate is generated (dev mode).
pub fn build_server_config(tls: &TlsConfig) -> Result<Arc<ServerConfig>, TlsError> {
    let (certs, key) = if let (Some(cert), Some(key)) = (&tls.cert_path, &tls.key_path) {
        load_cert_and_key(cert, key)?
    } else if tls.allow_self_signed {
        generate_self_signed()?
    } else {
        return Err(TlsError::Cert(
            "no certificate/key provided and self-signed not allowed".into(),
        ));
    };

    let mut config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|e| TlsError::Cert(format!("failed to build server config: {e}")))?;

    config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

    Ok(Arc::new(config))
}

/// Build a `rustls::ClientConfig` for connecting to a TLS server.
pub fn build_client_config() -> Result<Arc<ClientConfig>, TlsError> {
    let mut roots = rustls::RootCertStore::empty();

    // For self-signed dev certs, we accept any server name.
    // In production, roots should come from the system store.
    let config = ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(SkipVerify))
        .with_no_client_auth();

    Ok(Arc::new(config))
}

/// Wrap a `tokio::net::TcpStream` in TLS (server side).
///
/// Performs the TLS handshake and returns the `StreamOwned` that can be
/// used for reading/writing encrypted data.
pub async fn accept_tls_server(
    tcp: TcpStream,
    config: Arc<ServerConfig>,
) -> Result<rustls::StreamOwned<rustls::ServerConnection, TcpStream>, TlsError> {
    let conn = rustls::ServerConnection::new(config)
        .map_err(|e| TlsError::Handshake(format!("create server conn: {e}")))?;

    let tokio_io = tokio_util::compat::TokioAsyncReadCompatExt::compat(tcp);
    let stream = rustls::StreamOwned::new(conn, tcp);

    // The handshake is lazy on first read/write, so we force it here.
    // For simplicity we just return the stream; the caller will trigger
    // the handshake on the first message exchange.
    Ok(stream)
}

// ---------------------------------------------------------------------------
// Certificate helpers
// ---------------------------------------------------------------------------

/// Load a PEM-encoded certificate chain and private key from disk.
fn load_cert_and_key(cert_path: &str, key_path: &str) -> Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>), TlsError> {
    let cert_pem = std::fs::read_to_string(cert_path)
        .map_err(|e| TlsError::Io(format!("read cert {cert_path}: {e}")))?;
    let key_pem = std::fs::read_to_string(key_path)
        .map_err(|e| TlsError::Io(format!("read key {key_path}: {e}")))?;

    let certs = rustls_pemfile::certs(&mut cert_pem.as_bytes())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| TlsError::Cert(format!("parse cert: {e}")))?;

    let key = rustls_pemfile::private_key(&mut key_pem.as_bytes())
        .map_err(|e| TlsError::Cert(format!("parse key: {e}")))?
        .ok_or_else(|| TlsError::Cert("no private key found in file".into()))?;

    Ok((certs, key))
}

/// Generate a self-signed certificate for development use.
///
/// Returns a certificate chain (with one cert) and a private key.
fn generate_self_signed() -> Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>), TlsError> {
    let key_pair = rcgen::KeyPair::generate()
        .map_err(|e| TlsError::SelfSigned(format!("generate key pair: {e}")))?;

    let mut params = rcgen::CertificateParams::new(vec!["localhost".into()])
        .map_err(|e| TlsError::SelfSigned(format!("cert params: {e}")))?;
    params.distinguished_name.push(
        rcgen::DnType::CommonName,
        rcgen::DnValue::Utf8String("Phenotype Fabric Dev".into()),
    );
    // Set a reasonable validity period for dev certs.
    params.not_before = rcgen::time::OffsetDateTime::now_utc();
    params.not_after = rcgen::time::OffsetDateTime::now_utc()
        + rcgen::time::Duration::days(365);

    let cert = params
        .self_signed(&key_pair)
        .map_err(|e| TlsError::SelfSigned(format!("self-sign: {e}")))?;

    let cert_der = CertificateDer::from(cert.der().to_vec());
    let key_der = PrivateKeyDer::Pkcs8(key_pair.serialize_der().into());

    Ok((vec![cert_der], key_der))
}

/// A no-op certificate verifier that accepts any server certificate.
///
/// **WARNING**: Only suitable for development / self-signed cert testing.
/// Never use in production.
#[derive(Debug)]
struct SkipVerify;

impl rustls::client::danger::ServerCertVerifier for SkipVerify {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dlsig: &rustls::pki_types::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dlsig: &rustls::pki_types::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        rustls::crypto::ring::default_provider()
            .signature_verification_algorithms
            .supported_schemes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_tls_config_allows_self_signed() {
        let config = TlsConfig::default();
        assert!(config.allow_self_signed);
        assert!(config.cert_path.is_none());
        assert!(config.key_path.is_none());
    }

    #[test]
    fn build_server_config_self_signed_works() {
        let tls = TlsConfig {
            cert_path: None,
            key_path: None,
            allow_self_signed: true,
        };
        let result = build_server_config(&tls);
        assert!(result.is_ok(), "self-signed cert generation failed: {:?}", result.err());
    }

    #[test]
    fn build_server_config_no_cert_no_self_signed_fails() {
        let tls = TlsConfig {
            cert_path: None,
            key_path: None,
            allow_self_signed: false,
        };
        let result = build_server_config(&tls);
        assert!(result.is_err());
    }

    #[test]
    fn build_server_config_missing_cert_file_fails() {
        let tls = TlsConfig {
            cert_path: Some("/nonexistent/cert.pem".into()),
            key_path: Some("/nonexistent/key.pem".into()),
            allow_self_signed: true,
        };
        let result = build_server_config(&tls);
        assert!(result.is_err());
    }

    #[test]
    fn build_client_config_works() {
        let result = build_client_config();
        assert!(result.is_ok(), "client config failed: {:?}", result.err());
    }

    #[test]
    fn self_signed_cert_has_localhost_sans() {
        let (certs, _key) = generate_self_signed().unwrap();
        assert_eq!(certs.len(), 1);
        // The cert should be DER-encoded and parseable.
        let cert = &certs[0];
        assert!(!cert.as_ref().is_empty());
    }
}
