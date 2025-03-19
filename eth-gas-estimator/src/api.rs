use tracing::debug;
use crate::{
    error::ServiceError,
    estimator::{GasEstimator, DEFAULT_GAS_LIMIT, DEFAULT_GAS_PRICE},
    models:: {
        jsonrpc::{
            JsonRpcRequest, JsonRpcSuccess, JsonRpcError, EthEstimateGasParams,
            parse_hex_address, parse_hex_u256, parse_hex_bytes, format_hex_u256, parse_hex_u64, parse_hex_b256, parse_hex_or_dec_u8
        }
    }
};
use actix_web::{
    post, web, HttpRequest, HttpResponse
};
use std::sync::Arc;
use tracing::{error, info};
use alloy::{
    primitives::{Bytes, U256, B256},
    rpc::types::{TransactionInput, TransactionRequest},
    eips::{
        eip2930::{AccessList, AccessListItem},
    }
};

fn format_estimate_gas_params(params: &EthEstimateGasParams) -> String {
    let mut lines = Vec::new();

    if let Some(ref from) = params.from {
        lines.push(format!("from: {}", from));
    }
    if let Some(ref to) = params.to {
        lines.push(format!("to: {}", to));
    }
    if let Some(ref gas) = params.gas {
        lines.push(format!("gas: {}", gas));
    }
    if let Some(ref gas_price) = params.gas_price {
        lines.push(format!("gasPrice: {}", gas_price));
    }
    if let Some(ref max_fee) = params.max_fee_per_gas {
        lines.push(format!("maxFeePerGas: {}", max_fee));
    }
    if let Some(ref max_priority) = params.max_priority_fee_per_gas {
        lines.push(format!("maxPriorityFeePerGas: {}", max_priority));
    }
    if let Some(ref value) = params.value {
        lines.push(format!("value: {}", value));
    }
    if let Some(ref input) = params.input {
        lines.push(format!("input: {}", input));
    }
    if let Some(ref block) = params.block {
        lines.push(format!("block: {}", block));
    }
    if let Some(ref nonce) = params.nonce {
        lines.push(format!("nonce: {}", nonce));
    }
    if let Some(ref chain_id) = params.chain_id {
        lines.push(format!("chainId: {}", chain_id));
    }
    if let Some(ref access_list) = params.access_list {
        lines.push(format!("accessList: {:?}", access_list));
    }
    if let Some(ref tx_type) = params.transaction_type {
        lines.push(format!("type: {}", tx_type));
    }
    if let Some(ref blob_versioned_hashes) = params.blob_versioned_hashes {
        lines.push(format!("blobVersionedHashes: {:?}", blob_versioned_hashes));
    }
    if let Some(ref max_fee_per_blob_gas) = params.max_fee_per_blob_gas {
        lines.push(format!("maxFeePerBlobGas: {}", max_fee_per_blob_gas));
    }
    if let Some(ref sidecar) = params.sidecar {
        lines.push(format!("sidecar: {:?}", sidecar));
    }
    if let Some(ref auth_list) = params.authorization_list {
        lines.push(format!("authorizationList: {:?}", auth_list));
    }

    if lines.is_empty() {
        "[no fields set]".to_owned()
    } else {
        lines.join("\n  ")
    }
}

/// Endpoint to estimate gas for Ethereum transactions following the JSON-RPC protocol
/// This endpoint conforms to the Ethereum JSON-RPC specification for eth_estimateGas
#[post("/api/v1/eth/estimateGas")]
async fn estimate_gas_jsonrpc(
    req: HttpRequest,
    estimator: web::Data<Arc<GasEstimator>>,
    request: web::Json<JsonRpcRequest<Vec<EthEstimateGasParams>>>,
) -> HttpResponse {
    debug!(
        "Received JSON-RPC gas estimation request from {}",
        req.peer_addr()
            .map(|addr| addr.to_string())
            .unwrap_or_else(|| "unknown".into())
        );

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
    info!(
        "Received JSON-RPC params:\n  {}",
        format_estimate_gas_params(tx_params)
    );

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
            Err(ServiceError::RPCConnection(format!("RPC connection error: {}", e)))
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
        let price = u128::from(gas_price);
        tx_request.gas_price = Some(price);
        debug!("Parsed legacy gas price: {}", price);
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

    if let Some(access_list_vec) = &params.access_list {
        let mut items = Vec::new();
        for entry in access_list_vec {
            let address = parse_hex_address(&entry.address)?;
            let storage_keys = entry
                .storage_keys
                .iter()
                .map(|key_str| parse_hex_b256(key_str))
                .collect::<Result<Vec<B256>, _>>()?;
            items.push(AccessListItem { address, storage_keys });
        }
        tx_request.access_list = Some(AccessList(items.clone()));
        debug!("Parsed accessList with {} items", items.len());
    }

    // Transaction type (EIP-2718)
    if let Some(tx_type_str) = &params.transaction_type {
        debug!("Parsing transaction type: {}", tx_type_str);
        let tx_type_u8 = parse_hex_or_dec_u8(tx_type_str)?;
        tx_request.transaction_type = Some(tx_type_u8);
        debug!("Parsed transactionType: {}", tx_type_u8);
    }

    // EIP-4844: blobVersionedHashes
    if let Some(hashes_rpc) = &params.blob_versioned_hashes {
        debug!("Parsing blob versioned hashes");
        let mut hashes = Vec::new();
        for hash_str in hashes_rpc {
            debug!("Parsing hash: {}", hash_str);
            let h = parse_hex_b256(hash_str)
                .map_err(|e| {
                    debug!("Failed to parse hash {}: {:?}", hash_str, e);
                    e
                })?;
            hashes.push(h);
        }
        tx_request.blob_versioned_hashes = Some(hashes.clone());
        debug!("Parsed {} blob versioned hashes", hashes.len());
    }

    // EIP-4844: maxFeePerBlobGas
    if let Some(max_fee_blob_rpc) = &params.max_fee_per_blob_gas {
        debug!("Parsing max fee per blob gas");
        let max_fee_blob = parse_hex_u64(max_fee_blob_rpc)?;
        tx_request.max_fee_per_blob_gas = Some(max_fee_blob.into());
        debug!("Parsed max fee per blob gas: {:?}", max_fee_blob);
    }

    // sidecar
    if let Some(sidecar_rpc) = &params.sidecar {
        // Convert from your custom sidecar JSON structure into the `BlobTransactionSidecar`.
        // Possibly parse big-endian fields, etc. 
        let sidecar = sidecar_rpc;
        tx_request.sidecar = Some(sidecar.clone());
        debug!("Parsed sidecar: {:?}", sidecar);
    }

    // EIP-7702: authorizationList
    if let Some(auth_list_rpc) = &params.authorization_list {
        // Convert each item from the “AuthRpc” to the actual “SignedAuthorization”
        let mut parsed_auth = Vec::new();
        for auth_rpc_item in auth_list_rpc {
            let item = auth_rpc_item.to_authorization()?;
            parsed_auth.push(item);
        }
        tx_request.authorization_list = Some(parsed_auth.clone());
        debug!("Parsed {} items in authorizationList", parsed_auth.len());
    }

    debug!("Transaction request built: {:?}", tx_request);
    Ok(tx_request)
}