#![forbid(unsafe_code)]

use std::path::Path;

use russh::keys::{Algorithm, PrivateKey};

/// SFTP protocol version implemented by this crate.
pub const PROTOCOL_VERSION: u32 = 3;

/// Returns the SFTP protocol version implemented by this crate.
#[must_use]
pub const fn protocol_version() -> u32 {
    PROTOCOL_VERSION
}

/// Error returned when a host key cannot be used.
#[derive(Debug, thiserror::Error)]
pub enum HostKeyError {
    /// The host key file could not be read.
    #[error("cannot read host key: {0}")]
    Io(#[from] std::io::Error),

    /// The host key could not be parsed.
    #[error("cannot parse host key: {0}")]
    Parse(#[from] russh::keys::ssh_key::Error),

    /// RSA host keys are not allowed in v0.1.
    #[error("RSA host keys are not allowed in v0.1: RUSTSEC-2023-0071")]
    RsaUnsupported,

    /// The host key algorithm is not supported.
    #[error("unsupported host key algorithm: {0}")]
    Unsupported(String),
}

/// Validates that a host key file is a supported private key.
///
/// RSA host keys are rejected because the `rsa` crate has an open timing
/// side-channel advisory for private-key operations. RSA user keys remain
/// usable for public-key verification.
pub fn validate_host_key(path: &Path) -> Result<(), HostKeyError> {
    let text = std::fs::read_to_string(path)?;
    let key = PrivateKey::from_openssh(text.as_bytes())?;

    match key.algorithm() {
        Algorithm::Ed25519 | Algorithm::Ecdsa { .. } => Ok(()),
        Algorithm::Rsa { .. } => Err(HostKeyError::RsaUnsupported),
        other => Err(HostKeyError::Unsupported(format!("{other:?}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::HostKeyError;

    #[test]
    fn sftp_v3_is_the_target() {
        assert_eq!(super::protocol_version(), 3);
    }

    #[test]
    fn missing_host_key_is_an_io_error() {
        let error = super::validate_host_key(std::path::Path::new(
            "/nonexistent/latchix/host_ed25519_key",
        ))
        .expect_err("missing key must fail");
        assert!(matches!(error, HostKeyError::Io(_)));
    }
}
