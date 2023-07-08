use std::collections::HashMap;

use crate::data::{CardList, ListContext, SimpleCardList};
use crate::fetch::{CardListSource, ListRetriever, MoxfieldFetcher, MoxfieldList};
use anyhow::{anyhow, Result};

pub struct CardListRequest {
    pub context: ListContext,
    pub source: CardListSource,
}

pub fn get_lists(requests: Vec<CardListRequest>) -> Vec<CardList> {
    let mut mox_requests: Vec<MoxfieldList> = vec![];
    for req in requests.iter() {
        match &req.source {
            CardListSource::Moxfield(ml) => mox_requests.push(ml.clone()),
        };
    }
    let mox_results = MoxfieldFetcher::fetch(mox_requests.iter())
        .into_iter()
        .collect::<HashMap<&MoxfieldList, Result<SimpleCardList>>>();
    let cards = requests
        .into_iter()
        .filter_map(|req| {
            let res = match req.source {
                CardListSource::Moxfield(ml) => mox_results.get(&ml).unwrap().clone(),
            };
            match res {
                Ok(x) => Some(CardList {
                    context: req.context,
                    data: x.clone(),
                }),
                Err(x) => {
                    println!("{:?}", x);
                    None
                }
            }
        })
        .collect::<Vec<CardList>>();
    cards
}
