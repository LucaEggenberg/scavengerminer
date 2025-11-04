pub mod worker;

use crate::api::{ScavengerClient, TandCResponse};
use crate::address::{AddressBundle, AddressProvider};
use crate::Network;

use anyhow::{Context, Result};
use tracing::{info, warn, debug};

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

pub struct Miner<P: AddressProvider + Clone> {
    client: ScavengerClient,
    provider: P,
    workers: usize,
    _network: Network,

    // per-challenge state
    current_challenge_id: Arc<std::sync::Mutex<String>>,
    current_solutions: Arc<AtomicUsize>,

    // process-wide total
    global_solutions: Arc<AtomicUsize>,
}

impl<P: AddressProvider + Clone> Miner<P> {
    pub fn new(
        client: ScavengerClient,
        provider: P,
        workers: Option<usize>,
        network: Network,
    ) -> Self {
        let workers = workers.unwrap_or_else(|| {
            std::thread::available_parallelism()
                .map(|x| x.get())
                .unwrap_or(1)
        });

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

    /// Count existing submissions for this challenge by scanning all known addresses.
    async fn count_existing_solutions(&self, ch_id: &str) -> usize {
        let mut total = 0;

        match self.provider.all_addresses() {
            Ok(addrs) => {
                for a in addrs {
                    match self.client.probe_solution(&a.address, ch_id).await {
                        Ok(true) => {
                            debug!("probe: {} already has a solution for {}", a.address, ch_id);
                            total += 1;
                        }
                        Ok(false) => {
                            debug!("probe: {} has no solution for {}", a.address, ch_id);
                        }
                        Err(e) => {
                            warn!("probe failed for {}: {}", a.address, e);
                        }
                    }
                }
            }
            Err(e) => {
                warn!("all_addresses() failed: {}", e);
            }
        }

        total
    }

    /// Get an address that has not yet submitted for this challenge; register it before returning.
    async fn get_fresh_address(
        &self,
        challenge_id: &str,
        tandc: &TandCResponse,
    ) -> Result<AddressBundle> {
        loop {
            let a = self.provider.new_address()?;

            match self.client.probe_solution(&a.address, challenge_id).await {
                Ok(true) => {
                    info!("Skipping address {} (already submitted for {})", a.address, challenge_id);
                    continue; // try next address
                }
                Ok(false) => {
                    self.register_address(tandc, &a).await?;
                    return Ok(a);
                }
                Err(e) => {
                    warn!("probe failed for {}: {} — retrying next address", a.address, e);
                    continue;
                }
            }
        }
    }

    pub async fn run_loop(&self, tandc: TandCResponse) -> Result<()> {
        loop {
            let env = self.client.get_challenge().await?;

            match env.code.as_str() {
                "active" => {
                    let ch = env.challenge.context("missing challenge")?;
                    let ch_id = ch.challenge_id.clone();

                    // If we switched to a new challenge id, reset per-challenge counter.
                    {
                        let mut cid = self.current_challenge_id.lock().unwrap();
                        if *cid != ch_id {
                            *cid = ch_id.clone();
                            self.current_solutions.store(0, Ordering::Relaxed);
                        }
                    }

                    // Count already-submitted solutions (server-visible), then add what we’ve found
                    // during this process for the same challenge.
                    let existing = self.count_existing_solutions(&ch_id).await;
                    let per_ch_total = existing + self.current_solutions.load(Ordering::Relaxed);

                    info!(
                        "Challenge {} / {} — solutions: {} — total: {}",
                        ch.challenge_number,
                        ch.day,
                        per_ch_total,
                        self.global_solutions.load(Ordering::Relaxed)
                    );

                    // Pick an address that has not yet submitted for this challenge (probe+skip),
                    // then register it prior to mining.
                    let addr = self.get_fresh_address(&ch_id, &tandc).await?;

                    // Mine
                    let found = worker::mine_one_challenge(
                        &self.provider,
                        &addr,
                        &ch,
                        self.workers,
                    )
                    .await?;

                    if let Some(nonce_hex) = found {
                        // Update counters *before* submit log line
                        let now_ch = self.current_solutions.fetch_add(1, Ordering::Relaxed) + 1;
                        let total_proc = self.global_solutions.fetch_add(1, Ordering::Relaxed) + 1;

                        info!(
                            "Challenge {} — solutions: {} — total: {} — nonce={}",
                            ch_id,
                            existing + now_ch, // existing (server) + this-process count so far
                            total_proc,
                            nonce_hex
                        );

                        let resp = self
                            .client
                            .submit_solution(&addr.address, &ch_id, &nonce_hex)
                            .await?;

                        info!("Submitted OK (receipt {})", resp.crypto_receipt.timestamp);
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

    async fn register_address(
        &self,
        tandc: &TandCResponse,
        a: &AddressBundle,
    ) -> Result<()> {
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

        info!("Registering address {}", a.address);
        self.client.register(&a.address, &sig_hex, &pub_hex).await?;
        info!("Registration OK");

        Ok(())
    }
}