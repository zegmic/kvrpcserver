mod kvservice;

use std::sync::{Arc, Mutex};
use actix_web::{web, post, App, HttpRequest, HttpServer, Responder};
use serde::Deserialize;
use redis::{AsyncCommands, Client};
use crate::kvservice::KVService;


#[derive(Deserialize)]
struct JsonRPCRequest {
    jsonrpc: String,
    method: String,
    id: i32,
    params: Vec<String>
}

#[post("/")]
async fn index(svc: web::Data<Mutex<KVService>>, req: HttpRequest, json_request: web::Json<JsonRPCRequest>) -> impl Responder {
    println!("{}", req.connection_info().realip_remote_addr().unwrap());
    match json_request.method.as_str() {
        "get" => {
            svc.lock().unwrap().get(&json_request.params).await.ok().unwrap_or("ERR".to_string())
        },
        "set" => {
            svc.lock().unwrap().set(&json_request.params).await.ok().unwrap_or("ERR".to_string())
        },
        _ => "ERR".to_string()
    }
}


#[actix_web::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::open("redis://0.0.0.0:6379/")?;
    let conn = client.get_multiplexed_async_connection().await.unwrap();
    let mut kv_service = KVService::new(conn);

    HttpServer::new(move || App::new().app_data(web::Data::new(Mutex::new(kv_service.clone()))).service(index))
        .bind(("0.0.0.0", 8080))?
        .run()
        .await?;
    Ok(())
}