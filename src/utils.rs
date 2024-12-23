use crate::{mappings::{TickerMapper, self}, Stock, StockMessage, StockResponse};
use rand::thread_rng;
use rand_distr::Normal;
use reqwest::Client;
use serde_json;
use std::{
    collections::HashMap,
    error::Error,
    net::SocketAddr,
    time::{Instant, SystemTime, UNIX_EPOCH}, sync::Arc,
};
use tokio::io::AsyncWriteExt;
use tokio::net::UdpSocket;

/// Fetches the stock data.
///
/// # Parameters
/// - `client`: HTTP client.
/// - `api_key`: Key for polygon api.
///
pub async fn fetch_intraday_data(
    client: &Client,
    api_key: &str,
) -> Result<StockResponse, Box<dyn Error>> {
    const DATE: &str = "2024-11-26";

    let url: String = format!(
        "https://api.polygon.io/v2/aggs/grouped/locale/us/market/stocks/{}?adjusted=true&apiKey={}",
        DATE, api_key
    );

    let response: reqwest::Response = client.get(&url).send().await?;

    if !response.status().is_success() {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("API request failed with status: {}", response.status()),
        )));
    }

    let response_text: String = response.text().await?;
    let stock_data: StockResponse = serde_json::from_str(&response_text)?;

    println!("Completed fetching data");

    Ok(stock_data)
}

/// Broadcasts stock data using UDP Multicast.
///
/// # Parameters
/// - `data`: Hashmap of ticker and values.
/// - `ticker_mapper`: Mappings between tickers and numbers.
///
pub async fn live_multicast(
    data: HashMap<String, f64>, 
    ticker_mapper: TickerMapper,
) -> std::io::Result<()> {
    const ADDRESS: &str = "239.0.0.1";
    const PORT: u16 = 6000;

    let multicast_addr: SocketAddr = format!("{}:{}", ADDRESS, PORT)
        .parse()
        .expect("Invalid address");

    let socket = UdpSocket::bind("0.0.0.0:0")
        .await
        .expect("Failed to bind socket");

    let mut buffer: Vec<u8> = Vec::new();
    let mut stock_group: Vec<(u16, f64)> = Vec::new();
    let mut count: i32 = 0;

    let start: SystemTime = SystemTime::now();
    let duration: std::time::Duration = start.duration_since(UNIX_EPOCH).unwrap();
    let timestamp: u64 = duration.as_millis() as u64;

    for (stock_name, stock_value) in data.iter() {
        if let Some(encoded_ticker) = ticker_mapper.encode(&stock_name) {
            stock_group.push((encoded_ticker, *stock_value));
        } else {
            eprintln!("Failed to encode ticker: {}", stock_name);
            continue;
        }

        count += 1;

        if count == 100 {
            build_packet(timestamp, &stock_group, &mut buffer);
            socket.send_to(&buffer, multicast_addr).await?;
            buffer.clear();
            stock_group.clear();
            count = 0;
        }
    }

    if !stock_group.is_empty() {
        build_packet(timestamp, &stock_group, &mut buffer);
        socket.send_to(&buffer, multicast_addr).await?;
    }

    Ok(())
}


/// Send the preloaded data to the client.
///
/// # Parameters
/// - `data`: Hashmap with stock name and array of prices.
///
pub async fn send_preload(data: HashMap<String, Vec<f64>>) -> std::io::Result<()> {
    // not sure how to do this, maybe chunking up the packets

    Ok(())
}

/// Function to generate a normal distribution.
///
/// # Parameters
/// - `stock`: Information related to the stock.
/// - `samples`: Number of samples to take.
///
pub fn create_distribution(stock: &Stock, samples: usize) -> Normal<f64> {
    let percent_change: f64 = (stock.c - stock.o) / stock.o;

    let random_factor: f64 = (rand::random::<f64>() * 1.5).max(0.5) + (stock.o * 0.35);

    let volatility: f64 = (percent_change * random_factor) / (samples as f64).sqrt();

    Normal::new(0.0, volatility).unwrap()
}

/// Build packet for multicast.
///
/// # Parameters
/// - `timestamp`: Timestamp of the generate price.
/// - `stocks`: Array of the numbers and related prices.
/// - `buffer`: Buffer to store value to send
///
fn build_packet(timestamp: u64, stocks: &[(u16, f64)], buffer: &mut Vec<u8>) {
    buffer.extend_from_slice(&timestamp.to_be_bytes());

    for (ticker, price) in stocks {
        buffer.extend_from_slice(&ticker.to_be_bytes());
        buffer.extend_from_slice(&price.to_be_bytes());
    }
}
