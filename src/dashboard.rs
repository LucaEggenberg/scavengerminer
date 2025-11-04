use tokio::sync::watch;
use crate::mining::stats::GlobalStats;

/// Very simple terminal dashboard.
/// Prints aggregated stats once per second. No blocking stdout guards across await.
pub async fn launch_dashboard(mut rx: watch::Receiver<GlobalStats>) {
    let mut ticker = tokio::time::interval(std::time::Duration::from_secs(1));

    loop {
        tokio::select! {
            _ = ticker.tick() => {
                let st = rx.borrow().clone();

                // Light clear (optional)
                // print!("\x1B[2J\x1B[H");

                println!("=== Scavenger Miner – Session Stats ===");
                println!("Solutions submitted : {}", st.solutions_submitted);
                println!("Last accepted nonce : {}", st.last_nonce.as_deref().unwrap_or("-"));
                println!("Last receipt        : {}", st.last_receipt.as_deref().unwrap_or("-"));
                println!("Token estimate      : {}",
                    st.token_estimate.map(|v| format!("{:.4}", v)).unwrap_or_else(|| "n/a".into())
                );
                println!("(updates every 1s)\n");
            }

            // React to explicit updates quickly too
            changed = rx.changed() => {
                if changed.is_err() { break; } // sender dropped
            }
        }
    }
}