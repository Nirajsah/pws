use crate::wallet::{PersistentWallet, Signer};
use futures::lock::Mutex as AsyncMutex;
use linera_base::crypto::InMemorySigner;
use linera_client::{
    chain_listener::{ChainListener, ChainListenerConfig, ClientContext},
    client_options::ClientContextOptions,
};
use std::{sync::Arc, time::Duration};

use crate::wallet::Storage;

type Environment =
    linera_core::environment::Impl<Storage, linera_rpc::node_provider::NodeProvider, Signer>;

type ClientContextType =
    linera_client::client_context::ClientContext<Environment, PersistentWallet>;

type ChainClient = linera_core::client::ChainClient<Environment>;

// Currently, testing to make sure things are working, will be refactored.
pub const OPTIONS: ClientContextOptions = ClientContextOptions {
    send_timeout: linera_base::time::Duration::from_millis(4000),
    recv_timeout: linera_base::time::Duration::from_millis(4000),
    max_pending_message_bundles: 10,
    retry_delay: linera_base::time::Duration::from_millis(1000),
    max_retries: 10,
    wait_for_outgoing_messages: false,
    blanket_message_policy: linera_core::client::BlanketMessagePolicy::Accept,
    restrict_chain_ids_to: None,
    long_lived_services: false,
    blob_download_timeout: linera_base::time::Duration::from_millis(1000),
    certificate_batch_download_timeout: linera_base::time::Duration::from_millis(1000),
    certificate_download_batch_size: linera_core::client::DEFAULT_CERTIFICATE_DOWNLOAD_BATCH_SIZE,
    sender_certificate_download_batch_size:
        linera_core::client::DEFAULT_SENDER_CERTIFICATE_DOWNLOAD_BATCH_SIZE,
    chain_worker_ttl: Duration::from_secs(30),
    sender_chain_worker_ttl: Duration::from_millis(200),
    grace_period: linera_core::DEFAULT_GRACE_PERIOD,
    max_joined_tasks: 100,
    timing_interval: 1u64,
    timings: false,

    // TODO(linera-protocol#2944): separate these out from the
    // `ClientOptions` struct, since they apply only to the CLI/native
    // client
    wallet_state_path: None,
    keystore_path: None,
    with_wallet: None,
    chrome_trace_exporter: false,
    otel_trace_file: None,
    otel_exporter_otlp_endpoint: None,
};

pub struct Client {
    client_context: Arc<AsyncMutex<ClientContextType>>,
}

impl Client {
    pub async fn new(wallet: PersistentWallet) -> Result<Client, anyhow::Error> {
        let mut storage = wallet.get_storage().await?;
        let signer = wallet.signer.clone();
        wallet
            .wallet
            .genesis_config()
            .initialize_storage(&mut storage)
            .await?;
        let client_context = Arc::new(AsyncMutex::new(ClientContextType::new(
            storage.clone(),
            OPTIONS,
            wallet,
            signer,
        )));

        let client_context_clone = client_context.clone();
        let chain_listener = ChainListener::new(
            ChainListenerConfig {
                skip_process_inbox: false,
                ..ChainListenerConfig::default()
            },
            client_context_clone,
            storage,
            tokio_util::sync::CancellationToken::new(),
        )
        .run(true) // Enable background sync
        .await?;

        tokio::spawn(async move {
            if let Err(error) = chain_listener.await {
                println!("ChainListener error: {error:?}");
            }
        });

        println!("client initialized successfully");
        Ok(Self { client_context })
    }

    pub async fn default_chain_client(&self) -> Result<ChainClient, anyhow::Error> {
        let client_context = self.client_context.lock().await;
        let chain_id = client_context
            .wallet()
            .default_chain()
            .expect("A default chain should be configured");
        Ok(client_context.make_chain_client(chain_id))
    }

    pub async fn balance(&self) -> Result<String, anyhow::Error> {
        Ok(self
            .default_chain_client()
            .await?
            .query_balance()
            .await?
            .to_string())
    }
}
