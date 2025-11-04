pub mod blakehash;
pub mod worker;

use crate::api::{ScavengerClient, TandCResponse};
use crate::address::{AddressProvider, AddressBundle};
use anyhow::{Context, Result};
use tracing::{info, warn};
use crate::mining::worker::mine_one_challenge;
use crate::Network;

pub struct Miner<P: AddressProvider + Clone> {
    client: ScavengerClient,
    provider: P,
    workers: usize,
    network: Network,
}

impl<P: AddressProvider + Clone> Miner<P> {
    pub fn new(client: ScavengerClient, provider: P, workers: usize, network: Network) -> Self {
        Self { client, provider, workers, network }
    }

    pub async fn run_loop(&self, tandc: TandCResponse) -> Result<()> {
        loop {
            let env = self.client.get_challenge().await?;
            match env.code.as_str() {
                "active" => {
                    let ch = env.challenge.context("missing challenge")?;
                    info!(id=%ch.challenge_id, diff=%ch.difficulty, "mining challenge");

                    // Fresh address per solution attempt
                    let addr = self.provider.new_address()?;
                    self.register_address(&tandc, &addr).await?;

                    // mine
                    let found = mine_one_challenge(&self.provider, &addr, &ch, self.workers).await?;
                    if let Some(nonce_hex) = found {
                        info!(nonce=%nonce_hex, "submitting solution");
                        let resp = self.client.submit_solution(&addr.address, &ch.challenge_id, &nonce_hex).await?;
                        info!(receipt=%resp.crypto_receipt.timestamp, "submitted ok");
                    } else {
                        warn!("no solution found before next challenge or deadline");
                    }
                }
                "before" => {
                    if let Some(starts) = env.starts_at { info!(%starts, "mining not started yet"); }
                    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                }
                "after" => { warn!("mining ended"); break; }
                _ => { warn!(code=%env.code, "unknown code"); tokio::time::sleep(std::time::Duration::from_secs(5)).await; }
            }
        }
        Ok(())
    }

async fn register_address(&self, tandc: &TandCResponse, a: &AddressBundle) -> Result<()> {
        use crate::util::cip8::cose_sign1_ed25519_with_headers;

        // EXACT T&C message (trim trailing newline just like the docs example formatting)
        let payload = tandc.message.trim_end();

        // Build CIP-8/COSE_Sign1:
        // protected: {1:-8} (EdDSA)
        // unprotected: { "address": <raw addr bytes>, "hashed": false }
        // payload:    <payload>
        let cose = cose_sign1_ed25519_with_headers(
            &a.privkey,
            payload,
            &a.address_raw,
            false,
        );

        let sig_hex = hex::encode(cose);
        let pub_hex = hex::encode(a.pubkey);

        tracing::info!("registering address {}", a.address);
        let _receipt = self.client.register(&a.address, &sig_hex, &pub_hex).await?;
        tracing::info!("✅ registration successful");

        // optional donate_to (env: SCAV_ENABLE_DONATE=1, SCAV_DONATE_TO=addr1...)
        let enable = std::env::var("SCAV_ENABLE_DONATE").unwrap_or_default() == "1";
        let dest = std::env::var("SCAV_DONATE_TO").unwrap_or_default();
        if enable && !dest.is_empty() && dest != a.address {
            let msg = format!("Assign accumulated Scavenger rights to: {}", dest);
            // donate_to still expects a raw ed25519 sig over the text we send
            let d_sig = self.provider.sign_message_raw(&a.privkey, &msg)?;
            let d_hex = hex::encode(d_sig);
            match self.client.donate_to(&dest, &a.address, &d_hex).await {
                Ok(v) => tracing::info!("donate_to ok: {}", v),
                Err(e) => tracing::warn!("donate_to failed (ignored): {e:?}"),
            }
        }

        Ok(())
    }
}