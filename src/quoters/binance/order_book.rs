use super::*;
use connector::{BinanceAPIOrderBookUpdateData, BinanceAPIOrderBookData};


#[derive(Debug, Clone)]
pub struct BinanceOrderBook {
    data: BinanceOrderBookData,
    depth: u32,
}

#[derive(Debug, Clone)]
pub struct BinanceOrderBookData {
    pub last_update_time: u64,
    pub bids: Vec<Tick>,
    pub asks: Vec<Tick>,
}

impl BinanceOrderBook {

    pub fn new(depth: u32, data: BinanceOrderBookData) -> Self {
        Self { data, depth }
    }

    pub fn update_from_stream(
        &mut self, 
        update_data: BinanceAPIOrderBookUpdateData
    ) -> Result<()> {
        self.update_bids(Self::parse_side(update_data.b)?);
        self.update_asks(Self::parse_side(update_data.a)?);
        self.data.last_update_time = update_data.E;
        Ok(())
    }

    fn update_bids(&mut self, updated_ticks: Vec<Tick>) {
        // ? Assume ticks are ordered descending
        self.data.bids = Self::update_ticks(
            self.data.bids.clone(), 
            updated_ticks, 
            self.depth, 
            false
        );
    }

    fn update_asks(&mut self, updated_ticks: Vec<Tick>) {
        // ? Assume ticks are ordered ascending
        self.data.asks = Self::update_ticks(
            self.data.asks.clone(), 
            updated_ticks, 
            self.depth, 
            true
        );
    }

    fn update_ticks(
        mut book: Vec<Tick>, 
        updated_ticks: Vec<Tick>,
        depth: u32, 
        is_ascending: bool
    ) -> Vec<Tick> {
        // todo: efficient design
        if book.is_empty() {
            return updated_ticks;
        }
        for new_tick in updated_ticks {
            let mut is_detected = false;
            for i in 0..book.len() {
                let old_tick = &mut book[i];
                if old_tick.price == new_tick.price {
                    old_tick.qty = new_tick.qty;
                    is_detected = true;
                    break;
                }
                // else if 
                //     (is_ascending && old_tick.price>new_tick.price) || 
                //     (!is_ascending && old_tick.price<new_tick.price) 
                // {
                //     book.insert(i, new_tick);
                //     break;
                // } else if i == book.len()-1 {
                //     book.push(new_tick);
                //     break;
                // }
            }
            if !is_detected {
                book.push(new_tick);
            } 
            
        }
        book.sort_by(|a, b| {
            if is_ascending {
                a.price.partial_cmp(&b.price).unwrap()
            } else {
                b.price.partial_cmp(&a.price).unwrap()
            }
        });
        book.retain(|tick| tick.qty > 0.);
        book.truncate(depth as usize);
        book
    }

    pub fn query_exact_base(
        &self, 
        swap_type: SwapType, 
        base_amount: f64
    ) -> (f64, f64) {
        let book_side = if let SwapType::Sell = swap_type { 
            &self.data.bids 
        } else {
            &self.data.asks
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
            &self.data.bids 
        } else {
            &self.data.asks 
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

    fn parse_side(side: Vec<Vec<String>>) -> Result<Vec<Tick>> {
        side.iter().map(|tick| {
            Ok(Tick::new(
                tick[0].parse::<f64>()?, 
                tick[1].parse::<f64>()?
            ))
        } as Result<Tick>).collect()
    }

}

impl std::fmt::Display for BinanceOrderBook {

    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.data.bids.is_empty() || self.data.asks.is_empty() {
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
            let max_ask_price = self.data.asks.last().map(|t| t.price).unwrap_or_default();
            let max_bid_price = self.data.bids.first().map(|t| t.price).unwrap_or_default();
            let min_price_w = (max_ask_price.max(max_bid_price) as i32).to_string().len();
            
            let max_ask_qty = self.data.asks.iter().map(|t| t.qty).max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap_or_default();
            let max_bid_qty = self.data.bids.iter().map(|t| t.qty).max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap_or_default();
            let min_qty_w = (max_ask_qty.max(max_bid_qty) as i32).to_string().len();

            (min_price_w + dec_w, min_qty_w + dec_w)
        };
        let border_width = 1;
        let price_width = min_price_w + border_width;
        let qty_width = min_qty_w + border_width;

        let mut book_str = String::new();
        book_str.push_str("Asks:\n");
        for ask in self.data.asks.iter().rev() {
            book_str.push_str(
                &format!("\t{red_color}{0:>1$.2} @ {2:>3$.2}{no_color}\n", 
                    ask.price, price_width, ask.qty, qty_width
                )
            );
        }
        book_str.push_str("Bids:\n");
        for bid in self.data.bids.iter() {
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
        Self {
            data: BinanceOrderBookData {
                last_update_time: 0,
                bids: vec![],
                asks: vec![], 
            },
            depth: 0
        }
    }
}


impl TryFrom<&Vec<String>> for Tick {
    type Error = eyre::Report;

    fn try_from(value: &Vec<String>) -> Result<Self, Self::Error> {
        Ok(Tick::new(
            value[0].parse::<f64>()?, 
            value[1].parse::<f64>()?
        ))
    }
}

impl TryFrom<BinanceAPIOrderBookData> for BinanceOrderBookData {
    type Error = eyre::Report;

    fn try_from(value: BinanceAPIOrderBookData) -> Result<Self, Self::Error> {
        Ok(Self {
            bids: BinanceOrderBook::parse_side(value.bids)?,
            asks: BinanceOrderBook::parse_side(value.asks)?,
            last_update_time: utils::get_epoch_ms(),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tick {
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
    fn test_update_empty() {
        let mut book = BinanceOrderBook {
            data: BinanceOrderBookData {
                last_update_time: 0, 
                bids: vec![], 
                asks: vec![] 
            },
            depth: 3,
        };
        let updated_bids = vec![
            Tick::new(1890., 1.),
            Tick::new(1889., 0.21),
            Tick::new(1888., 3.22),
        ];
        let updated_asks = vec![
            Tick::new(1890., 4.21),
            Tick::new(1891., 3.2),
            Tick::new(1892., 0.2),
        ];
        book.update_bids(updated_bids.clone());
        book.update_asks(updated_asks.clone());

        let update_bids_len = updated_bids.len();
        assert_eq!(book.data.bids.len(), update_bids_len);
        for i in 0..update_bids_len {
            assert_eq!(book.data.bids[i], updated_bids[i]);
        }

        let update_asks_len = updated_asks.len();
        assert_eq!(book.data.asks.len(), update_asks_len);
        for i in 0..update_asks_len {
            assert_eq!(book.data.asks[i], updated_asks[i]);
        }
    }

    #[test]
    fn test_update_bids_nonempty() {
        let old_bids = vec![
            Tick::new(1890., 0.1),
            Tick::new(1888., 3.22),
        ];
        let mut book = BinanceOrderBook {
            data: BinanceOrderBookData {
                last_update_time: 0, 
                bids: old_bids.clone(), 
                asks: vec![] 
            }, 
            depth: 3,
        };
        let updated_ticks = vec![
            Tick::new(1890., 1.),
            Tick::new(1889., 0.21),
        ];
        book.update_bids(updated_ticks.clone());

        assert_eq!(book.data.bids[0], updated_ticks[0]);
        assert_eq!(book.data.bids[1], updated_ticks[1]);
        assert_eq!(book.data.bids[2], old_bids[1]);
    }

    #[test]
    fn test_update_asks_nonempty() {
        let old_asks = vec![
            Tick::new(1891., 0.1),
            Tick::new(1893., 3.22),
        ];
        let mut book = BinanceOrderBook {
            data: BinanceOrderBookData {
                last_update_time: 0, 
                asks: old_asks.clone(), 
                bids: vec![] 
            }, 
            depth: 4,
        };
        let updated_ticks = vec![
            Tick::new(1891., 1.),
            Tick::new(1892., 0.9),
            Tick::new(1894., 0.21),
        ];
        book.update_asks(updated_ticks.clone());

        assert_eq!(book.data.asks[0], updated_ticks[0]);
        assert_eq!(book.data.asks[1], updated_ticks[1]);
        assert_eq!(book.data.asks[2], old_asks[1]);
        assert_eq!(book.data.asks[3], updated_ticks[2]);
    }

    #[test]
    fn test_update_rm_zero() {
        let old_asks = vec![
            Tick::new(1891., 0.1),
            Tick::new(1893., 3.22),
        ];
        let mut book = BinanceOrderBook {
            data: BinanceOrderBookData {
                last_update_time: 0, 
                asks: old_asks.clone(), 
                bids: vec![] 
            }, 
            depth: 2,
        };
        let updated_ticks = vec![
            Tick::new(1891., 0.),
        ];
        book.update_asks(updated_ticks.clone());

        assert_eq!(book.data.asks.len(), 1);
        assert_eq!(book.data.asks[0], old_asks[1]);
    }

    #[test]
    fn test_update_truncate() {
        let old_asks = vec![
            Tick::new(1891., 0.1),
        ];
        let mut book = BinanceOrderBook {
            data: BinanceOrderBookData {
                last_update_time: 0, 
                asks: old_asks.clone(), 
                bids: vec![] 
            }, 
            depth: 1,
        };
        let updated_ticks = vec![
            Tick::new(1892., 0.9),
        ];
        book.update_asks(updated_ticks.clone());

        assert_eq!(book.data.asks.len(), 1);
        assert_eq!(book.data.asks[0], old_asks[0]);
    }

    #[test]
    fn test_query_exact_base_sell() {
        let book = BinanceOrderBook {
            data: BinanceOrderBookData {
                last_update_time: 0, 
                bids: vec![
                    Tick::new(1890., 1.),
                    Tick::new(1889., 0.21),
                    Tick::new(1888., 3.22),
                ],
                asks: vec![] 
            }, 
            depth: 3,
        };
        let base_amount = 5.;
        let avl_bid_qty = 4.43;
        let target_out = 8366.05;
        let (base_used, quote_out) = book.query_exact_base(
            SwapType::Sell, 
            base_amount
        );
        let allowed_err = 0.00001;
        assert!(base_used-avl_bid_qty < allowed_err);
        assert!(quote_out-target_out < allowed_err);
    }

    #[test]
    fn test_query_exact_base_buy() {
        let book = BinanceOrderBook {
            data: BinanceOrderBookData {
                asks: vec![
                Tick::new(1888., 3.22),
                Tick::new(1889., 0.21),
                Tick::new(1890., 1.),
                ],
                bids: vec![
                    Tick::new(1890., 1.),
                    Tick::new(1889., 0.21),
                    Tick::new(1888., 3.22),
                ],
                last_update_time: 0, 
            },
            depth: 3,
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
            data: BinanceOrderBookData {
                bids: vec![
                    Tick::new(1890., 1.),
                    Tick::new(1889., 0.21),
                    Tick::new(1888., 3.22),
                ],
                asks: vec![], 
                last_update_time: 0, 
            },
            depth: 3,
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
            data: BinanceOrderBookData {
                bids: vec![],
                asks: vec![
                    Tick::new(1888., 3.22),
                    Tick::new(1889., 0.21),
                    Tick::new(1890., 1.),
                ], 
                last_update_time: 0, 
            },
            depth: 3,
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