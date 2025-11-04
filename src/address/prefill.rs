use anyhow::Result;
use crate::address::{AddressProvider, AddressBundle};
use crate::util::bech::bech32_decode_to_bytes;

use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct PrefillProvider<P: AddressProvider + Clone> {
    inner: P,
    list: Arc<Mutex<Vec<AddressBundle>>>,
    index: Arc<Mutex<usize>>,
}

impl<P: AddressProvider + Clone> PrefillProvider<P> {
    pub fn new(inner: P, keystore_dir: &str) -> Result<Self> {
        let mut entries: Vec<(std::time::SystemTime, PathBuf)> = Vec::new();

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

        entries.sort_by_key(|(t, _)| *t);

        let mut list = Vec::new();

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
                                    address_raw: bech32_decode_to_bytes(addr),
                                });
                            }
                        }
                    }
                }
            }
        }

        println!("Loaded {} existing addresses (oldest first)", list.len());
        for (i, a) in list.iter().enumerate() {
            println!("  {}. {}", i + 1, a.address);
        }

        Ok(Self {
            inner,
            list: Arc::new(Mutex::new(list)),
            index: Arc::new(Mutex::new(0)),
        })
    }
}

impl<P: AddressProvider + Clone> AddressProvider for PrefillProvider<P> {
    fn new_address(&self) -> Result<AddressBundle> {
        // Fall back to delegating to the inner provider
        self.inner.new_address()
    }

    fn sign_message_raw(&self, privkey: &[u8; 32], message: &str) -> Result<[u8; 64]> {
        self.inner.sign_message_raw(privkey, message)
    }

    fn current_index(&self) -> usize {
        *self.index.lock().unwrap()
    }

    fn total_addresses(&self) -> usize {
        self.list.lock().unwrap().len()
    }

    fn next_address(&self) -> Result<AddressBundle> {
        let mut list = self.list.lock().unwrap();

        if list.is_empty() {
            // No prefilled keys — generate a fresh one
            return self.inner.new_address();
        }

        let mut i = self.index.lock().unwrap();
        let idx = *i;
        *i = (*i + 1) % list.len();

        let a = list[idx].clone();
        println!("Using existing address (rr index {}): {}", idx, a.address);
        Ok(a)
    }

    fn all_addresses(&self) -> Result<Vec<AddressBundle>> {
        Ok(self.list.lock().unwrap().clone())
    }
}