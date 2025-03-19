use std::str::FromStr;
use alloy_primitives::hex;
use alloy::primitives::{Address, Bytes, U256};
use serde::{Deserialize, Serialize};

/// JSON-RPC 2.0 request structure
///
/// This structure represents a standard JSON-RPC request with generic parameters.
#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest<T> {
    /// JSON-RPC protocol version (should be "2.0")
    pub jsonrpc: String,
    
    /// Method name to call
    pub method: String,
    
    /// Method parameters
    pub params: T,
    
    /// Request identifier
    pub id: serde_json::Value,
}

/// JSON-RPC 2.0 successful response
///
/// This structure represents a standard JSON-RPC successful response with generic result.
#[derive(Debug, Serialize)]
pub struct JsonRpcSuccess<T> {
    /// JSON-RPC protocol version (always "2.0")
    pub jsonrpc: String,
    
    /// Request identifier (matching the request)
    pub id: serde_json::Value,
    
    /// Method result
    pub result: T,
}

/// JSON-RPC 2.0 error response
///
/// This structure represents a standard JSON-RPC error response.
#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    /// JSON-RPC protocol version (always "2.0")
    pub jsonrpc: String,
    
    /// Request identifier (matching the request)
    pub id: serde_json::Value,
    
    /// Error details
    pub error: JsonRpcErrorDetail,
}

/// JSON-RPC 2.0 error detail
///
/// This structure contains the detailed error information in a JSON-RPC error response.
#[derive(Debug, Serialize)]
pub struct JsonRpcErrorDetail {
    /// Error code
    pub code: i32,
    
    /// Error message
    pub message: String,
    
    /// Additional error data (optional)
    pub data: Option<serde_json::Value>,
}

/// Parameters for eth_estimateGas JSON-RPC method
///
/// This structure contains the parameters for the eth_estimateGas method
/// following the Ethereum JSON-RPC specification.
#[derive(Debug, Deserialize)]
pub struct EthEstimateGasParams {
    /// Sender address (optional)
    #[serde(default)]
    pub from: Option<String>,
    
    /// Recipient address (optional for contract creation)
    pub to: Option<String>,
    
    /// Gas limit (optional)
    #[serde(default)]
    pub gas: Option<String>,

    /// Legacy gas price (optional)
    #[serde(default, rename = "gasPrice")]
    pub gas_price: Option<String>,

    /// EIP-1559 max fee per gas (optional)
    #[serde(default, rename = "maxFeePerGas")]
    pub max_fee_per_gas: Option<String>,
    
    /// EIP-1559 max priority fee per gas (optional)
    #[serde(default, rename = "maxPriorityFeePerGas")]
    pub max_priority_fee_per_gas: Option<String>,

    /// Transaction value in wei (optional)
    #[serde(default)]
    pub value: Option<String>,

    /// Transaction input data (optional)
    /// Can be specified as either "data" or "input"
    #[serde(default, rename = "data", alias = "input")]
    pub input: Option<String>,

    /// Block number or tag for context (optional, defaults to "latest")
    #[serde(default)]
    pub block: Option<String>,

    /// Transaction nonce (optional)
    #[serde(default)]
    pub nonce: Option<String>,

    /// Chain ID (optional)
    #[serde(default, rename = "chainId")]
    pub chain_id: Option<String>,
}

impl JsonRpcError {
    /// Create a new JSON-RPC invalid parameters error
    ///
    /// # Arguments
    ///
    /// * `id` - Request identifier
    /// * `message` - Error message
    ///
    /// # Returns
    ///
    /// * A formatted JSON-RPC error response
    pub fn invalid_params(id: serde_json::Value, message: String) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            error: JsonRpcErrorDetail {
                code: -32602,
                message,
                data: None,
            },
        }
    }

    /// Create a new JSON-RPC internal error
    ///
    /// # Arguments
    ///
    /// * `id` - Request identifier
    /// * `message` - Error message
    ///
    /// # Returns
    ///
    /// * A formatted JSON-RPC error response
    pub fn internal_error(id: serde_json::Value, message: String) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            error: JsonRpcErrorDetail {
                code: -32603,
                message,
                data: None,
            },
        }
    }
}

impl<T> JsonRpcSuccess<T> {
    /// Create a new JSON-RPC success response
    ///
    /// # Arguments
    ///
    /// * `id` - Request identifier
    /// * `result` - Response result
    ///
    /// # Returns
    ///
    /// * A formatted JSON-RPC success response
    pub fn new(id: serde_json::Value, result: T) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result,
        }
    }
}

/// Helper functions to parse hex values from JSON-RPC requests using alloy primitives.

/// Parse a hexadecimal address string into an `Address`.
///
/// Expects a string starting with "0x" and 40 hex digits (20 bytes).
///
/// # Arguments
///
/// * `hex` - The hexadecimal address string
///
/// # Returns
///
/// * `Result<Address, String>` - Parsed address or error message
pub fn parse_hex_address(hex: &str) -> Result<Address, String> {
    if !hex.starts_with("0x") {
        return Err("Address must start with 0x".to_string());
    }
    Address::from_str(hex)
        .map_err(|e| format!("Invalid address: {}", e))
}

/// Parse a hexadecimal string into a `U256` value.
///
/// Expects a string starting with "0x".
///
/// # Arguments
///
/// * `hex` - The hexadecimal string
///
/// # Returns
///
/// * `Result<U256, String>` - Parsed value or error message
pub fn parse_hex_u256(hex: &str) -> Result<U256, String> {
    let hex = hex
        .strip_prefix("0x")
        .ok_or_else(|| "Hex value must start with 0x".to_string())?;
    U256::from_str_radix(hex, 16).map_err(|e| format!("Invalid hex value: {}", e))
}

/// Parse a hexadecimal string into a `u64` value.
///
/// Expects a string starting with "0x".
///
/// # Arguments
///
/// * `hex` - The hexadecimal string
///
/// # Returns
///
/// * `Result<u64, String>` - Parsed value or error message
pub fn parse_hex_u64(hex: &str) -> Result<u64, String> {
    let hex = hex
        .strip_prefix("0x")
        .ok_or_else(|| "Hex value must start with 0x".to_string())?;
    u64::from_str_radix(hex, 16).map_err(|e| format!("Invalid u64 hex value: {}", e))
}

/// Parse a hexadecimal string into a `Bytes` value.
///
/// Expects a string starting with "0x". If the hex string contains no data (i.e. "0x"),
/// an empty `Bytes` value is returned.
///
/// # Arguments
///
/// * `hex` - The hexadecimal string
///
/// # Returns
///
/// * `Result<Bytes, String>` - Parsed bytes or error message
pub fn parse_hex_bytes(hex: &str) -> Result<Bytes, String> {
    let hex = hex
        .strip_prefix("0x")
        .ok_or_else(|| "Hex data must start with 0x".to_string())?;
    if hex.is_empty() {
        return Ok(Bytes::new());
    }
    let data = hex::decode(hex).map_err(|e| format!("Invalid hex data: {}", e))?;
    Ok(Bytes::from(data))
}

/// Format a `U256` value into a hexadecimal string prefixed with "0x".
///
/// # Arguments
///
/// * `value` - The U256 value to format
///
/// # Returns
///
/// * String representation of the value in hexadecimal
pub fn format_hex_u256(value: U256) -> String {
    format!("0x{:x}", value)
}