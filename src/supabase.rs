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
    fn primary_key() -> &'static str;

    /// Insert the record into Supabase
    async fn insert(&self, client: &SupabaseClient) -> Result<()>;

    async fn insert_many(records: Vec<Self>, client: &SupabaseClient) -> Result<()>
    where
        Self: Sized;

    async fn replace(&self, client: &SupabaseClient) -> Result<()>;

    async fn replace_all(records: Vec<Self>, client: &SupabaseClient) -> Result<()>
    where
        Self: Sized;
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

    pub async fn insert_many<T: SupabaseModel>(&self, records: &[T]) -> Result<()> {
        let table = T::table_name();
        let endpoint = format!("{}/rest/v1/{}", self.url, table);

        let res = self
            .client
            .post(&endpoint)
            .header("apikey", &self.key)
            .header("Authorization", format!("Bearer {}", self.key))
            .header("Content-Type", "application/json")
            .header("Prefer", "return=representation")
            .json(records)
            .send()
            .await?;

        let status = res.status();
        let body = res.text().await?;

        if !status.is_success() {
            anyhow::bail!("Failed to insert records: {}", body);
        }

        println!("[Supabase] Inserted into `{}`: {}", table, body);
        Ok(())
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

    pub async fn upsert<T: SupabaseModel>(&self, record: &T) -> Result<()> {
        let table = T::table_name();
        let endpoint = format!("{}/rest/v1/{}", self.url, table);

        let res = self
            .client
            .post(&endpoint)
            .header("apikey", &self.key)
            .header("Authorization", format!("Bearer {}", self.key))
            .header("Content-Type", "application/json")
            .header("Prefer", "resolution=merge-duplicates")
            .json(record)
            .send()
            .await?;

        let status = res.status();
        let body = res.text().await?;

        if !status.is_success() {
            anyhow::bail!("Failed to upsert record: {} - {}", status, body);
        }

        println!("[Supabase] ✓ Upserted into `{}`", table);
        Ok(())
    }

    pub async fn delete_all<T: SupabaseModel>(&self) -> Result<&Self> {
        let table = T::table_name();
        let pk = T::primary_key();
        let endpoint = format!("{}/rest/v1/{}?{}=neq.", self.url, table, pk);

        let res = self
            .client
            .delete(&endpoint)
            .header("apikey", &self.key)
            .header("Authorization", format!("Bearer {}", self.key))
            .header("Content-Type", "application/json")
            .send()
            .await?;

        let status = res.status();
        let body = res.text().await?;

        if !status.is_success() {
            anyhow::bail!("Failed to delete table `{}`: {}", table, body);
        }

        println!("[Supabase] Deleted all rows from `{}`", table);
        Ok(self)
    }
}
