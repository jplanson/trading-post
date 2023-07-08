mod data;
mod fetch;
mod glue;

use crate::data::{CardListType, ListContext};
use crate::fetch::{CardListSource, ListRetriever, MoxfieldBoard, MoxfieldFetcher, MoxfieldList};
use crate::glue::{get_lists, CardListRequest};
use anyhow::Result;
use std::collections::HashSet;

fn main() -> Result<()> {
    let lc1 = ListContext {
        user: String::from("Spacewalk"),
        category: CardListType::TradeBinder,
    };
    let lc2 = ListContext {
        user: String::from("Spacewalk"),
        category: CardListType::WishList,
    };
    let cs1 = CardListSource::Moxfield(MoxfieldList {
        deck_id: String::from("n9-ZrMnGIU2mLoND3UyZvQ"),
        boards: vec![MoxfieldBoard::Main, MoxfieldBoard::Side],
    });
    let cs2 = CardListSource::Moxfield(MoxfieldList {
        deck_id: String::from("n9-ZrMnGIU2mLoND3UyZvQ"),
        boards: vec![MoxfieldBoard::Maybe],
    });

    let cr1 = CardListRequest {
        context: lc1,
        source: cs1,
    };
    let cr2 = CardListRequest {
        context: lc2,
        source: cs2,
    };

    let card_lists = get_lists(vec![cr1, cr2]);
    for cl in card_lists.iter() {
        println!(
            "USER: {}, NUM CARDS: {}",
            cl.context.user,
            cl.data.cards.len()
        );
    }

    // let deck_ids = vec![MoxfieldList::basic("n9-ZrMnGIU2mLoND3UyZvQ")];
    // let res = MoxfieldFetcher::fetch(deck_ids.iter());
    // for (mx_list, res_card) in res.into_iter() {
    //     match res_card {
    //         Ok(x) => println!("{}", x.cards.join("\n")),
    //         Err(x) => println!("{}", x),
    //     };
    // }
    Ok(())
}
