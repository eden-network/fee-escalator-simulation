use futures::{stream::StreamExt, sink::SinkExt};
use tokio::net::TcpStream;
use lazy_static::lazy_static;
use regex::Regex;
use tokio_tungstenite::{
    tungstenite::protocol::Message, 
    WebSocketStream,
    MaybeTlsStream,
    connect_async, 
};

// todo: panic if there is no update for X seconds

use super::*;


lazy_static! {
    pub static ref BOOK_STREAM_KEY_REGEX: Regex = Regex::new(r"[a-z]+@depth[0-9]+@[0-9]*ms").unwrap();
}

#[derive(serde::Deserialize, Debug)]
pub struct BinanceAPIOrderBookUpdate {
    stream: String,
    data: BinanceAPIOrderBookUpdateData,
}

#[derive(serde::Deserialize, Debug)]
pub struct BinanceAPIOrderBookUpdateData {
    pub e: String, // event type
    pub E: u64, // event time
    pub s: String, // market ticker
    pub U: u64, // first update ID in event
    pub u: u64, // final update ID in event
    pub b: Vec<Vec<String>>, // bids to be updated (sorted descending)
    pub a: Vec<Vec<String>>, // asks to be updated (sorted ascending)
}

// #[derive(serde::Deserialize, Debug)]
// pub struct BinanceAPIOrderBook {
//     stream: String,
//     data: BinanceAPIOrderBookData,
// }

// #[derive(serde::Deserialize, Debug)]
// #[serde(rename_all = "camelCase")]
// pub struct BinanceAPIOrderBookData {
//     last_update_id: u64,
//     pub bids: Vec<Vec<String>>, // sorted desc
//     pub asks: Vec<Vec<String>>, // sorted asc
// }

pub(super) async fn start_stream(
    stream_base_endpoint: &str, 
    market_tickers: Vec<MarketTicker>,
    refresh_rate_ms: RefreshRate,
    books: OrderBooksShared
) -> Result<()> {
    let mut stream = connect(
        stream_base_endpoint, 
        market_tickers.clone(), 
        refresh_rate_ms,
    ).await?;

    while let Some(msg) = stream.next().await {
        match msg {
            Ok(msg) => {
                if let Message::Ping(ping) = msg {
                    stream.send(Message::Pong(ping)).await?;
                    continue;
                }
                if msg.is_binary() || msg.is_text() {
                    handle_update(books.clone(), msg.to_text().unwrap())?;
                    continue;
                }
                println!("Unhandled message: {msg:?}");
            },
            Err(e) => {
                println!("Error receiving message: {e}");
            }
        }
    }
    Ok(())
}

async fn connect(
    stream_base_endpoint: &str,
    market_tickers: Vec<String>,
    interval_ms: RefreshRate,
) -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>> {
    let stream_endpoint = make_stream_endpoint(
        stream_base_endpoint, 
        market_tickers, 
        interval_ms
    );
    println!("Connecting to {}", stream_endpoint);
    let (socket, response) = connect_async(stream_endpoint)
        .await?;
    // todo: check response status

    Ok(socket)
}

fn make_stream_endpoint(
    stream_base_endpoint: &str,
    market_tickers: Vec<String>,
    interval_ms: RefreshRate,
) -> String {
    let stream_keys = make_stream_keys(market_tickers, interval_ms);
    format!("{}/stream?streams={}", stream_base_endpoint, stream_keys.join("/"))
}

fn make_stream_keys(
    market_tickers: Vec<String>,
    interval_ms: RefreshRate,
) -> Vec<String> {
    market_tickers.iter()
        .map(|market_ticker| format!("{}@depth@{}ms", 
            market_ticker, interval_ms as u32
        ))
        .collect()
}

fn handle_update(books: OrderBooksShared, msg: &str) -> Result<()> {
    match serde_json::from_str::<BinanceAPIOrderBookUpdate>(msg) {
        Ok(order_book_update) => {
            let ref ticker = order_book_update.data.s;
            let mut book = books.get(&ticker.to_lowercase())
                .unwrap()
                .lock().expect("Could not lock book");
            book.0.update(order_book_update.data)?;
        }
        Err(e) => {
            println!("Error parsing order book update: {e}");
        }
    };
    Ok(())
}

// fn handle_update(books: OrderBooksShared, msg: &str) -> Result<()> {
//     match serde_json::from_str::<BinanceAPIOrderBookUpdate>(msg) {
//         Ok(order_book_update) => {
//             if BOOK_STREAM_KEY_REGEX.is_match(&order_book_update.stream) {
//                 let ticker = order_book_update.stream.split("@").next().unwrap();
//                 let mut book = books
//                     .get(&ticker.to_string()).unwrap()
//                     .lock().unwrap();
//                 book.0 = BinanceOrderBook::try_from(order_book_update.data)?;
//                 // println!("Refreshed {ticker} book"); 
//             }
//         }
//         Err(e) => {
//             println!("Error parsing order book update: {e}");
//         }
//     };
//     Ok(())
// }