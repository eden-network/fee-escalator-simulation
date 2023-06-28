use std::{collections::HashMap, fmt::Debug};
use serde::Deserialize;
use eyre::Result;
use reqwest; 


const API_URL: &str = "https://api.1inch.io/v5.0";

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OneInchResponse {
    // from_token: Token, 
    // to_token: Token,
    pub to_token_amount: String, 
    // from_token_amount: String, 
    // protocols: Vec<Vec<Vec<Protocol>>>, 
    // estimated_gas: u64,
}

// #[derive(Deserialize, Debug)]
// #[serde(rename_all = "camelCase")]
// struct Token {
//     symbol: String,
//     name: String, 
//     decimals: u8,
//     address: String, 
//     #[serde(rename = "logoURI")]
//     logo_uri: String, 
//     tags: Vec<String>
// }

// #[derive(Deserialize, Debug)]
// #[serde(rename_all = "camelCase")]
// struct Protocol {
//     name: String, 
//     part: u8, 
//     from_token_address: String, 
//     to_token_address: String
// }

pub struct OneInchClient {
    client: reqwest::Client,
    chain_id: u32,
    connector_tokens: Option<u8>,
    complexity_level: Option<u8>,
    main_route_parts: Option<u8>,
    parts: Option<u8>,
}

impl OneInchClient {

    pub async fn create(
        chain_id: u32,
        connector_tokens: Option<u8>,
        complexity_level: Option<u8>,
        main_route_parts: Option<u8>,
        parts: Option<u8>,
    ) -> Result<Self> {
        let client = Self::create_http_client().await?;
        Ok(OneInchClient {
            client,
            chain_id,
            connector_tokens,
            complexity_level,
            main_route_parts,
            parts,
        })
    }

    pub async fn query(
        &self,
        sell_token: String, 
        buy_token: String, 
        sell_amount: u128, 
    ) -> Result<OneInchResponse> {
        let params = Self::construct_params(
            sell_token, 
            buy_token, 
            sell_amount, 
            self.connector_tokens, 
            self.complexity_level, 
            self.main_route_parts, 
            self.parts
        );

        let full_url = format!("{API_URL}/{}/quote", self.chain_id);
        let res = self.send_get_json_request(&full_url, &params).await?;
        let oneinch_res = res.json::<OneInchResponse>().await?;

        Ok(oneinch_res)
    }

    pub fn chain_id(&self) -> u32 {
        self.chain_id
    }

    async fn send_get_json_request<'s>(
        &self,
        url: &str, 
        params: &HashMap<&'s str, String>
    ) -> Result<reqwest::Response> {
        let res = self.client.get(url)
            .header("accept", "application/json")
            .query(params)
            .send()
            .await?;
        if res.status().as_u16() != 200 {
            // todo: open as text?
            return Err(eyre::eyre!(format!("Request failed: {res:?}")));
        }
        Ok(res)
    }

    fn construct_params<'a>(
        sell_token: String, 
        buy_token: String, 
        sell_amount: u128, 
        connector_tokens: Option<u8>,
        complexity_level: Option<u8>,
        main_route_parts: Option<u8>,
        parts: Option<u8>,
    ) -> HashMap<&'a str, String> {
        let mut params = HashMap::new();
        params.insert("fromTokenAddress", sell_token.to_string());
        params.insert("toTokenAddress", buy_token.to_string());
        params.insert("amount", sell_amount.to_string());
    
        if let Some(connector_tokens) = connector_tokens {
            assert!(connector_tokens <= 5, "Max 5 connector tokens");
            params.insert("connectorTokens", connector_tokens.to_string());
        }
        if let Some(complexity_level) = complexity_level {
            assert!(complexity_level <= 3, "Max 3 complexity level");
            params.insert("complexityLevel", complexity_level.to_string());
        }
        if let Some(main_route_parts) = main_route_parts {
            assert!(main_route_parts <= 50, "Max 50 main route parts");
            params.insert("mainRouteParts", main_route_parts.to_string());
        }
        if let Some(parts) = parts {
            assert!(parts <= 100, "Max 100 parts");
            params.insert("parts", parts.to_string());
        }
    
        params
    }
    
    async fn create_http_client() -> Result<reqwest::Client> {
        let proxy = Self::create_http_proxy()?;
        let client = reqwest::Client::builder().proxy(proxy).build()?;
        Ok(client)
    }
    
    fn create_http_proxy() -> Result<reqwest::Proxy> {
        let username = std::env::var("PROXY_USERNAME").expect("PROXY_USERNAME not set");
        let password = std::env::var("PROXY_PASSWORD").expect("PROXY_PASSWORD not set");
        let host = std::env::var("PROXY_HOST").expect("PROXY_HOST not set");
        let port = std::env::var("PROXY_PORT").expect("PROXY_PORT not set");
        let url = format!("http://{username}:{password}@{host}:{port}");
        let proxy = reqwest::Proxy::https(url)?;
        Ok(proxy)
    }
    

}


#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_oneinch_query() -> Result<()> {
        dotenv::dotenv().ok();

        let chain_id = 42161;
        let sell_token = "0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE";
        let buy_token = "0xaf88d065e77c8cC2239327C5EDb3A432268e5831";
        let sell_amount = 100000000000000000000;
        let connector_tokens = None; 
        let complexity_level = None;
        let main_route_parts = None;
        let parts = None;


        let client = OneInchClient::create(
            chain_id,
            connector_tokens,
            complexity_level,
            main_route_parts,
            parts,
        ).await?;

        let res = client.query(
            sell_token.to_string(), 
            buy_token.to_string(), 
            sell_amount, 
        ).await?;

        println!("{:#?}", res);

        Ok(())
    }

}