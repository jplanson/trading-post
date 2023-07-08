use anyhow::Result;
use std::collections::{HashSet, HashMap};
use chrono::{DateTime, Utc};
use serde_json::Value;
use serde::Deserialize;
use serde_json;
use futures::{StreamExt, stream};
use tokio;

use crate::fetch::ListRetriever;
use crate::data::{Card, SimpleCardList};

// ---- Raw Moxfield JSON Datafields ---- //

#[derive(Deserialize)]
struct MoxfieldCard {
    id: String,
    name: String,
}

#[derive(Deserialize)]
struct MoxfieldCardInfo {
    quantity: u32,
    card: MoxfieldCard,
}

type MoxfieldCardList = HashMap<String, MoxfieldCardInfo>;

#[derive(Deserialize)]
struct MoxfieldRawBoard {
    count: u32,
    cards: MoxfieldCardList,
}

type MoxfieldBoards = HashMap<String, MoxfieldRawBoard>;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MoxfieldDeck {
    last_updated_at_utc: String,
    boards: MoxfieldBoards,
}

// -------------------------------------- //

#[derive(Clone, Hash, PartialEq, Eq)]
pub enum MoxfieldBoard {
    Main,
    Side,
    Maybe,
}

#[derive(Hash, PartialEq, Eq, Clone)]
pub struct MoxfieldList {
    pub deck_id: String,
    pub boards: Vec<MoxfieldBoard>,
}

impl MoxfieldList {
    pub fn from_vec(deck_id: String, boards: Vec<MoxfieldBoard>) -> Self {
        MoxfieldList { deck_id, boards }
    }
    pub fn basic(deck_id: &str) -> Self {
        MoxfieldList { deck_id: String::from(deck_id), boards: vec![MoxfieldBoard::Main] }
    }
}

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

fn json_to_simple_card_list(json: Value) -> Result<SimpleCardList> {
    let mut deck: MoxfieldDeck = serde_json::value::from_value(json)?;
    let last_updated: DateTime<Utc> = String::from(deck.last_updated_at_utc).parse()?;
    let cards = deck.boards.remove(&String::from("mainboard")).unwrap().cards.into_iter().map(|(_k, v)| {
        v.card.name
    }).collect::<Vec<Card>>();
    Ok(SimpleCardList {
        last_updated,
        cards,
    })
}

pub fn get_decks<'a>(card_lists: impl Iterator<Item=&'a MoxfieldList>) -> Vec<(&'a MoxfieldList, Result<SimpleCardList>)> {
    let stream = stream::unfold(card_lists.into_iter(), |mut card_lists_iter| async {
        let card_list = card_lists_iter.next()?;
        let net_resp = get_deck_json(&card_list.deck_id).await;
        Some(((card_list, net_resp), card_lists_iter))
    });
    let runtime = tokio::runtime::Builder::new_current_thread().enable_io().enable_time().build().unwrap();
    let results = runtime.block_on(stream.collect::<Vec<(&MoxfieldList, Result<Value>)>>());
    results.into_iter().map(|(list, res_json)| match res_json {
        Ok(json) => (list, json_to_simple_card_list(json)),
        Err(err) => (list, Err(err)),
    }).collect::<Vec<(&MoxfieldList, Result<SimpleCardList>)>>()
}

// ---- Interface ---- //

pub struct MoxfieldFetcher {}

impl ListRetriever<MoxfieldList> for MoxfieldFetcher {
    fn fetch<'a>(list: impl Iterator<Item=&'a MoxfieldList>) -> Vec<(&'a MoxfieldList, Result<SimpleCardList>)> {
        get_decks(list)
    }
}
