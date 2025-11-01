use std::error::Error as StdError;
use std::fmt;
use std::ops::{Deref, DerefMut};

use linera_base::crypto::InMemorySigner;
use linera_base::identifiers::AccountOwner;
use linera_client::wallet::Wallet;
use linera_faucet_client::Faucet;
use linera_persistent::{Persist, PersistExt};
use linera_views::lru_caching::StorageCacheConfig;
use linera_views::rocks_db::{
    PathWithGuard, RocksDbSpawnMode, RocksDbStoreConfig, RocksDbStoreInternalConfig,
};

pub type Storage =
    linera_storage::DbStorage<linera_views::rocks_db::RocksDbDatabase, linera_storage::WallClock>;
pub type Signer = InMemorySigner;

impl Deref for PersistentWallet {
    type Target = Wallet;

    fn deref(&self) -> &Self::Target {
        &self.wallet
    }
}

impl DerefMut for PersistentWallet {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.wallet.as_mut()
    }
}

#[derive(Debug)]
pub struct WasmPersistError {
    inner: String,
}

impl WasmPersistError {
    pub fn new(msg: impl Into<String>) -> Self {
        Self { inner: msg.into() }
    }
}

impl fmt::Display for WasmPersistError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner)
    }
}

impl StdError for WasmPersistError {}

// These are safe in single-threaded WASM
unsafe impl Send for WasmPersistError {}
unsafe impl Sync for WasmPersistError {}

// Implement Persist trait
impl Persist for PersistentWallet {
    type Error = WasmPersistError;

    fn as_mut(&mut self) -> &mut Self::Target {
        self.wallet.as_mut()
    }

    async fn persist(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn into_value(self) -> Self::Target {
        self.wallet.into_value()
    }
}

/// A wallet that stores the user's chains and keys in memory.
pub struct PersistentWallet {
    pub wallet: linera_persistent::Memory<Wallet>,
    pub signer: Signer,
    pub storage: Storage,
}

// for local testing
const FAUCET_URL: &str = "http://localhost:8080";

impl PersistentWallet {
    pub async fn new() -> Result<Self, anyhow::Error> {
        let faucet = Faucet::new(FAUCET_URL.to_string());
        let mut wallet = linera_persistent::Memory::new(linera_client::wallet::Wallet::new(
            faucet.genesis_config().await?,
        ));

        let mut signer = InMemorySigner::new(None);
        let owner = signer.generate_new();

        let account_owner: AccountOwner = AccountOwner::from(owner);
        let description = faucet.claim(&account_owner).await?;

        wallet
            .mutate(|wallet| {
                wallet.assign_new_chain_to_owner(
                    account_owner,
                    description.id(),
                    description.timestamp(),
                )
            })
            .await??;

        let inner_config = RocksDbStoreInternalConfig {
            path_with_guard: PathWithGuard::new("./linera_storage".into()),
            spawn_mode: RocksDbSpawnMode::SpawnBlocking, // Best for tokio multi-threaded
            max_stream_queries: 20,                      // Higher for better concurrency
        };

        let config = RocksDbStoreConfig {
            inner_config,
            storage_cache_config: StorageCacheConfig {
                max_entry_size: 200_000, // need to update these values to what's actually needed. 
                max_cache_size: 100000,
                max_cache_entries: 100000,
            },
        };

        let storage = linera_storage::DbStorage::maybe_create_and_connect(&config, "linera", None)
            .await
            .expect("failed to create storage");

        Ok(PersistentWallet {
            wallet,
            signer,
            storage,
        })
    }

    pub async fn get_storage(&self) -> Result<Storage, anyhow::Error> {
        Ok(self.storage.clone())
    }
}
