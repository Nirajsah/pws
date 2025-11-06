#![recursion_limit = "256"]

use crate::{client::Client, wallet::PersistentWallet};
pub mod client;
pub mod resource;
pub mod wallet;
use crate::resource::start_resource_logger;
use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to the wallet directory (must contain wallet.json, keystore.json, and client.db)
    #[arg(long = "with-wallet", value_name = "PATH", global = true)]
    wallet_path: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Metrics,
    /// Deploy an application and run as server
    Deploy {
        /// Path to the project directory containing the contract and service WASM files
        #[arg(long, value_name = "PATH")]
        path: PathBuf,

        /// JSON-encoded initialization arguments for the application
        #[arg(long = "json-argument", value_name = "JSON")]
        json_argument: Option<String>,
    },

    /// Subscribe and watch an existing application
    Watch {
        /// Application ID to subscribe to
        #[arg(long, value_name = "APP_ID")]
        app_id: String,
    },
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
    let client_dir = wallet_path.join("client.db");

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

    if !client_dir.exists() {
        anyhow::bail!(
            "Missing client.db directory in wallet directory: {}",
            wallet_path.display()
        );
    }

    if !client_dir.is_dir() {
        anyhow::bail!("client.db is not a directory: {}", client_dir.display());
    }

    println!(
        "✓ Wallet directory validation successful: {}",
        wallet_path.display()
    );
    println!("  - wallet.json: found");
    println!("  - keystore.json: found");
    println!("  - client.db: found");

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
    let persistent_wallet = PersistentWallet::new().await?;
    let client_context = Client::new(persistent_wallet).await?;

    // Handle commands
    match args.command {
        Commands::Metrics => {
            start_resource_logger();
        }
        Commands::Deploy {
            path,
            json_argument,
        } => {
            println!("🚀 Deploying application...");
            println!("  - Project path: {}", path.display());

            if let Some(ref json_arg) = json_argument {
                println!("  - JSON argument: {}", json_arg);
            }

            client_context
                .publish_and_create(Some(path), json_argument, None, None, None, None)
                .await?;

            println!("✓ Deployment complete");
        }

        Commands::Watch { app_id } => {
            println!(" Watch mode enabled");
            println!(" - Application ID: {}", app_id);

            let app = client_context.frontend().application(&app_id).await?;

            // Subscribe to events from the app chain
            let sub_query = r#"{ "query": "mutation { subscribe }" }"#;
            let result = app.query(sub_query).await?;

            println!("✓ Subscribed successfully: {:?}", result);
            println!(" Watching for events...");
        }
    }

    // Setup notification handler
    client_context.on_notification(|n| {
        println!(
            "notification received in main.rs, now we can fetch: {:?}",
            n
        )
    });

    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(300)).await;
    }
}
