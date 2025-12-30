#![recursion_limit = "256"]
#![allow(dead_code)]

use crate::supabase::{SupabaseClient, SupabaseModel};
use crate::{client::Client, wallet::PersistentWallet};
pub mod chain;
pub mod client;
pub mod client_manager;
pub mod models;
pub mod resource;
pub mod storage;
pub mod supabase;
pub mod wallet;
use crate::resource::start_resource_logger;
use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use client_manager::ChainClientManager;
use models::tournament::{
    participants_query, ParticipantResponse, Tournament, TournamentParticipant, TournamentResponse,
    QUERY_TOURNAMENTS,
};
use models::{
    CountResponse, GameCount, LeaderBoardResponse, Leaderboard, MatchHistory, MatchHistoryDB,
    MatchHistoryResponse,
};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to the wallet directory (must contain wallet.json, keystore.json, and client.db)
    #[arg(long = "with-wallet", value_name = "PATH", global = true)]
    wallet_path: Option<PathBuf>,

    #[arg(long = "with-keystore", value_name = "PATH", global = true)]
    keystore_path: Option<PathBuf>,

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
    /// Subscribe and watch an existing application
    ChainService {
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

// Cache struct
#[derive(Clone, Debug)]
struct CachedState {
    count: Option<u64>,
    leaderboard: Option<Vec<Leaderboard>>,
    matches: Option<MatchHistory>,
    tournaments: HashMap<String, Tournament>,
    participants: HashMap<String, HashMap<String, TournamentParticipant>>,
}

fn init_logging() {
    tracing_subscriber::Registry::default()
        .with(fmt::layer().with_target(true).without_time()) // show targets, optional timestamps
        .with(EnvFilter::from_default_env()) // reads RUST_LOG
        .init();
}

const SUB_QUERY: &str = r#"{ "query": "mutation { subscribe }" }"#;

#[tokio::main]
async fn main() -> Result<()> {
    init_logging();

    let args = Args::parse();

    // Validate wallet directory if provided
    if let Some(ref wallet_path) = args.wallet_path {
        validate_wallet_directory(wallet_path).context("Wallet directory validation failed")?;
    }

    // Initialize the persistent wallet
    let persistent_wallet = PersistentWallet::new(args.keystore_path).await?;
    let client_context = Client::new(&persistent_wallet, None).await?;

    let chain = client_context.chain(None).await?;

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

            let app = chain.application(&app_id).await?;

            app.query(SUB_QUERY).await?;

            // Create shared cache
            let cache = Arc::new(Mutex::new(CachedState {
                count: None,
                leaderboard: None,
                matches: None,
                tournaments: HashMap::new(),
                participants: HashMap::new(),
            }));

            let app_arc = Arc::new(app);
            let supabase_client = Arc::new(SupabaseClient::new()?);
            let cache_clone = Arc::clone(&cache);

            chain.on_notification(move || {
                let app = Arc::clone(&app_arc);
                let cache = Arc::clone(&cache_clone);
                let supabase_client = Arc::clone(&supabase_client);

                async move {
                    let response_t = match app.query(QUERY_TOURNAMENTS).await {
                        Ok(r) => r,
                        Err(e) => {
                            eprintln!("✗ Leaderboard query failed: {}", e);
                            return;
                        }
                    };

                    let tournaments_resp: TournamentResponse =
                        match serde_json::from_str(&response_t) {
                            Ok(d) => d,
                            Err(e) => {
                                eprintln!("✗ Failed to parse tournaments: {:?}", e);
                                return;
                            }
                        };
                    println!("tournament: {:?}", tournaments_resp);

                    let mut cache_guard = cache.lock().await;

                    for tournament in tournaments_resp.data.all_tournaments {
                        // Check if tournament changed
                        let should_update = match cache_guard.tournaments.get(&tournament.tournament_id) {
                            Some(cached_t) => cached_t != &tournament,
                            None => true,
                        };

                        if should_update {
                             println!("Tournament {} changed or new, updating Supabase...", tournament.tournament_id);
                             // Use insert which maps to upsert for TournamentDB to avoid full delete/insert cycle
                             match tournament.for_db().insert(&supabase_client).await {
                                Ok(_) => {
                                    println!("✓ Updated tournament {} in Supabase", tournament.tournament_name);
                                    cache_guard.tournaments.insert(tournament.tournament_id.clone(), tournament.clone());
                                }
                                Err(e) => eprintln!("✗ Failed to update tournament: {}", e),
                            }
                        }

                        let query = participants_query(&tournament.tournament_id);
                        let response_p = match app.query(&query).await {
                            Ok(r) => r,
                            Err(e) => {
                                eprintln!("✗ Participants query failed: {}", e);
                                return;
                            }
                        };

                        let participants_resp: ParticipantResponse =
                            match serde_json::from_str(&response_p) {
                                Ok(d) => d,
                                Err(e) => {
                                    eprintln!("✗ Failed to parse participants: {}", e);
                                    return;
                                }
                            };

                        let current_participants_map: HashMap<String, TournamentParticipant> = participants_resp
                            .data
                            .participants
                            .into_iter()
                            .map(|p| (p.id.clone(), p))
                            .collect();

                        let tournament_participants_cache = cache_guard.participants.entry(tournament.tournament_id.clone()).or_default();

                        for (p_id, participant) in &current_participants_map {
                            let p_should_update = match tournament_participants_cache.get(p_id) {
                                Some(cached_p) => cached_p != participant,
                                None => true,
                            };

                            if p_should_update {
                                println!("Participant {} changed or new, updating Supabase...", p_id);
                                match participant
                                    .for_db(tournament.tournament_id.clone())
                                    .insert(&supabase_client)
                                    .await
                                {
                                    Ok(_) => {
                                        println!("✓ Updated participant {} in Supabase", p_id);
                                        // Update the specific participant in the cache
                                        tournament_participants_cache.insert(p_id.clone(), participant.clone());
                                    }
                                    Err(e) => eprintln!("✗ Failed to update participant: {}", e),
                                }
                            }
                        }
                    }
                    // Leaderboard
                    let query_leaderboard = r#"{ "query": "query { leaderboard { elo id name matches won lost } }" }"#;
                    let response_l = match app.query(query_leaderboard).await {
                         Ok(r) => r,
                         Err(e) => {
                             eprintln!("✗ Leaderboard query failed: {}", e);
                             return;
                         }
                    };

                    let query_count = r#"{ "query": "query { count }" }"#;
                    let response_c = match app.query(query_count).await {
                        Ok(r) => r,
                        Err(e) => {
                            eprintln!("✗ Count query failed: {}", e);
                            return;
                        }
                    };

                    let query_matches = r#"{ "query": "query { matchHistoryLast { you { id name } opponent { id name } blobHash } }" }"#;
                    let response_m = match app.query(query_matches).await {
                        Ok(r) => r,
                        Err(e) => {
                            eprintln!("✗ Matches query failed: {}", e);
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

                    let matches_data: Option<MatchHistoryResponse> =
                        match serde_json::from_str(&response_m) {
                            Ok(d) => Some(d),
                            Err(e) => {
                                eprintln!("✗ Failed to parse match history: {}", e);
                                None
                            }
                        };

                    let new_leaderboard = leaderboard_data.data.leaderboard;
                    let new_count = count_data.data.count;

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
                            }
                            Err(e) => eprintln!("✗ Failed to update leaderboard: {}", e),
                        }
                    }

                    if let Some(match_history) = matches_data {
                         if let Some(new_match) = match_history.data.match_history_last {
                            // Update Match history if changed
                            if cache_guard.matches.as_ref() != Some(&new_match) {
                                println!("Last match update: {:?}", new_match);

                                match MatchHistoryDB::insert(&new_match.for_db(), &supabase_client)
                                    .await
                                {
                                    Ok(_) => {
                                        println!("✓ Updated matches list in Supabase");
                                        cache_guard.matches = Some(new_match);
                                    }
                                    Err(e) => eprintln!("✗ Failed to update matches list: {}", e),
                                }
                            }
                        }
                    }
                }
            });

            println!(" Watching for events...");
        }
        Commands::ChainService { app_id } => {
            let app = chain.application(&app_id.clone()).await?;

            app.query(SUB_QUERY).await?;
            let app_arc = Arc::new(app);

            let client_manager = ChainClientManager::default();
            let (tx, mut rx) = tokio::sync::mpsc::channel(16);

            chain.on_notification(move || {
                let chains = r#"{ "query": "query { tournamentChains }" }"#;
                let app = Arc::clone(&app_arc);
                let tx = tx.clone();

                async move {
                    let chain_response = match app.query(chains).await {
                        Ok(r) => r,
                        Err(e) => {
                            eprintln!("✗ Chain query failed: {}", e);
                            return;
                        }
                    };

                    let chains: Option<TournamentChainsResponse> =
                        match serde_json::from_str(&chain_response) {
                            Ok(d) => Some(d),
                            Err(e) => {
                                eprintln!("✗ Failed to parse tournament chains: {}", e);
                                None
                            }
                        };

                    if let Some(chains) = chains {
                        if chains.data.tournament_chains.len() > 0 {
                            tx.send(chains.data.tournament_chains)
                                .await
                                .expect("failed to send update");
                        }
                    }
                }
            });

            tokio::spawn(async move {
                while let Some(chains) = rx.recv().await {
                    for id in chains {
                        client_manager
                            .ensure_running(id, &chain.client, &app_id)
                            .await;
                    }
                }
            });
            println!("Watching for tournament Chains...");
        }
    }
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(300)).await;
    }
}

#[derive(Debug, Deserialize)]
pub struct TournamentChainsResponse {
    pub data: TournamentChains,
}

#[derive(Debug, Deserialize)]
pub struct TournamentChains {
    #[serde(rename = "tournamentChains")]
    pub tournament_chains: Vec<String>,
}
