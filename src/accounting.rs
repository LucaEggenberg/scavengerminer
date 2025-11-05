use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

/// One JSONL record per accepted submission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptRecord {
    /// ISO string from the API receipt (e.g. "2025-11-04T21:26:06.133Z")
    pub timestamp: String,
    /// Address we submitted with (for accounting / auditing)
    pub address: String,
    /// Challenge id (e.g. "**D06C22")
    pub challenge_id: String,
    /// Day number from the challenge (e.g. 6)
    pub day: u32,
    /// Challenge number within the day (e.g. 22)
    pub challenge_number: u32,
}

/// Holds file locations and utilities for accounting.
/// All files are stored inside the keystore directory:
///   - 00receipts.jsonl: one receipt per line
///   - 00star_rates.json: JSON array of per-day STAR rates (index 0 = day 1)
pub struct Accounting {
    receipts_path: PathBuf,
    star_rates_path: PathBuf,
}

impl Accounting {
    /// Construct using env var KEYSTORE (default "keystore").
    pub fn new_from_env() -> Result<Self> {
        let root = std::env::var("KEYSTORE").unwrap_or_else(|_| "keystore".to_string());
        Self::new(root)
    }

    /// Construct using an explicit keystore directory.
    pub fn new<P: AsRef<Path>>(keystore_dir: P) -> Result<Self> {
        let root = keystore_dir.as_ref();
        fs::create_dir_all(root)?;
        let receipts_path = root.join("00receipts.jsonl");
        let star_rates_path = root.join("00star_rates.json");
        Ok(Self { receipts_path, star_rates_path })
    }

    /// Append one receipt (as a JSON object on its own line).
    pub fn append_receipt(&self, rec: &ReceiptRecord) -> Result<()> {
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.receipts_path)?;
        let line = serde_json::to_string(rec)?;
        writeln!(f, "{}", line)?;
        Ok(())
    }

    /// Read all receipts (fast enough for typical sizes).
    pub fn read_all_receipts(&self) -> Result<Vec<ReceiptRecord>> {
        if !self.receipts_path.exists() {
            return Ok(Vec::new());
        }
        let f = OpenOptions::new().read(true).open(&self.receipts_path)?;
        let reader = BufReader::new(f);
        let mut out = Vec::new();
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() { continue; }
            match serde_json::from_str::<ReceiptRecord>(&line) {
                Ok(rec) => out.push(rec),
                Err(e) => {
                    // keep going if there's a corrupt line
                    tracing::warn!("Ignoring malformed receipt line: {e}");
                }
            }
        }
        Ok(out)
    }

    /// Persist daily STAR rates (index 0 => day 1).
    pub fn write_star_rates(&self, rates: &[u64]) -> Result<()> {
        let tmp = serde_json::to_string_pretty(rates)?;
        fs::write(&self.star_rates_path, tmp)?;
        Ok(())
    }

    /// Load daily STAR rates (index 0 => day 1). Empty if not present.
    pub fn read_star_rates(&self) -> Result<Vec<u64>> {
        if !self.star_rates_path.exists() {
            return Ok(Vec::new());
        }
        let txt = fs::read_to_string(&self.star_rates_path)?;
        let v: Vec<u64> = serde_json::from_str(&txt)?;
        Ok(v)
    }

    /// Compute totals: (solutions_count, total_star, total_night_float)
    pub fn totals(&self) -> Result<(u64, u128, f64)> {
        let receipts = self.read_all_receipts()?;
        let star_rates = self.read_star_rates()?;

        // Group receipts by day: day => count
        let mut by_day: HashMap<u32, u64> = HashMap::new();
        for r in receipts.iter() {
            *by_day.entry(r.day).or_insert(0) += 1;
        }

        // Sum STAR = sum_over_day(count(day) * rate(day))
        let mut total_star: u128 = 0;
        for (day, count) in by_day {
            if day == 0 {
                // If ever seen, treat as 0 rate (docs say day 0/1 behavior special),
                // but it's safer not to assume negative indexing.
                continue;
            }
            let idx = (day - 1) as usize;
            if let Some(rate_star) = star_rates.get(idx) {
                total_star = total_star.saturating_add((*rate_star as u128) * (count as u128));
            }
        }

        let solutions = receipts.len() as u64;
        let total_night = (total_star as f64) / 1_000_000.0;
        Ok((solutions, total_star, total_night))
    }

    /// Convenience log helper.
    pub fn log_totals(&self) {
        match self.totals() {
            Ok((solutions, star, night)) => {
                tracing::info!(
                    "Accounting — solutions total: {} — STAR: {} — NIGHT: {:.6}",
                    solutions, star, night
                );
            }
            Err(e) => tracing::warn!("Accounting totals unavailable: {e}"),
        }
    }
}