use anyhow::Result;

#[derive(Clone)]
pub struct AddressBundle {
    pub address: String,
    pub pubkey: [u8; 32],
    pub privkey: [u8; 32],
    pub address_raw: Vec<u8>,
}

pub trait AddressProvider: Send + Sync {
    /// Generate a new address (baseline capability).
    fn new_address(&self) -> Result<AddressBundle>;

    /// Sign raw message bytes with the 32-byte private key.
    fn sign_message_raw(&self, privkey: &[u8; 32], message: &str) -> Result<[u8; 64]>;

    /// Optional: current rotation index among a prefilled pool (default: 0).
    fn current_index(&self) -> usize {
        0
    }

    /// Optional: total known addresses in rotation (default: 0 if unknown).
    fn total_addresses(&self) -> usize {
        0
    }

    /// Optional: fetch the next address in a rotation.
    /// Default falls back to generating a fresh one.
    fn next_address(&self) -> Result<AddressBundle> {
        self.new_address()
    }

    /// Optional: expose the whole address list for callers that need it.
    /// Default: empty (provider may not have a prefilled store).
    fn all_addresses(&self) -> Result<Vec<AddressBundle>> {
        Ok(Vec::new())
    }
}

// Re-export concrete providers
pub mod shelley;
pub mod prefill;