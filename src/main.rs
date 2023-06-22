mod adapter;

use adapter::binance::BinanceAdapter;

#[tokio::main]
async fn main() {
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
