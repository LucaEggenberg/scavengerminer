use anyhow::Result;
use rand_chacha::{
    rand_core::{RngCore, SeedableRng},
    ChaCha12Rng,
};
use std::sync::{mpsc, Arc};
use std::sync::atomic::{AtomicBool, Ordering};

use ashmaize::{Rom, RomGenerationType, hash};

use crate::address::{AddressBundle, AddressProvider};
use crate::api::types::Challenge;

const LOOPS: u32 = 8;
const INSTR: u32 = 256;

fn build_rom(no_pre_mine_ascii: &str) -> Rom {
    let seed = no_pre_mine_ascii.as_bytes();
    const ROM_SIZE: usize = 1_073_741_824; // 1 GiB
    const PRE_SIZE: usize = 16 * 1024 * 1024; // 16 MiB
    const MIXING: usize = 4;

    Rom::new(
        &seed,
        RomGenerationType::TwoStep {
            pre_size: PRE_SIZE,
            mixing_numbers: MIXING,
        },
        ROM_SIZE,
    )
}

#[inline]
fn ash_hash(preimage: &[u8], rom: &Rom) -> [u8; 64] {
    hash(preimage, rom, LOOPS, INSTR)
}

#[inline]
fn matches_diff(h: &[u8; 64], diff: &str) -> bool {
    // Compare first 4 bytes (big-endian semantics) with the mask rule `(h | d) == d`
    let h_hex = hex::encode(&h[0..4]);
    let h_val = u32::from_str_radix(&h_hex, 16).unwrap();
    let d_val = u32::from_str_radix(&diff[0..8], 16).unwrap();
    (h_val | d_val) == d_val
}

fn build_preimage(
    nonce_hex: &str,
    address: &str,
    challenge_id: &str,
    difficulty: &str,
    no_pre_mine: &str,
    latest_submission: &str,
    no_pre_mine_hour: &str,
) -> String {
    let mut s = String::with_capacity(
        nonce_hex.len()
            + address.len()
            + challenge_id.len()
            + difficulty.len()
            + no_pre_mine.len()
            + latest_submission.len()
            + no_pre_mine_hour.len(),
    );
    s.push_str(nonce_hex);
    s.push_str(address);
    s.push_str(challenge_id);
    s.push_str(difficulty);
    s.push_str(no_pre_mine);
    s.push_str(latest_submission);
    s.push_str(no_pre_mine_hour);
    s
}

pub async fn mine_one_challenge<P: AddressProvider + Clone>(
    _provider: &P,
    addr: &AddressBundle,
    ch: &Challenge,
    workers: usize,
) -> Result<Option<String>> {
    // Build ROM once and share
    let rom = Arc::new(build_rom(&ch.no_pre_mine));

    // Signal to stop all workers as soon as one finds a solution
    let found_flag = Arc::new(AtomicBool::new(false));

    // Channel to get the winning nonce
    let (tx_winner, rx_winner) = mpsc::channel::<[u8; 8]>();

    // Parse deadline if present
    let deadline = chrono::DateTime::parse_from_rfc3339(&ch.latest_submission)
        .ok()
        .map(|d| d.with_timezone(&chrono::Utc));

    let address = addr.address.clone();
    let challenge_id = ch.challenge_id.clone();
    let difficulty = ch.difficulty.clone();
    let npm = ch.no_pre_mine.clone();
    let npm_h = ch.no_pre_mine_hour.clone();
    let latest = ch.latest_submission.clone();

    let mut threads = Vec::with_capacity(workers);

    for worker_id in 0..workers {
        let rom = rom.clone();
        let found_flag = found_flag.clone();
        let tx_winner = tx_winner.clone();

        let address = address.clone();
        let challenge_id = challenge_id.clone();
        let difficulty = difficulty.clone();
        let npm = npm.clone();
        let npm_h = npm_h.clone();
        let latest = latest.clone();
        let deadline = deadline.clone();

        threads.push(std::thread::spawn(move || {
            // Per-thread deterministic RNG seed
            let mut seed = [0u8; 32];
            seed[..8].copy_from_slice(&(worker_id as u64).to_le_bytes());
            let mut rng = ChaCha12Rng::from_seed(seed);

            // Tight compute loop; check stop/deadline periodically
            // Optionally batch a few iterations between checks for throughput
            const BATCH: usize = 256;
            loop {
                if found_flag.load(Ordering::Relaxed) {
                    return None;
                }
                if let Some(dead) = deadline {
                    if chrono::Utc::now() > dead {
                        return None;
                    }
                }

                for _ in 0..BATCH {
                    // Make a 64-bit nonce
                    let mut nonce = [0u8; 8];
                    rng.fill_bytes(&mut nonce);
                    let nonce_hex = hex::encode(nonce);

                    // Canonical preimage
                    let preimage = build_preimage(
                        &nonce_hex,
                        &address,
                        &challenge_id,
                        &difficulty,
                        &npm,
                        &latest,
                        &npm_h,
                    );

                    let digest = ash_hash(preimage.as_bytes(), &rom);

                    if matches_diff(&digest, &difficulty) {
                        // Announce and stop others
                        found_flag.store(true, Ordering::Relaxed);
                        let _ = tx_winner.send(nonce);
                        return Some(nonce);
                    }
                }
            }
        }));
    }

    // Wait for winner or deadline timeout
    let maybe_nonce = if let Some(dead) = deadline {
        let now = chrono::Utc::now();
        if dead > now {
            let dur = (dead - now).to_std().unwrap_or_default();
            rx_winner.recv_timeout(dur).ok()
        } else {
            None
        }
    } else {
        rx_winner.recv().ok()
    };

    // Ensure all threads exit
    for t in threads {
        let _ = t.join();
    }

    Ok(maybe_nonce.map(hex::encode))
}