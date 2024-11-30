use actix_cors::Cors;
use actix_web::{App, HttpServer};

mod preload;
mod on_fly;
mod models;
use models::*;
use on_fly::*;
use preload::*;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Alternate between preload and on_fly

    //preload().await;
    on_fly().await;

    HttpServer::new(|| {
        App::new().wrap(Cors::default())
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
