use anyhow::Result;
use ed25519_dalek::{SigningKey, VerifyingKey, Signer};

use tokio::fs;
use crate::address::{AddressBundle, AddressProvider};
use crate::util::bech::{blake2b224, bech32_encode};
use crate::Network;

#[derive(Clone)]
pub struct ShelleyProvider {
    hrp: String,
    network_id: u8,
    keystore_dir: String,
}

impl ShelleyProvider {
    pub async fn new(network: Network, keystore_dir: &str) -> Result<Self> {
        fs::create_dir_all(keystore_dir).await.ok();
        Ok(Self {
            hrp: network.bech32_hrp().to_string(),
            network_id: network.network_id(),
            keystore_dir: keystore_dir.to_string(),
        })
    }
}

impl AddressProvider for ShelleyProvider {
    fn new_address(&self) -> Result<AddressBundle> {
        use rand::rngs::OsRng;
        let mut rng = OsRng;

        let signing = SigningKey::generate(&mut rng);
        let verifying: VerifyingKey = (&signing).into();

        let pk = verifying.to_bytes();
        let sk = signing.to_bytes();

        // Build raw address
        let header: u8 = (0b0110 << 4) | (self.network_id & 0x0f);
        let pkh = blake2b224(&pk);

        let mut raw = Vec::new();
        raw.push(header);
        raw.extend_from_slice(&pkh);

        let address = bech32_encode(&self.hrp, &raw);

        // Persist JSON
        let rec = serde_json::json!({
            "address": address,
            "pubkey_hex": hex::encode(pk),
            "privkey_hex": hex::encode(sk),
        });

        let path = format!("{}/{}.json", self.keystore_dir, hex::encode(pk));
        std::fs::write(path, serde_json::to_vec_pretty(&rec).unwrap())?;

        Ok(AddressBundle {
            address,
            pubkey: pk,
            privkey: sk,
            address_raw: raw,
        })
    }

    fn sign_message_raw(&self, privkey: &[u8; 32], message: &str) -> Result<[u8; 64]> {
        let sk = SigningKey::from_bytes(privkey);
        let sig = sk.sign(message.as_bytes());
        Ok(sig.to_bytes())
    }

    /// Shelley provider does NOT store addresses — only Prefill does.
    fn all_addresses(&self) -> Result<Vec<AddressBundle>> {
        Ok(Vec::new())
    }
}