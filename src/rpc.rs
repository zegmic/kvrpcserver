use std::process::Command;
use std::sync::Mutex;
use actix_web::body::BoxBody;
use actix_web::{HttpRequest, HttpResponse, post, Responder, web};
use actix_web::http::header::ContentType;
use actix_web::web::{Data, Json};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;
use tokio::sync::oneshot;
use crate::{rate_limiting, storage};
use crate::storage::KVService;


#[derive(Deserialize)]
struct JSONRPCRequest {
    jsonrpc: String,
    method: String,
    id: i32,
    params: Vec<String>,
}

#[derive(Serialize)]
struct JSONRPCOkResponse {
    jsonrpc: String,
    result: String,
    id: i32,
}

#[derive(Serialize)]
struct JSONRPCErrorDetails {
    code: i32,
    message: String,
}

#[derive(Serialize)]
struct JSONRPCErrorResponse {
    jsonrpc: String,
    error: JSONRPCErrorDetails,
    id: i32,
}

enum JSONRPCResponse {
    Ok(JSONRPCOkResponse),
    Err(JSONRPCErrorResponse),
}

impl Responder for JSONRPCResponse {
    type Body = BoxBody;

    fn respond_to(self, _req: &HttpRequest) -> HttpResponse<Self::Body> {
        let body = match &self {
            JSONRPCResponse::Ok(resp) => serde_json::to_string(resp).unwrap(),
            JSONRPCResponse::Err(resp) => serde_json::to_string(resp).unwrap()
        };

        HttpResponse::Ok()
            .content_type(ContentType::json())
            .body(body)
    }
}

fn rpc_error(id: i32, code: i32, message: &str) -> JSONRPCResponse {
    JSONRPCResponse::Err(JSONRPCErrorResponse {
        jsonrpc: "2.0".to_string(),
        error: JSONRPCErrorDetails { code, message: message.to_string() },
        id,
    })
}

#[post("/")]
async fn index(storage_tx: Data<Sender<storage::Command>>, rate_limiting: Data<Mutex<rate_limiting::Service>>, req: HttpRequest, json_request: Json<JSONRPCRequest>) -> JSONRPCResponse {
    if rate_limiting.lock().unwrap().limit_reached(req.connection_info().realip_remote_addr().unwrap_or("noip")).await {
        return rpc_error(json_request.id, 100429, "Rate limit reached");
    }

    handle(storage_tx.get_ref(), &json_request).await
}

async fn handle(tx: &Sender<storage::Command>, json_request: &Json<JSONRPCRequest>) -> JSONRPCResponse {
    match json_request.method.as_str() {
        "get" => handle_get(tx, &json_request).await,
        "set" => handle_set(tx, &json_request).await,
        _ => rpc_error(json_request.id, -32601, "Method not available")
    }
}

async fn handle_set(tx: &Sender<storage::Command>, json_request: &&Json<JSONRPCRequest>) -> JSONRPCResponse {
    if json_request.params.len() != 2 {
        return rpc_error(json_request.id, -32602, "Two parameters are required for set function")
    }

    let (res_tx, res_rx) = oneshot::channel();
    let command = storage::Command::Set {
        key: json_request.params.get(0).unwrap().to_string(),
        value: json_request.params.get(1).unwrap().to_string(),
        res: res_tx
    };
    tx.send(command).await.unwrap();
    match res_rx.await.unwrap() {
        Ok(_) => JSONRPCResponse::Ok(JSONRPCOkResponse {
            jsonrpc: "2.0".to_string(),
            result: "value inserted".to_string(),
            id: json_request.id,
        }),
        Err(err) => rpc_error(json_request.id, -32603, err.to_string().as_str())
    }
}

async fn handle_get(tx: &Sender<storage::Command>, json_request: &&Json<JSONRPCRequest>) -> JSONRPCResponse {
    if json_request.params.len() != 1 {
        return rpc_error(json_request.id, -32602, "One parameter is required for set function")
    }

    let (res_tx, res_rx) = oneshot::channel();
    let command = storage::Command::Get {
        key: json_request.params.get(0).unwrap().to_string(),
        res: res_tx
    };
    tx.send(command).await.unwrap();

    match res_rx.await.unwrap() {
        Ok(res) => JSONRPCResponse::Ok(JSONRPCOkResponse {
            jsonrpc: "2.0".to_string(),
            result: res,
            id: json_request.id,
        }),
        Err(err) => rpc_error(json_request.id, -32603, err.to_string().as_str())
    }
}
