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
        // CIP-8/30 COSE_Sign1 signature over the exact T&C message string
        let cose = self.provider.sign_cip8_message(&a.privkey, &a.pubkey, &tandc.message)?;
        let sig_hex = hex::encode(cose);
        let pub_hex = hex::encode(a.pubkey);
        let _receipt = self.client.register(&a.address, &sig_hex, &pub_hex).await?;

        // optional donate_to (toggle + destination)
        let enable = std::env::var("SCAV_ENABLE_DONATE").unwrap_or_default() == "1";
        let dest = std::env::var("SCAV_DONATE_TO").unwrap_or_default();

        if enable && !dest.is_empty() && dest != a.address {
            // spec: sign the text message exactly: "Assign accumulated Scavenger rights to: <dest>"
            let msg = format!("Assign accumulated Scavenger rights to: {}", dest);
            let sig = self.provider.sign_cip8_message(&a.privkey, &a.pubkey, &msg)?;
            let sig_hex = hex::encode(sig);
            match self.client.donate_to(&dest, &a.address, &sig_hex).await {
                Ok(v) => tracing::info!("donate_to ok: {}", v),
                Err(e) => tracing::warn!("donate_to failed (ignored): {e:?}"),
            }
        }

        Ok(())
    }
}