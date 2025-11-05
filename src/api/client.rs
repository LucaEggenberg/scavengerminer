use super::types::*;
use anyhow::Context;
use reqwest::Url;

#[derive(Clone)]
pub struct ScavengerClient {
    base: Url,
    http: reqwest::Client,
}

impl ScavengerClient {
    pub fn new(base: String) -> anyhow::Result<Self> {
        let base = Url::parse(&base)?;
        let http = reqwest::Client::builder()
            .user_agent("Mozilla/5.0")
            .default_headers({
                let mut h = reqwest::header::HeaderMap::new();
                h.insert(reqwest::header::ACCEPT, "*/*".parse().unwrap());
                h
            })
            .build()?;
        Ok(Self { base, http })
    }

    pub async fn get_tandc(&self, version: Option<&str>) -> anyhow::Result<TandCResponse> {
        let url = match version {
            Some(v) => self.base.join(&format!("/TandC/{v}"))?,
            None => self.base.join("/TandC")?,
        };
        let resp = self.http.get(url).send().await?.error_for_status()?;
        Ok(resp.json().await?)
    }

    pub async fn register(&self, address: &str, signature_hex: &str, pubkey_hex: &str) -> anyhow::Result<RegistrationReceipt> {
        let url = self.base.join(&format!("/register/{}/{}/{}", address, signature_hex, pubkey_hex))?;
        let resp = self.http.post(url).json(&serde_json::json!({})).send().await?;
        let resp = resp.error_for_status().context("register failed")?;
        Ok(resp.json().await?)
    }

    pub async fn get_challenge(&self) -> anyhow::Result<ChallengeEnvelope> {
        let url = self.base.join("/challenge")?;
        let resp = self.http.get(url).send().await?.error_for_status()?;
        Ok(resp.json().await?)
    }

    pub async fn submit_solution(
        &self,
        address: &str,
        challenge_id: &str,
        nonce_hex: &str,
    ) -> anyhow::Result<CryptoReceiptEnvelope> {
        let url = self
            .base
            .join(&format!("/solution/{}/{}/{}", address, challenge_id, nonce_hex))?;
        let resp = self.http.post(url).json(&serde_json::json!({})).send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("submit failed: {} – {}", status, body);
        }
        Ok(resp.json().await?)
    }

    /// Fetch STAR-per-receipt array for each day.
    /// Endpoint: GET /work_to_star_rate
    /// - Example: [10882519, 7692307, 12487254]
    /// - 1 NIGHT = 1_000_000 STAR
    pub async fn get_work_to_star_rate(&self) -> anyhow::Result<Vec<u64>> {
        let url = self.base.join("/work_to_star_rate")?;
        let resp = self.http.get(url).send().await?.error_for_status()?;
        // Server returns a plain JSON array of integers
        let v: Vec<u64> = resp.json().await?;
        Ok(v)
    }

    /// Works 100% with current backend:
    /// Try to submit an intentionally invalid nonce.
    /// If solution already exists → server returns "Solution already exists".
    pub async fn probe_solution(&self, address: &str, challenge_id: &str) -> anyhow::Result<bool> {
        let fake_nonce = "0000000000000000";

        let url = self
            .base
            .join(&format!("/solution/{}/{}/{}", address, challenge_id, fake_nonce))?;

        let resp = self.http.post(url).json(&serde_json::json!({})).send().await?;

        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();

        if status == 400 && body.contains("Solution already exists") {
            return Ok(true);
        }

        if status == 400 {
            return Ok(false); // "invalid nonce" → means not solved yet
        }

        if status.is_success() {
            return Ok(false); // should never happen for fake nonce
        }

        Err(anyhow::anyhow!(
            "unexpected solution probe response {} body={}",
            status,
            body
        ))
    }

    pub async fn donate_to(
        &self,
        dest_addr: &str,
        src_addr: &str,
        sig_hex: &str,
    ) -> anyhow::Result<String> {
        let url = self.base.join(&format!(
            "/donate_to/{}/{}/{}",
            dest_addr, src_addr, sig_hex
        ))?;

        let resp = self.http
            .post(url)
            .json(&serde_json::json!({}))
            .send()
            .await?
            .error_for_status()?;

        Ok(resp.text().await?)
    }
}