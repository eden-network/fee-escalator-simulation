use ethers::providers::{Provider, Http};
use ethers::contract::abigen;
use ethers::types::{H160, U256};
use std::sync::Arc;
use eyre::Result;
use futures::future::join_all;

use super::super::Quoter;
use crate::asset::Domain;


abigen!(UniV3StaticQuoter, "./src/quoters/crypto/abis/UniV3Quoter.json");

const ENABLED_FEE_AMOUNTS: [u32; 3] = [500, 3000, 10000];

pub struct UniV3Quoter {
    quoter_contract: UniV3StaticQuoter<Provider<Http>>,
    chain_id: u32,
}

impl UniV3Quoter {

    pub fn create(
        rpc_url: &str,
        contract_address: &str,
        chain_id: u32,
    ) -> Result<Self> {
        let provider = Arc::new(Provider::<Http>::try_from(rpc_url)?);
        let address = contract_address.parse::<H160>()?;
        let quoter_contract = UniV3StaticQuoter::new(address, provider);
        Ok(Self { quoter_contract, chain_id })
    }

    async fn query_all(
        &self,
        token_in: &str,
        token_out: &str,
        amount_in: u128,
    ) -> Result<u128> {
        let fs_iter = ENABLED_FEE_AMOUNTS.map(|fee| {
            self.query_single(&token_in, &token_out, amount_in, fee)
        });
        let quotes = join_all(fs_iter).await.into_iter().collect::<Result<Vec<u128>>>()?;
        let best_quote = quotes.into_iter().max().unwrap(); // .ok_or(Err(eyre::eyre!("No valid quote")))?
        
        Ok(best_quote)
    }

    async fn query_single(
        &self,
        token_in: &str,
        token_out: &str,
        amount_in: u128,
        fee: u32,
    ) -> Result<u128> {
        let params = QuoteExactInputSingleParams {
            token_in: token_in.parse()?,
            token_out: token_out.parse()?,
            fee: fee,
            amount_in: U256::from(amount_in),
            sqrt_price_limit_x96: U256::zero(),
        };
        let out: U256 = self.quoter_contract.quote_exact_input_single(params).await?;
        Ok(out.as_u128())
    }

}

#[async_trait::async_trait]
impl Quoter for UniV3Quoter {

    async fn query(
        &self, 
        domain_sell_asset_id: String,
        domain_buy_asset_id: String,
        domain_sell_amount: f64,
    ) -> Result<f64> {
        let domain_buy_amount = self.query_all(
            &domain_sell_asset_id, 
            &domain_buy_asset_id, 
            domain_sell_amount as u128
        ).await?;
        Ok(domain_buy_amount as f64)
    }
    
    fn get_domain_id(&self) -> Domain {
        (self.chain_id as i32).try_into().expect("Invalid domain id")
    }

}

#[cfg(test)]
mod tests {
    use super::*; 

    #[tokio::main]
    #[test]
    async fn test_query_single_eth_usdt() {
        dotenv::dotenv().ok();

        let rpc_url = std::env::var("ARB_RPC_URL").unwrap();
        let arb_eden_static_quoter = "0xc80f61d1bdAbD8f5285117e1558fDDf8C64870FE";
        let usdt = "0xFd086bC7CD5C481DCC9C85ebE478A1C0b69FCbb9";
        let weth = "0x82aF49447D8a07e3bd95BD0d56f35241523fBab1";
        let chain_id = 42161;
        let amount_in = 1e18 as u128;
        let fee = 3000;

        let quoter = UniV3Quoter::create(
            &rpc_url, 
            &arb_eden_static_quoter.to_string(),
            chain_id
        ).unwrap();
        let res = quoter.query_single(
            weth,
            usdt,
            amount_in,
            fee,
        ).await;
        assert!(res.is_ok());
        let res = res.unwrap();
        assert!(res > 0);
        println!("res: {}", res);
    }

    #[tokio::main]
    #[test]
    async fn test_query_all_eth_usdt() {
        dotenv::dotenv().ok();

        let rpc_url = std::env::var("ARB_RPC_URL").unwrap();
        let arb_eden_static_quoter = "0xc80f61d1bdAbD8f5285117e1558fDDf8C64870FE";
        let usdt = "0xFd086bC7CD5C481DCC9C85ebE478A1C0b69FCbb9";
        let weth = "0x82aF49447D8a07e3bd95BD0d56f35241523fBab1";
        let chain_id = 42161;
        let amount_in = 1e18 as u128;

        let quoter = UniV3Quoter::create(
            &rpc_url, 
            &arb_eden_static_quoter.to_string(), 
            chain_id
        ).unwrap();
        let res = quoter.query_all(weth, usdt, amount_in).await;
        assert!(res.is_ok());
        let res = res.unwrap();
        assert!(res > 0);
        println!("res: {}", res);
    }
}