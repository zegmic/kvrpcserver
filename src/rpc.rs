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
    Err(JSONRPCErrorResponse)
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


#[post("/")]
async fn index(svc: web::Data<Mutex<KVService>>, req: HttpRequest, json_request: web::Json<JSONRPCRequest>) ->  JSONRPCResponse {
    println!("{}", req.connection_info().realip_remote_addr().unwrap());
    handle(svc, &json_request).await
}

async fn handle(svc: Data<Mutex<KVService>>, json_request: &Json<JSONRPCRequest>) -> JSONRPCResponse {
    match json_request.method.as_str() {
        "get" => JSONRPCResponse::Ok(JSONRPCOkResponse {
            jsonrpc: "2.0".to_string(),
            result: svc.lock().unwrap().get(&json_request.params).await.ok().unwrap_or("ERR".to_string()),
            id: json_request.id,
        }),
        "set" => JSONRPCResponse::Ok(JSONRPCOkResponse {
            jsonrpc: "2.0".to_string(),
            result: svc.lock().unwrap().set(&json_request.params).await.ok().unwrap_or("ERR".to_string()),
            id: json_request.id
        }),
        _ => JSONRPCResponse::Err(JSONRPCErrorResponse {
            jsonrpc: "2.0".to_string(),
            id: json_request.id,
            error: JSONRPCErrorDetails {
                code: -32601,
                message: "Method not available".to_string(),
            }
        })
    }
}
