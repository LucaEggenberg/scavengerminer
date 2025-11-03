use blake2::{Blake2b512, Digest};

/// Context for parity with former design (no state needed for Blake2b)
pub struct BlakeCtx;
impl BlakeCtx {
    pub fn new(_: &str) -> Self { BlakeCtx }
}

/// Blake2b-512 then take the first 32 bytes
pub fn hash_preimage(_ctx: &BlakeCtx, preimage: &[u8]) -> [u8; 32] {
    let mut h = Blake2b512::new();
    h.update(preimage);
    let full = h.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&full[..32]);
    out
}