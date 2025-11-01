#![recursion_limit = "256"]

use crate::{client::Client, wallet::PersistentWallet};
pub mod client;
pub mod wallet;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let p = PersistentWallet::new().await?;

    let client_context = Client::new(p).await?;
    let balance = client_context.balance().await?;
    println!("balance: {:?}", balance);

    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
    }
}
