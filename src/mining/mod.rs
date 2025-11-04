pub mod worker;

use crate::api::{ScavengerClient, TandCResponse};
use crate::address::{AddressBundle, AddressProvider};
use crate::Network;

use anyhow::{Context, Result};
use tracing::{info, warn};

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

pub struct Miner<P: AddressProvider + Clone> {
    client: ScavengerClient,
    provider: P,
    workers: usize,
    _network: Network,

    current_challenge_id: Arc<std::sync::Mutex<String>>,
    current_solutions: Arc<AtomicUsize>,
    global_solutions: Arc<AtomicUsize>,
}

impl<P: AddressProvider + Clone> Miner<P> {
    pub fn new(
        client: ScavengerClient,
        provider: P,
        workers: Option<usize>,
        network: Network,
    ) -> Self {
        let workers = workers
            .unwrap_or_else(|| std::thread::available_parallelism().map(|x| x.get()).unwrap_or(1));

        Self {
            client,
            provider,
            workers,
            _network: network,

            current_challenge_id: Arc::new(std::sync::Mutex::new(String::new())),
            current_solutions: Arc::new(AtomicUsize::new(0)),
            global_solutions: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn worker_count(&self) -> usize {
        self.workers
    }

    pub async fn run_loop(&self, tandc: TandCResponse) -> Result<()> {
        loop {
            let env = self.client.get_challenge().await?;

            match env.code.as_str() {
                "active" => {
                    let ch = env.challenge.context("missing challenge")?;
                    let ch_id = ch.challenge_id.clone();

                    // Detect challenge change → reset per-challenge counter.
                    {
                        let mut cid = self.current_challenge_id.lock().unwrap();
                        if *cid != ch_id {
                            *cid = ch_id.clone();
                            self.current_solutions.store(0, Ordering::Relaxed);
                        }
                    }

                    // Show where we are in the prefilled rotation.
                    let total = self.provider.total_addresses();
                    let idx_before = self.provider.current_index();
                    info!("Challenge {} — address index {}/{}", ch.challenge_number, idx_before, total.max(1));

                    let addr: AddressBundle = {
                        let total = self.provider.total_addresses().max(1);
                        let mut picked: Option<AddressBundle> = None;

                        // Try all existing addresses first
                        for _ in 0..total {
                            let a = self.provider.next_address()?;
                            match self.client.probe_solution(&a.address, &ch_id).await {
                                Ok(true) => {
                                    info!("Skipping address {} (already used for {})", a.address, ch_id);
                                    continue;
                                }
                                Ok(false) => {
                                    picked = Some(a);
                                    break;
                                }
                                Err(e) => {
                                    warn!("Probe failed for {}: {}", a.address, e);
                                    continue;
                                }
                            }
                        }

                        // If none of the prefilled addresses were usable → generate NEW addresses until success.
                        match picked {
                            Some(a) => a,
                            None => loop {
                                warn!("All existing addresses used for {} — generating new address", ch_id);

                                let a = self.provider.new_address()?;

                                match self.client.probe_solution(&a.address, &ch_id).await {
                                    Ok(true) => {
                                        warn!("impossible, fresh address can't have a solution :/ {}", a.address);
                                        continue;
                                    }
                                    Ok(false) => break a,
                                    Err(e) => {
                                        warn!("Probe failed for new address {}: {}", a.address, e);
                                        continue;
                                    }
                                }
                            },
                        }
                    };

                    // Register only after we know we’ll actually mine with it.
                    info!("Registering address {}", addr.address);
                    self.register_address(&tandc, &addr).await?;
                    info!("Registration OK");

                    // Mine with this address.
                    let found = worker::mine_one_challenge(
                        &self.provider,
                        &addr,
                        &ch,
                        self.workers,
                    )
                    .await?;

                    if let Some(nonce_hex) = found {
                        // Update counters first so the next log line reflects new totals.
                        self.current_solutions.fetch_add(1, Ordering::Relaxed);
                        self.global_solutions.fetch_add(1, Ordering::Relaxed);

                        let per_ch = self.current_solutions.load(Ordering::Relaxed);
                        let total_all = self.global_solutions.load(Ordering::Relaxed);

                        info!(
                            "Challenge {} — index {}/{} — nonce={}",
                            ch.challenge_number,
                            // show the index we *just moved to* in rotation for reference
                            self.provider.current_index().saturating_sub(1),
                            total.max(1),
                            nonce_hex
                        );

                        // Submit.
                        let resp = self
                            .client
                            .submit_solution(&addr.address, &ch_id, &nonce_hex)
                            .await?;

                        info!(
                            "Submitted OK (receipt {}) — solutions this challenge: {} — total: {}",
                            resp.crypto_receipt.timestamp,
                            per_ch,
                            total_all
                        );
                    } else {
                        warn!("No solution found before next round / deadline");
                    }
                }

                "before" => {
                    if let Some(starts) = env.starts_at {
                        info!("Mining not started yet — begins at {}", starts);
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                }

                "after" => {
                    warn!("Mining finished");
                    break;
                }

                other => {
                    warn!("Unknown challenge state {}", other);
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                }
            }
        }

        Ok(())
    }

    async fn register_address(&self, tandc: &TandCResponse, a: &AddressBundle) -> Result<()> {
        use crate::util::cip8::cose_sign1_ed25519_with_headers;

        let payload = tandc.message.trim_end();

        let cose = cose_sign1_ed25519_with_headers(&a.privkey, payload, &a.address_raw, false);

        let sig_hex = hex::encode(cose);
        let pub_hex = hex::encode(a.pubkey);

        self.client.register(&a.address, &sig_hex, &pub_hex).await?;
        Ok(())
    }
}