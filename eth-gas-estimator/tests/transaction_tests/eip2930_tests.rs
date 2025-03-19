//! Tests for EIP-2930 (Type 1) transactions with access lists
//!
//! These tests verify gas estimation for transactions using
//! EIP-2930 with explicitly specified account and storage access lists.

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
mod helpers; // Assumes your helpers are in tests/helpers.rs
use helpers::spawn_anvil;

#[actix_web::test]
async fn test_eip2930_access_list_transaction_rpc() {

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

    // Build a JSON-RPC request for eth_estimateGas with EIP-2930 access list parameters.
    // In this example, we use placeholder addresses; you may replace these with real addresses.
    let request = json!({
        "jsonrpc": "2.0",
        "method": "eth_estimateGas",
        "params": [{
            "from": "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266",
            "to": "0x70997970C51812dc3A010C7d01b50e0d17dc79C8",
            "gas": "0x7a120",                      // Example gas limit (500,000 in hex)
            "gasPrice": "0x2540be400",              // 10 Gwei (may be ignored in EIP-2930)
            "value": "0xde0b6b3a7640000",           // 1 ETH
            "accessList": [
                {
                    "address": "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                    "storageKeys": [
                        "0x0101010101010101010101010101010101010101010101010101010101010101",
                        "0x0202020202020202020202020202020202020202020202020202020202020202"
                    ]
                }
            ],
            "transactionType": "0x1"               // Indicates EIP-2930 transaction
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

    // Extract the gas estimate from the response and assert it is within an expected range.
    let gas_estimate_str = response["result"]
        .as_str()
        .expect("No result field in response");
    let gas_estimate = U256::from_str_radix(
        &gas_estimate_str.trim_start_matches("0x"),
        16
    ).expect("Failed to parse gas estimate");

    assert_eq!(gas_estimate, U256::from(27200));

    // Clean up: kill the Anvil process.
    anvil_process.kill().expect("Failed to kill Anvil process");
}