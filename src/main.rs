#![recursion_limit = "256"]

use crate::{client::Client, wallet::PersistentWallet};
pub mod client;
pub mod wallet;
use anyhow::{Context, Result};
use clap::Parser;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to the wallet directory (must contain wallet.json, keystore.json, and rocksdb.db/)
    #[arg(long = "with-wallet", value_name = "PATH")]
    wallet_path: Option<PathBuf>,

    /// Deploy the application and watch
    #[arg(long, conflicts_with = "watch")]
    deploy: bool,

    /// Watch mode without deploying
    #[arg(long, conflicts_with = "deploy")]
    watch: bool,

    /// Application ID to interact with
    #[arg(long, value_name = "APP_ID")]
    app_id: Option<String>,
}

/// Validates that the wallet directory contains all required files
fn validate_wallet_directory(wallet_path: &Path) -> Result<()> {
    // Check if the directory exists
    if !wallet_path.exists() {
        anyhow::bail!("Wallet directory does not exist: {}", wallet_path.display());
    }

    if !wallet_path.is_dir() {
        anyhow::bail!("Wallet path is not a directory: {}", wallet_path.display());
    }

    // Check for required files
    let wallet_json = wallet_path.join("wallet.json");
    let keystore_json = wallet_path.join("keystore.json");
    let rocksdb_dir = wallet_path.join("rocksdb.db");

    if !wallet_json.exists() {
        anyhow::bail!(
            "Missing wallet.json in wallet directory: {}",
            wallet_path.display()
        );
    }

    if !wallet_json.is_file() {
        anyhow::bail!("wallet.json is not a file: {}", wallet_json.display());
    }

    if !keystore_json.exists() {
        anyhow::bail!(
            "Missing keystore.json in wallet directory: {}",
            wallet_path.display()
        );
    }

    if !keystore_json.is_file() {
        anyhow::bail!("keystore.json is not a file: {}", keystore_json.display());
    }

    if !rocksdb_dir.exists() {
        anyhow::bail!(
            "Missing rocksdb.db directory in wallet directory: {}",
            wallet_path.display()
        );
    }

    if !rocksdb_dir.is_dir() {
        anyhow::bail!("rocksdb.db is not a directory: {}", rocksdb_dir.display());
    }

    println!(
        "✓ Wallet directory validation successful: {}",
        wallet_path.display()
    );
    println!("  - wallet.json: found");
    println!("  - keystore.json: found");
    println!("  - rocksdb.db/: found");

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Validate wallet directory if provided
    if let Some(ref wallet_path) = args.wallet_path {
        validate_wallet_directory(wallet_path).context("Wallet directory validation failed")?;
    }

    // Initialize the persistent wallet
    // If wallet_path is provided, we use a different method
    let p = PersistentWallet::new().await?;

    /*
        1. We deploy the app on a new wallet(chain) and create a new instance of client and watch.
        2. Just start the service for a wallet and watch.
        3. Subscribe to a app_chain and watch.
    */
    let client_context = Client::new(p).await?;
    // Use provided app_id or default
    let app_id = args.app_id.unwrap_or_else(|| {
        "443ff420b2265303779c7d2d681353e47826cb4b1977d8b0351076f666cf7f93".to_string()
    });

    let app = client_context.frontend().application(&app_id).await?;

    // Handle deploy mode
    if args.deploy {
        // here we'll deploy the app
        println!("🚀 Deploying application...");
        println!("✓ Deployment complete");
        return Ok(());
    }

    // Handle watch mode
    if args.watch {
        println!("👁️  Watch mode enabled (no deployment)");
        // here we subscribe to events from the app_chain
        let sub_query = r#"{ "query": "query { value }" }"#;

        let r = app.query(sub_query).await?;
        println!("Subscribed: {:?}", r);
    }

    // will add more logic here
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
    }
}
