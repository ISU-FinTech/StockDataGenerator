use std::{
    collections::HashMap, env, error::Error, sync::{Arc, Mutex}, thread::sleep, time::{Duration, Instant}
};
use rand::{thread_rng, Rng};
use rand_distr::{Distribution, Normal};
use reqwest::Client;
use dotenvy::dotenv;
use rayon::{ThreadPoolBuilder, iter::{IntoParallelRefIterator, ParallelIterator}};

use crate::*;

pub async fn on_fly() -> std::io::Result<()> {
    dotenv().ok();
    let api_key: String = env::var("API_KEY").expect("No API key set");
    const NUM_THREADS: usize = 100;
    const SAMPLES_PER_SECOND: usize = 10;
    const TOTAL_SAMPLES: usize = SAMPLES_PER_SECOND * 60 * 60 * 8;

    // Fetch stock data
    let client: Client = Client::new();
    let stock_data: StockResponse = match fetch_intraday_data(&client, &api_key).await {
        Ok(data) => data,
        Err(err) => {
            eprintln!("Error fetching stock data: {}", err);
            return Ok(()); 
        }
    };

    let data: &[Stock] = &stock_data.results;

    let thread_pool: rayon::ThreadPool = ThreadPoolBuilder::new()
        .num_threads(NUM_THREADS)
        .build()
        .unwrap();

    // Store prices in a HashMap
    let current_prices: Arc<Mutex<HashMap<String, f64>>> = Arc::new(Mutex::new(
        data.iter().map(|stock: &Stock| (stock.T.clone(), stock.o)).collect::<HashMap<String, f64>>(),
    ));

    // Store a normal distribution for each stock
    let normals: HashMap<String, Normal<f64>> = data.iter()
        .filter_map(|stock: &Stock| {
            let percent_change: f64 = (stock.c - stock.o) / stock.o;
            let random_factor: f64 = (thread_rng().gen::<f64>() * 1.5).max(0.5) + (stock.o * 0.35);
            let volatility: f64 = (percent_change * random_factor) / (TOTAL_SAMPLES as f64).sqrt();

            Normal::new(0.0, volatility).ok().map(|normal: Normal<f64>| (stock.T.clone(), normal))
        })
        .collect();

    // We generate one value per stock every 100ms interval
    for i in 1..TOTAL_SAMPLES - 1 {
        let start_time: Instant = Instant::now();
        let current_prices_clone: Arc<Mutex<HashMap<String, f64>>> = Arc::clone(&current_prices);

        thread_pool.install(|| {
            data.par_iter().for_each(|stock: &Stock| {
                let ticker_name: &String = &stock.T;
                let close: f64 = stock.c;
                let mut rng: rand::prelude::ThreadRng = thread_rng();

                let mut current_prices_guard: std::sync::MutexGuard<'_, HashMap<String, f64>> = current_prices_clone.lock().unwrap();
                let prev_value: f64 = *current_prices_guard.get(ticker_name).unwrap();
                let remaining_steps: f64 = (TOTAL_SAMPLES - i) as f64;

                if let Some(normal) = normals.get(ticker_name) {
                    let noise: f64 = normal.sample(&mut rng);
                    let correction: f64 = (close - prev_value) / remaining_steps;
                    let new_price: f64 = prev_value + noise + correction;

                    current_prices_guard.insert(ticker_name.clone(), new_price);
                }
            });
        });

        let elapsed: Duration = start_time.elapsed();

        if elapsed < Duration::from_millis(100) {
            sleep(Duration::from_millis(100) - elapsed);
        }

        println!("Simulation {:?} Completed in {:?}", i, elapsed);

        // Jackson UDP send
    }

    {
        let mut current_prices_guard: std::sync::MutexGuard<'_, HashMap<String, f64>> = current_prices.lock().unwrap();
        for (_, stock) in data.iter().enumerate() {
            let ticker = &stock.T;
            let close = stock.c; 
    
            current_prices_guard.insert(ticker.clone(), close);
        }
    }

    // Final Jackson UDP send

    println!("Done");

    Ok(())
}

async fn fetch_intraday_data(client: &Client, api_key: &str) -> Result<StockResponse, Box<dyn Error>> {
    const DATE: &str = "2024-11-26";

    let url: String = format!(
        "https://api.polygon.io/v2/aggs/grouped/locale/us/market/stocks/{}?adjusted=true&apiKey={}",
        DATE,
        api_key
    );

    let response = client.get(&url).send().await?;
    
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
