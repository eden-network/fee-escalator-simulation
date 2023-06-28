mod quoters;
mod asset;

use quoters::binance::{BinanceQuoter, self};
use quoters::oneinch::OneInchQuoter;
use quoters::crypto::UniV3Quoter;
use quoters::Quoter;
use asset::{Domain, supported_assets};


#[tokio::main]
async fn main() -> eyre::Result<()> {
    dotenv::dotenv().ok();

    // gen
    let loop_wait_ms = 2000;
    const BPS: f64 = 10000.;

    // binance
    let book_depth = 200;
    let refresh_rate_ms = 100;
    let binance_fee_bps = 7.5;

    let binance_quoter = BinanceQuoter::create(
        vec![
            // binance::suppported_markets::ETHUSDT, 
            // binance::suppported_markets::BTCUSDT,
            binance::suppported_markets::ARBUSDT,
        ],
        book_depth,
        refresh_rate_ms,
    ).await?;

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

    // UniV3

    let rpc_url = std::env::var("ARB_RPC_URL").unwrap();
    let arb_eden_static_quoter = "0xc80f61d1bdAbD8f5285117e1558fDDf8C64870FE";
    let chain_id = Domain::Arbitrum as u32;

    let univ3_quoter = UniV3Quoter::create(
        &rpc_url, 
        &arb_eden_static_quoter.to_string(), 
        chain_id
    ).unwrap();


    // todo: make this in command-line args
    // trade
    // let sell_asset = supported_assets::ETH;
    let sell_asset = &supported_assets::ARB;
    let buy_asset = &supported_assets::USDT;
    let sell_amount_fixed = 700_000.;

    let apply_binance_fee = |x: f64| {
        x * (1. - binance_fee_bps/BPS)
    };

    loop {
        let oneinch_amount_out = match oneinch_quoter.get_amount_out(&sell_asset, &buy_asset, sell_amount_fixed).await {
            Ok(amount_out) => {
                println!("\tOneInch: {:.2} {} -> {:.2} {}", sell_amount_fixed, sell_asset.id, amount_out, buy_asset.id);
                amount_out
            },
            Err(e) => {
                println!("Error: {}", e);
                0.
            },
        };
        let binance_amount_out = match binance_quoter.get_amount_out(&sell_asset, &buy_asset, sell_amount_fixed).await {
            Ok(amount_out) => {
                println!("\tBinance: {:.2} {} -> {:.2} {}", sell_amount_fixed, sell_asset.id, amount_out, buy_asset.id);
                apply_binance_fee(amount_out)
            },
            Err(e) => {
                println!("Error: {}", e);
                0.
            },
        };
        let univ3_amount_out = match univ3_quoter.get_amount_out(&sell_asset, &buy_asset, sell_amount_fixed).await {
            Ok(amount_out) => {
                println!("\tUniV3: {:.2} {} -> {:.2} {}", sell_amount_fixed, sell_asset.id, amount_out, buy_asset.id);
                amount_out
            },
            Err(e) => {
                println!("Error: {}", e);
                0.
            },
        };
        println!("binance_amount_out/one_inch_amount_out: {:.2} bps", (1.-binance_amount_out/oneinch_amount_out)*BPS);
        println!("binance_amount_out/univ3_amount_out: {:.2} bps", (1.-binance_amount_out/univ3_amount_out)*BPS);
        println!();
        tokio::time::sleep(std::time::Duration::from_millis(loop_wait_ms)).await;
    }

    Ok(())
}
