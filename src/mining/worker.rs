use anyhow::Result;
use rand_chacha::{
    rand_core::{RngCore, SeedableRng},
    ChaCha12Rng,
};
use std::sync::{mpsc, Arc, Mutex};

use ashmaize::{Rom, RomGenerationType, hash};

use crate::address::{AddressBundle, AddressProvider};
use crate::api::types::Challenge;

const LOOPS: u32 = 8;
const INSTR: u32 = 256;

fn build_rom(no_pre_mine_ascii: &str) -> Rom {
    let seed = no_pre_mine_ascii.as_bytes();
    const ROM_SIZE: usize = 1_073_741_824;
    const PRE_SIZE: usize = 16 * 1024 * 1024;
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

fn matches_diff(h: &[u8; 64], diff: &str) -> bool {
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

    let rom = Arc::new(build_rom(&ch.no_pre_mine));

    // FIXED: Receiver wrapped in Arc<Mutex<_>>
    let (tx_found, rx_inner) = mpsc::channel::<[u8; 8]>();
    let rx_found = Arc::new(Mutex::new(rx_inner));

    let deadline = chrono::DateTime::parse_from_rfc3339(&ch.latest_submission)
        .ok()
        .map(|d| d.with_timezone(&chrono::Utc));

    let address = addr.address.clone();
    let challenge_id = ch.challenge_id.clone();
    let difficulty = ch.difficulty.clone();
    let npm = ch.no_pre_mine.clone();
    let npm_h = ch.no_pre_mine_hour.clone();
    let latest = ch.latest_submission.clone();

    let mut threads = Vec::new();

    for worker_id in 0..workers {
        let rom = rom.clone();
        let tx_found = tx_found.clone();
        let rx_found = rx_found.clone(); // <-- now cloneable

        let address = address.clone();
        let challenge_id = challenge_id.clone();
        let difficulty = difficulty.clone();
        let npm = npm.clone();
        let npm_h = npm_h.clone();
        let latest = latest.clone();
        let deadline = deadline.clone();

        threads.push(std::thread::spawn(move || {
            let mut seed = [0u8; 32];
            seed[..8].copy_from_slice(&(worker_id as u64).to_le_bytes());
            let mut rng = ChaCha12Rng::from_seed(seed);

            loop {
                if let Some(dead) = deadline {
                    if chrono::Utc::now() > dead {
                        return None;
                    }
                }

                // FIXED: safe shared receiver check
                if rx_found.lock().unwrap().try_recv().is_ok() {
                    return None;
                }

                let mut nonce = [0u8; 8];
                rng.fill_bytes(&mut nonce);
                let nonce_hex = hex::encode(nonce);

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
                    let _ = tx_found.send(nonce);
                    return Some(nonce);
                }
            }
        }));
    }

    // unchanged
    for t in threads {
        match t.join() {
            Ok(Some(nonce)) => return Ok(Some(hex::encode(nonce))),
            _ => continue,
        }
    }

    Ok(None)
}