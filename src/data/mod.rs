use anyhow::Result;
use chrono::{DateTime, Utc};
use std::collections::HashSet;

pub struct Card {
    name: String,
}

pub struct User {
    id: String,
    username: String,
}

pub enum CardListType {
    TradeBinder,
    WishList,
}

pub struct CardList<'a> {
    user: &'a User,
    last_updated: DateTime<Utc>,
    category: CardListType,
    cards: HashSet<&'a Card>,
}

pub trait TradeSearcher {
    fn init() -> Result<()>;
    fn find_users_with_card(
        card: &Card,
        list_type: Option<CardListType>,
    ) -> Result<HashSet<&CardList>>;
}
