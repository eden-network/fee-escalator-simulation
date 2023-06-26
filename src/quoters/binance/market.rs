use std::collections::{HashMap, HashSet};

type Asset = String;
type Base = &'static str;
type Quote = &'static str;

#[derive(Clone, Copy)]
pub struct Market(pub Base, pub Quote);

impl Market {
    
    pub fn ticker(&self) -> String {
        format!("{}{}", self.0, self.1).to_lowercase()
    }

    pub fn base(&self) -> Asset {
        self.0.to_string()
    }


    pub fn quote(&self) -> Asset {
        self.1.to_string()
    }
}

pub struct Markets {
    markets: HashMap<(String, String), Market>,
    pub tickers: Vec<String>,
}

impl Markets {

    fn new() -> Self {
        Self {
            markets: HashMap::new(), 
            tickers: Vec::new(),
        }
    }

    fn add(mut self, market: Market) -> Self {
        let (base, quote) = (String::from(market.0), String::from(market.1));
        self.markets.insert((base.clone(), quote.clone()), market);
        self.markets.insert((quote, base), market);
        self.tickers.push(market.ticker());
        self
    }

    pub fn get_ticker<T, D>(&self, asset_a: T, asset_b: D) -> Option<String>
        where T: Into<Asset>, D: Into<Asset>
    {
        self.markets.get(&(asset_a.into(), asset_b.into())).map(|m| m.ticker())
    }

    pub fn get<T, D>(&self, asset_a: T, asset_b: D) -> Option<&Market>
        where T: Into<Asset>, D: Into<Asset>
    {
        self.markets.get(&(asset_a.into(), asset_b.into()))
    }

}

impl From<Vec<Market>> for Markets {
    fn from(value: Vec<Market>) -> Self {
        value.into_iter()
            .fold(Self::new(), |markets, market| markets.add(market))
    }
}
