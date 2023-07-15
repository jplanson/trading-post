use html_parser::{Dom, Node};
use futures::{StreamExt, stream};

use crate::data::{Card, SimpleCardList};
use crate::fetch::{FetchError, FetchResult, ListRetriever};

// ---- Interface ---- //

pub type DeckboxList = String;

pub struct DeckboxFetcher {}

pub async fn get_deck_html(deck_id: &str) -> Result<String, FetchError> {
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
    println!("Got res!");
    res
    // <table class='set_cards with_details simple_table' id='set_cards_table_details'>
}

fn html_to_simple_card_list(db_list: &DeckboxList, html: String) -> FetchResult {
    println!("Now to parse db_list");
    println!("HTML: {}", &html);
    // let parsed = Dom::parse(html.as_str()).map_err(|err| FetchError::DataParseError(Box::new(err)))?;
    // println!("Parsed db list");
    // for child in parsed.children {
    //     match child {
    //         Node::Element(x) => println!("{:?}", x.id),
    //         _ => (),
    //     };
    // }
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
