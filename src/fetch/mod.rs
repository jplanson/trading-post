mod deckbox;
mod moxfield;

use crate::data::SimpleCardList;
use std::error;

pub use crate::fetch::deckbox::{DeckboxFetcher, DeckboxList};
pub use crate::fetch::moxfield::{MoxfieldBoard, MoxfieldFetcher, MoxfieldList};

#[derive(Debug)]
pub enum FetchError {
    RetrievalError(Box<dyn error::Error>),
    DataParseError(Box<dyn error::Error>),
}

pub enum CardListSource {
    Moxfield(MoxfieldList),
    Deckbox(DeckboxList),
}

pub type FetchResult = Result<SimpleCardList, FetchError>;

pub trait ListRetriever<S> {
    fn fetch<'a>(list: impl Iterator<Item = &'a S>) -> Vec<(&'a S, FetchResult)>;
}
