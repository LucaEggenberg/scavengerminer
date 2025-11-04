pub mod blakehash;
pub mod worker;
pub mod stats;

use crate::api::{ScavengerClient, TandCResponse};
use crate::address::{AddressProvider, AddressBundle};
use anyhow::{Context, Result};
use tracing::{info, warn};

use crate::mining::worker::mine_one_challenge;
use crate::mining::stats::GlobalStats;
use crate::Network;
use tokio::sync::watch;

pub struct Miner<P: AddressProvider + Clone> {
    client: ScavengerClient,
    provider: P,
    workers: usize,
    network: Network,
    stats: Option<watch::Sender<GlobalStats>>, // single aggregated stats channel
}

impl<P: AddressProvider + Clone> Miner<P> {
    pub fn new(client: ScavengerClient, provider: P, workers: Option<usize>, network: Network) -> Self {
        let workers = workers.unwrap_or_else(|| {
            std::thread::available_parallelism().map(|x| x.get()).unwrap_or(1)
        });

        Self {
            client,
            provider,
            workers,
            network,
            stats: None,
        }
    }

    /// Attach the single aggregated stats broadcaster
    pub fn with_stats(mut self, tx_stats: watch::Sender<GlobalStats>) -> Self {
        self.stats = Some(tx_stats);
        self
    }

    pub fn worker_count(&self) -> usize {
        self.workers
    }

    pub async fn run_loop(&self, tandc: TandCResponse) -> Result<()> {
        // session-local counters we publish to dashboard
        let mut session_stats = GlobalStats::new();

        loop {
            let env = self.client.get_challenge().await?;
            match env.code.as_str() {
                "active" => {
                    let ch = env.challenge.context("missing challenge")?;
                    info!(id=%ch.challenge_id, diff=%ch.difficulty, "mining challenge");

                    // Fresh address per attempt
                    let addr = self.provider.new_address()?;
                    self.register_address(&tandc, &addr).await?;

                    // Mine
                    let found = mine_one_challenge(
                        &self.provider,
                        &addr,
                        &ch,
                        self.workers,
                    ).await?;

                    if let Some(nonce_hex) = found {
                        info!(nonce=%nonce_hex, "submitting solution");
                        let resp = self.client
                            .submit_solution(&addr.address, &ch.challenge_id, &nonce_hex)
                            .await?;

                        // Update session stats on success
                        session_stats.solutions_submitted += 1;
                        session_stats.last_nonce = Some(nonce_hex);
                        session_stats.last_receipt = Some(resp.crypto_receipt.timestamp.clone());

                        // If you later add an API to fetch token estimates, set token_estimate here:
                        // session_stats.token_estimate = Some(fetched_value);

                        if let Some(tx) = &self.stats {
                            let _ = tx.send(session_stats.clone());
                        }

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

        let payload = tandc.message.trim_end();

        let cose = cose_sign1_ed25519_with_headers(
            &a.privkey,
            payload,
            &a.address_raw,
            false,
        );

        let sig_hex = hex::encode(cose);
        let pub_hex = hex::encode(a.pubkey);

        tracing::info!("registering address {}", a.address);
        self.client.register(&a.address, &sig_hex, &pub_hex).await?;
        tracing::info!("✅ registration successful");

        Ok(())
    }
}