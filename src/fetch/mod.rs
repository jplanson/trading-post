mod moxfield;

use crate::data::SimpleCardList;
use anyhow::Result;

pub use crate::fetch::moxfield::{MoxfieldBoard, MoxfieldFetcher, MoxfieldList};

pub enum CardListSource {
    Moxfield(MoxfieldList),
}

pub trait ListRetriever<S> {
    fn fetch<'a>(list: impl Iterator<Item = &'a S>) -> Vec<(&'a S, Result<SimpleCardList>)>;
}
