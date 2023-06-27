mod quoters;
mod asset;

use quoters::binance::{BinanceQuoter, suppported_markets};
use quoters::oneinch::OneInchQuoter;
use quoters::Quoter;
use asset::Domain;

// todo: more binance ticks - no depth

#[tokio::main]
async fn main() -> eyre::Result<()> {
    dotenv::dotenv().ok();

    // gen
    let loop_wait_ms = 2000;

    // binance
    let book_depth = 20;
    let refresh_rate_ms = 100;

    let mut binance_quoter = BinanceQuoter::new(
        vec![suppported_markets::ETHUSDT, suppported_markets::BTCUSDT],
        book_depth,
        refresh_rate_ms,
    );
    binance_quoter.start_stream();

    // 1inch
    let domain = Domain::Arbitrum;
    let connector_tokens = None; 
    let complexity_level = Some(1);
    let main_route_parts = None;
    let parts = Some(10);

    let oneinch_quoter = OneInchQuoter::create(
        domain as u32,
        connector_tokens,
        complexity_level,
        main_route_parts,
        parts
    ).await?;


    // todo: make this in command-line args
    // trade
    let sell_asset = asset::Asset::new("eth")
        .add_domain(Domain::Binance, "ETH", 0)
        .add_domain(domain, "0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE", 18);
    let buy_asset = asset::Asset::new("usdt")
        .add_domain(Domain::Binance, "USDT", 0)
        .add_domain(domain, "0xFd086bC7CD5C481DCC9C85ebE478A1C0b69FCbb9", 6);
    let sell_amount_fixed = 50.;

    loop {
        let oneinch_amount_out = match oneinch_quoter.get_amount_out(&sell_asset, &buy_asset, sell_amount_fixed).await {
            Ok(amount_out) =>{
                println!("{} {} -> {} {}", sell_amount_fixed, sell_asset.id, amount_out, buy_asset.id);
                amount_out
            },
            Err(e) => {
                println!("Error: {}", e);
                0.
            },
        };
        let binance_amount_out = match binance_quoter.get_amount_out(&sell_asset, &buy_asset, sell_amount_fixed).await {
            Ok(amount_out) =>{
                println!("{} {} -> {} {}", sell_amount_fixed, sell_asset.id, amount_out, buy_asset.id);
                amount_out
            },
            Err(e) => {
                println!("Error: {}", e);
                0.
            },
        };
        println!("binance_amount_out/one_inch_amount_out: {}", binance_amount_out/oneinch_amount_out);
        tokio::time::sleep(std::time::Duration::from_millis(loop_wait_ms)).await;
    }

    Ok(())
}
