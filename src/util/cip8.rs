use ed25519_dalek::{SigningKey, Signer};
use ciborium::{ser, value::Value};

fn cbor_to_vec(v: &Value) -> Vec<u8> {
    let mut out = Vec::new();
    ser::into_writer(v, &mut out).expect("CBOR serialize");
    out
}

pub fn cose_sign1_ed25519(privkey: &[u8;32], message: &str) -> Vec<u8> {
    let sk = SigningKey::from_bytes(privkey);

    // protected header: { 1: -8 }  (alg: EdDSA)
    let protected_map = Value::Map(vec![
        (Value::Integer(1i64.into()), Value::Integer((-8i64).into())),
    ]);
    let protected_bstr = cbor_to_vec(&protected_map);

    // Sig_structure = ["Signature1", protected_bstr, external_aad=b"", payload_bstr]
    let sig_structure = Value::Array(vec![
        Value::Text("Signature1".into()),
        Value::Bytes(protected_bstr.clone()),
        Value::Bytes(Vec::new()),
        Value::Bytes(message.as_bytes().to_vec()),
    ]);

    let to_sign = cbor_to_vec(&sig_structure);
    let sig = sk.sign(&to_sign).to_bytes().to_vec();

    // COSE_Sign1 = [ protected_bstr, {}, payload_bstr, signature_bstr ]
    let cose_sign1 = Value::Array(vec![
        Value::Bytes(protected_bstr),
        Value::Map(vec![]),
        Value::Bytes(message.as_bytes().to_vec()),
        Value::Bytes(sig),
    ]);

    cbor_to_vec(&cose_sign1)
}