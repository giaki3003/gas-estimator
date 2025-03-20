//! Tests for EIP-7702 authorization list transactions via JSON-RPC endpoint
//!
//! These tests verify gas estimation for transactions using
//! EIP-7702 with signed authorization lists.

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
async fn test_eip7702_authorization_transaction_rpc() {

    // Initialize the logger.
    init_logger();

    // Spawn an Anvil instance and obtain its RPC URL.
    let (mut anvil_process, rpc_url) = spawn_anvil();

    // Create an Ethereum client using the RPC URL and wrap it in an Arc.
    let client = Arc::new(EthereumClient::new(&rpc_url).await.unwrap());

    // Build a GasEstimator from the client and RPC URL.
    let estimator = GasEstimator::new(client, &rpc_url);

    // Initialize the Actix application with your API endpoints and the tracing logger.
    let app = test::init_service(
        App::new()
            .wrap(TracingLogger::default())
            .app_data(web::Data::new(Arc::new(estimator)))
            .configure(api::configure)
    ).await;

    // Build a JSON-RPC request for eth_estimateGas that includes an authorization list.
    let request = json!({
        "jsonrpc": "2.0",
        "method": "eth_estimateGas",
        "params": [{
            "from": "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266",
            "to": "0x70997970C51812dc3A010C7d01b50e0d17dc79C8",
            "gas": "0x7a120",                      // 500,000 in hex
            "maxFeePerGas": "0x4a817c800",          // 20 Gwei in hex
            "maxPriorityFeePerGas": "0x77359400",     // 2 Gwei in hex
            "value": "0xde0b6b3a7640000",           // 1 ETH in hex
            // EIP-7702 authorization list
            "authorizationList": [{
                "chainId": "0x1",
                "address": "0xcccccccccccccccccccccccccccccccccccccccc",
                "nonce": "0x2a",
                "yParity": "0x1",
                "r": "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "s": "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
            }],
            "transactionType": "0x4"               // Type 4 transaction with an auth list (EIP-7702)
        }],
        "id": 1
    });

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

    println!("Response: {:?}", response);

    // Extract the gas estimate from the response and parse it as U256.
    let gas_estimate_str = response["result"]
        .as_str()
        .expect("No result field in response");
    let gas_estimate = U256::from_str_radix(gas_estimate_str.trim_start_matches("0x"), 16)
        .expect("Failed to parse gas estimate");

    // Assert that the gas estimate exactly matches the expected value.
    assert_eq!(gas_estimate, U256::from(46000));

    // Clean up: kill the Anvil process.
    anvil_process.kill().expect("Failed to kill Anvil process");
}
