use anyhow::Result;
use chrono::{DateTime, Utc};
use std::collections::HashSet;

pub type Card = String;

#[derive(Clone)]
pub struct SimpleCardList {
    pub last_updated: DateTime<Utc>,
    pub cards: Vec<Card>,
}

impl SimpleCardList {
    pub fn with_current_time(cards: Vec<Card>) -> Self {
        SimpleCardList {
            last_updated: chrono::Utc::now(),
            cards,
        }
    }
}

pub enum CardListType {
    TradeBinder,
    WishList,
}

pub struct ListContext {
    pub user: String,
    pub category: CardListType,
}

pub struct CardList {
    pub context: ListContext,
    pub data: SimpleCardList,
}

pub trait TradeSearcher {
    fn init() -> Result<()>;
    fn find_users_with_card(
        card: &Card,
        list_type: Option<CardListType>,
    ) -> Result<HashSet<&CardList>>;
}
