//! Tests for EIP-4844 (Type 3) blob transactions via JSON-RPC endpoint
//!
//! These tests verify gas estimation for transactions using
//! EIP-4844 blob transactions with versioned hashes.

use crate::init_logger;
use actix_web::{test, web, App, http::StatusCode};
use alloy::primitives::{U256};
use serde_json::json;
use std::sync::Arc;
use tracing_actix_web::TracingLogger;

use eth_gas_estimator::{
    api,
    estimator::GasEstimator,
    rpc::EthereumClient,
};

#[path = "../api_tests/helpers.rs"]
mod helpers; // Assumes your helpers (including spawn_anvil) are in tests/helpers.rs
use helpers::spawn_anvil;

#[actix_web::test]
async fn test_eip4844_blob_transaction_rpc() {

    init_logger();

    // Spawn an Anvil instance and obtain its RPC URL.
    let (mut anvil_process, rpc_url) = spawn_anvil();

    // Create an Ethereum client using the RPC URL and wrap it in an Arc.
    let client = Arc::new(EthereumClient::new(&rpc_url).await.unwrap());

    // Build a GasEstimator using the client and RPC URL.
    let estimator = GasEstimator::new(client, &rpc_url);

    // Initialize the Actix application with your API endpoints and the tracing logger.
    let app = test::init_service(
        App::new()
            .wrap(TracingLogger::default())
            .app_data(web::Data::new(Arc::new(estimator)))
            .configure(api::configure)
    ).await;

    // Build a JSON-RPC request for eth_estimateGas with EIP-4844 parameters.
    // We use placeholder addresses for sender and receiver.
    let request = json!({
        "jsonrpc": "2.0",
        "method": "eth_estimateGas",
        "params": [{
            "from": "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266",
            "to": "0x70997970C51812dc3A010C7d01b50e0d17dc79C8",
            "gas": "0x7a120",                      // Example gas limit (500,000 in hex)
            "maxFeePerGas": "0x4a817c800",          // 20 Gwei (in hex)
            "maxPriorityFeePerGas": "0x77359400",     // 2 Gwei (in hex)
            "maxFeePerBlobGas": "0x989680",          // 10,000,000 wei (in hex)
            "blobVersionedHashes": [
                "0x01aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "0x01bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
            ],
            "transactionType": "0x3"               // Indicates an EIP-4844 transaction
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
    assert_eq!(resp.status(), StatusCode::OK);

    // Parse the response body as JSON.
    let body = test::read_body(resp).await;
    let response: serde_json::Value = serde_json::from_slice(&body)
        .expect("Failed to parse JSON response");

    // For debugging purposes, print the response.
    println!("Response: {:?}", response);

    // Extract the gas estimate from the response.
    let gas_estimate_str = response["result"]
        .as_str()
        .expect("No result field in response");
    let gas_estimate = U256::from_str_radix(&gas_estimate_str.trim_start_matches("0x"), 16)
        .expect("Failed to parse gas estimate");

    // Assert that the estimated gas exactly matches the expected value.
    assert_eq!(gas_estimate, U256::from(21000));

    // Clean up: kill the Anvil process.
    anvil_process.kill().expect("Failed to kill Anvil process");
}
