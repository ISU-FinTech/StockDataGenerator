use actix_web::{get, web, HttpResponse, Responder};
use serde_json::json;
use tokio::sync::Mutex;

use crate::{mappings::TickerMapper, Stock};

#[get("/gethash")]
async fn get_hash(mapper: web::Data<TickerMapper>) -> impl Responder {
    let number_to_ticker = &mapper.number_to_ticker;

    let json_data = json!(number_to_ticker);

    HttpResponse::Ok().json(json_data)
}


