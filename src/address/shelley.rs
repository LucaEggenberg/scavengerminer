use anyhow::Result;
use ed25519_dalek::{SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use tokio::fs;

use crate::address::{AddressBundle, AddressProvider};
use crate::util::bech::{blake2b224, bech32_encode};
use crate::util::cip8::cose_sign1_ed25519;
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
        Ok(Self { hrp: network.bech32_hrp().to_string(), network_id: network.network_id(), keystore_dir: keystore_dir.to_string() })
    }
}

impl ShelleyProvider {
    fn build_enterprise_address(&self, pubkey: &[u8;32]) -> String {
        // enterprise key address header: 0b0110_000n (type=0b0110=6, network id in lower 4 bits)
        let header: u8 = (0b0110 << 4) | (self.network_id & 0x0f);
        let pkh = blake2b224(pubkey);
        let mut addr = Vec::with_capacity(1 + 28);
        addr.push(header);
        addr.extend_from_slice(&pkh);
        bech32_encode(&self.hrp, &addr)
    }
}

impl AddressProvider for ShelleyProvider {
    fn new_address(&self) -> Result<AddressBundle> {
        let mut rng = OsRng;
        let signing = SigningKey::generate(&mut rng);
        let verifying: VerifyingKey = (&signing).into();
        let pk = verifying.to_bytes();
        let sk = signing.to_bytes();
        let address = self.build_enterprise_address(&pk);

        // persist minimal keystore record for later donate_to (optional)
        let rec = serde_json::json!({
            "address": address,
            "pubkey_hex": hex::encode(pk),
            "privkey_hex": hex::encode(sk),
        });
        let path = format!("{}/{}.json", self.keystore_dir, hex::encode(pk));
        let _ = std::fs::write(path, serde_json::to_vec_pretty(&rec).unwrap());

        Ok(AddressBundle { address, pubkey: pk, privkey: sk })
    }

    fn sign_cip8_message(&self, privkey: &[u8;32], _pubkey: &[u8;32], message: &str) -> Result<Vec<u8>> {
        Ok(cose_sign1_ed25519(privkey, message))
    }
}