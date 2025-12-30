// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use linera_base::{
    crypto::InMemorySigner,
    identifiers::{AccountOwner, ChainId},
};
use linera_client::config::GenesisConfig;
use linera_core::wallet;
use linera_faucet_client::Faucet;
use linera_persistent::{self as persistent, Persist};
use linera_views::{
    lru_prefix_cache::StorageCacheConfig,
    rocks_db::{PathWithGuard, RocksDbSpawnMode, RocksDbStoreConfig, RocksDbStoreInternalConfig},
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::storage::Storage;

#[derive(Clone)]
pub struct PersistentWallet {
    pub(crate) wallet: Wallet,
    storage: Storage,
    pub signer: InMemorySigner,
}

/// A wallet that stores the user's chains and keys in memory.
#[derive(Clone, Serialize, Deserialize)]
pub struct Wallet {
    pub(crate) chains: wallet::Memory,
    pub(crate) default: Option<ChainId>,
    pub(crate) genesis_config: GenesisConfig,
}

// for local testing
const FAUCET_URL: &str = "http://localhost:8079";
// const FAUCET_URL: &str = "https://faucet.testnet-conway.linera.net/";

impl PersistentWallet {
    pub fn signer_address(&self) -> AccountOwner {
        self.signer.keys()[0].0
    }

    pub fn create_keystore(
        keystore_path: PathBuf,
    ) -> Result<persistent::File<InMemorySigner>, anyhow::Error> {
        if keystore_path.exists() {
            println!("Keystore exists: {}", keystore_path.display());
        }
        Ok(persistent::File::read(&keystore_path)?)
    }
    pub async fn new(keystore_path: Option<PathBuf>) -> Result<Self, anyhow::Error> {
        let faucet = Faucet::new(FAUCET_URL.to_string());

        let mut wallet = Wallet {
            chains: wallet::Memory::default(),
            default: None,
            genesis_config: faucet.genesis_config().await?,
        };

        let (signer, owner) = if let Some(keystore_path) = keystore_path {
            let signer = Self::create_keystore(keystore_path)?;
            let owner = signer.keys()[0].0;
            (signer, owner)
        } else {
            let mut signer = InMemorySigner::new(None);
            signer.generate_new();
            let signer = persistent::File::new(Path::new("keystore.json"), signer.clone())?;
            let owner = signer.keys()[0].0;
            (signer, owner)
        };

        let description = faucet.claim(&owner).await?;

        let chain_id = description.id();
        wallet.chains.insert(
            chain_id,
            wallet::Chain {
                owner: Some(owner),
                ..description.into()
            },
        );

        if wallet.default.is_none() {
            wallet.default = Some(chain_id);
        }

        let inner_config = RocksDbStoreInternalConfig {
            path_with_guard: PathWithGuard::new("./client.db".into()),
            spawn_mode: RocksDbSpawnMode::SpawnBlocking, // Best for tokio multi-threaded
            max_stream_queries: 20,                      // Higher for better concurrency
        };

        let config = RocksDbStoreConfig {
            inner_config,
            storage_cache_config: StorageCacheConfig {
                max_cache_size: 100000,
                max_cache_entries: 100000,
                max_cache_find_key_values_size: 100000,
                max_cache_find_keys_size: 100000,
                max_cache_value_size: 100000,
                max_find_key_values_entry_size: 100000,
                max_find_keys_entry_size: 100000,
                max_value_entry_size: 100000,
            },
        };

        let storage = linera_storage::DbStorage::maybe_create_and_connect(
            &config,
            "linera",
            Some(linera_execution::WasmRuntime::Wasmer),
        )
        .await
        .expect("failed to create storage");

        persistent::File::new(Path::new("wallet.json"), wallet.clone())?;

        Ok(PersistentWallet {
            wallet,
            signer: signer.into_value(),
            storage,
        })
    }

    pub async fn get_storage(&self) -> Result<Storage, anyhow::Error> {
        Ok(self.storage.clone())
    }
}
