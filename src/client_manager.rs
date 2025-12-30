use linera_base::identifiers::ChainId;
use std::{collections::HashMap, str::FromStr, sync::Arc};
use tokio::sync::Mutex;

use crate::{
    chain::{Application, Chain},
    client::Client,
};

#[derive(Clone, Default)]
pub struct ChainClientManager {
    clients: Arc<Mutex<HashMap<ChainId, Arc<RunningChain>>>>,
}

/// A running instance of a [`Chain`](crate::chain::Chain) with cached state and application access.
/// This wraps an active [`Chain`](crate::chain::Chain), maintains a per-chain cache,
/// and runs background tasks that process notifications for that chain.
pub struct RunningChain {
    pub(crate) chain: Chain,
    pub(crate) app: Application,

    pub cache: Mutex<ChainStateCache>, // for our use case we keep a cache state, to avoid redundant Db updates
}

#[derive(Default)]
pub struct ChainStateCache {
    participants: HashMap<ChainId, String>,
    tournament: HashMap<ChainId, String>,
    matches: String,
    // TODO: does not fit what we need
}

impl RunningChain {
    pub fn new(chain: Chain, app: Application) -> Self {
        Self {
            chain,
            app,
            cache: Mutex::new(ChainStateCache::default()),
        }
    }

    /// this runs the notification service while querying and updating
    pub fn start_background_task(self: &Arc<Self>) {
        let this = Arc::clone(self);
        self.chain.on_notification(move || {
            let this = Arc::clone(&this);
            async move {
                match this
                    .app
                    .query(r#"{ "query": "query { notifications }" }"#)
                    .await
                {
                    Ok(_value) => {
                        todo!()
                    }
                    Err(_e) => eprintln!("query error on"),
                }
            }
        });
    }
}

impl ChainClientManager {
    /// Convenience: caller doesn’t need the handle
    pub async fn ensure_running(&self, chain_id: String, client: &Client, app_id: &str) {
        if let Ok(chain_id) = ChainId::from_str(&chain_id) {
            let _ = self.try_spawn_chain(chain_id, client, app_id).await;
        }
    }

    pub async fn try_spawn_chain(
        &self,
        chain_id: ChainId,
        main_client: &Client,
        app_id: &str,
    ) -> Arc<RunningChain> {
        let mut map = self.clients.lock().await;

        if let Some(rc) = map.get(&chain_id) {
            return rc.clone();
        }

        // First-time creation
        let chain = main_client.assign_and_make_client(chain_id).await.unwrap();
        let app = chain.application(app_id).await.unwrap();

        let running = Arc::new(RunningChain::new(chain, app));
        running.start_background_task(); // handle notification
        map.insert(chain_id, running.clone());

        println!("Started background task for chain: {chain_id}");
        running
    }
}
