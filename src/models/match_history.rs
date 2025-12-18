use crate::supabase::{SupabaseClient, SupabaseModel};
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

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
