use eyre::Result;

use super::*;
use order_book::{BinanceOrderBook, BinanceOrderBookData, SwapType};
use market::{Market, Markets};
use super::super::Quoter;
use crate::asset::{Asset, Domain};


const BINANCE_STREAM_ENDPOINT: &str = "wss://stream.binance.com:9443";
const BINANCE_API_ENDPOINT: &str = "https://api.binance.com";

pub struct BinanceQuoter {
    refresh_rate_ms: RefreshRate,
    order_books: Arc<HashMap<MarketTicker, Arc<Mutex<(BinanceOrderBook,)>>>>,
    pub markets: Markets,
    stream_started: bool,
}

impl BinanceQuoter {

    pub async fn create(
        markets: Vec<Market>,
        book_depth: u32,
        refresh_rate_ms: u32,
    ) -> Result<Self> {
        let markets: Markets = markets.into();
        let refresh_rate_ms = refresh_rate_ms.try_into().expect("Invalid refresh rate");

        let mut order_books = HashMap::new();
        for market_ticker in &markets.tickers {
            let book = connector::fetch_book(
                BINANCE_API_ENDPOINT, 
                market_ticker, 
                book_depth
            ).await?;
            let orderbook = BinanceOrderBook::new(
                book_depth, 
                BinanceOrderBookData::try_from(book)?
            );
            order_books.insert(
                market_ticker.clone(),
                Arc::new(Mutex::new((orderbook,)))
            );
        }
        let mut quoter = Self {
            order_books: Arc::new(order_books),
            stream_started: false,
            refresh_rate_ms,
            markets,
        };
        quoter.start_stream();
        Ok(quoter)
    }

    fn start_stream(&mut self) {
        tokio::spawn(connector::start_stream(
            BINANCE_STREAM_ENDPOINT,
                self.markets.tickers.clone(),
                self.refresh_rate_ms,
                self.order_books.clone()
            )
        );
        self.stream_started = true;
    }

    pub fn get_book(&self, market: &MarketTicker) -> Result<BinanceOrderBook> {
        let book = self.order_books.get(market).unwrap();
        let book = book.lock().unwrap().0.clone();
        Ok(book)
    }

    pub async fn query(
        &self, 
        sell_token: String,
        buy_token: String,
        sell_amount: f64,
    ) -> Result<f64> {
        let market = self.markets.get(&sell_token, &buy_token)
            .ok_or(eyre::eyre!(format!("Unsupported Binance market between {sell_token} and {buy_token}")))?;
        let book = self.get_book(&market.ticker())?;
        let (amount_used, amount_bought) = if sell_token == market.base() {
            book.query_exact_base(SwapType::Sell, sell_amount)
        } else {
            book.query_exact_quote(SwapType::Sell, sell_amount)
        };
        if amount_used != sell_amount {
            Err(eyre::eyre!(format!("Partial fill: {amount_used}/{sell_amount}")))
        } else {
            Ok(amount_bought)
        }        
    }


}

#[async_trait::async_trait]
impl Quoter for BinanceQuoter {

    async fn query(
        &self,
        domain_sell_asset_id: String,
        domain_buy_asset_id: String,
        domain_sell_amount: f64,
    ) -> Result<f64> {
        self.query(
            domain_sell_asset_id, 
            domain_buy_asset_id, 
            domain_sell_amount
        ).await
    }

    fn get_domain_id(&self) -> Domain {
        Domain::Binance
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    async fn debug_query(
        quoter: &BinanceQuoter,
        sell_token: String,
        buy_token: String,
        sell_amount: f64,
    ) {
        match quoter.query(
            sell_token.clone(),
            buy_token.clone(),
            sell_amount,
        ).await {
            Ok(amount_out) => println!("{sell_amount:?} {sell_token} -> {amount_out:?} {buy_token}"),
            Err(e) => println!("Error: {}", e),
        }
    }

    #[tokio::test]
    async fn test_binance_stream() {
        let book_depth = 50;
        let refresh_rate_ms = 100;
        let mut quoter = BinanceQuoter::create(
            vec![suppported_markets::ETHUSDT, suppported_markets::BTCUSDT],
            book_depth,
            refresh_rate_ms,
        ).await.unwrap();
        quoter.start_stream();

        let mut interval = tokio::time::interval(std::time::Duration::from_millis(100));
        loop {
            interval.tick().await;
            println!("{}", quoter.get_book(&suppported_markets::ETHUSDT.ticker()).unwrap());

            debug_query(
                &quoter, 
                String::from("ETH"), 
                String::from("USDT"), 
                10.
            ).await;
            debug_query(
                &quoter, 
                String::from("USDT"), 
                String::from("ETH"), 
                30_000.
            ).await;
        }
        
    }
}