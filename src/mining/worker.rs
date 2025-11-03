use super::blakehash::{hash_preimage, BlakeCtx};
use crate::address::{AddressProvider, AddressBundle};
use crate::api::types::Challenge;
use anyhow::Result;
use rand::{RngCore, SeedableRng};
use rand::rngs::StdRng;
use tokio::sync::watch;

fn match_difficulty(hash: &[u8;32], diff_hex: &str) -> bool {
    // Diff is 4-byte hex mask; every zero bit implies zero bit in hash[0..4]
    let mask = u32::from_str_radix(diff_hex, 16).unwrap_or(0);
    let h0 = u32::from_be_bytes([hash[0],hash[1],hash[2],hash[3]]);
    (h0 & mask) == 0
}

pub async fn mine_one_challenge<P: AddressProvider + Clone>(
    _provider: &P,
    addr: &AddressBundle,
    ch: &Challenge,
    workers: usize,
) -> Result<Option<String>> {
    // Initialize AshMaize ROM once per challenge day using no_pre_mine
    let ctx = BlakeCtx::new(&ch.no_pre_mine);

    // Cancellation when a worker finds a nonce
    let (tx, rx) = watch::channel::<Option<[u8;8]>>(None);

    let latest_deadline = chrono::DateTime::parse_from_rfc3339(&ch.latest_submission)
        .ok()
        .map(|dt| dt.with_timezone(&chrono::Utc));

    let mut tasks = Vec::with_capacity(workers);
    for _ in 0..workers {
        let rx = rx.clone();
        let tx = tx.clone();
        let ch = ch.clone();
        let address = addr.address.clone();
        let ctx = BlakeCtx::new(&ch.no_pre_mine);
        tasks.push(tokio::spawn(async move {
            // Use a Send RNG (StdRng) so this future is Send and works with tokio::spawn
            let mut seed = [0u8; 32];
            rand::rngs::OsRng.fill_bytes(&mut seed);
            let mut rng = StdRng::from_seed(seed);
            loop {
                if let Some(nonce) = *rx.borrow() { return Some(nonce); }

                if let Some(deadline) = latest_deadline {
                    if chrono::Utc::now() > deadline { return None; }
                }

                let mut nonce = [0u8;8];
                rng.fill_bytes(&mut nonce);
                let nonce_hex = hex::encode(nonce);

                // Build preimage per docs order
                let preimage = format!(
                    "{}{}{}{}{}{}{}",
                    nonce_hex,
                    address,
                    ch.challenge_id,
                    ch.difficulty,
                    ch.no_pre_mine,
                    ch.latest_submission,
                    ch.no_pre_mine_hour
                );
                let h = hash_preimage(&ctx, preimage.as_bytes());
                if match_difficulty(&h, &ch.difficulty) {
                    let _ = tx.send(Some(nonce));
                    return Some(nonce);
                }

                tokio::task::yield_now().await;
            }
        }));
    }

    for t in tasks {
        if let Ok(Some(nonce)) = t.await { return Ok(Some(hex::encode(nonce))); }
    }

    let last = *rx.borrow(); // copy Option<[u8;8]> out of Ref so it drops before end-of-scope
    Ok(last.map(|n| hex::encode(n)))
}