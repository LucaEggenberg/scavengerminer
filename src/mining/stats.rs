use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GlobalStats {
    pub solutions_submitted: u64,
    pub last_nonce: Option<String>,
    pub last_receipt: Option<String>,

    /// Optional: estimated tokens (e.g., NIGHT). Leave None if not available.
    pub token_estimate: Option<f64>,
}

impl GlobalStats {
    pub fn new() -> Self {
        Self {
            solutions_submitted: 0,
            last_nonce: None,
            last_receipt: None,
            token_estimate: None,
        }
    }
}