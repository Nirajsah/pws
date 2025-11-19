use crate::supabase::{SupabaseClient, SupabaseModel};
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct LeaderboardData {
    pub leaderboard: Vec<Leaderboard>,
}

#[derive(Debug, Deserialize)]
pub struct LeaderBoardResponse {
    pub data: LeaderboardData,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Leaderboard {
    pub id: String,
    pub name: Option<String>,
    pub elo: u32,
    pub matches: u32,
    pub won: u32,
    pub lost: u32,
}

#[async_trait]
impl SupabaseModel for Leaderboard {
    fn table_name() -> &'static str {
        "leaderboard"
    }

    fn primary_key() -> &'static str {
        "id"
    }

    async fn insert(&self, client: &SupabaseClient) -> Result<()> {
        client.insert(self).await
    }

    async fn insert_many(records: Vec<Self>, client: &SupabaseClient) -> Result<()> {
        client.insert_many(&records).await
    }

    async fn replace(&self, _client: &SupabaseClient) -> Result<()> {
        anyhow::bail!("replace not supported for Leaderboard")
    }

    async fn replace_all(records: Vec<Self>, client: &SupabaseClient) -> Result<()> {
        client
            .delete_all::<Self>()
            .await?
            .insert_many(&records)
            .await
    }
}

#[derive(Debug, Deserialize)]
pub struct CountResponse {
    pub data: CountData,
}

#[derive(Debug, Deserialize)]
pub struct CountData {
    pub count: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct GameCount {
    pub id: String,
    pub count: String,
}

#[async_trait]
impl SupabaseModel for GameCount {
    fn table_name() -> &'static str {
        "gameCount"
    }

    fn primary_key() -> &'static str {
        "id"
    }

    async fn insert(&self, client: &SupabaseClient) -> Result<()> {
        client.upsert(self).await
    }

    async fn insert_many(_records: Vec<Self>, _client: &SupabaseClient) -> Result<()> {
        anyhow::bail!("insert_many not supported for GameCount")
    }

    async fn replace(&self, client: &SupabaseClient) -> Result<()> {
        client.delete_all::<Self>().await?.insert(self).await
    }

    async fn replace_all(_records: Vec<Self>, _client: &SupabaseClient) -> Result<()> {
        anyhow::bail!("replace_all not supported for GameCount")
    }
}
