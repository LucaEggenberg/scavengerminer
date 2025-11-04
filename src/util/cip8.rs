use ciborium::{ser, value::Value};
use ed25519_dalek::{SigningKey, Signer};

fn cbor_to_vec(v: &Value) -> Vec<u8> {
    let mut out = Vec::new();
    ser::into_writer(v, &mut out).expect("CBOR serialize");
    out
}

/// Build COSE_Sign1 for /register (CIP-8/30 style, as in docs):
/// - protected: {1: -8}  (alg = EdDSA)
/// - unprotected: { "address": bstr(raw_address), "hashed": false }
/// - payload: exact T&C text (UTF-8, trimmed by caller if needed)
/// - signature: Ed25519 over Sig_structure = ["Signature1", protected_bstr, h"", payload_bstr]
pub fn cose_sign1_ed25519_with_headers(
    privkey: &[u8; 32],
    payload_utf8: &str,
    address_raw: &[u8],
    hashed: bool,
) -> Vec<u8> {
    // protected header map { 1: -8 }  (alg: EdDSA)
    let protected_map = Value::Map(vec![
        (Value::Integer(1i64.into()), Value::Integer((-8i64).into())),
    ]);
    let protected_bstr = cbor_to_vec(&protected_map);

    // unprotected header map { "address": bstr(...), "hashed": false }
    let unprotected = Value::Map(vec![
        (Value::Text("address".into()), Value::Bytes(address_raw.to_vec())),
        (Value::Text("hashed".into()),  Value::Bool(hashed)),
    ]);

    // payload bytes (bstr)
    let payload_bytes = payload_utf8.as_bytes().to_vec();

    // Sig_structure = ["Signature1", protected_bstr, external_aad(b""), payload_bstr]
    let sig_structure = Value::Array(vec![
        Value::Text("Signature1".into()),
        Value::Bytes(protected_bstr.clone()),
        Value::Bytes(Vec::new()),
        Value::Bytes(payload_bytes.clone()),
    ]);
    let to_sign = cbor_to_vec(&sig_structure);

    // Ed25519 sign
    let sk = SigningKey::from_bytes(privkey);
    let sig = sk.sign(&to_sign).to_bytes().to_vec();

    // COSE_Sign1 array: [protected_bstr, unprotected_map, payload_bstr, signature_bstr]
    let cose_sign1 = Value::Array(vec![
        Value::Bytes(protected_bstr),
        unprotected,
        Value::Bytes(payload_bytes),
        Value::Bytes(sig),
    ]);

    cbor_to_vec(&cose_sign1)
}