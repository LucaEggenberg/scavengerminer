mod api;
mod address;
mod mining;
mod util;
pub mod accounting;
pub mod donations;

use clap::{Parser, Subcommand, ValueEnum};
use tracing_subscriber::EnvFilter;
use crate::address::AddressProvider; 

#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum Network { Mainnet, Preprod }
impl Network {
    pub fn bech32_hrp(&self) -> &'static str { match self { Network::Mainnet => "addr", Network::Preprod => "addr_test" } }
    pub fn network_id(&self) -> u8 { match self { Network::Mainnet => 1, Network::Preprod => 0 } }
}

#[derive(Parser, Debug)]
#[command(author, version, about = "Scavenger Miner (Rust) - headless", long_about = None)]
struct Cli {
    /// Scavenger API base URL
    #[arg(long, env = "SCAVENGER_API", default_value = "https://scavenger.prod.gd.midnighttge.io")]
    api: String,

    /// Network (mainnet or preprod)
    #[arg(long, env = "NETWORK", value_enum, default_value_t = Network::Preprod)]
    network: Network,

    /// Number of worker threads per challenge (defaults to all CPU cores)
    #[arg(long, env = "WORKERS")]
    workers: Option<usize>,

    /// Log level (error|warn|info|debug|trace)
    #[arg(long, env = "RUST_LOG", default_value = "info")]
    log: String,

    /// Directory to store generated keys (JSON)
    #[arg(long, env = "KEYSTORE", default_value = "keystore")]
    keystore: String,

    /// Enable donate_to calls after registering address (optional)
    #[arg(long, env = "ENABLE_DONATE", default_value_t = false)]
    enable_donate: bool,

    /// Destination address to consolidate to (required if ENABLE_DONATE=true)
    #[arg(long, env = "DONATE_TO", default_value = "")]
    donate_to: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Run the miner loop (auto-generate & register addresses, mine, submit)
    Mine,
    /// Just fetch the current challenge and print it
    Challenge,
    /// Generate a real Shelley enterprise address and print it
    GenAddr,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let log = cli.log.clone(); // avoid moving cli
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(log))
        .init();

    match cli.command {
        Commands::Mine => cmd_mine(cli).await?,
        Commands::Challenge => cmd_challenge(cli).await?,
        Commands::GenAddr => cmd_gen_addr(cli).await?,
    }

    Ok(())
}

async fn cmd_challenge(cli: Cli) -> anyhow::Result<()> {
    let client = api::ScavengerClient::new(cli.api)?;
    let ch = client.get_challenge().await?;
    println!("{}", serde_json::to_string_pretty(&ch)?);
    Ok(())
}

async fn cmd_gen_addr(cli: Cli) -> anyhow::Result<()> {
    let ap = address::shelley::ShelleyProvider::new(cli.network, &cli.keystore).await?;
    let a = ap.new_address()?;
    println!("address: {}\npubkey_hex: {}", a.address, hex::encode(a.pubkey));
    Ok(())
}

async fn cmd_mine(cli: Cli) -> anyhow::Result<()> {
    use mining::Miner;

    let client = api::ScavengerClient::new(cli.api.clone())?;
    let tandc = client.get_tandc(None).await?;
    tracing::info!(version=?tandc.version, "fetched T&C");

    let shelley = address::shelley::ShelleyProvider::new(cli.network, &cli.keystore).await?;
    let addr_provider = address::prefill::PrefillProvider::new(shelley, &cli.keystore)?;
    let mut miner = Miner::new(
        client, 
        addr_provider, 
        cli.workers, 
        cli.network,
        cli.enable_donate,
        if cli.donate_to.is_empty() { None } else { Some(cli.donate_to) }
    );

    // Run miner with stats
    miner.run_loop(tandc).await
}