use super::super::{Quoter, Domain, Result};
use super::api::OneInchClient;

pub struct OneInchQuoter {
    client: OneInchClient,
}

impl OneInchQuoter {

    pub async fn create(
        chain_id: u32,
        connector_tokens: Option<u8>,
        complexity_level: Option<u8>,
        main_route_parts: Option<u8>,
        parts: Option<u8>,
    ) -> Result<Self> {
        Ok(OneInchQuoter {
            client: OneInchClient::create(
                chain_id,
                connector_tokens,
                complexity_level,
                main_route_parts,
                parts,
            ).await?
        })
    }

    pub async fn query_buy_amount(
        &self, 
        sell_token: String,
        buy_token: String,
        buy_amount: u128,   
    ) -> Result<u128> {
        let res = self.client.query(
            sell_token, 
            buy_token, 
            buy_amount, 
        ).await?;
        let buy_amount = res.to_token_amount.parse::<u128>()?;
        Ok(buy_amount)
    }

}

#[async_trait::async_trait]
impl Quoter for OneInchQuoter {

    async fn query(
        &self, 
        domain_sell_asset_id: String,
        domain_buy_asset_id: String,
        domain_sell_amount: f64,
    ) -> Result<f64> {
        let domain_buy_amount = self.query_buy_amount(
            domain_sell_asset_id, 
            domain_buy_asset_id, 
            domain_sell_amount as u128
        ).await?;
        Ok(domain_buy_amount as f64)
    }
    
    fn get_domain_id(&self) -> Domain {
        (self.client.chain_id() as i32).try_into().expect("Invalid domain id")
    }

}