use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde::Serialize;
use std::env;

/// Trait representing a model that can be persisted to Supabase
#[async_trait]
pub trait SupabaseModel: Serialize + Send + Sync {
    /// The name of the table in Supabase
    fn table_name() -> &'static str;

    /// Insert the record into Supabase
    async fn insert(&self, client: &SupabaseClient) -> Result<()>;

    // Optionally implement fetching or other CRUD operations later
    // async fn fetch_all(client: &SupabaseClient) -> Result<Vec<Self>> where Self: Sized;
}

/// Represents a Supabase HTTP client
pub struct SupabaseClient {
    client: Client,
    url: String,
    key: String,
}

impl SupabaseClient {
    pub fn new() -> Result<Self> {
        dotenv::dotenv().ok();
        let url = env::var("SUPABASE_URL")?;
        let key = env::var("SUPABASE_KEY")?;
        Ok(Self {
            client: Client::new(),
            url,
            key,
        })
    }

    /// Generic insert function usable by all Supabase models
    pub async fn insert<T: SupabaseModel>(&self, record: &T) -> Result<()> {
        let table = T::table_name();
        let endpoint = format!("{}/rest/v1/{}", self.url, table);

        let res = self
            .client
            .post(&endpoint)
            .header("apikey", &self.key)
            .header("Authorization", format!("Bearer {}", self.key))
            .header("Content-Type", "application/json")
            .header("Prefer", "return=representation")
            .json(record)
            .send()
            .await?;

        let status = res.status();
        let body = res.text().await?;

        if !status.is_success() {
            anyhow::bail!("Failed to insert record: {}", body);
        }

        println!("[Supabase] Inserted into `{}`: {}", table, body);
        Ok(())
    }
}
