use anyhow::Result;
use rand::RngCore;
use rand_chacha::ChaCha12Rng;
use rand_chacha::rand_core::SeedableRng;
use tokio::sync::watch;

use blake2::Blake2b512;
use blake2::digest::Digest;

use crate::address::{AddressBundle, AddressProvider};
use crate::api::types::Challenge;

// Mask rule
#[inline]
fn matches_difficulty(hash32: &[u8; 32], diff_hex: &str) -> bool {
    let mask = u32::from_str_radix(diff_hex, 16).unwrap_or(0);
    let h0 = u32::from_be_bytes([hash32[0], hash32[1], hash32[2], hash32[3]]);
    (h0 & mask) == 0
}

#[inline]
fn blake2b512_first32(preimage: &[u8]) -> [u8; 32] {
    let mut h = Blake2b512::new();
    h.update(preimage);
    let out64 = h.finalize();
    let mut out32 = [0u8; 32];
    out32.copy_from_slice(&out64[..32]);
    out32
}

pub async fn mine_one_challenge<P: AddressProvider + Clone>(
    _provider: &P,
    addr: &AddressBundle,
    ch: &Challenge,
    workers: usize,
) -> Result<Option<String>> {

    // cancel when a winner is found
    let (tx, rx) = watch::channel::<Option<[u8; 8]>>(None);

    let deadline = chrono::DateTime::parse_from_rfc3339(&ch.latest_submission)
        .ok()
        .map(|dt| dt.with_timezone(&chrono::Utc));

    // clone challenge fields only once
    let address = addr.address.clone();
    let challenge_id = ch.challenge_id.clone();
    let difficulty = ch.difficulty.clone();
    let no_pre_mine = ch.no_pre_mine.clone();
    let latest_submission = ch.latest_submission.clone();
    let no_pre_mine_hour = ch.no_pre_mine_hour.clone();

    let mut tasks = Vec::with_capacity(workers);

    for w in 0..workers {
        let rx = rx.clone();
        let tx = tx.clone();

        let address = address.clone();
        let challenge_id = challenge_id.clone();
        let difficulty = difficulty.clone();
        let no_pre_mine = no_pre_mine.clone();
        let latest_submission = latest_submission.clone();
        let no_pre_mine_hour = no_pre_mine_hour.clone();
        let deadline = deadline.clone();

        tasks.push(tokio::spawn(async move {
            // Send-safe RNG
            let mut seed = [0u8; 32];
            seed[..8].copy_from_slice(&rand::random::<u64>().to_le_bytes());
            seed[8..16].copy_from_slice(&(w as u64).to_le_bytes());
            let mut rng = ChaCha12Rng::from_seed(seed);

            loop {
                if let Some(nonce) = *rx.borrow() {
                    return Some(nonce);
                }
                if let Some(d) = deadline {
                    if chrono::Utc::now() > d {
                        return None;
                    }
                }

                // 8-byte random nonce
                let mut nonce = [0u8; 8];
                rng.fill_bytes(&mut nonce);
                let nonce_hex = hex::encode(nonce);

                // FINAL preimage format (confirmed correct order):
                // nonce_hex + address + challenge_id + difficulty + no_pre_mine +
                // latest_submission + no_pre_mine_hour
                let mut preimage = String::with_capacity(
                    nonce_hex.len()
                        + address.len()
                        + challenge_id.len()
                        + difficulty.len()
                        + no_pre_mine.len()
                        + latest_submission.len()
                        + no_pre_mine_hour.len(),
                );
                preimage.push_str(&nonce_hex);
                preimage.push_str(&address);
                preimage.push_str(&challenge_id);
                preimage.push_str(&difficulty);
                preimage.push_str(&no_pre_mine);
                preimage.push_str(&latest_submission);
                preimage.push_str(&no_pre_mine_hour);

                let h32 = blake2b512_first32(preimage.as_bytes());
                if matches_difficulty(&h32, &difficulty) {
                    let _ = tx.send(Some(nonce));
                    return Some(nonce);
                }

                tokio::task::yield_now().await;
            }
        }));
    }

    // wait for winner
    for t in tasks {
        if let Ok(Some(nonce)) = t.await {
            return Ok(Some(hex::encode(nonce)));
        }
    }

    // avoid E0597 — copy result before returning
    let maybe = (*rx.borrow()).clone();
    Ok(maybe.map(hex::encode))
}