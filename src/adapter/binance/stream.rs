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
#[serde(rename_all = "camelCase")]
pub struct BinanceAPIOrderBookUpdateData {
    last_update_id: u64,
    pub bids: Vec<Vec<String>>, // sorted desc
    pub asks: Vec<Vec<String>>, // sorted asc
}

pub(super) async fn start_stream(
    stream_base_endpoint: String, 
    market_tickers: Vec<MarketTicker>,
    book_size: BookSize,
    refresh_rate_ms: RefreshRate,
    books: OrderBooksShared
) -> Result<()> {
    let mut stream = connect(
        &stream_base_endpoint, 
        market_tickers.clone(), 
        book_size, 
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
    book_size: BookSize,
    interval_ms: RefreshRate,
) -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>> {
    let stream_endpoint = make_stream_endpoint(
        stream_base_endpoint, 
        market_tickers, 
        book_size, 
        interval_ms
    );
    println!("Connecting to {}", stream_endpoint);
    let (socket, response) = connect_async(stream_endpoint)
        .await?;
    // if response.status().as_u16() != 200 {
    //     return Err(eyre::eyre!("Unexpected status code: {:?}", response.body()));
    // }
    Ok(socket)
}

fn make_stream_endpoint(
    stream_base_endpoint: &str,
    market_tickers: Vec<String>,
    book_size: BookSize,
    interval_ms: RefreshRate,
) -> String {
    let stream_keys = make_stream_keys(market_tickers, book_size, interval_ms);
    format!("{}/stream?streams={}", stream_base_endpoint, stream_keys.join("/"))
}

fn make_stream_keys(
    market_tickers: Vec<String>,
    book_size: BookSize,
    interval_ms: RefreshRate,
) -> Vec<String> {
    market_tickers.iter()
        .map(|market_ticker| format!("{}@depth{}@{}ms", 
            market_ticker, book_size as u8, interval_ms as u32
        ))
        .collect()
}

fn handle_update(books: OrderBooksShared, msg: &str) -> Result<()> {
    match serde_json::from_str::<BinanceAPIOrderBookUpdate>(msg) {
        Ok(order_book_update) => {
            if BOOK_STREAM_KEY_REGEX.is_match(&order_book_update.stream) {
                let ticker = order_book_update.stream.split("@").next().unwrap();
                let mut book = books
                    .get(&ticker.to_string()).unwrap()
                    .lock().unwrap();
                book.0 = BinanceOrderBook::try_from(order_book_update.data)?;
                // println!("Refreshed {ticker} book"); 
            }
        }
        Err(e) => {
            println!("Error parsing order book update: {e}");
        }
    };
    Ok(())
}