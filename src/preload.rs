use std::{env, error::Error, sync::Arc};
use rand::thread_rng;
use rand_distr::{Distribution, Normal};
use reqwest::Client;
use dotenvy::dotenv;
use rayon::{iter::{IntoParallelRefIterator, ParallelIterator}, ThreadPoolBuilder};
use tokio::sync::Mutex;

use crate::*;

pub async fn preload() -> std::io::Result<()> {
    dotenv().ok();
    let api_key = env::var("API_KEY").expect("No API key set");
    const NUM_THREADS: usize = 100;
    const SAMPLES_PER_SECOND: usize = 10;
    const TOTAL_SAMPLES: usize = SAMPLES_PER_SECOND * 60 * 60 * 8;

    // Fetch Stock Data
    let client: Client = Client::new();
    let stock_data: StockResponse = match fetch_intraday_data(&client, &api_key).await {
        Ok(data) => data,
        Err(err) => {
            eprintln!("Error fetching stock data: {}", err);
            return Ok(()); 
        }
    };

    let data: &[Stock] = &stock_data.results;

    let thread_pool: rayon::ThreadPool = ThreadPoolBuilder::new().num_threads(NUM_THREADS).build().unwrap();
    let stocks: Arc<Mutex<Vec<ProcessedStock>>> = Arc::new(Mutex::new(Vec::new()));

    thread_pool.install(|| {
        data.par_iter().for_each(|stock: &Stock| {
            /*
                Each thread will fully simulate the values for each stock
             */
            let ticker_name: String = stock.T.clone();
            let open: f64 = stock.o;
            let close: f64 = stock.c;
            let total_samples: usize = TOTAL_SAMPLES;

            let mut values: Vec<f64> = vec![0.0; total_samples];
            values[0] = open;
        
            let percent_change: f64 = (close - open) / open;
            let random_factor: f64 = (rand::random::<f64>() * 1.5).max(0.5) + (open * 0.35);
            let volatility: f64 = (percent_change * random_factor) / (total_samples as f64).sqrt();
        
            // Compute Normal Distribution with previously calculates statistics
            let normal: Normal<f64> = Normal::new(0.0, volatility).unwrap();
            let mut rng: rand::prelude::ThreadRng = thread_rng();
        
            // Generate the new samples
            for i in 1..total_samples - 1 {
                let prev_price: f64 = values[i - 1];
                let remaining_steps: f64 = (total_samples - i) as f64;
        
                let noise: f64 = normal.sample(&mut rng);
                let correction: f64 = (close - prev_price) / remaining_steps;
                let new_price: f64 = prev_price + noise + correction;
        
                values[i] = new_price;
            }
        
            values[total_samples - 1] = close;

            // If other threads try to block stocks when another thread is still working they will be added to queue
            let mut stocks_lock: tokio::sync::MutexGuard<'_, Vec<ProcessedStock>> = stocks.blocking_lock();
            let processed_stock: ProcessedStock = ProcessedStock {
                T: ticker_name.clone(),
                prices: values,
            };

            stocks_lock.push(processed_stock);
            println!("Completed Simulation for {:?}", ticker_name);
        });
    });

    // Send data

    Ok(())
}

async fn fetch_intraday_data(client: &Client, api_key: &str) -> Result<StockResponse, Box<dyn Error>> {
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

    println!("{:?}", stock_data);

    Ok(stock_data)
}
