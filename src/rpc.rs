use std::sync::Mutex;
use actix_web::body::BoxBody;
use actix_web::{HttpRequest, HttpResponse, post, Responder, web};
use actix_web::http::header::ContentType;
use actix_web::web::{Data, Json};
use serde::{Deserialize, Serialize};
use crate::kvservice::KVService;

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
async fn index(svc: Data<Mutex<KVService>>, req: HttpRequest, json_request: web::Json<JSONRPCRequest>) -> JSONRPCResponse {
    handle(svc, &json_request).await
}

async fn handle(svc: Data<Mutex<KVService>>, json_request: &Json<JSONRPCRequest>) -> JSONRPCResponse {
    match json_request.method.as_str() {
        "get" => handle_get(&svc, &json_request).await,
        "set" => handle_set(&svc, &json_request).await,
        _ => rpc_error(json_request.id, -32601, "Method not available")
    }
}

async fn handle_set(svc: &Data<Mutex<KVService>>, json_request: &&Json<JSONRPCRequest>) -> JSONRPCResponse {
    if json_request.params.len() != 2 {
        return rpc_error(json_request.id, -32602, "Two parameters are required for set function")
    }
    let result = svc.lock().unwrap().set(&json_request.params).await;
    if let Ok(value) = result {
        return JSONRPCResponse::Ok(JSONRPCOkResponse {
            jsonrpc: "2.0".to_string(),
            result: value,
            id: json_request.id,
        });
    }
    rpc_error(json_request.id, -32603, result.err().unwrap().as_str())
}

async fn handle_get(svc: &Data<Mutex<KVService>>, json_request: &&Json<JSONRPCRequest>) -> JSONRPCResponse {
    if json_request.params.len() != 1 {
        return rpc_error(json_request.id, -32602, "One parameter is required for set function")
    }

    JSONRPCResponse::Ok(JSONRPCOkResponse {
        jsonrpc: "2.0".to_string(),
        result: svc.lock().unwrap().get(&json_request.params).await.ok().unwrap_or("ERR".to_string()),
        id: json_request.id,
    })
}
