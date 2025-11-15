use crate::supabase::{SupabaseClient, SupabaseModel};
use anyhow::Result;
use async_trait::async_trait;
use serde::Serialize;

#[derive(Serialize)]
pub struct LeaderBoard {}

#[async_trait]
impl SupabaseModel for LeaderBoard {
    fn table_name() -> &'static str {
        "leaderboard"
    }

    async fn insert(&self, client: &SupabaseClient) -> Result<()> {
        client.insert(self).await
    }
}
