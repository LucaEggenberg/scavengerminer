use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TandCResponse {
    pub version: String,
    pub content: String,
    pub message: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ChallengeEnvelope {
    pub code: String,
    #[serde(default)]
    pub challenge: Option<Challenge>,
    #[serde(default)]
    pub mining_period_ends: Option<String>,
    #[serde(default)]
    pub max_day: Option<u32>,
    #[serde(default)]
    pub total_challenges: Option<u32>,
    #[serde(default)]
    pub current_day: Option<u32>,
    #[serde(default)]
    pub next_challenge_starts_at: Option<String>,
    #[serde(default)]
    pub starts_at: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Challenge {
    pub challenge_id: String,
    pub day: u32,
    pub challenge_number: u32,
    pub issued_at: String,
    pub latest_submission: String,
    pub difficulty: String,
    pub no_pre_mine: String,
    pub no_pre_mine_hour: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RegistrationReceipt {
    pub registrationReceipt: RegistrationReceiptInner,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RegistrationReceiptInner {
    pub preimage: String,
    pub signature: String,
    pub timestamp: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CryptoReceiptEnvelope {
    pub crypto_receipt: CryptoReceipt,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CryptoReceipt {
    pub preimage: String,
    pub timestamp: String,
    pub signature: String,
}