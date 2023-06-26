pub mod oneinch;
pub mod binance;

use crate::asset::{Asset, Domain};
use eyre::Result;

#[async_trait::async_trait]
pub trait Quoter {

    async fn get_amount_out(
        &self,
        sell_asset: &Asset, 
        buy_asset: &Asset,
        sell_amount: f64
    ) -> Result<f64> {
        let domain_id = self.get_domain_id();
        let domain_sell_asset_id = sell_asset.get_domain_id(domain_id)?;
        let domain_buy_asset_id = buy_asset.get_domain_id(domain_id)?;
        let domain_sell_amount = sell_asset.convert_from_zero(domain_id, sell_amount)?;
        let domain_buy_amount = self.query(
            domain_sell_asset_id, 
            domain_buy_asset_id, 
            domain_sell_amount
        ).await?;
        let buy_amount = buy_asset.convert_to_zero(domain_id, domain_buy_amount as f64)?;
        Ok(buy_amount)
    }

    async fn query(
        &self, 
        domain_sell_asset_id: String,
        domain_buy_asset_id: String,
        domain_sell_amount: f64,
    ) -> Result<f64>;

    fn get_domain_id(&self) -> Domain;

}