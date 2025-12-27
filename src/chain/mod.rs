// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use futures::StreamExt;
use linera_base::identifiers::AccountOwner;
use linera_core::client::ChainClient;

pub mod application;
use crate::client::{Client, Environment};
pub use application::Application;

#[derive(Clone)]
pub struct Chain {
    pub(crate) client: Client,
    pub(crate) chain_client: ChainClient<Environment>,
}

pub struct TransferParams {
    pub donor: Option<AccountOwner>,
    pub amount: u64,
    pub recipient: linera_base::identifiers::Account,
}

pub struct AddOwnerOptions {
    pub weight: u64,
}

impl Chain {
    /// Sets a callback to be called when a notification is received
    /// from the network.
    ///
    /// # Errors
    /// If we fail to subscribe to the notification stream.
    ///
    /// # Panics
    /// If the handler function fails.
    pub fn on_notification<F, Fut>(&self, f: F)
    where
        F: Fn() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let mut notifications = self.chain_client.subscribe().unwrap();
        tokio::spawn(async move {
            while let Some(_notification) = notifications.next().await {
                f().await;
            }
        });
    }

    /// Retrieves an application for querying.
    ///
    /// # Errors
    /// If the application ID is invalid.
    pub async fn application(&self, id: &str) -> Result<Application, anyhow::Error> {
        Ok(Application {
            client: self.client.clone(),
            chain_client: self.chain_client.clone(),
            id: id.parse()?,
        })
    }
}
