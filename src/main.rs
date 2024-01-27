use std::env;
use std::time::Duration;

use actix_web::{App, HttpServer};
use actix_web::web::Data;
use redis::Client;

mod storage;
mod rpc;
mod rate_limiting;

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    let redis_url = env::var("REDIS_URL").unwrap();
    let client = Client::open(redis_url)?;
    let conn = client.get_multiplexed_async_connection().await?;

    let kv_service = storage::KVService::new(conn.clone());
    let kv_tx = kv_service.run();

    let rate_limiting = rate_limiting::Service::new(Duration::from_secs(1), 1, conn.clone());
    let rate_limiting_tx = rate_limiting.run();

    HttpServer::new(move || App::new()
        .app_data(Data::new(kv_tx.clone()))
        .app_data(Data::new(rate_limiting_tx.clone()))
        .service(rpc::index))
        .bind(("0.0.0.0", 8080))?
        .run()
        .await?;
    Ok(())
}