extern crate markup5ever_rcdom as rcdom;

use core::cell::RefCell;
use futures::{StreamExt, stream};
use html5ever::{local_name, ParseOpts, parse_document, LocalNameStaticSet};
use html5ever::tree_builder::TreeBuilderOpts;
use html5ever::serialize::{SerializeOpts, serialize};
use html5ever::tendril::TendrilSink;
use rcdom::{RcDom, NodeData, Handle};
use markup5ever::interface::Attribute;
use string_cache::Atom;

use crate::data::{Card, SimpleCardList};
use crate::fetch::{FetchError, FetchResult, ListRetriever};

// ---- Interface ---- //

pub type DeckboxList = String;

pub struct DeckboxFetcher {}

pub async fn get_deck_html(deck_id: &str) -> Result<String, FetchError> {
    // @Todo - no making a new Client for every request >:(
    let client = reqwest::Client::new();
    let url = format!("https://deckbox.org/sets/{}", deck_id);
    println!("Sending request!");
    let res = client
        .get(&url)
        .body("")
        .send()
        .await.map_err(|err| {
            FetchError::RetrievalError(Box::new(err))
        })?
        .text().await.map_err(|err| {
            FetchError::DataParseError(Box::new(err))
        });
    res
    // <table class='set_cards with_details simple_table' id='set_cards_table_details'>
}

fn get_by_attr<'a>(handle: Handle, name: &Atom<LocalNameStaticSet>, value: &str) -> Option<Handle> {
    // @Todo - ensure this algorithm doesn't involve cloning the underlying data
    match handle.data {
        NodeData::Element { ref attrs, .. } => {
            for attr in attrs.borrow().iter() {
                if attr.name.local == *name && attr.value.to_string() == value {
                    return Some(handle.clone());
                }
            }
        },
        _ => (),
    }
    for child in handle.children.borrow().iter() {
        let opt_res = get_by_attr(child.clone(), name, value);
        if opt_res.is_some() {
            return opt_res;
        }
    }
    return None;
}

fn html_to_simple_card_list(db_list: &DeckboxList, html: String) -> FetchResult {
    let opts = ParseOpts {
        tree_builder: TreeBuilderOpts {
            drop_doctype: true,
            ..Default::default()
        },
        ..Default::default()
    };
    let dom = parse_document(RcDom::default(), opts)
        .from_utf8().read_from(&mut html.as_bytes()).unwrap();
    let Some(card_table_handle) = get_by_attr(dom.document, &local_name!("id"), "set_cards_table_details") else {
        panic!("Failed to find table of cards"); // @Todo - handle this error instead
    };
    println!("{:?}", card_table_handle);
    let mut cards: Vec<String> = vec![];
    let card_handles = card_table_handle.children.borrow();
    let mut card_handles_iter = card_handles.iter();
    card_handles_iter.next(); // Skip the header and empty row
    card_handles_iter.next();
    for child in card_handles_iter {
        println!("{:?}", child);
        let Some(card_entry) = get_by_attr(child.clone(), &local_name!("class"), "simple") else {
            continue;
        };
        match card_entry.data {
            NodeData::Text { ref contents } => {
                cards.push(contents.borrow().to_string());
            },
            _ => (),
        }
    }
    println!("{:?}", cards);
    Ok(SimpleCardList::with_current_time(vec![]))
}

pub fn get_decks<'a>(
    card_lists: impl Iterator<Item = &'a DeckboxList>,
) -> Vec<(&'a DeckboxList, FetchResult)> {
    // Retrieve decks from Deckbox in an async fashion for performance
    let stream = stream::unfold(card_lists.into_iter(), |mut deck_ids_iter| async {
        let deck_id = deck_ids_iter.next()?;
        let net_resp = get_deck_html(&deck_id).await;
        Some(((deck_id, net_resp), deck_ids_iter))
    });
    let runtime = tokio::runtime::Builder::new_current_thread().enable_io().enable_time().build().unwrap();
    let results = runtime.block_on(stream.collect::<Vec<(&DeckboxList, Result<String, FetchError>)>>());

    // Convert html/error results into output format
    results.into_iter().map(|(list, res_html)| match res_html {
        Ok(html) => (list, html_to_simple_card_list(list, html)),
        Err(err) => {
            (list, Err(err))
        },
    }).collect::<Vec<(&DeckboxList, Result<SimpleCardList, FetchError>)>>()
}

impl ListRetriever<DeckboxList> for DeckboxFetcher {
    fn fetch<'a>(
        list: impl Iterator<Item = &'a DeckboxList>,
    ) -> Vec<(&'a DeckboxList, FetchResult)> {
        get_decks(list)
    }
}
