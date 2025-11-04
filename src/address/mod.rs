pub mod shelley;
pub mod prefill;

use anyhow::Result;

#[derive(Debug, Clone)]
pub struct AddressBundle {
    pub address: String,
    pub pubkey: [u8; 32],
    pub privkey: [u8; 32],
    pub address_raw: Vec<u8>,
}

pub trait AddressProvider: Send + Sync + 'static {
    fn new_address(&self) -> Result<AddressBundle>;
    fn sign_message_raw(&self, privkey: &[u8; 32], message: &str) -> Result<[u8; 64]>;
}