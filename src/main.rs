use actix_cors::Cors;
use actix_web::{App, HttpServer};

mod preload;
mod on_fly;
mod models;
mod utils;

use models::*;
use on_fly::*;
use preload::*;
use utils::*;

/// Main function.
#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Alternate between preload and on_fly
    const NUM_THREADS: usize = 100;
    const SAMPLES_PER_SECOND: usize = 10;
    const TOTAL_SAMPLES: usize = SAMPLES_PER_SECOND * 60 * 60 * 8;

    //preload(NUM_THREADS, TOTAL_SAMPLES).await;
    on_fly(NUM_THREADS, TOTAL_SAMPLES).await;

    HttpServer::new(|| {
        App::new().wrap(Cors::default())
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
