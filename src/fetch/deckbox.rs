extern crate markup5ever_rcdom as rcdom;

use std::fmt;
use futures::{StreamExt, stream};
use html5ever::{local_name, ParseOpts, parse_document, LocalNameStaticSet};
use html5ever::tree_builder::TreeBuilderOpts;
use html5ever::tendril::TendrilSink;
use rcdom::{RcDom, NodeData, Handle};
use markup5ever::interface::{Attribute, QualName};
use string_cache::Atom;
use std::rc::Rc;
use std::cell::RefCell;
use std::error::Error;
use std::collections::HashSet;
use lazy_static::lazy_static;

use crate::data::{Card, SimpleCardList};
use crate::fetch::{FetchError, FetchResult, ListRetriever};

// ---- Interface ---- //

pub type DeckboxList = String;

pub struct DeckboxFetcher {}

// ---- DOM Traversal ---- //

// @Cleanup - could be a separate utility at some point

type WalkFn<'f> = &'f dyn Fn(usize, Handle) -> bool;
type WalkEltFn<'f> = &'f dyn Fn(usize, &QualName, &RefCell<Vec<Attribute>>) -> bool;
#[allow(dead_code)]
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

#[allow(dead_code)]
fn walk_text(handle: Handle, _depth: usize, stop: WalkTextFn<'_>) -> Option<Handle> {
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

#[allow(dead_code)]
fn get_by_text(handle: Handle, comp_str: &str) -> Option<Handle> {
    walk_text(handle, 0, &|_d, s| s.as_str() == comp_str)
}


#[allow(dead_code)]
fn pretty_print(handle: Handle) {
    walk_elements(handle, &|d, qn, _vs| {
        println!("{}{}", "  ".repeat(d), qn.local);
        false
    });
}

// ---- Logic ---- //

#[derive(Debug)]
struct DeckboxHtmlParseError {
    key: String,
    value: String,
}

impl fmt::Display for DeckboxHtmlParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Failed to find element with key-value pair '{}':'{}'", &self.key, &self.value)
    }
}

impl Error for DeckboxHtmlParseError {}

lazy_static! {
    static ref PARSE_OPTS: ParseOpts = ParseOpts {
        tree_builder: TreeBuilderOpts {
            drop_doctype: true,
            ..Default::default()
        },
        ..Default::default()
    };
}

static CARD_TABLE_ID : &str = "set_cards_table_details";
static PAGE_CTRLS_CLASS : &str = "pagination_controls";

fn get_cards_from_html(html: String) -> Result<(Vec<Card>, Option<String>), FetchError> {

    // Parse `htmt` to an object
    let res_dom = parse_document(RcDom::default(), PARSE_OPTS.clone())
        .from_utf8().read_from(&mut html.as_bytes());
    let dom = match res_dom {
        Ok(x) => x,
        Err(x) => return Err(FetchError::DataParseError(Box::new(x))),
    };

    // Find table of cards by `id` attr
    let Some(card_table_handle) = get_by_attr(Rc::clone(&dom.document), local_name!("id"), CARD_TABLE_ID) else {
        return Err(FetchError::DataParseError(Box::new(DeckboxHtmlParseError {key: "id".to_owned(), value: CARD_TABLE_ID.to_owned()})));
    };
    let Some(tbody) = get_by_name(card_table_handle, local_name!("tbody")) else {
        return Err(FetchError::DataParseError(Box::new(DeckboxHtmlParseError {key: "name".to_owned(), value: "tbody".to_owned()})));
    };

    // Retrieve a card entry from each row of the table
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
        // @Todo - else: print a warning in debug mode?
    }

    // If "next" button exists, access to get URL subpath for next page
    let Some(control_div) = get_by_attr(Rc::clone(&dom.document), local_name!("class"), PAGE_CTRLS_CLASS) else {
        return Err(FetchError::DataParseError(Box::new(DeckboxHtmlParseError {key: "class".to_owned(), value: PAGE_CTRLS_CLASS.to_owned()})));
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
    Ok((cards.into_iter().collect::<Vec<String>>(), next_path))
}

pub async fn get_deck(db_list: &DeckboxList) -> FetchResult {

    let mut cards: Vec<String> = vec![];

    // @Todo - no making a new Client for every request >:(
    let client = reqwest::Client::new();

    let mut opt_url = Some(format!("https://deckbox.org/sets/{}", db_list));
    while let Some(url) = opt_url {

        // Retrieve HTML for page
        let page = client
            .get(&url)
            .body("")
            .send()
            .await.map_err(|err| {
                FetchError::RetrievalError(Box::new(err))
            })?
            .text().await.map_err(|err| {
                FetchError::DataParseError(Box::new(err))
            })?;

        // Try extracting card info and URL for next page from page
        let (mut page_cards, next_page_subpath) = get_cards_from_html(page)?;
        cards.append(&mut page_cards);
        // If no "Next" button found, opt_url is None and we break from the loop
        opt_url = next_page_subpath.map(|s| format!("https://deckbox.org{}", s));
    }

    Ok(SimpleCardList::with_current_time(cards))
}

pub fn get_decks<'a>(
    card_lists: impl Iterator<Item = &'a DeckboxList>,
) -> Vec<(&'a DeckboxList, FetchResult)> {
    // Retrieve decks from Deckbox in an async fashion for performance
    let stream = stream::unfold(card_lists.into_iter(), |mut deck_ids_iter| async {
        let deck_id = deck_ids_iter.next()?;
        let fetch_res = get_deck(deck_id).await;
        Some(((deck_id, fetch_res), deck_ids_iter))
    });
    let runtime = tokio::runtime::Builder::new_current_thread().enable_io().enable_time().build().unwrap();
    runtime.block_on(stream.collect::<Vec<(&DeckboxList, FetchResult)>>())
}

impl ListRetriever<DeckboxList> for DeckboxFetcher {
    fn fetch<'a>(
        list: impl Iterator<Item = &'a DeckboxList>,
    ) -> Vec<(&'a DeckboxList, FetchResult)> {
        get_decks(list)
    }
}
