use std::{collections::HashMap};
use eyre::Result;
use num_derive::FromPrimitive;

// todo: each asset for a particular domain is different asset - different risk params based on the domain security model
// todo: need to create quoters between related assets (e.g. ETH on Binance and ETH on Arbitrum) but also specify the risk impact on the conversion price

type AssetId = String;
type Decimals = u8;

#[derive(Debug, Clone, Copy, std::cmp::PartialEq, std::cmp::Eq, std::hash::Hash, FromPrimitive)]
pub enum Domain {
    Binance = -1,
    Arbitrum = 42161,
}

impl TryFrom<i32> for Domain {
    type Error = eyre::Report;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        num_traits::FromPrimitive::from_i32(value).ok_or(eyre::eyre!(format!("Unsupported domain {value}")))
    }
}

pub enum Domains2 {
    Centralised(Centralised),
    Decentralised(Decentralised),
}

type ChainId = u32;
type DomainUID = String;

pub enum Centralised {
    Binance, 
    Coinbase,
}

impl From<Centralised> for Domains2 {
    fn from(value: Centralised) -> Self {
        Domains2::Centralised(value)
    }
}

pub enum Decentralised {
    EVM(EVM),
    NonEVM(NonEVM)
}

impl From<Decentralised> for Domains2 {
    fn from(value: Decentralised) -> Self {
        Domains2::Decentralised(value)
    }
}

pub enum EVM {
    Arbitrum = 42161,
    Optimism = 10,
    Polygon = 137,
    BSC = 56,
    Ethereum = 1,
}

impl From<EVM> for Domains2 {
    fn from(value: EVM) -> Self {
        Domains2::Decentralised(Decentralised::EVM(value))
    }
}

pub enum NonEVM {
    Solana, 
    Algorand,
    Cardano,
    Aptos,
}

impl From<NonEVM> for Domains2 {
    fn from(value: NonEVM) -> Self {
        Domains2::Decentralised(Decentralised::NonEVM(value))
    }
}

/**
 *  let domain1 = asset::EVM::Arbitrum;
    let domain2 = asset::Centralised::Binance;

    fn smth<T, D: Into<asset::Domains2>>(a: T, b: D) {
        // println!("{:?} {:?}", a, b);
    }

    smth(domain1, domain2);
 * 
 */

#[derive(Clone, Debug)]
pub struct Asset {
    pub id: String,
    domain_info: HashMap<Domain, (AssetId, Decimals)>,
}

impl Asset {

    pub fn new(id: &str) -> Self {
        Self { id: id.to_string(), domain_info: HashMap::new() }
    }

    pub fn add_domain<T: ToString>(
        mut self, 
        domain: Domain, 
        id: T, 
        dec: Decimals
    ) -> Self {
        self.domain_info.insert(domain, (id.to_string(), dec));
        self
    }

    pub fn get_domain_id(&self, domain: Domain) -> Result<AssetId> {
        self.get_domain_info(domain).map(|(id, _)| id)
    }

    pub fn get_domain_decimals(&self, domain: Domain) -> Result<Decimals> {
        self.get_domain_info(domain).map(|(_, dec)| dec)
    }

    pub fn get_domain_info(
        &self, 
        domain: Domain
    ) -> Result<(AssetId, Decimals)> {
        self.domain_info.get(&domain).cloned()
            .ok_or(eyre::eyre!(format!("Domain ID {domain:?} not supported")))
    }

    pub fn convert_from_zero(
        &self, 
        target_domain: Domain,
        amount: f64
    ) -> Result<f64> {
        let origin_dec = 0;
        let target_dec = self.get_domain_decimals(target_domain)?;
        Ok(self.convert(origin_dec, target_dec, amount))
    }

    pub fn convert_to_zero(
        &self, 
        origin_domain: Domain,
        amount: f64
    ) -> Result<f64> {
        let origin_dec = self.get_domain_decimals(origin_domain)?;
        let target_dec = 0;
        Ok(self.convert(origin_dec, target_dec, amount))
    }

    pub fn convert_between_domains(
        &self, 
        origin_domain: Domain,
        target_domain: Domain,
        amount: f64
    ) -> Result<f64> {
        let origin_dec = self.get_domain_decimals(origin_domain)?;
        let target_dec = self.get_domain_decimals(target_domain)?;
        Ok(self.convert(origin_dec, target_dec, amount))
    }

    pub fn convert(
        &self, 
        origin_dec: Decimals,
        target_dec: Decimals,
        mut amount: f64
    ) -> f64 {
        let net_dec = target_dec as i32 - origin_dec as i32;
        if net_dec > 0 {
            amount = amount * (10u128.pow(net_dec as u32) as f64);
        } else {
            amount = amount / (10u128.pow(-net_dec as u32) as f64);
        }
        amount
    }

}

pub mod supported_assets {
    use super::{Domain, Asset};

    lazy_static::lazy_static! {
        pub static ref ETH: Asset = Asset::new("eth")
            .add_domain(Domain::Binance, "ETH", 0)
            .add_domain(Domain::Arbitrum, "0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE", 18);
        pub static ref USDT: Asset = Asset::new("usdt")
            .add_domain(Domain::Binance, "USDT", 0)
            .add_domain(Domain::Arbitrum, "0xFd086bC7CD5C481DCC9C85ebE478A1C0b69FCbb9", 6);
        pub static ref ARB: Asset = Asset::new("arb")
            .add_domain(Domain::Binance, "ARB", 0)
            .add_domain(Domain::Arbitrum, "0x912CE59144191C1204E64559FE8253a0e49E6548", 18);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_asset() {
        let eth = Asset::new("eth")
            .add_domain(Domain::Binance, "ETH", 0)
            .add_domain(Domain::Arbitrum, "0x82aF49447D8a07e3bd95BD0d56f35241523fBab1", 18);

        assert_eq!(eth.get_domain_info(Domain::Binance).unwrap(), ("ETH".to_string(), 0));
        assert_eq!(eth.get_domain_info(Domain::Arbitrum).unwrap(), ("0x82aF49447D8a07e3bd95BD0d56f35241523fBab1".to_string(), 18));
    }

}