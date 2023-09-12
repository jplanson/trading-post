extern crate markup5ever_rcdom as rcdom;

use futures::{StreamExt, stream};
use html5ever::{local_name, ParseOpts, parse_document, LocalNameStaticSet};
use html5ever::tree_builder::TreeBuilderOpts;
use html5ever::tendril::TendrilSink;
use rcdom::{RcDom, NodeData, Handle};
use markup5ever::interface::{Attribute, QualName};
use string_cache::Atom;
use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashSet;

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

type WalkFn<'f> = &'f dyn Fn(usize, Handle) -> bool;
type WalkEltFn<'f> = &'f dyn Fn(usize, &QualName, &RefCell<Vec<Attribute>>) -> bool;
type WalkTextFn<'f> = &'f dyn Fn(usize, String) -> bool;

fn walk(handle: Handle, depth: usize, stop: WalkFn<'_>) -> Option<Handle> {
    if stop(depth, Rc::clone(&handle)) {
        return Some(handle)
    }
    for child in handle.children.borrow().iter() {
        if let Some(handle) = walk(Rc::clone(child), depth + 1, stop) {
            return Some(handle);
        }
    }
    None
}

fn walk_elements(handle: Handle, stop: WalkEltFn<'_>) -> Option<Handle> {
    walk(handle, 0, &|d, h: Handle | {
        if let NodeData::Element {
            ref name,
            ref attrs,
            ..
        } = h.data
        {
            return stop(d, name, attrs);
        }
        false
    })
}

fn walk_text(handle: Handle, depth: usize, stop: WalkTextFn<'_>) -> Option<Handle> {
    walk(handle, 0, &|d, h: Handle | {
        if let NodeData::Text {
            ref contents
        } = h.data
        {
            return stop(d, contents.borrow().to_string());
        }
        false
    })
}

fn get_by_name(handle: Handle, srch_name: Atom<LocalNameStaticSet>) -> Option<Handle> {
    walk_elements(handle, &|_d, qn: &QualName, _vs| qn.local == srch_name)
}

fn get_by_attr(handle: Handle, name: Atom<LocalNameStaticSet>, value: &str) -> Option<Handle> {
    walk_elements(handle, &|_d, _qn, vs| {
        for attr in vs.borrow().iter() {
            if attr.name.local == name && attr.value.to_string() == value {
                return true;
            }
        }
        false
    })
}

fn get_by_child_text(handle: Handle, comp_str: &str) -> Option<Handle> {
    walk(handle, 0, &|_d, h| {
        if let NodeData::Element {..} = h.data {
            for child in h.children.borrow().iter() {
                let child_handle = Rc::clone(child);
                if let NodeData::Text { ref contents } = child_handle.data {
                    return contents.borrow().to_string() == comp_str;
                }
            }
        }
        false
    })
}

fn get_by_text(handle: Handle, comp_str: &str) -> Option<Handle> {
    walk_text(handle, 0, &|_d, s| s.as_str() == comp_str)
}


fn pretty_print(handle: Handle) -> () {
    walk_elements(handle, &|d, qn, _vs| {
        println!("{}{}", "  ".repeat(d), qn.local);
        false
    });
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
    // let document = Rc::clone(&dom.document);
    let Some(card_table_handle) = get_by_attr(Rc::clone(&dom.document), local_name!("id"), "set_cards_table_details") else {
        panic!("Failed to find table of cards"); // @Todo - handle this error instead
    };
    let Some(tbody) = get_by_name(card_table_handle, local_name!("tbody")) else {
        panic!("Failed to find tbody");
    };
    let mut cards: HashSet<String> = HashSet::<String>::new();
    for (i, child) in tbody.children.borrow().iter().enumerate() {
        if i == 0 { // Skip header
            continue;
        }
        if let Some(card_a_elt) = get_by_attr(Rc::clone(child), local_name!("class"), "simple") {
            let a_children = card_a_elt.children.borrow();
            if a_children.len() != 1 {
                continue;
            }
            if let NodeData::Text {
                ref contents
            } = a_children.get(0).unwrap().data {
                cards.insert(contents.borrow().to_string());
            };
        };
    }
    println!("{:?}", cards);
    // Find if next button exists, and get URL for next page
    let Some(control_div) = get_by_attr(Rc::clone(&dom.document), local_name!("class"), "pagination_controls") else {
        panic!("Couldn't find pagination controls")
    };
    let mut next_path: Option<String> = None;
    if let Some(next_href) = get_by_child_text(control_div, "Next") {
        if let NodeData::Element {
            ref attrs,
            ..
        } = next_href.data {
            for attr in attrs.borrow().iter() {
                if attr.name.local == local_name!("href") {
                    next_path = Some(attr.value.to_string());
                }
            }
        }
    }
    println!("{:?}", next_path);
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
