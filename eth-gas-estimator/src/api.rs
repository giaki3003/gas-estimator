use tracing::debug;
use crate::{
    error::ServiceError,
    estimator::{GasEstimator, DEFAULT_GAS_LIMIT, DEFAULT_GAS_PRICE},
    models:: {
        jsonrpc::{
            JsonRpcRequest, JsonRpcSuccess, JsonRpcError, EthEstimateGasParams,
            parse_hex_address, parse_hex_u256, parse_hex_bytes, format_hex_u256, parse_hex_u64
        }
    }
};
use actix_web::{
    post, web, HttpRequest, HttpResponse
};
use std::sync::Arc;
use tracing::{error, info};
use alloy::{
    primitives::{Bytes, U256},
    rpc::types::{TransactionInput, TransactionRequest},
};

/// Endpoint to estimate gas for Ethereum transactions following the JSON-RPC protocol
/// This endpoint conforms to the Ethereum JSON-RPC specification for eth_estimateGas
#[post("/api/v1/eth/estimateGas")]
async fn estimate_gas_jsonrpc(
    req: HttpRequest,
    estimator: web::Data<Arc<GasEstimator>>,
    request: web::Json<JsonRpcRequest<Vec<EthEstimateGasParams>>>,
) -> HttpResponse {
    debug!("Received JSON-RPC gas estimation request from {}", req.peer_addr().unwrap_or_else(|| "unknown".parse().unwrap()));

    // Validate JSON-RPC version
    if request.jsonrpc != "2.0" {
        return HttpResponse::BadRequest().json(JsonRpcError::invalid_params(
            request.id.clone(),
            "Invalid JSON-RPC version. Expected 2.0".to_string(),
        ));
    }

    // Validate method
    if request.method != "eth_estimateGas" {
        return HttpResponse::BadRequest().json(JsonRpcError::invalid_params(
            request.id.clone(),
            format!("Unsupported method: {}", request.method),
        ));
    }

    // Validate params - ensure we have transaction parameters
    if request.params.is_empty() {
        return HttpResponse::BadRequest().json(JsonRpcError::invalid_params(
            request.id.clone(),
            "Missing transaction parameters".to_string(),
        ));
    }

    // Get the transaction parameters from the first element in the params array
    let tx_params = &request.params[0];

    // Convert JSON-RPC parameters to a TransactionRequest
    let tx_request = match build_transaction_request(tx_params).await {
        Ok(req) => req,
        Err(err_msg) => {
            return HttpResponse::BadRequest().json(JsonRpcError::invalid_params(
                request.id.clone(),
                err_msg,
            ));
        }
    };

    // Log the received transaction parameters in a structured format
    info!(
        "Received transaction:
        - from:                     {}
        - to:                       {}
        - gas:                      {}
        - gas_price:                {}
        - max_fee_per_gas:          {}
        - max_priority_fee_per_gas: {}
        - value:                    {}
        - input:                    {}
        - block:                    {}
        - nonce:                    {}
        - chain_id:                 {}",
        tx_params.from.as_deref().unwrap_or("None"),
        tx_params.to.as_deref().unwrap_or("None"),
        tx_params.gas.as_deref().unwrap_or("None"),
        tx_params.gas_price.as_deref().unwrap_or("None"),
        tx_params.max_fee_per_gas.as_deref().unwrap_or("None"),
        tx_params
            .max_priority_fee_per_gas
            .as_deref()
            .unwrap_or("None"),
        tx_params.value.as_deref().unwrap_or("None"),
        tx_params.input.as_deref().unwrap_or("None"),
        tx_params.block.as_deref().unwrap_or("None"),
        tx_params.nonce.as_deref().unwrap_or("None"),
        tx_params.chain_id.as_deref().unwrap_or("None"),
    );

    // Estimate gas using the service
    match estimator.estimate_raw_gas(&tx_request).await {
        Ok(gas_limit) => {
            info!("Gas estimation successful: {}", gas_limit);
            // Return successful response with the estimated gas limit
            HttpResponse::Ok().json(JsonRpcSuccess::new(
                request.id.clone(),
                format_hex_u256(gas_limit),
            ))
        }
        Err(e) => {
            error!("Gas estimation failed: {:?}", e);
            // Return error response
            HttpResponse::InternalServerError().json(JsonRpcError::internal_error(
                request.id.clone(),
                format!("Gas estimation failed: {}", e),
            ))
        }
    }
}

/// Service health check endpoint that verifies RPC connection is working
#[post("/api/v1/health")]
async fn health_check(
    estimator: web::Data<Arc<GasEstimator>>,
) -> Result<HttpResponse, ServiceError> {
    info!("Health check requested");

    // Try to get the latest block to verify RPC connection is working
    let eth_client = &estimator.eth_client;
    match eth_client.get_latest_block().await {
        Ok(block) => {
            // Return health status along with latest block info
            let response = serde_json::json!({
                "status": "ok",
                "latest_block": block.header.number,
                "timestamp": block.header.timestamp,
            });
            Ok(HttpResponse::Ok().json(response))
        }
        Err(e) => {
            error!("Health check failed: {:?}", e);
            Err(ServiceError::RPCConnectionError(format!("RPC connection error: {}", e)))
        }
    }
}

/// Configure the API routes for the service
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(estimate_gas_jsonrpc)
       .service(health_check);
}

/// Build a transaction request from JSON-RPC parameters
///
/// This function converts the JSON-RPC parameters into an Alloy TransactionRequest,
/// validating and parsing each field as needed.
async fn build_transaction_request(
    params: &EthEstimateGasParams,
) -> Result<TransactionRequest, String> {
    let mut tx_request = TransactionRequest::default();
    debug!("Building transaction request with params: {:?}", params);

    // Parse and set the from address
    if let Some(from_str) = &params.from {
        debug!("Parsing 'from' address: {}", from_str);
        let from = parse_hex_address(from_str)?;
        tx_request.from = Some(from);
        debug!("Parsed 'from' address: {:?}", from);
    }

    // Parse and set the to address (required for contract calls, optional for deployments)
    if let Some(to_str) = &params.to {
        debug!("Parsing 'to' address: {}", to_str);
        let to = parse_hex_address(to_str)?;
        tx_request.to = Some(to.into());
        debug!("Parsed 'to' address: {:?}", to);
    } else if params.input.is_none() {
        // Either 'to' or 'input' is required for a valid transaction
        let error_msg = "Either 'to' or 'input' must be provided";
        debug!("{}", error_msg);
        return Err(error_msg.to_string());
    }

    // Parse and set the gas limit (optional)
    if let Some(gas_str) = &params.gas {
        debug!("Parsing gas limit: {}", gas_str);
        let gas = parse_hex_u64(gas_str)?;
        tx_request.gas = Some(gas);
        debug!("Parsed gas limit: {}", gas);
    } else {
        // Use default gas limit if not provided
        debug!("No gas limit provided, using default: {}", DEFAULT_GAS_LIMIT);
        tx_request.gas = Some(DEFAULT_GAS_LIMIT);
    }

    // Parse and set the transaction value (optional)
    if let Some(value_str) = &params.value {
        debug!("Parsing value: {}", value_str);
        let value = parse_hex_u256(value_str)?;
        tx_request.value = Some(value);
        debug!("Parsed value: {:?}", value);
    } else {
        // Default to zero value if not provided
        debug!("No value provided, defaulting to U256::ZERO");
        tx_request.value = Some(U256::ZERO);
    }

    // Parse and set the input data (optional)
    if let Some(input_str) = &params.input {
        debug!("Parsing input data: {}", input_str);
        let input_data = parse_hex_bytes(input_str)?;
        tx_request.input = TransactionInput::from(input_data.clone());
        debug!("Parsed input data: {:?}", input_data);
    } else {
        // Default to empty input if not provided
        debug!("No input data provided, using empty Bytes");
        tx_request.input = TransactionInput::from(Bytes::new());
    }

    // Handle gas pricing - this can be legacy (gasPrice) or EIP-1559 (maxFeePerGas and maxPriorityFeePerGas)
    if let Some(gas_price_str) = &params.gas_price {
        debug!("Parsing legacy gas price: {}", gas_price_str);
        let gas_price = parse_hex_u256(gas_price_str)?;
        if let Ok(price) = u128::try_from(gas_price) {
            tx_request.gas_price = Some(price);
            debug!("Parsed legacy gas price: {}", price);
        } else {
            debug!("Failed to convert gas price to u128");
        }
    } else if let (Some(max_fee_str), Some(priority_fee_str)) = (&params.max_fee_per_gas, &params.max_priority_fee_per_gas) {
        debug!("Parsing EIP-1559 gas pricing: maxFeePerGas: {}, maxPriorityFeePerGas: {}", max_fee_str, priority_fee_str);
        let max_fee = parse_hex_u256(max_fee_str)?;
        let priority_fee = parse_hex_u256(priority_fee_str)?;
        
        // Convert to u128 for the transaction request
        if let Ok(max_fee_u128) = u128::try_from(max_fee) {
            tx_request.max_fee_per_gas = Some(max_fee_u128);
            debug!("Parsed max fee per gas: {}", max_fee_u128);
        } else {
            debug!("Failed to convert max fee per gas to u128");
        }
        
        if let Ok(priority_fee_u128) = u128::try_from(priority_fee) {
            tx_request.max_priority_fee_per_gas = Some(priority_fee_u128);
            debug!("Parsed max priority fee per gas: {}", priority_fee_u128);
        } else {
            debug!("Failed to convert max priority fee per gas to u128");
        }
    } else {
        // Use default gas price if none provided
        debug!("No gas pricing provided, defaulting to gas price: {}", DEFAULT_GAS_PRICE);
        let gas_price = DEFAULT_GAS_PRICE;
        match u128::try_from(gas_price) {
            Ok(price) => {
                tx_request.gas_price = Some(price);
                debug!("Parsed legacy gas price: {}", price);
            }
            Err(_) => {
                debug!("Failed to convert gas price to u128");
            }
        }
    }

    // Handle additional transaction fields - nonce and chain_id
    if let Some(nonce_str) = &params.nonce {
        debug!("Parsing nonce: {}", nonce_str);
        let nonce_u64 = parse_hex_u64(nonce_str)?;
        tx_request.nonce = Some(nonce_u64);
        debug!("Parsed nonce: {}", nonce_u64);
    }

    if let Some(chainid_str) = &params.chain_id {
        debug!("Parsing chainId: {}", chainid_str);
        let chainid_u64 = parse_hex_u64(chainid_str)?;
        tx_request.chain_id = Some(chainid_u64);
        debug!("Parsed chainId: {}", chainid_u64);
    }

    // Handle block parameter (defaults to latest)
    let _block_tag = params.block.as_deref().unwrap_or("latest");
    debug!("Using block tag: {}", _block_tag);
    // Note: block parameter is used to replicate eth spec, but right now we always default to the latest - !TODO: implement arbitrary block requests
    debug!("Transaction request built: {:?}", tx_request);
    Ok(tx_request)
}