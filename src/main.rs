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

    let app_id = "443ff420b2265303779c7d2d681353e47826cb4b1977d8b0351076f666cf7f93";

    let app = client_context.frontend().application(app_id).await?;

    let query = r#"{ "query": "query { value }" }"#;

    let muta = r#"{ "query": "mutation { increment(value: 10) }" }"#;

    let res = app.query(muta).await?;

    println!("update from the query: {:?}", res);

    let res = app.query(query).await?;

    println!("result from the query: {:?}", res);

    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
    }
}
