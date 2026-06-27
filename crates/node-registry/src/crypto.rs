use hmac::{Hmac, Mac};
use sha2::Sha256;
use hex;
use http::header::{HeaderMap, HeaderName};
use crate::error::{RegistryError, Result};

type HmacSha256 = Hmac<Sha256>;

/// Verify HMAC-SHA256 signature
pub fn verify_signature(payload: &[u8], signature: &str, secret: &[u8]) -> Result<bool> {
    let mut mac = HmacSha256::new_from_slice(secret)
        .map_err(|e| RegistryError::Crypto(e.to_string()))?;
    mac.update(payload);
    let expected = mac.finalize().into_bytes();
    let provided = hex::decode(signature)
        .map_err(|e| RegistryError::Crypto(e.to_string()))?;
    Ok(expected.as_slice() == provided.as_slice())
}

/// Extract signature from headers
pub fn extract_signature(headers: &HeaderMap) -> Option<String> {
    let header_name = HeaderName::from_static("x-signature");
    headers
        .get(&header_name)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("hmac-sha256="))
        .map(String::from)
}

/// Sign a payload (for testing/client use)
pub fn sign_payload(payload: &[u8], secret: &[u8]) -> String {
    let mut mac = HmacSha256::new_from_slice(secret).expect("HMAC can take key of any size");
    mac.update(payload);
    let result = mac.finalize();
    hex::encode(result.into_bytes())
}
