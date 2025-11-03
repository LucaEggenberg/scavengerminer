pub mod shelley;
pub mod prefill; 

use anyhow::Result;

#[derive(Debug, Clone)]
pub struct AddressBundle {
    pub address: String,   // bech32 addr/addr_test
    pub pubkey: [u8; 32],  // raw ed25519 public key
    pub privkey: [u8; 32], // raw ed25519 secret scalar
}

pub trait AddressProvider: Send + Sync + 'static {
    fn new_address(&self) -> Result<AddressBundle>;
    /// Sign exact UTF-8 message as required by /TandC using CIP-8/COSE_Sign1 Ed25519
    fn sign_cip8_message(&self, privkey: &[u8;32], pubkey: &[u8;32], message: &str) -> Result<Vec<u8>>;
}