use rayon::prelude::*;
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use rand::Rng;
use rand_distr::{Normal, Distribution};
use std::collections::HashMap;

use crate::mappings::TickerMapper;
use crate::models::Stock;
use crate::utils::{create_distribution, live_multicast};

pub async fn on_fly(
    threads: usize, 
    total_samples: usize, 
    mapper: TickerMapper, 
    data: &[Stock]
) -> std::io::Result<()> {
    let thread_pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .unwrap();

    let current_prices: Arc<Mutex<HashMap<String, f64>>> = Arc::new(
        Mutex::new(data.iter().map(|stock| (stock.T.clone(), stock.o)).collect::<HashMap<String, f64>>())
    );

    let normals: HashMap<String, Normal<f64>> = data.iter()
        .filter_map(|stock| {
            let normal = create_distribution(stock, total_samples);
            Some((stock.T.clone(), normal))
        })
        .collect();

    for i in 1..total_samples - 1 {
        let start_time: Instant = Instant::now();
        let current_prices_clone: Arc<Mutex<HashMap<String, f64>>> = Arc::clone(&current_prices);

        thread_pool.install(|| {
            data.par_iter().for_each(|stock| {
                let ticker_name = &stock.T;
                let close = stock.c;
                let mut rng = rand::thread_rng();

                let mut current_prices_guard = current_prices_clone.lock().unwrap();
                let prev_value = *current_prices_guard.get(ticker_name).unwrap();
                let remaining_steps = (total_samples - i) as f64;

                if let Some(normal) = normals.get(ticker_name) {
                    let noise = normal.sample(&mut rng);
                    let correction = (close - prev_value) / remaining_steps;
                    let new_price = prev_value + noise + correction;

                    current_prices_guard.insert(ticker_name.clone(), new_price);
                }
            });
        });

        let cloned_data = {
            let current_prices_guard = current_prices.lock().unwrap();
            current_prices_guard.clone()
        };

        live_multicast(cloned_data, mapper.clone()).await;

        let elapsed = start_time.elapsed();
        println!("Simulation completed in {:?}", elapsed);
    }

    let start_time: Instant = Instant::now();

    {
        let mut current_prices_guard = current_prices.lock().unwrap();
        for stock in data.iter() {
            let ticker = &stock.T;
            let close = stock.c;
            current_prices_guard.insert(ticker.clone(), close);
        }
    }

    let cloned_data = {
        let current_prices_guard = current_prices.lock().unwrap();
        current_prices_guard.clone()
    };

    live_multicast(cloned_data, mapper.clone()).await;

    let elapsed = start_time.elapsed();
    println!("Simulation completed in {:?}", elapsed);

    println!("Done");

    Ok(())
}
