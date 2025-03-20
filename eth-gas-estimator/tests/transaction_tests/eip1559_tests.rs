//! Tests for EIP-1559 (Type 2) transactions via the JSON-RPC endpoint
//!
//! This test spawns an Anvil instance, creates an Actix web server configured
//! with your API endpoints, and sends a JSON-RPC request for gas estimation
//! using an EIP-1559 style transaction.

use crate::init_logger;
use tracing_actix_web::TracingLogger;
use actix_web::{test, web, App, http::StatusCode};
use alloy::primitives::{U256};
use serde_json::json;
use std::sync::Arc;

use eth_gas_estimator::{
    api,
    estimator::GasEstimator,
    rpc::EthereumClient,
};

#[path = "../api_tests/helpers.rs"]
mod helpers; // Assumes your helpers are in tests/helpers.rs
use helpers::spawn_anvil;

#[actix_web::test]
async fn test_eip1559_transaction_estimation_rpc() {
    init_logger();
    // Spawn an Anvil instance and get its RPC URL.
    let (mut anvil_process, rpc_url) = spawn_anvil();

    // Create an Ethereum client using the RPC URL (unwrap the result)
    let client = Arc::new(EthereumClient::new(&rpc_url).await.unwrap());

    // Build a GasEstimator from the client and RPC URL.
    let estimator = GasEstimator::new(client, &rpc_url);

    // Initialize the Actix application with the API endpoints.
    let app = test::init_service(
        App::new()
            .wrap(TracingLogger::default())
            .app_data(web::Data::new(Arc::new(estimator)))
            .configure(api::configure)
    ).await;

    // Build a JSON-RPC request for eth_estimateGas with EIP-1559 parameters.
    // Using two of Anvil's pre-funded accounts:
    //  - Sender (account 0): 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266
    //  - Receiver (account 1): 0x70997970C51812dc3A010C7d01b50e0d17dc79C8
    let request = json!({
        "jsonrpc": "2.0",
        "method": "eth_estimateGas",
        "params": [{
            "from": "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266",
            "to": "0x70997970C51812dc3A010C7d01b50e0d17dc79C8",
            // EIP-1559 fields (hex encoded values)
            "maxFeePerGas": "0x4a817c800",          // 20,000,000,000 (20 Gwei)
            "maxPriorityFeePerGas": "0x77359400",   // 2,000,000,000 (2 Gwei)
            "value": "0xde0b6b3a7640000",           // 1 ETH
            "transactionType": "0x2"                // Indicates an EIP-1559 transaction
        }],
        "id": 1
    });

    // Create the test request to the eth_estimateGas endpoint.
    let req = test::TestRequest::post()
        .uri("/api/v1/eth/estimateGas")
        .set_json(&request)
        .to_request();

    // Call the service and capture the response.
    let resp = test::call_service(&app, req).await;

    // Ensure the status is OK.
    assert_eq!(resp.status(), StatusCode::OK);

    // Parse the response body as JSON.
    let body = test::read_body(resp).await;
    let response: serde_json::Value = serde_json::from_slice(&body)
        .expect("Failed to parse JSON response");

    // Print the response for debugging.
    println!("Response: {:?}", response);

    // You can either assert an exact gas estimate if you know it
    // or check that the returned gas is within an expected range.
    // For a simple transfer, the gas should be at least 21000.
    let gas_estimate_str = response["result"].as_str().expect("No result field");
    let gas_estimate = U256::from_str_radix(&gas_estimate_str.trim_start_matches("0x"), 16)
        .expect("Failed to parse gas estimate");
    assert_eq!(gas_estimate, U256::from(21000));

    // Clean up: kill the Anvil process.
    anvil_process.kill().expect("Failed to kill Anvil process");
}