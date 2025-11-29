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

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct Player {
    pub id: String,
    pub name: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct MatchHistory {
    pub you: Player,
    pub opponent: Player,
    #[serde(rename = "blobHash")]
    pub blob_hash: String,
}

#[derive(Debug, Deserialize)]
pub struct MatchHistoryResponse {
    pub data: MatchHistoryLast,
}

#[derive(Debug, Deserialize)]
pub struct MatchHistoryLast {
    #[serde(rename = "matchHistoryLast")]
    pub match_history_last: Option<MatchHistory>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MatchHistoryDB {
    #[serde(rename = "player1Id")]
    pub player_1_id: String,
    #[serde(rename = "player1Name")]
    pub player_1_name: Option<String>,

    #[serde(rename = "player2Id")]
    pub player_2_id: String,
    #[serde(rename = "player2Name")]
    pub player_2_name: Option<String>,

    #[serde(rename = "blobHash")]
    pub blob_hash: String,
}

impl MatchHistory {
    pub fn for_db(&self) -> MatchHistoryDB {
        let data = self.clone();
        MatchHistoryDB {
            player_1_id: data.you.id,
            player_1_name: data.you.name,
            player_2_id: data.opponent.id,
            player_2_name: data.opponent.name,
            blob_hash: data.blob_hash,
        }
    }
}

#[async_trait]
impl SupabaseModel for MatchHistoryDB {
    fn table_name() -> &'static str {
        "matchHistory"
    }

    fn primary_key() -> &'static str {
        "id"
    }

    async fn insert(&self, client: &SupabaseClient) -> Result<()> {
        client.insert(self).await
    }

    async fn insert_many(_records: Vec<Self>, _client: &SupabaseClient) -> Result<()> {
        anyhow::bail!("insert_many not supported for MatchHistory")
    }

    async fn replace(&self, client: &SupabaseClient) -> Result<()> {
        client.delete_all::<Self>().await?.insert(self).await
    }

    async fn replace_all(_records: Vec<Self>, _client: &SupabaseClient) -> Result<()> {
        anyhow::bail!("replace_all not supported for MatchHistory")
    }
}
