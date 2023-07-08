mod data;
mod fetch;

use crate::fetch::moxfield::{get_decks, MoxfieldList};
use anyhow::Result;
use std::collections::HashSet;

fn main() -> Result<()> {
    let deck_ids = vec![MoxfieldList::basic("n9-ZrMnGIU2mLoND3UyZvQ")];
    let res = get_decks(deck_ids.into_iter());
    for (mx_list, res_card) in res.into_iter() {
        match res_card {
            Ok(x) => println!("{}", x.into_iter().map(|y| y.name).collect::<Vec<String>>().join("\n")),
            Err(x) => println!("{}", x),
        };
    }
    Ok(())
}
