pub mod moxfield;

use crate::data::SimpleCardList;
use crate::fetch::moxfield::MoxfieldList;
use anyhow::Result;

pub enum CardListSource {
    Moxfield(MoxfieldList),
}

pub trait ListRetriever<S> {
    fn fetch(list: impl Iterator<Item = S>) -> Vec<(S, Result<SimpleCardList>)>;
}
