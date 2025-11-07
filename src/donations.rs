use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

/// Single donation event written to JSONL log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DonationRecord {
    /// Donor (source) address that consolidates its claims away.
    pub source: String,
    /// Destination address that receives the consolidated claims.
    pub target: String,
    /// ISO timestamp when we recorded the donation.
    pub timestamp: String,
}

/// JSONL manager for donations:
///   keystore/00donations.jsonl
pub struct Donations {
    path: PathBuf,
}

impl Donations {
    /// Construct using env var KEYSTORE (defaults to "keystore")
    pub fn new_from_env() -> Result<Self> {
        let root = std::env::var("KEYSTORE").unwrap_or_else(|_| "keystore".to_string());
        Self::new(root)
    }

    /// Construct from explicit keystore directory.
    pub fn new<P: AsRef<Path>>(keystore_dir: P) -> Result<Self> {
        let root = keystore_dir.as_ref();
        fs::create_dir_all(root)?;
        let path = root.join("00donations.jsonl");
        Ok(Self { path })
    }

    /// Append one donation record (as JSON per line).
    pub fn append_donation(&self, rec: &DonationRecord) -> Result<()> {
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        let line = serde_json::to_string(rec)?;
        writeln!(f, "{}", line)?;
        Ok(())
    }

    /// Read all donation records.
    pub fn read_all(&self) -> Result<Vec<DonationRecord>> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }
        let f = OpenOptions::new().read(true).open(&self.path)?;
        let reader = BufReader::new(f);
        let mut out = Vec::new();
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<DonationRecord>(&line) {
                Ok(rec) => out.push(rec),
                Err(e) => {
                    // tolerate a bad line; continue
                    tracing::warn!("Ignoring malformed donation line: {e}");
                }
            }
        }
        Ok(out)
    }

    /// Quick helpers built from the log.
    fn build_sets(&self) -> Result<(HashSet<String>, HashSet<String>)> {
        let mut sources = HashSet::new();
        let mut targets = HashSet::new();
        for r in self.read_all()? {
            sources.insert(r.source);
            targets.insert(r.target);
        }
        Ok((sources, targets))
    }

    /// Check if `source -> target` is allowed.
    ///
    /// Rules we enforce locally (to avoid 403s):
    /// 1) A source can donate only once (no repeats).
    /// 2) No CHAINs: target must not be a source (i.e., must not have donated to anyone).
    /// 3) Target must not already be a target for another donation (keep it simple and flat).
    /// 4) (Optional) If `require_receipt == true`, caller must confirm source has ≥1 receipt.
    ///
    /// Return Ok(()) if allowed; otherwise Err(..) contains the reason.
    pub fn can_donate(
        &self,
        source: &str,
        target: &str,
        require_receipt: bool,
        has_source_receipts: bool,
    ) -> Result<()> {
        // Optional receipt requirement
        if require_receipt && !has_source_receipts {
            bail!("source address has no receipts yet");
        }

        let (sources, targets) = self.build_sets()?;

        // Rule 1: source can donate only once
        if sources.contains(source) {
            bail!("source {} already donated", source);
        }

        // Rule 2: no chains (target must not be a source anywhere)
        if sources.contains(target) {
            bail!("target {} has acted as a source before (donation chain not allowed)", target);
        }

        // Rule 3: target must not already be a target (simple, flat consolidation)
        if targets.contains(target) {
            bail!("target {} already received a donation (single-level only)", target);
        }

        // Also: source must not appear as a target already (if it already received a donation,
        // the server will forbid it donating onwards)
        if targets.contains(source) {
            bail!("source {} already received a donation (cannot donate further)", source);
        }

        Ok(())
    }
}