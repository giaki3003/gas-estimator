//! Integration tests for the API endpoints

use tracing_actix_web::TracingLogger;
use tracing_subscriber::EnvFilter;
use actix_web::{
    test, web, App,
    http::StatusCode,
};
use std::sync::Arc;
use serde_json::json;

use eth_gas_estimator::{
    api,
    estimator::GasEstimator,
    rpc::EthereumClient,
};

mod helpers;
use helpers::spawn_anvil;

#[actix_web::test]
async fn test_health_check() {
    // Spawn an Anvil process.
    let (mut anvil_process, rpc_url) = spawn_anvil();
    
    // Create an Ethereum client from the RPC URL and wrap it in an Arc.
    let client = Arc::new(EthereumClient::new(&rpc_url).await.unwrap());
    
    // Build a GasEstimator using the client and RPC URL.
    let estimator = GasEstimator::new(client, &rpc_url);

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(Arc::new(estimator)))
            .configure(api::configure)
    ).await;
    
    // Make request to the health check endpoint.
    let req = test::TestRequest::post()
        .uri("/api/v1/health")
        .to_request();
        
    let resp = test::call_service(&app, req).await;
    
    // Verify a successful response.
    assert_eq!(resp.status(), StatusCode::OK);
    
    // Parse the response JSON.
    let body = test::read_body(resp).await;
    let response: serde_json::Value = serde_json::from_slice(&body)
        .expect("Failed to parse JSON response");
        
    // Check that the response has the expected fields.
    assert_eq!(response["status"], "ok");
    assert!(response.get("latest_block").is_some());
    assert!(response.get("timestamp").is_some());
    
    // Clean up: kill the Anvil process.
    anvil_process.kill().expect("Failed to kill Anvil process");
}

#[actix_web::test]
async fn test_estimate_gas_endpoint() {
    let filter = EnvFilter::from_default_env()
        .add_directive("eth_gas_estimator=debug".parse().unwrap())
        .add_directive("actix_web=error".parse().unwrap())
        .add_directive("hyper=warn".parse().unwrap())
        .add_directive("reqwest=warn".parse().unwrap());
    
    // Initialize the tracing subscriber with our filter
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .init();
    // Spawn an Anvil process.
    let (mut anvil_process, rpc_url) = spawn_anvil();
    
    // Create an Ethereum client from the RPC URL and wrap it in an Arc.
    let client = Arc::new(EthereumClient::new(&rpc_url).await.unwrap());
    
    // Build a GasEstimator using the client and RPC URL.
    let estimator = GasEstimator::new(client, &rpc_url);

    let app = test::init_service(
        App::new()
            .wrap(TracingLogger::default())
            .app_data(web::Data::new(Arc::new(estimator)))
            .configure(api::configure)
    ).await;
    
    // Construct a JSON-RPC request for a simple ETH transfer.
    let request = json!({
        "jsonrpc": "2.0",
        "method": "eth_estimateGas",
        "params": [{
            "from": "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266",
            "to": "0x70997970C51812dc3A010C7d01b50e0d17dc79C8",
            "value": "0xde0b6b3a7640000" // 1 ETH
        }],
        "id": 1
    });
    
    // Make request to the estimateGas endpoint.
    let req = test::TestRequest::post()
        .uri("/api/v1/eth/estimateGas")
        .set_json(&request)
        .to_request();
        
    let resp = test::call_service(&app, req).await;
    
    // Verify a successful response.
    assert_eq!(resp.status(), StatusCode::OK);
    
    // Parse the response JSON.
    let body = test::read_body(resp).await;
    let response: serde_json::Value = serde_json::from_slice(&body)
        .expect("Failed to parse JSON response");
        
    // Check the JSON-RPC response format.
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 1);
    assert!(response["result"].is_string());
    assert_eq!(response["result"].as_str().unwrap(), "0x5208"); // 21000 in hex

    // Clean up the Anvil process.
    anvil_process.kill().expect("Failed to kill Anvil process");
}

#[actix_web::test]
async fn test_invalid_request_handling() {
    // Spawn an Anvil process.
    let (mut anvil_process, rpc_url) = spawn_anvil();
    
    // Create an Ethereum client from the RPC URL and wrap it in an Arc.
    let client = Arc::new(EthereumClient::new(&rpc_url).await.unwrap());
    
    // Build a GasEstimator using the client and RPC URL.
    let estimator = GasEstimator::new(client, &rpc_url);

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(Arc::new(estimator)))
            .configure(api::configure)
    ).await;
    
    // Construct an invalid JSON-RPC request (missing required fields).
    let request = json!({
        "jsonrpc": "2.0",
        "method": "eth_estimateGas",
        "params": [{}],
        "id": 2
    });
    
    // Make request.
    let req = test::TestRequest::post()
        .uri("/api/v1/eth/estimateGas")
        .set_json(&request)
        .to_request();
        
    let resp = test::call_service(&app, req).await;
    
    // Verify an error response.
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    
    // Parse the error response JSON.
    let body = test::read_body(resp).await;
    let response: serde_json::Value = serde_json::from_slice(&body)
        .expect("Failed to parse JSON response");
        
    // Check error response format.
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 2);
    assert!(response["error"].is_object());
    assert_eq!(response["error"]["code"], -32602); // Invalid params
    assert!(response["error"]["message"].as_str().unwrap().contains("Either 'to' or 'input' must be provided"));

    // Clean up the Anvil process.
    anvil_process.kill().expect("Failed to kill Anvil process");
}