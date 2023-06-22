// todo: streaming binance order book + fetch price + do snapshot of order book
// todo: split this between: stream, book and main
// todo: consider using bignum library for precision

use tokio_tungstenite::{connect_async, tungstenite::protocol::Message, MaybeTlsStream};
use websocket::futures::{Stream, Sink};
use futures::{stream::{StreamExt}, sink::{SinkExt}};
use tokio::net::TcpStream;
use std::time::Duration;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Mutex, Arc};
use tokio_tungstenite::WebSocketStream;
use eyre::Result;
use lazy_static::lazy_static;
use regex::Regex;

const STREAM_BASE_ENDPOINT: &str = "wss://stream.binance.com:9443";
lazy_static! {
    pub static ref BOOK_STREAM_KEY_REGEX: Regex = Regex::new(r"[a-z]+@depth[0-9]+@[0-9]*ms").unwrap();
}

type MarketTicker = String;
type OrderBooksShared = Arc<HashMap<String, Arc<Mutex<(BinanceOrderBook,)>>>>;

// todo: should this be side(bid, ask)?
enum SwapType {
    Buy,
    Sell,
}

#[derive(Clone, Copy)]
enum RefreshRate {
    Fast = 100, 
    Slow = 1000,
}

#[derive(Clone, Copy)]
enum BookSize {
    Five = 5,
    Ten = 10,
    Twenty = 20,
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

struct BinanceAdapter {
    stream_base_endpoint: String,
    market_tickers: Vec<String>,
    book_size: BookSize,
    refresh_rate_ms: RefreshRate,
    order_books: Arc<HashMap<String, Arc<Mutex<(BinanceOrderBook,)>>>>,
    last_update_tm: u64,
}

#[derive(Debug, Clone)]
struct BinanceOrderBook {
    bids: Vec<Tick>,
    asks: Vec<Tick>,
}

impl BinanceOrderBook {

    fn query_exact_base(
        &self, 
        swap_type: SwapType, 
        base_amount: f64
    ) -> (f64, f64) {
        let book_side = if let SwapType::Sell = swap_type { 
            &self.bids 
        } else {
            &self.asks
        };

        let mut base_left = base_amount;
        let mut quote_used = 0.;
        for order in book_side.iter() {
            let base_fill = order.qty.min(base_left);
            let quote_fill = base_fill * order.price;
            base_left -= base_fill;
            quote_used += quote_fill;

            if base_left == 0. {
                break;
            }
        }
        let base_used = base_amount - base_left;
        (base_used, quote_used)
    }

    fn query_exact_quote(
        &self, 
        swap_type: SwapType,
        quote_amount: f64
    ) -> (f64, f64) {
        let book_side = if let SwapType::Sell = swap_type { 
            &self.bids 
        } else {
            &self.asks 
        };

        let mut quote_left = quote_amount;
        let mut base_used = 0.;
        for order in book_side.iter() {
            let quote_fill = quote_left.min(order.qty * order.price);
            let base_fill = quote_fill / order.price;
            quote_left -= quote_fill;
            base_used += base_fill;

            if quote_left == 0. {
                break;
            }
        }
        let quote_used = quote_amount - quote_left;
        (quote_used, base_used) 
    }

}

impl Default for BinanceOrderBook {
    fn default() -> Self {
        Self {
            bids: vec![],
            asks: vec![],
        }
    }
}

impl std::fmt::Display for BinanceOrderBook {

    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // todo: use tui create
        // todo: add quantity to slippage graph (maybe)
        // todo: add visual for tick quantity (maybe)
        let red_color = "\x1b[0;31m";
        let green_color = "\x1b[0;32m";
        let no_color = "\x1b[0m";

        let (min_price_w, min_qty_w) = {
            let dec_w = 3;
            let max_ask_price = self.asks.last().map(|t| t.price).unwrap_or_default();
            let max_bid_price = self.bids.first().map(|t| t.price).unwrap_or_default();
            let min_price_w = (max_ask_price.max(max_bid_price) as i32).to_string().len();
            
            let max_ask_qty = self.asks.iter().map(|t| t.qty).max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap_or_default();
            let max_bid_qty = self.bids.iter().map(|t| t.qty).max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap_or_default();
            let min_qty_w = (max_ask_qty.max(max_bid_qty) as i32).to_string().len();

            (min_price_w + dec_w, min_qty_w + dec_w)
        };
        let border_width = 1;
        let price_width = min_price_w + border_width;
        let qty_width = min_qty_w + border_width;

        let mut book_str = String::new();
        book_str.push_str("Asks:\n");
        for ask in self.asks.iter().rev() {
            book_str.push_str(
                &format!("\t{red_color}{0:>1$.2} @ {2:>3$.2}{no_color}\n", 
                    ask.price, price_width, ask.qty, qty_width
                )
            );
        }
        book_str.push_str("Bids:\n");
        for bid in self.bids.iter() {
            book_str.push_str(
                &format!("\t{green_color}{0:>1$.2} @ {2:>3$.2}{no_color}\n", 
                    bid.price, price_width, bid.qty, qty_width
                )
            );
        }
        write!(f, "{}", book_str)
    }

}

#[derive(Debug, Clone)]
struct Tick {
    qty: f64,
    price: f64,
}

impl Tick {
    fn new(price: f64, qty: f64) -> Self {
        Self { price: price, qty: qty }
    }
}

impl TryFrom<BinanceOrderBookUpdateData> for BinanceOrderBook {
    type Error = eyre::Report;

    fn try_from(value: BinanceOrderBookUpdateData) -> Result<Self, Self::Error> {
        let parse_side = |side: Vec<Vec<String>>| -> Result<Vec<Tick>> {
            side.iter().map(|bid| {
                Ok(Tick::new(
                    bid[0].parse::<f64>()?, 
                    bid[1].parse::<f64>()?
                ))
            } as Result<Tick>).collect::<Result<Vec<Tick>>>()
        };

        Ok(Self {
            bids: parse_side(value.bids)?,
            asks: parse_side(value.asks)?
        })
    }
}

#[derive(serde::Deserialize, Debug)]
struct BinanceOrderBookUpdate {
    stream: String,
    data: BinanceOrderBookUpdateData,
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct BinanceOrderBookUpdateData {
    last_update_id: u64,
    bids: Vec<Vec<String>>, // sorted desc
    asks: Vec<Vec<String>>, // sorted asc
}

impl BinanceAdapter {

    pub fn new(
        stream_base_endpoint: String,
        market_tickers: Vec<MarketTicker>,
        book_size: u8,
        refresh_rate_ms: u32,
    ) -> Self {
        let book_size = book_size.try_into().expect("Invalid book size");
        let refresh_rate_ms = refresh_rate_ms.try_into().expect("Invalid refresh rate");

        let mut order_books = HashMap::new();
        for market_ticker in market_tickers.iter() {
            order_books.insert(
                market_ticker.clone(), 
                Arc::new(Mutex::new((BinanceOrderBook::default(),)))
            );
        }
        Self {
            order_books: Arc::new(order_books),
            market_tickers: market_tickers.clone(),
            stream_base_endpoint,
            last_update_tm: 0,
            refresh_rate_ms,
            book_size,
        }
    }

    fn start_stream(&self) {
        tokio::spawn(Self::_start_stream(
                self.stream_base_endpoint.clone(),
                self.market_tickers.clone(),
                self.book_size,
                self.refresh_rate_ms,
                self.order_books.clone()
            )
        );
    }

    fn get_book(&self, market: MarketTicker) -> Result<BinanceOrderBook> {
        let book = self.order_books.get(&market).unwrap();
        let book = book.lock().unwrap().0.clone();
        Ok(book)
    }

    fn get_epoch_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap().as_millis() as u64
    }

}


#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_binance_stream() {
        let adapter = BinanceAdapter::new(
            String::from("wss://stream.binance.com:9443"),
            vec!["ethusdt".to_string(), "btcusdt".to_string()],
            20,
            100,
        );
        adapter.start_stream();

        let mut interval = tokio::time::interval(std::time::Duration::from_millis(100));
        loop {
            interval.tick().await;
            println!("{}", adapter.get_book("ethusdt".to_string()).unwrap());
        }
        
    }

    #[test]
    fn test_query_exact_base_sell() {
        let book = BinanceOrderBook {
            bids: vec![
                Tick::new(1890., 1.),
                Tick::new(1889., 0.21),
                Tick::new(1888., 3.22),
            ],
            asks: vec![]
        };
        let base_amount = 5.;
        let avl_bid_qty = 4.43;
        let target_out = 8366.05;
        let (base_used, quote_out) = book.query_exact_base(
            SwapType::Sell, 
            base_amount
        );
        assert_eq!(base_used, avl_bid_qty);
        assert_eq!(quote_out, target_out);
    }

    #[test]
    fn test_query_exact_base_buy() {
        let book = BinanceOrderBook {
            bids: vec![],
            asks: vec![
                Tick::new(1888., 3.22),
                Tick::new(1889., 0.21),
                Tick::new(1890., 1.),
            ]
        };
        let base_amount = 5.;
        let avl_ask_qty = 4.43;
        let target_out = 8366.05;
        let (base_used, quote_out) = book.query_exact_base(
            SwapType::Buy, 
            base_amount
        );
        assert_eq!(base_used, avl_ask_qty);
        assert_eq!(quote_out, target_out);
    }

    #[test]
    fn test_query_exact_quote_sell() {
        let book = BinanceOrderBook {
            bids: vec![
                Tick::new(1890., 1.),
                Tick::new(1889., 0.21),
                Tick::new(1888., 3.22),
            ],
            asks: vec![]
        };
        let quote_amount = 9000.;
        let avl_bid_qty_quote = 8366.05;
        let target_out = 4.43;
        let (quote_used, base_out) = book.query_exact_quote(
            SwapType::Sell, 
            quote_amount
        );
        assert_eq!(quote_used, avl_bid_qty_quote);
        assert_eq!(base_out, target_out);
    }

    #[test]
    fn test_query_exact_quote_buy() {
        let book = BinanceOrderBook {
            bids: vec![],
            asks: vec![
                Tick::new(1888., 3.22),
                Tick::new(1889., 0.21),
                Tick::new(1890., 1.),
            ]
        };
        let quote_amount = 9000.;
        let avl_ask_qty_quote = 8366.05;
        let target_out = 4.43;
        let (quote_used, base_out) = book.query_exact_quote(
            SwapType::Buy, 
            quote_amount
        );
        let allowed_err = 0.00001;
        assert!(quote_used-avl_ask_qty_quote < allowed_err);
        assert!(base_out-target_out < allowed_err);
    }
}