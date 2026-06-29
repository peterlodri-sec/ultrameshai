use loop_engineering_node_registry::crypto;
use http::header::{HeaderMap, HeaderName};

#[test]
fn test_sign_and_verify_roundtrip() {
    let payload = b"heartbeat:vm-01:1700000000";
    let secret = b"supersecret";

    let signature = crypto::sign_payload(payload, secret);
    let ok = crypto::verify_signature(payload, &signature, secret)
        .expect("verify should succeed");
    assert!(ok, "valid signature should verify");
}

#[test]
fn test_verify_wrong_secret_fails() {
    let payload = b"data";
    let secret = b"secret123";
    let wrong = b"wrong456";

    let signature = crypto::sign_payload(payload, secret);
    let ok = crypto::verify_signature(payload, &signature, wrong)
        .expect("verify should succeed (no error)");
    assert!(!ok, "wrong secret should fail verification");
}

#[test]
fn test_verify_tampered_payload_fails() {
    let payload = b"original";
    let tampered = b"tampered";
    let secret = b"secret";

    let signature = crypto::sign_payload(payload, secret);
    let ok = crypto::verify_signature(tampered, &signature, secret)
        .expect("verify should succeed (no error)");
    assert!(!ok, "tampered payload should fail verification");
}

#[test]
fn test_extract_signature() {
    let mut headers = HeaderMap::new();
    headers.insert(
        HeaderName::from_static("x-signature"),
        "hmac-sha256=abcdef1234567890".parse().unwrap(),
    );

    let sig = crypto::extract_signature(&headers);
    assert_eq!(sig, Some("abcdef1234567890".to_string()));
}

#[test]
fn test_extract_signature_missing() {
    let headers = HeaderMap::new();
    let sig = crypto::extract_signature(&headers);
    assert_eq!(sig, None);
}

#[test]
fn test_sign_payload_deterministic() {
    let payload = b"test";
    let secret = b"key";
    let sig1 = crypto::sign_payload(payload, secret);
    let sig2 = crypto::sign_payload(payload, secret);
    assert_eq!(sig1, sig2, "same input produces same signature");
}
