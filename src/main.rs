mod core;
mod data;
mod fetch;

#[macro_use]
extern crate html5ever;

use crate::core::{get_lists, CardListRequest};
use crate::data::{CardListType, ListContext};
use crate::fetch::{CardListSource, MoxfieldBoard, MoxfieldList};
use anyhow::Result;

#[allow(unused_variables)]
fn main() -> Result<()> {
    let lc1 = ListContext {
        user: String::from("Spacewalk"),
        category: CardListType::TradeBinder,
    };
    let lc2 = ListContext {
        user: String::from("Spacewalk"),
        category: CardListType::WishList,
    };
    let cs2 = CardListSource::Moxfield(MoxfieldList {
        deck_id: String::from("n9-ZrMnGIU2mLoND3UyZvQ"),
        boards: vec![MoxfieldBoard::Main],
    });
    let cs3 = CardListSource::Moxfield(MoxfieldList {
        deck_id: String::from("5n4958HbEEG7m-27geLYtQ"),
        boards: vec![MoxfieldBoard::Main],
    });
    let cs4 = CardListSource::Deckbox(String::from("2948938"));

    let cr1 = CardListRequest {
        context: lc1,
        source: cs4,
    };
    let cr2 = CardListRequest {
        context: lc2,
        source: cs3,
    };

    let card_lists = get_lists(vec![cr1]); // cr2]);
    for cl in card_lists.iter() {
        println!(
            "USER: {}, NUM CARDS: {}",
            cl.context.user,
            cl.data.cards.len()
        );
        // println!("{}", cl.data.cards.join("\n"));
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
