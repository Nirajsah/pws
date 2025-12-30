// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

/*!
# `linera-web`

This module defines the JavaScript bindings to the client API.

It is compiled to Wasm, with a JavaScript wrapper to inject its imports, and published on
NPM as `@linera/client`.

The `signer` subdirectory contains a TypeScript interface specifying the types of objects
that can be passed as signers — cryptographic integrations used to sign transactions, as
well as a demo implementation (not recommended for production use) that stores a private
key directly in memory and uses it to sign.
*/

// We sometimes need functions in this module to be async in order to
// ensure the generated code will return a `Promise`.
#![allow(clippy::unused_async)]

use anyhow::Ok;
use futures::lock::Mutex as AsyncMutex;
use linera_base::{crypto::InMemorySigner, identifiers::ChainId};
use linera_client::{
    chain_listener::{ChainListener, ClientContext as _},
    util::wait_for_next_round,
};
use linera_core::{client::ListeningMode, JoinSetExt};
use std::sync::Arc;

use crate::{chain::Chain, storage::Storage, wallet::PersistentWallet};

pub type Network = linera_rpc::node_provider::NodeProvider;

pub type Environment =
    linera_core::environment::Impl<Storage, Network, InMemorySigner, linera_core::wallet::Memory>;

/// The full client API, exposed to the wallet implementation. Calls
/// to this API can be trusted to have originated from the user's
/// request.
#[derive(Clone)]
pub struct Client {
    // This use of `futures::lock::Mutex` is safe because we only
    // expose concurrency to the browser, which must always run all
    // futures on the global task queue.
    // It does nothing here in this single-threaded context, but is
    // hard-coded by `ChainListener`.
    pub client_context: Arc<AsyncMutex<linera_client::ClientContext<Environment>>>,
    pub persistent: PersistentWallet,
}

impl Client {
    /// Creates a new client and connects to the network.
    ///
    /// # Errors
    /// On transport or protocol error, if persistent storage is
    /// unavailable, or if `options` is incorrectly structured.
    pub async fn new(
        w: &PersistentWallet,
        options: Option<linera_client::Options>,
    ) -> Result<Client, anyhow::Error> {
        let options = options.unwrap_or_default();

        let mut storage = w.get_storage().await?;
        w.wallet
            .genesis_config
            .initialize_storage(&mut storage)
            .await?;

        let client_context = linera_client::ClientContext::new(
            storage.clone(),
            w.wallet.chains.clone(),
            w.signer.clone(),
            &options,
            w.wallet.default,
            w.wallet.genesis_config.clone(),
        )
        .await?;

        // The `Arc` here is useless, but it is required by the `ChainListener` API.
        #[expect(clippy::arc_with_non_send_sync)]
        let client_context = Arc::new(AsyncMutex::new(client_context));
        let client_clone = client_context.clone();
        let chain_listener = ChainListener::new(
            options.chain_listener_config,
            client_clone,
            storage,
            tokio_util::sync::CancellationToken::new(),
            tokio::sync::mpsc::unbounded_channel().1,
        )
        .run(true) // Enable background sync
        .await?;

        tokio::spawn(async move {
            if let Err(error) = chain_listener.await {
                println!("ChainListener error: {error:?}");
            }
        });

        eprintln!("Linera Web client successfully initialized");

        Ok(Client {
            client_context,
            persistent: w.clone(),
        })
    }

    /// Connect to a chain on the Linera network.
    /// If no chain is provided, Default chain is used
    /// # Errors
    ///
    /// If the wallet could not be read or chain synchronization fails.
    pub async fn chain(&self, chain: Option<ChainId>) -> Result<Chain, anyhow::Error> {
        let mut ctx = self.client_context.lock().await; // Lock the client context
        let chain_id = chain.unwrap_or_else(|| ctx.default_chain());
        let chain_client = ctx.make_chain_client(chain_id).await?;

        chain_client.synchronize_from_validators().await?;
        chain_client.process_inbox().await?;

        ctx.update_wallet(&chain_client).await?;

        drop(ctx);

        let chain = Chain {
            chain_client,
            client: self.clone(),
        };
        Ok(chain)
    }

    /// Connect to a chain on the Linera network.
    /// If no chain is provided, Default chain is used
    /// # Errors
    ///
    /// If the wallet could not be read or chain synchronization fails.
    pub async fn assign_and_make_client(&self, chain_id: ChainId) -> Result<Chain, anyhow::Error> {
        let owner = self.persistent.signer_address();
        let mut ctx = self.client_context.lock().await;

        if !ctx.wallet().chain_ids().contains(&chain_id) {
            ctx.assign_new_chain_to_key(chain_id, owner).await?;
        }

        ctx.client.track_chain(chain_id);
        let chain_client = ctx.make_chain_client(chain_id).await?;

        let (listener, _listnen_handle, mut notificiation_stream) =
            chain_client.listen(ListeningMode::FullChain).await?;

        ctx.chain_listeners.spawn_task(listener);

        chain_client.synchronize_from_validators().await?;

        loop {
            let (_, maybe_timeout) = {
                let result = chain_client.process_inbox().await;
                ctx.update_wallet_from_client(&chain_client).await?;
                result?
            };
            if maybe_timeout.is_some() {
                wait_for_next_round(&mut notificiation_stream, maybe_timeout.unwrap()).await;
                continue;
            } else {
                break;
            }
        }

        drop(ctx);

        Ok(Chain {
            chain_client,
            client: self.clone(),
        })
    }
}
