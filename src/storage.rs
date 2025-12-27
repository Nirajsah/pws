// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use linera_views::{
    lru_prefix_cache::StorageCacheConfig,
    rocks_db::{PathWithGuard, RocksDbSpawnMode, RocksDbStoreConfig, RocksDbStoreInternalConfig},
};

pub type Storage =
    linera_storage::DbStorage<linera_views::rocks_db::RocksDbDatabase, linera_storage::WallClock>;

/// Create and return the storage implementation.
///
/// # Errors
/// If the storage can't be initialized.
pub async fn get_storage() -> Result<Storage, linera_views::ViewError> {
    let inner_config = RocksDbStoreInternalConfig {
        path_with_guard: PathWithGuard::new("./linera".into()),
        spawn_mode: RocksDbSpawnMode::SpawnBlocking, // Best for tokio multi-threaded
        max_stream_queries: 20,                      // Higher for better concurrency
    };

    let config = RocksDbStoreConfig {
        inner_config,
        storage_cache_config: StorageCacheConfig {
            // mock values, need to find suitable values
            max_cache_size: 100000,
            max_cache_entries: 100000,
            max_cache_find_key_values_size: 10000,
            max_cache_find_keys_size: 10000,
            max_cache_value_size: 1000,
            max_find_key_values_entry_size: 1000,
            max_find_keys_entry_size: 1000,
            max_value_entry_size: 1000,
        },
    };

    let storage = linera_storage::DbStorage::maybe_create_and_connect(
        &config,
        "linera",
        Some(linera_execution::WasmRuntime::Wasmer),
    )
    .await
    .expect("failed to create storage");

    Ok(storage)
}
