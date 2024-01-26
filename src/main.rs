use std::sync::Mutex;

use actix_web::{App, HttpServer};
use actix_web::web::Data;
use redis::Client;

mod kvservice;
mod rpc;

#[actix_web::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::open("redis://0.0.0.0:6379/")?;
    let conn = client.get_multiplexed_async_connection().await.unwrap();
    let kv_service = kvservice::KVService::new(conn);

    HttpServer::new(move || App::new().app_data(Data::new(Mutex::new(kv_service.clone()))).service(rpc::index))
        .bind(("0.0.0.0", 8080))?
        .run()
        .await?;
    Ok(())
}