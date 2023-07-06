use anyhow::Result;
use std::collections::HashSet;
use serde_json::Value;
use futures::executor::block_on;
use futures::{StreamExt, stream};
use futures::future::Future;

pub async fn get_deck_json(deck_id: &str) -> Result<Value> {
    let client = reqwest::Client::new();
    let res = client
        .get(format!(
            "https://api2.moxfield.com/v3/decks/all/{}",
            deck_id
        ))
        .body("")
        .send()
        .await?
        .json::<Value>()
        .await?;
    Ok(res)
}

pub async fn get_deck(deck_id: &str) -> Result<()> {
    // let res = get_deck_json(deck_id).await?;
    println!("DECK ID: {}", deck_id);
    Ok(())
}

pub fn get_decks(deck_ids: &HashSet<&str>) -> Result<()> {
    let mut stream = stream::unfold(deck_ids.into_iter(), |mut vals| async {
        let val = vals.next()?;
        let res = get_deck(val).await;
        Some((res, vals))
    });
    let results = block_on(stream.collect::<Vec<Result<()>>>());
    Ok(())
}
