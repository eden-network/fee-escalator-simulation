mod quoter;
mod order_book;
mod connector;
mod utils;
mod market;

pub use quoter::BinanceQuoter;
pub use market::Market;

use std::{
    collections::HashMap,
    sync::{Mutex, Arc}
};
use eyre::Result;
use order_book::BinanceOrderBook;


type MarketTicker = String;
type OrderBooksShared = Arc<HashMap<String, Arc<Mutex<(BinanceOrderBook,)>>>>;


#[derive(Clone, Copy)]
enum RefreshRate {
    Fast = 100, 
    Slow = 1000,
}

impl TryFrom<u32> for RefreshRate {
    type Error = &'static str;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            100 => Ok(Self::Fast),
            1000 => Ok(Self::Slow),
            _ => Err("Invalid refresh rate"),
        }
    }
}

#[derive(Clone, Copy)]
enum BookSize {
    Five = 5,
    Ten = 10,
    Twenty = 20,
    
}

impl TryFrom<u8> for BookSize {
    type Error = &'static str;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            5 => Ok(Self::Five),
            10 => Ok(Self::Ten),
            20 => Ok(Self::Twenty),
            _ => Err("Invalid book size"),
        }
    }
}

pub mod suppported_markets {
    use super::Market;

    // Supported markets
    pub const ETHUSDT: Market = Market("ETH", "USDT");
    pub const BTCUSDT: Market = Market("BTC", "USDT");
}

