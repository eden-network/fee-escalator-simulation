use eyre::Result;

use super::*;
use order_book::{BinanceOrderBook, SwapType};
use market::{Market, Markets};
use super::super::Quoter;
use crate::asset::{Asset, Domain};


const BinanceStreamEndpoint: &str = "wss://stream.binance.com:9443";

pub struct BinanceQuoter {
    book_size: BookSize,
    refresh_rate_ms: RefreshRate,
    order_books: Arc<HashMap<MarketTicker, Arc<Mutex<(BinanceOrderBook,)>>>>,
    pub markets: Markets,
    stream_started: bool,
}

impl BinanceQuoter {

    pub fn new(
        markets: Vec<Market>,
        book_size: u8,
        refresh_rate_ms: u32,
    ) -> Self {
        let markets: Markets = markets.into();
        let book_size = book_size.try_into().expect("Invalid book size");
        let refresh_rate_ms = refresh_rate_ms.try_into().expect("Invalid refresh rate");

        let mut order_books = HashMap::new();
        for market_ticker in &markets.tickers {
            order_books.insert(
                market_ticker.clone(),
                // todo: add last update timestamp
                Arc::new(Mutex::new((BinanceOrderBook::default(),)))
            );
        }
        Self {
            order_books: Arc::new(order_books),
            stream_started: false,
            refresh_rate_ms,
            book_size,
            markets,
        }
    }

    pub fn start_stream(&mut self) {
        tokio::spawn(stream::start_stream(
            BinanceStreamEndpoint,
                self.markets.tickers.clone(),
                self.book_size,
                self.refresh_rate_ms,
                self.order_books.clone()
            )
        );
        self.stream_started = true;
    }

    pub fn get_book(&self, market: &MarketTicker) -> Result<BinanceOrderBook> {
        if !self.stream_started {
            println!("Stream not yet started!");
        }
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
        println!("BinanceQuoter::query {:?}", (&domain_sell_asset_id, &domain_buy_asset_id, &domain_sell_amount));
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
        let mut quoter = BinanceQuoter::new(
            vec![suppported_markets::ETHUSDT, suppported_markets::BTCUSDT],
            20,
            100,
        );
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