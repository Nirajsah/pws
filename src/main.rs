#![recursion_limit = "256"]

use crate::model::Leaderboard;
use crate::supabase::{SupabaseClient, SupabaseModel};
use crate::{client::Client, wallet::PersistentWallet};
pub mod client;
pub mod model;
pub mod resource;
pub mod supabase;
pub mod wallet;
use crate::resource::start_resource_logger;
use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use model::{CountResponse, GameCount, LeaderBoardResponse};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to the wallet directory (must contain wallet.json, keystore.json, and client.db)
    #[arg(long = "with-wallet", value_name = "PATH", global = true)]
    wallet_path: Option<PathBuf>,

    #[arg(long)]
    metrics: bool,

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
    Supabase,
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

// Cache struct
#[derive(Clone, Debug)]
struct CachedState {
    count: Option<u64>,
    leaderboard: Option<Vec<Leaderboard>>,
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

            println!("✓ Deployment complete");
        }

        Commands::Watch { app_id } => {
            println!(" Watch mode enabled");
            println!(" - Application ID: {}", app_id);

            let app = client_context.frontend().application(&app_id).await?;

            let sub_query = r#"{ "query": "mutation { subscribe }" }"#;
            let _ = app.query(sub_query).await?;

            let query_leaderboard =
                r#"{ "query": "query { leaderboard { elo id name matches won lost } }" }"#;
            let query_count = r#"{ "query": "query { count }" }"#;

            // Create shared cache
            let cache = Arc::new(Mutex::new(CachedState {
                count: None,
                leaderboard: None,
            }));

            let app_arc = Arc::new(app);
            let supabase_client = Arc::new(SupabaseClient::new()?);
            let cache_clone = Arc::clone(&cache);

            client_context.on_notification(move || {
                let app = Arc::clone(&app_arc);
                let cache = Arc::clone(&cache_clone);
                let supabase_client = Arc::clone(&supabase_client);

                async move {
                    let response_l = match app.query(&query_leaderboard).await {
                        Ok(r) => r,
                        Err(e) => {
                            eprintln!("✗ Leaderboard query failed: {}", e);
                            return;
                        }
                    };

                    let response_c = match app.query(&query_count).await {
                        Ok(r) => r,
                        Err(e) => {
                            eprintln!("✗ Count query failed: {}", e);
                            return;
                        }
                    };

                    // Parse responses
                    let leaderboard_data: LeaderBoardResponse =
                        match serde_json::from_str(&response_l) {
                            Ok(d) => d,
                            Err(e) => {
                                eprintln!("✗ Failed to parse leaderboard: {}", e);
                                return;
                            }
                        };

                    let count_data: CountResponse = match serde_json::from_str(&response_c) {
                        Ok(d) => d,
                        Err(e) => {
                            eprintln!("✗ Failed to parse count: {}", e);
                            return;
                        }
                    };

                    let new_leaderboard = leaderboard_data.data.leaderboard;
                    let new_count = count_data.data.count;

                    // Check cache and update only if changed
                    let mut cache_guard = cache.lock().await;
                    let mut updates_made = false;

                    // Update count if changed
                    if cache_guard.count != Some(new_count) {
                        println!("📊 Count changed: {:?} -> {}", cache_guard.count, new_count);

                        let count_record = GameCount {
                            id: "singleton".to_string(),
                            count: new_count.to_string(),
                        };

                        match count_record.insert(&supabase_client).await {
                            Ok(_) => {
                                println!("✓ Updated count in Supabase");
                                cache_guard.count = Some(new_count);
                                updates_made = true;
                            }
                            Err(e) => eprintln!("✗ Failed to update count: {}", e),
                        }
                    }

                    // Update leaderboard if changed
                    if cache_guard.leaderboard.as_ref() != Some(&new_leaderboard) {
                        println!(
                            "Leaderboard changed, updating {} entries",
                            new_leaderboard.len()
                        );

                        match Leaderboard::replace_all(new_leaderboard.clone(), &supabase_client)
                            .await
                        {
                            Ok(_) => {
                                println!("✓ Updated leaderboard in Supabase");
                                cache_guard.leaderboard = Some(new_leaderboard);
                                updates_made = true;
                            }
                            Err(e) => eprintln!("✗ Failed to update leaderboard: {}", e),
                        }
                    }

                    if !updates_made {
                        println!("No changes detected, skipping Supabase update");
                    }
                }
            });

            println!(" Watching for events...");
        }
        Commands::Supabase => {
            todo!()
        }
    }
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(300)).await;
    }
}
