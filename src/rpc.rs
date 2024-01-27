use actix_web::{HttpRequest, HttpResponse, post, Responder};
use actix_web::body::BoxBody;
use actix_web::http::header::ContentType;
use actix_web::web::{Data, Json};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;
use tokio::sync::oneshot;

use crate::{rate_limiting, storage};

#[derive(Serialize, Deserialize)]
struct JSONRPCRequest {
    #[serde(rename = "jsonrpc")]
    _json_rpc: String,
    method: String,
    id: i32,
    params: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct JSONRPCOkResponse {
    #[serde(rename = "jsonrpc")]
    json_rpc: String,
    result: String,
    id: i32,
}

#[derive(Serialize, Deserialize)]
struct JSONRPCErrorDetails {
    code: i32,
    message: String,
}

#[derive(Serialize, Deserialize)]
struct JSONRPCErrorResponse {
    #[serde(rename = "jsonrpc")]
    json_rpc: String,
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
        json_rpc: "2.0".to_string(),
        error: JSONRPCErrorDetails { code, message: message.to_string() },
        id,
    })
}

#[post("/")]
async fn index(storage_tx: Data<Sender<storage::Command>>, rate_limiting_tx: Data<Sender<rate_limiting::Command>>, req: HttpRequest, json_request: Json<JSONRPCRequest>) -> JSONRPCResponse {
    let (res_tx, res_rx) = oneshot::channel();
    let command = rate_limiting::Command::LimitReached {
        ip: req.connection_info().realip_remote_addr().unwrap_or("noip").to_string(),
        res: res_tx,
    };

    rate_limiting_tx.send(command).await.unwrap();
    if res_rx.await.unwrap() {
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
        return rpc_error(json_request.id, -32602, "Two parameters are required for set function");
    }

    let (res_tx, res_rx) = oneshot::channel();
    let command = storage::Command::Set {
        key: json_request.params.first().unwrap().to_string(),
        value: json_request.params.get(1).unwrap().to_string(),
        res: res_tx,
    };
    tx.send(command).await.unwrap();
    match res_rx.await.unwrap() {
        Ok(_) => JSONRPCResponse::Ok(JSONRPCOkResponse {
            json_rpc: "2.0".to_string(),
            result: "value inserted".to_string(),
            id: json_request.id,
        }),
        Err(err) => rpc_error(json_request.id, -32603, err.to_string().as_str())
    }
}

async fn handle_get(tx: &Sender<storage::Command>, json_request: &&Json<JSONRPCRequest>) -> JSONRPCResponse {
    if json_request.params.len() != 1 {
        return rpc_error(json_request.id, -32602, "One parameter is required for set function");
    }

    let (res_tx, res_rx) = oneshot::channel();
    let command = storage::Command::Get {
        key: json_request.params.first().unwrap().to_string(),
        res: res_tx,
    };
    tx.send(command).await.unwrap();

    match res_rx.await.unwrap() {
        Ok(res) => JSONRPCResponse::Ok(JSONRPCOkResponse {
            json_rpc: "2.0".to_string(),
            result: res,
            id: json_request.id,
        }),
        Err(err) => rpc_error(json_request.id, -32603, err.to_string().as_str())
    }
}


#[cfg(test)]
mod tests {
    use actix_web::{test, App};
    use super::*;

    #[actix_web::test]
    async fn test_valid_message_post() {
        let (kv_tx, limit_tx) = prepare_deps("123".to_string(), false);

        let app = test::init_service(App::new()
            .app_data(Data::new(kv_tx.clone()))
            .app_data(Data::new(limit_tx.clone()))
            .service(index)).await;

        let req = test::TestRequest::post().uri("/").set_json(JSONRPCRequest {
            _json_rpc: "2.0".to_string(),
            method: "get".to_string(),
            id: 11,
            params: vec!["mykey".to_string()],
        }).to_request();
        let resp: JSONRPCOkResponse = test::call_and_read_body_json(&app, req).await;
        assert_eq!(resp.id, 11);
        assert_eq!(resp.result, "123");
    }

    #[actix_web::test]
    async fn test_non_existing_message() {
        let (kv_tx, limit_tx) = prepare_deps("123".to_string(), false);

        let app = test::init_service(App::new()
            .app_data(Data::new(kv_tx.clone()))
            .app_data(Data::new(limit_tx.clone()))
            .service(index)).await;

        let req = test::TestRequest::post().uri("/").set_json(JSONRPCRequest {
            _json_rpc: "2.0".to_string(),
            method: "del".to_string(),
            id: 11,
            params: vec!["mykey".to_string()],
        }).to_request();
        let resp: JSONRPCErrorResponse = test::call_and_read_body_json(&app, req).await;
        assert_eq!(resp.error.code, -32601);
    }

    #[actix_web::test]
    async fn test_invalid_args() {
        let (kv_tx, limit_tx) = prepare_deps("123".to_string(), false);

        let app = test::init_service(App::new()
            .app_data(Data::new(kv_tx.clone()))
            .app_data(Data::new(limit_tx.clone()))
            .service(index)).await;

        let req = test::TestRequest::post().uri("/").set_json(JSONRPCRequest {
            _json_rpc: "2.0".to_string(),
            method: "get".to_string(),
            id: 11,
            params: vec![],
        }).to_request();
        let resp: JSONRPCErrorResponse = test::call_and_read_body_json(&app, req).await;
        assert_eq!(resp.error.code, -32602);
    }

    #[actix_web::test]
    async fn test_limit_reached() {
        let (kv_tx, limit_tx) = prepare_deps("123".to_string(), true);

        let app = test::init_service(App::new()
            .app_data(Data::new(kv_tx.clone()))
            .app_data(Data::new(limit_tx.clone()))
            .service(index)).await;

        let req = test::TestRequest::post().uri("/").set_json(JSONRPCRequest {
            _json_rpc: "2.0".to_string(),
            method: "get".to_string(),
            id: 11,
            params: vec!["12345".to_string()],
        }).to_request();
        let resp: JSONRPCErrorResponse = test::call_and_read_body_json(&app, req).await;
        assert_eq!(resp.error.code, 100429);
    }

    fn prepare_deps(get_res: String, limit_res: bool) -> (Sender<storage::Command>, Sender<rate_limiting::Command>) {
        let (kv_tx, mut kv_rx) = tokio::sync::mpsc::channel::<storage::Command>(1);
        tokio::spawn(async move {
            while let Some(cmd) = kv_rx.recv().await {
                match cmd {
                    storage::Command::Get { key: _, res } => {
                        res.send(Ok(get_res.clone())).unwrap();
                    }
                    _ => {}
                }
            }
        });

        let (limit_tx, mut limit_rx) = tokio::sync::mpsc::channel::<rate_limiting::Command>(1);
        tokio::spawn(async move {
            while let Some(cmd) = limit_rx.recv().await {
                match cmd {
                    rate_limiting::Command::LimitReached { ip: _, res } => {
                        res.send(limit_res).unwrap();
                    }
                }
            }
        });
        (kv_tx, limit_tx)
    }
}

