use std::collections::HashMap;

use crate::StockResponse;

pub struct TickerMapper {
    ticker_to_number: HashMap<String, u16>,
    number_to_ticker: HashMap<u16, String>,
}

/// Function to create mappings between ticker (5 bytes) and number (2 bytes).
/// 
/// # Parameters
/// - `stock_data`: Data to encode/decode.
/// 
impl TickerMapper {
    pub fn new(stock_data: &StockResponse) -> Self {
        let mut ticker_to_number: HashMap<String, u16> = HashMap::new();
        let mut number_to_ticker: HashMap<u16, String> = HashMap::new();

        for (index, stock) in stock_data.results.iter().enumerate() {
            let ticker: String = stock.T.clone();
            let number: u16 = index as u16;

            ticker_to_number.insert(ticker.clone(), number);
            number_to_ticker.insert(number, ticker);
        }

        TickerMapper {
            ticker_to_number,
            number_to_ticker,
        }
    }

    pub fn encode(&self, ticker: &str) -> Option<u16> {
        self.ticker_to_number.get(ticker).cloned()
    }

    // We need to send the number to ticker hashmap to the client side so they can decode
    pub fn decode(&self, number: u16) -> Option<String> {
        self.number_to_ticker.get(&number).cloned()
    }
}
