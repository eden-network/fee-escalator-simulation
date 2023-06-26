use super::*;
use stream::BinanceAPIOrderBookUpdateData;


#[derive(Debug, Clone)]
pub struct BinanceOrderBook {
    bids: Vec<Tick>,
    asks: Vec<Tick>,
}

impl BinanceOrderBook {

    pub fn query_exact_base(
        &self, 
        swap_type: SwapType, 
        base_amount: f64
    ) -> (f64, f64) {
        let book_side = if let SwapType::Sell = swap_type { 
            &self.bids 
        } else {
            &self.asks
        };

        let mut base_left = base_amount;
        let mut quote_used = 0.;
        for order in book_side.iter() {
            let base_fill = order.qty.min(base_left);
            let quote_fill = base_fill * order.price;
            base_left -= base_fill;
            quote_used += quote_fill;

            if base_left == 0. {
                break;
            }
        }
        let base_used = base_amount - base_left;
        (base_used, quote_used)
    }

    pub fn query_exact_quote(
        &self, 
        swap_type: SwapType,
        quote_amount: f64
    ) -> (f64, f64) {
        let book_side = if let SwapType::Sell = swap_type { 
            &self.bids 
        } else {
            &self.asks 
        };

        let mut quote_left = quote_amount;
        let mut base_used = 0.;
        for order in book_side.iter() {
            let quote_fill = quote_left.min(order.qty * order.price);
            let base_fill = quote_fill / order.price;
            quote_left -= quote_fill;
            base_used += base_fill;

            if quote_left == 0. {
                break;
            }
        }
        let quote_used = quote_amount - quote_left;
        (quote_used, base_used) 
    }

}

impl std::fmt::Display for BinanceOrderBook {

    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.bids.is_empty() || self.asks.is_empty() {
            return Ok(());
        }

        // todo: use tui create
        // todo: add quantity to slippage graph (maybe)
        // todo: add visual for tick quantity (maybe)
        let red_color = "\x1b[0;31m";
        let green_color = "\x1b[0;32m";
        let no_color = "\x1b[0m";

        let (min_price_w, min_qty_w) = {
            let dec_w = 3;
            let max_ask_price = self.asks.last().map(|t| t.price).unwrap_or_default();
            let max_bid_price = self.bids.first().map(|t| t.price).unwrap_or_default();
            let min_price_w = (max_ask_price.max(max_bid_price) as i32).to_string().len();
            
            let max_ask_qty = self.asks.iter().map(|t| t.qty).max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap_or_default();
            let max_bid_qty = self.bids.iter().map(|t| t.qty).max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap_or_default();
            let min_qty_w = (max_ask_qty.max(max_bid_qty) as i32).to_string().len();

            (min_price_w + dec_w, min_qty_w + dec_w)
        };
        let border_width = 1;
        let price_width = min_price_w + border_width;
        let qty_width = min_qty_w + border_width;

        let mut book_str = String::new();
        book_str.push_str("Asks:\n");
        for ask in self.asks.iter().rev() {
            book_str.push_str(
                &format!("\t{red_color}{0:>1$.2} @ {2:>3$.2}{no_color}\n", 
                    ask.price, price_width, ask.qty, qty_width
                )
            );
        }
        book_str.push_str("Bids:\n");
        for bid in self.bids.iter() {
            book_str.push_str(
                &format!("\t{green_color}{0:>1$.2} @ {2:>3$.2}{no_color}\n", 
                    bid.price, price_width, bid.qty, qty_width
                )
            );
        }
        write!(f, "{}", book_str)
    }

}

impl Default for BinanceOrderBook {
    fn default() -> Self {
        Self { bids: vec![], asks: vec![] }
    }
}

impl TryFrom<BinanceAPIOrderBookUpdateData> for BinanceOrderBook {
    type Error = eyre::Report;

    fn try_from(value: BinanceAPIOrderBookUpdateData) -> Result<Self, Self::Error> {
        let parse_side = |side: Vec<Vec<String>>| -> Result<Vec<Tick>> {
            side.iter().map(|bid| {
                Ok(Tick::new(
                    bid[0].parse::<f64>()?, 
                    bid[1].parse::<f64>()?
                ))
            } as Result<Tick>).collect::<Result<Vec<Tick>>>()
        };

        Ok(Self {
            bids: parse_side(value.bids)?,
            asks: parse_side(value.asks)?
        })
    }
}

#[derive(Debug, Clone, Copy)]
struct Tick {
    qty: f64,
    price: f64,
}

impl Tick {
    fn new(price: f64, qty: f64) -> Self {
        Self { price: price, qty: qty }
    }
}

pub enum SwapType {
    Buy,
    Sell,
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_exact_base_sell() {
        let book = BinanceOrderBook {
            bids: vec![
                Tick::new(1890., 1.),
                Tick::new(1889., 0.21),
                Tick::new(1888., 3.22),
            ],
            asks: vec![]
        };
        let base_amount = 5.;
        let avl_bid_qty = 4.43;
        let target_out = 8366.05;
        let (base_used, quote_out) = book.query_exact_base(
            SwapType::Sell, 
            base_amount
        );
        assert_eq!(base_used, avl_bid_qty);
        assert_eq!(quote_out, target_out);
    }

    #[test]
    fn test_query_exact_base_buy() {
        let book = BinanceOrderBook {
            bids: vec![],
            asks: vec![
                Tick::new(1888., 3.22),
                Tick::new(1889., 0.21),
                Tick::new(1890., 1.),
            ]
        };
        let base_amount = 5.;
        let avl_ask_qty = 4.43;
        let target_out = 8366.05;
        let (base_used, quote_out) = book.query_exact_base(
            SwapType::Buy, 
            base_amount
        );
        assert_eq!(base_used, avl_ask_qty);
        assert_eq!(quote_out, target_out);
    }

    #[test]
    fn test_query_exact_quote_sell() {
        let book = BinanceOrderBook {
            bids: vec![
                Tick::new(1890., 1.),
                Tick::new(1889., 0.21),
                Tick::new(1888., 3.22),
            ],
            asks: vec![]
        };
        let quote_amount = 9000.;
        let avl_bid_qty_quote = 8366.05;
        let target_out = 4.43;
        let (quote_used, base_out) = book.query_exact_quote(
            SwapType::Sell, 
            quote_amount
        );
        assert_eq!(quote_used, avl_bid_qty_quote);
        assert_eq!(base_out, target_out);
    }

    #[test]
    fn test_query_exact_quote_buy() {
        let book = BinanceOrderBook {
            bids: vec![],
            asks: vec![
                Tick::new(1888., 3.22),
                Tick::new(1889., 0.21),
                Tick::new(1890., 1.),
            ]
        };
        let quote_amount = 9000.;
        let avl_ask_qty_quote = 8366.05;
        let target_out = 4.43;
        let (quote_used, base_out) = book.query_exact_quote(
            SwapType::Buy, 
            quote_amount
        );
        let allowed_err = 0.00001;
        assert!(quote_used-avl_ask_qty_quote < allowed_err);
        assert!(base_out-target_out < allowed_err);
    }
}