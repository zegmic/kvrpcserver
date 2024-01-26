use std::sync::Mutex;
use std::time::Duration;

use actix_web::{App, HttpServer};
use actix_web::web::Data;
use redis::Client;

mod storage;
mod rpc;
mod rate_limiting;

#[actix_web::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::open("redis://0.0.0.0:6379/")?;
    let conn = client.get_multiplexed_async_connection().await.unwrap();
    let kv_service = storage::KVService::new(conn);
    let rate_limiting = rate_limiting::Service::new(Duration::from_secs(10), 1);

    HttpServer::new(move || App::new()
        .app_data(Data::new(Mutex::new(kv_service.clone())))
        .app_data(Data::new(Mutex::new(rate_limiting.clone())))
        .service(rpc::index))
        .bind(("0.0.0.0", 8080))?
        .run()
        .await?;
    Ok(())
}