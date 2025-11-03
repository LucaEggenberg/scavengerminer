use anyhow::Result;
use crate::address::{AddressProvider, AddressBundle};
use std::sync::{Arc, Mutex};
use std::fs;
use std::path::PathBuf;

/// Holds inner provider + sorted (oldest→newest) address queue.
#[derive(Clone)]
pub struct PrefillProvider<P: AddressProvider + Clone> {
    inner: P,
    queue: Arc<Mutex<Vec<AddressBundle>>>,
}

impl<P: AddressProvider + Clone> PrefillProvider<P> {
    pub fn new(inner: P, keystore_dir: &str) -> Result<Self> {
        let mut entries: Vec<(std::time::SystemTime, PathBuf)> = Vec::new();

        // Collect all *.json keystore entries with their timestamps
        for entry in fs::read_dir(keystore_dir)? {
            let e = entry?;
            let path = e.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(meta) = e.metadata() {
                    let t = meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                    entries.push((t, path));
                }
            }
        }

        // Sort by oldest → newest
        entries.sort_by_key(|(t, _)| *t);

        let mut list = Vec::new();

        // Load address bundles in sorted order
        for (_, path) in entries {
            if let Ok(data) = fs::read_to_string(&path) {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&data) {
                    if let (Some(addr), Some(pk), Some(sk)) = (
                        v.get("address").and_then(|x| x.as_str()),
                        v.get("pubkey_hex").and_then(|x| x.as_str()),
                        v.get("privkey_hex").and_then(|x| x.as_str()),
                    ) {
                        if let (Ok(pubkey), Ok(privkey)) = (hex::decode(pk), hex::decode(sk)) {
                            if pubkey.len() == 32 && privkey.len() == 32 {
                                let mut pk32 = [0u8; 32];
                                let mut sk32 = [0u8; 32];
                                pk32.copy_from_slice(&pubkey);
                                sk32.copy_from_slice(&privkey);

                                list.push(AddressBundle {
                                    address: addr.to_string(),
                                    pubkey: pk32,
                                    privkey: sk32,
                                });
                            }
                        }
                    }
                }
            }
        }

        println!("✅ Loaded {} existing addresses (oldest first)", list.len());
        for (i, a) in list.iter().enumerate() {
            println!("  {}. {}", i + 1, a.address);
        }

        Ok(Self {
            inner,
            queue: Arc::new(Mutex::new(list)),
        })
    }
}

impl<P: AddressProvider + Clone> AddressProvider for PrefillProvider<P> {
    fn new_address(&self) -> Result<AddressBundle> {
        let mut q = self.queue.lock().unwrap();

        // Oldest first = pop from front (pop(0))
        if !q.is_empty() {
            let b = q.remove(0);
            println!("🔁 Using existing (oldest) address: {}", b.address);
            return Ok(b);
        }

        // No more existing keys – generate a fresh one
        self.inner.new_address()
    }

    fn sign_cip8_message(
        &self,
        privkey: &[u8; 32],
        pubkey: &[u8; 32],
        message: &str,
    ) -> Result<Vec<u8>> {
        self.inner.sign_cip8_message(privkey, pubkey, message)
    }
}