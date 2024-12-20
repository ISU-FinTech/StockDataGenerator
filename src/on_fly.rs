use std::{
    collections::HashMap, env, error::Error, sync::{Arc, Mutex}, thread::sleep, time::{Duration, Instant}
};
use rand::{thread_rng, Rng};
use rand_distr::{Distribution, Normal};
use reqwest::Client;
use dotenvy::dotenv;
use rayon::{ThreadPoolBuilder, iter::{IntoParallelRefIterator, ParallelIterator}};

use crate::*;

/// Function to generate stock data points on the fly, generating and sending one round takes < 75ms.
/// 
/// # Parameters
/// - `threads`: Number of threads to use.
/// - `total_samples`: Total number of values to generate per stock.
/// 
pub async fn on_fly(threads: usize, total_samples: usize) -> std::io::Result<()> {
    dotenv().ok();
    let api_key: String = env::var("API_KEY").expect("No API key set");
    

    // Fetch stock data
    let client: Client = Client::new();
    let stock_data: StockResponse = match fetch_intraday_data(&client, &api_key).await {
        Ok(data) => data,
        Err(err) => {
            eprintln!("Error fetching stock data: {}", err);
            return Ok(()); 
        }
    };

    let mapper: TickerMapper = TickerMapper::new(&stock_data);

    let data: &[Stock] = &stock_data.results;

    let thread_pool: rayon::ThreadPool = ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .unwrap();

    // Store prices in a HashMap
    let current_prices: Arc<Mutex<HashMap<String, f64>>> = Arc::new(Mutex::new(
        data.iter().map(|stock: &Stock| (stock.T.clone(), stock.o)).collect::<HashMap<String, f64>>(),
    ));

    // Store a normal distribution for each stock
    let normals: HashMap<String, Normal<f64>> = data.iter()
        .filter_map(|stock: &Stock| {
            let normal: Normal<f64> = create_distribution(stock, total_samples);

            Some((stock.T.clone(), normal))
        })
        .collect();

    // We generate one value per stock every 100ms interval
    for i in 1..total_samples - 1 {
        let start_time: Instant = Instant::now();
        let current_prices_clone: Arc<Mutex<HashMap<String, f64>>> = Arc::clone(&current_prices);

        thread_pool.install(|| {
            data.par_iter().for_each(|stock: &Stock| {
                let ticker_name: &String = &stock.T;
                let close: f64 = stock.c;
                let mut rng: rand::prelude::ThreadRng = thread_rng();

                let mut current_prices_guard: std::sync::MutexGuard<'_, HashMap<String, f64>> = current_prices_clone.lock().unwrap();
                let prev_value: f64 = *current_prices_guard.get(ticker_name).unwrap();
                let remaining_steps: f64 = (total_samples - i) as f64;

                if let Some(normal) = normals.get(ticker_name) {
                    let noise: f64 = normal.sample(&mut rng);
                    let correction: f64 = (close - prev_value) / remaining_steps;
                    let new_price: f64 = prev_value + noise + correction;

                    current_prices_guard.insert(ticker_name.clone(), new_price);
                }
            });
        });

        let cloned_data: HashMap<String, f64> = {
            let current_prices_guard: std::sync::MutexGuard<'_, HashMap<String, f64>> = current_prices.lock().unwrap();
            current_prices_guard.clone()
        };

        live_multicast(cloned_data, &mapper).await;

        let elapsed: Duration = start_time.elapsed();

        println!("Simulation completed in {:?}", elapsed);
    }

    let start_time: Instant = Instant::now();

    {
        let mut current_prices_guard: std::sync::MutexGuard<'_, HashMap<String, f64>> = current_prices.lock().unwrap();
        for (_, stock) in data.iter().enumerate() {
            let ticker = &stock.T;
            let close = stock.c; 
    
            current_prices_guard.insert(ticker.clone(), close);
        }
    }

    let cloned_data: HashMap<String, f64> = {
        let current_prices_guard: std::sync::MutexGuard<'_, HashMap<String, f64>> = current_prices.lock().unwrap();
        current_prices_guard.clone()
    };

    live_multicast(cloned_data, &mapper).await;

    let elapsed: Duration = start_time.elapsed();

    println!("Simulation completed in {:?}", elapsed);

    println!("Done");

    Ok(())
}
