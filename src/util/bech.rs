use blake2::{Blake2bVar, digest::{Update, VariableOutput}};
use bech32::{ToBase32, Variant};

pub fn blake2b224(data: &[u8]) -> [u8;28] {
    let mut hasher = Blake2bVar::new(28).unwrap();
    hasher.update(data);
    let mut out = [0u8;28];
    hasher.finalize_variable(&mut out).unwrap();
    out
}

pub fn bech32_encode(hrp: &str, data: &[u8]) -> String {
    bech32::encode(hrp, data.to_base32(), Variant::Bech32).expect("bech32 encode")
}