use std::{collections::HashMap, env, sync::Arc, time::{Duration, Instant}};
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

    // fetch data
    let client: Client = Client::new();
    let stock_data: StockResponse = match fetch_intraday_data(&client, &api_key).await {
        Ok(data) => data,
        Err(err) => {
            eprintln!("Error fetching stock data: {}", err);
            return Ok(()); 
        }
    };

    let data: &[Stock] = &stock_data.results;

    let thread_pool: rayon::ThreadPool = rayon::ThreadPoolBuilder::new().num_threads(NUM_THREADS).build().unwrap();
    let stocks: Arc<Mutex<HashMap<String, Vec<f64>>>> = Arc::new(Mutex::new(HashMap::new()));

    thread_pool.install(|| {
        data.par_iter().for_each(|stock: &Stock| {
            /*
                One thread does the entire simulation of a stock
             */
            let start_time: Instant = Instant::now();
            let ticker_name: String = stock.T.clone();
            let open: f64 = stock.o;
            let close: f64 = stock.c;
            let total_samples: usize = TOTAL_SAMPLES;

            let mut values: Vec<f64> = vec![0.0; total_samples];
            values[0] = open;

            let percent_change: f64 = (close - open) / open;
            let random_factor: f64 = (rand::random::<f64>() * 1.5).max(0.5) + (open * 0.35);
            let volatility: f64 = (percent_change * random_factor) / (total_samples as f64).sqrt();

            let normal: Normal<f64> = Normal::new(0.0, volatility).unwrap();
            let mut rng: rand::prelude::ThreadRng = rand::thread_rng();

            for i in 1..total_samples - 1 {
                let prev_price: f64 = values[i - 1];
                let remaining_steps: f64 = (total_samples - i) as f64;

                let noise: f64 = normal.sample(&mut rng);
                let correction: f64 = (close - prev_price) / remaining_steps;
                let new_price: f64 = prev_price + noise + correction;

                values[i] = new_price;
            }

            values[total_samples - 1] = close;

            let mut stocks_lock: tokio::sync::MutexGuard<'_, HashMap<String, Vec<f64>>> = stocks.blocking_lock();
            stocks_lock.insert(ticker_name.clone(), values);

            let elapsed: Duration = start_time.elapsed();
            println!("Completed Simulation for {:?} in {:?}", ticker_name, elapsed);
        });
    });

    let cloned_data: HashMap<String, Vec<f64>> = {
        let current_prices_guard: tokio::sync::MutexGuard<'_, HashMap<String, Vec<f64>>> = stocks.lock().await; 
        current_prices_guard.clone()
    };

    send_preload(cloned_data);

    Ok(())
}
