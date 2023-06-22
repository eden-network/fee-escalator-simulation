use eyre::Result;

use super::*;
use order_book::BinanceOrderBook;


pub struct BinanceAdapter {
    stream_base_endpoint: String,
    market_tickers: Vec<String>,
    book_size: BookSize,
    refresh_rate_ms: RefreshRate,
    order_books: Arc<HashMap<String, Arc<Mutex<(BinanceOrderBook,)>>>>,
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
            refresh_rate_ms,
            book_size,
        }
    }

    pub fn start_stream(&self) {
        tokio::spawn(stream::start_stream(
                self.stream_base_endpoint.clone(),
                self.market_tickers.clone(),
                self.book_size,
                self.refresh_rate_ms,
                self.order_books.clone()
            )
        );
    }

    pub fn get_book(&self, market: MarketTicker) -> Result<BinanceOrderBook> {
        let book = self.order_books.get(&market).unwrap();
        let book = book.lock().unwrap().0.clone();
        Ok(book)
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
}