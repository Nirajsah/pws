use crate::wallet::{PersistentWallet, Signer};
use futures::lock::Mutex as AsyncMutex;
use linera_base::identifiers::ApplicationId;
use linera_client::{
    chain_listener::{ChainListener, ChainListenerConfig, ClientContext},
    client_options::ClientContextOptions,
};
use linera_core::{
    data_types::ClientOutcome,
    node::{ValidatorNode, ValidatorNodeProvider},
};
use std::{collections::HashMap, sync::Arc, time::Duration};

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

#[derive(Clone)]
pub struct Frontend(Client);

#[derive(Clone)]
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

        // CRITICAL: Synchronize all chains before starting listener, WE CAN'T OMIT THIS
        {
            let mut guard = client_context.lock().await;
            let chain_ids: Vec<_> = guard.wallet().chain_ids();
            for chain_id in chain_ids {
                let client = guard.make_chain_client(chain_id);
                client.synchronize_from_validators().await?;
                guard.update_wallet(&client).await?;
            }
        }

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

    async fn apply_client_command<Fut, T, E>(
        &self,
        chain_client: &ChainClient,
        mut command: impl FnMut() -> Fut,
    ) -> Result<Result<T, E>, linera_client::Error>
    where
        Fut: Future<Output = Result<ClientOutcome<T>, E>>,
    {
        let result = loop {
            use ClientOutcome::{Committed, WaitForTimeout};
            let timeout = match command().await {
                Ok(Committed(outcome)) => break Ok(Ok(outcome)),
                Ok(WaitForTimeout(timeout)) => timeout,
                Err(e) => break Ok(Err(e)),
            };
            let mut stream = chain_client.subscribe()?;
            linera_client::util::wait_for_next_round(&mut stream, timeout).await;
        };

        self.client_context
            .lock()
            .await
            .update_wallet(chain_client)
            .await?;

        result
    }

    pub async fn balance(&self) -> Result<String, anyhow::Error> {
        Ok(self
            .default_chain_client()
            .await?
            .query_balance()
            .await?
            .to_string())
    }

    pub fn frontend(&self) -> Frontend {
        Frontend(self.clone())
    }
}

pub struct Application {
    client: Client,
    id: ApplicationId,
}

impl Frontend {
    /// Gets the version information of the validators of the current network.
    ///
    /// # Errors
    /// If a validator is unreachable.
    ///
    /// # Panics
    /// If no default chain is set for the current wallet.
    pub async fn validator_version_info(&self) -> Result<(), anyhow::Error> {
        let mut client_context = self.0.client_context.lock().await;
        let chain_id = client_context
            .wallet()
            .default_chain()
            .expect("No default chain");
        let chain_client = client_context.make_chain_client(chain_id);
        chain_client.synchronize_from_validators().await?;
        let result = chain_client.local_committee().await;
        client_context.update_wallet(&chain_client).await?;
        let committee = result?;
        let node_provider = client_context.make_node_provider();

        let mut validator_versions = HashMap::new();

        for (name, state) in committee.validators() {
            match node_provider
                .make_node(&state.network_address)?
                .get_version_info()
                .await
            {
                Ok(version_info) => {
                    if validator_versions
                        .insert(name, version_info.clone())
                        .is_some()
                    {
                        println!("duplicate validator entry for validator {name:?}");
                    }
                }
                Err(e) => {
                    println!("failed to get version information for validator {name:?}:\n{e:?}");
                }
            }
        }

        println!("validator info: {:?}", validator_versions);

        Ok(())
    }

    /// Retrieves an application for querying.
    ///
    /// # Errors
    /// If the application ID is invalid.
    pub async fn application(&self, id: &str) -> Result<Application, anyhow::Error> {
        Ok(Application {
            client: self.0.clone(),
            id: id.parse()?,
        })
    }
}

impl Application {
    /// Performs a query against an application's service.
    ///
    /// # Errors
    /// If the application ID is invalid, the query is incorrect, or
    /// the response isn't valid UTF-8.
    ///
    /// # Panics
    /// On internal protocol errors.
    // TODO(#14) allow passing bytes here rather than just strings
    // TODO(#15) a lot of this logic is shared with `linera_service::node_service`
    pub async fn query(&self, query: &str) -> Result<String, anyhow::Error> {
        let chain_client = self.client.default_chain_client().await?;

        let linera_execution::QueryOutcome {
            response: linera_execution::QueryResponse::User(response),
            operations,
        } = chain_client
            .query_application(linera_execution::Query::User {
                application_id: self.id,
                bytes: query.as_bytes().to_vec(),
            })
            .await?
        else {
            panic!("system response to user query")
        };

        if !operations.is_empty() {
            let _hash = self
                .client
                .apply_client_command(&chain_client, || {
                    chain_client.execute_operations(operations.clone(), vec![])
                })
                .await??;
        }

        Ok(String::from_utf8(response)?)
    }
}
