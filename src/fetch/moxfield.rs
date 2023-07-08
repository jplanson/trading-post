use anyhow::{anyhow, Result};
use std::collections::{HashSet, HashMap};
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

// -------------------------------------- //

#[derive(Hash, PartialEq, Eq)]
pub enum MoxfieldBoard {
    Main,
    Side,
    Maybe,
}

pub struct MoxfieldList {
    deck_id: String,
    boards: HashSet<MoxfieldBoard>,
}

impl MoxfieldList {
    pub fn from_vec(deck_id: String, boards: Vec<MoxfieldBoard>) -> Self {
        MoxfieldList { deck_id, boards: HashSet::from_iter(boards.into_iter()) }
    }
    pub fn basic(deck_id: &str) -> Self {
        MoxfieldList { deck_id: String::from(deck_id), boards: HashSet::from_iter(vec![MoxfieldBoard::Main].into_iter()) }
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

fn json_to_simple_card_list(mut json: Value) -> Result<SimpleCardList> {
    let Some(deck_obj) = json.as_object_mut() else {
        return Err(anyhow!("Malformed json"));
    };
    let Some(boards_json) = deck_obj.remove("boards") else {
        return Err(anyhow!("Malformed json"));
    };
    let mut boards: MoxfieldBoards = serde_json::value::from_value(boards_json)?;
    let card_list = boards.remove(&String::from("mainboard")).unwrap().cards.into_iter().map(|(_k, v)| {
        Card { name: v.card.name }
    }).collect::<Vec<Card>>();
    Ok(card_list)
}

pub fn get_decks(card_lists: impl Iterator<Item=MoxfieldList>) -> Vec<(MoxfieldList, Result<SimpleCardList>)> {
    let stream = stream::unfold(card_lists.into_iter(), |mut card_lists_iter| async {
        let card_list = card_lists_iter.next()?;
        let net_resp = get_deck_json(&card_list.deck_id).await;
        Some(((card_list, net_resp), card_lists_iter))
    });
    let runtime = tokio::runtime::Builder::new_current_thread().enable_io().enable_time().build().unwrap();
    let results = runtime.block_on(stream.collect::<Vec<(MoxfieldList, Result<Value>)>>());
    results.into_iter().map(|(list, res_json)| match res_json {
        Ok(json) => (list, json_to_simple_card_list(json)),
        Err(err) => (list, Err(err)),
    }).collect::<Vec<(MoxfieldList, Result<SimpleCardList>)>>()
}

// ---- Interface ---- //

struct MoxfieldFetcher {
}

impl ListRetriever<MoxfieldList> for MoxfieldFetcher {
    fn fetch(list: impl Iterator<Item=MoxfieldList>) -> Vec<(MoxfieldList, Result<SimpleCardList>)> {
        get_decks(list)
    }
}
