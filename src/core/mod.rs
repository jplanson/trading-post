use std::collections::HashMap;

use crate::data::{CardList, ListContext, SimpleCardList};
use crate::fetch::{
    CardListSource, DeckboxFetcher, DeckboxList, FetchResult, ListRetriever, MoxfieldFetcher,
    MoxfieldList,
};

pub struct CardListRequest {
    pub context: ListContext,
    pub source: CardListSource,
}

pub fn get_lists(requests: Vec<CardListRequest>) -> Vec<CardList> {
    let mut mox_requests: Vec<MoxfieldList> = vec![];
    let mut db_requests: Vec<DeckboxList> = vec![];
    for req in requests.iter() {
        match &req.source {
            CardListSource::Moxfield(ml) => mox_requests.push(ml.clone()),
            CardListSource::Deckbox(dl) => db_requests.push(dl.clone()),
        };
    }
    let mut mox_results = MoxfieldFetcher::fetch(mox_requests.iter())
        .into_iter()
        .collect::<HashMap<&MoxfieldList, FetchResult>>();
    let mut db_results = DeckboxFetcher::fetch(db_requests.iter())
        .into_iter()
        .collect::<HashMap<&DeckboxList, FetchResult>>();
    requests
        .into_iter()
        .filter_map(|req| {
            let res = match req.source {
                CardListSource::Moxfield(ml) => mox_results.remove(&ml).unwrap(),
                CardListSource::Deckbox(db) => db_results.remove(&db).unwrap(),
            };
            match res {
                Ok(x) => Some(CardList {
                    context: req.context,
                    data: x,
                }),
                Err(x) => {
                    // @Todo - better error handling
                    println!("{:?}", x);
                    None
                }
            }
        })
        .collect::<Vec<CardList>>()
}
