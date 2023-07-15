use std::fmt;
use std::error::Error;
use std::collections::{HashSet, HashMap};
use chrono::{DateTime, Utc};
use serde_json::Value;
use serde::Deserialize;
use serde_json;
use futures::{StreamExt, stream};
use tokio;

use crate::fetch::{FetchResult, FetchError, ListRetriever};
use crate::data::{Card, SimpleCardList};

// ---- Raw Moxfield JSON Datafields ---- //

#[derive(Debug, Deserialize)]
struct MoxfieldCard {
    id: String,
    name: String,
}

#[derive(Debug, Deserialize)]
struct MoxfieldCardInfo {
    quantity: u32,
    card: MoxfieldCard,
}

type MoxfieldCardList = HashMap<String, MoxfieldCardInfo>;

#[derive(Debug, Deserialize)]
struct MoxfieldRawBoard {
    count: u32,
    cards: MoxfieldCardList,
}

type MoxfieldBoards = HashMap<String, MoxfieldRawBoard>;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MoxfieldDeck {
    last_updated_at_utc: String,
    boards: MoxfieldBoards,
}

// -------------------------------------- //

#[derive(Debug)]
struct MoxfieldRetrieveError {
    http_status: u32,
    deck_id: String,
}

impl MoxfieldRetrieveError {
    pub fn new(http_status: u32, deck_id: &str) -> Self {
        MoxfieldRetrieveError {
            http_status,
            deck_id: String::from(deck_id),
        }
    }
}

impl fmt::Display for MoxfieldRetrieveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Received HTTP status code {} accessing deck with ID '{}'", &self.http_status, &self.deck_id)
    }
}

impl Error for MoxfieldRetrieveError {}

// -------------------------------------- //

#[derive(Clone, Hash, PartialEq, Eq)]
pub enum MoxfieldBoard {
    Main,
    Side,
    Maybe,
}

fn mox_board_to_json_field(board: &MoxfieldBoard) -> &'static str {
    match board {
        MoxfieldBoard::Main => "mainboard",
        MoxfieldBoard::Side => "sideboard",
        MoxfieldBoard::Maybe => "maybeboard",
    }
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

pub async fn get_deck_json(deck_id: &str) -> Result<Value, FetchError> {
    let client = reqwest::Client::new();
    let url = format!("https://api2.moxfield.com/v3/decks/all/{}", deck_id);
    client
        .get(&url)
        .body("")
        .send()
        .await.map_err(|err| {
            FetchError::RetrievalError(Box::new(err))
        })?
        .json::<Value>()
        .await.map_err(|err| {
            FetchError::DataParseError(Box::new(err))
        })
}

#[derive(Debug, Deserialize)]
struct MoxfieldStatus {
    pub status: u32,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum MoxfieldAPIResponse {
    Base(MoxfieldStatus),
    Deck(MoxfieldDeck),
}


fn moxdeck_to_simple_card_list(mut deck: MoxfieldDeck, boards: &HashSet<&MoxfieldBoard>) -> FetchResult {
    let last_updated: DateTime<Utc> = String::from(deck.last_updated_at_utc).parse().map_err(|err| FetchError::DataParseError(Box::new(err)))?;
    let mut all_cards: Vec<Card> = vec![];
    for board in boards.into_iter() {
        let mox_board = deck.boards.remove(&String::from(mox_board_to_json_field(board))).unwrap();
        mox_board.cards.into_iter().for_each(|(_k, v)| {
            all_cards.push(v.card.name);
        });
    }
    Ok(SimpleCardList {
        last_updated,
        cards: all_cards,
    })
}

fn json_to_simple_card_list(mf_list: &MoxfieldList, json: Value) -> FetchResult {
    let mf_resp = serde_json::value::from_value::<MoxfieldAPIResponse>(json).map_err(|deck_parse_err|
        FetchError::DataParseError(Box::new(deck_parse_err)))?;
    let mx_board = match mf_resp {
        MoxfieldAPIResponse::Base(x) => Err(FetchError::RetrievalError(Box::new(MoxfieldRetrieveError::new(x.status, mf_list.deck_id.as_str())))),
        MoxfieldAPIResponse::Deck(x) => Ok(x),
    }?;
    moxdeck_to_simple_card_list(mx_board, &HashSet::from_iter(mf_list.boards.iter()))
}

pub fn get_decks<'a>(card_lists: impl Iterator<Item=&'a MoxfieldList>) -> Vec<(&'a MoxfieldList, FetchResult)> {

    // Retrieve decks from Moxfield in an async fashion for performance
    let stream = stream::unfold(card_lists.into_iter(), |mut card_lists_iter| async {
        let card_list = card_lists_iter.next()?;
        let net_resp = get_deck_json(&card_list.deck_id).await;
        Some(((card_list, net_resp), card_lists_iter))
    });
    let runtime = tokio::runtime::Builder::new_current_thread().enable_io().enable_time().build().unwrap();
    let results = runtime.block_on(stream.collect::<Vec<(&MoxfieldList, Result<Value, FetchError>)>>());

    // Convert json/error results into output format
    results.into_iter().map(|(list, res_json)| match res_json {
        Ok(json) => (list, json_to_simple_card_list(list, json)),
        Err(err) => {
            (list, Err(err))
        },
    }).collect::<Vec<(&MoxfieldList, Result<SimpleCardList, FetchError>)>>()
}

// ---- Interface ---- //

pub struct MoxfieldFetcher {}

impl ListRetriever<MoxfieldList> for MoxfieldFetcher {
    fn fetch<'a>(list: impl Iterator<Item=&'a MoxfieldList>) -> Vec<(&'a MoxfieldList, FetchResult)> {
        get_decks(list)
    }
}
