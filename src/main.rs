use std::env;
use actix_cors::Cors;
use actix_web::{http::header, web::{self, Data}, App, HttpResponse, HttpServer};
use dotenvy::dotenv;
use reqwest::Client;
use tokio::sync::Mutex;
use std::sync::Arc;

mod preload;
mod on_fly;
mod models;
mod utils;
mod mappings;
mod routes;

use mappings::TickerMapper;
use models::*;
use on_fly::*;
use preload::*;
use routes::get_hash;
use utils::*;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    const NUM_THREADS: usize = 100;
    const SAMPLES_PER_SECOND: usize = 10;
    const TOTAL_SAMPLES: usize = SAMPLES_PER_SECOND * 60 * 60 * 8;

    let api_key = match std::env::var("API_KEY") {
        Ok(key) => key,
        Err(_) => {
            eprintln!("API_KEY is not set in environment variables");
            return Err(std::io::Error::new(std::io::ErrorKind::Other, "API_KEY missing"));
        }
    };

    let client: Client = Client::new();

    let stock_data = match fetch_intraday_data(&client, &api_key).await {
        Ok(data) => data,
        Err(err) => {
            eprintln!("Error fetching stock data: {}", err);
            return Err(std::io::Error::new(std::io::ErrorKind::Other, "Error fetching stock data"));
        }
    };

    let mapper = TickerMapper::new(&stock_data);
    let mapper_clone1 = mapper.clone();  
    let mapper_clone2 = mapper.clone();  

    tokio::spawn({
        let stock_results = stock_data.results; 
        async move {
            if let Err(err) = on_fly(NUM_THREADS, TOTAL_SAMPLES, mapper_clone2, &stock_results).await {
                eprintln!("Error during on_fly execution: {}", err);
            }
        }
    });

    HttpServer::new(move || {
        App::new()
            .wrap(
                Cors::default()
                    .allow_any_origin()
                    .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
                    .allowed_headers(vec![
                        header::AUTHORIZATION,
                        header::CONTENT_TYPE,
                    ])
                    .supports_credentials()
                    .max_age(3600),
            )
            .app_data(Data::new(mapper_clone1.clone()))
            .route("/", web::get().to(|| async { HttpResponse::Ok().body("CORS enabled!") }))
            .service(get_hash)
    })
    .bind(("127.0.0.1", 3000))?
    .run()
    .await
}

