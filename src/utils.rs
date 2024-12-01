use std::{collections::HashMap, error::Error, net::SocketAddr};
use reqwest::Client;
use tokio::net::UdpSocket;
use crate::StockResponse;
use serde_json;

pub async fn fetch_intraday_data(client: &Client, api_key: &str) -> Result<StockResponse, Box<dyn Error>> {
    const DATE: &str = "2024-11-26";

    let url: String = format!(
        "https://api.polygon.io/v2/aggs/grouped/locale/us/market/stocks/{}?adjusted=true&apiKey={}",
        DATE,
        api_key
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

pub async fn live_multicast(data: HashMap<String, f64>) -> std::io::Result<()> {
    const ADDRESS: &str = "239.0.0.1";
    const PORT: u16 = 6000;

    println!("Starting multicast function");

    let message: String = match serde_json::to_string(&data) {
        Ok(msg) => msg,
        Err(err) => {
            eprintln!("Message error: {}", err);
            return Err(std::io::Error::new(std::io::ErrorKind::Other, "Message failed"));
        }
    };

    println!("Message size: {} bytes", message.len());

    let multicast_addr: SocketAddr = match format!("{}:{}", ADDRESS, PORT).parse() {
        Ok(addr) => addr,
        Err(err) => {
            eprintln!("Address error: {}", err);
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid address"));
        }
    };

    let socket: UdpSocket = match UdpSocket::bind("0.0.0.0:0").await {
        Ok(sock) => sock,
        Err(err) => {
            eprintln!("Socket error: {}", err);
            return Err(err);
        }
    };

    /*
        Here is a problem this for loop take ~50-70 ms which greatly slows down each simulation to ~130-150 ms
        Need to find a away to speed up this operation
    */
    for (stock_name, stock_value) in data {
        let stock_message: String = serde_json::to_string(&(stock_name.clone(), stock_value)).unwrap();
        let stock_message_bytes: &[u8] = stock_message.as_bytes();
        
        socket.send_to(stock_message_bytes, &multicast_addr).await?;
        //println!("Sent stock: '{}' with value: {}", stock_name, stock_value);
    }

    println!("Multicast function completed");

    Ok(())
}


pub async fn send_preload(data: HashMap<String, Vec<f64>>) -> std::io::Result<()> {
    // not sure how to do this, maybe chunking up the packets

    Ok(())
}
