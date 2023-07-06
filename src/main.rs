mod data;
mod fetch;

use std::collections::HashSet;
use crate::fetch::get_decks;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let deck_ids = HashSet::from_iter(vec!["n9-ZrMnGIU2mLoND3UyZvQ", "a", "b", "c", "d", "e", "f", "g", "h", "i", "j"].into_iter());
    get_decks(&deck_ids);
    Ok(())
}
