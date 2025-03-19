use std::str::FromStr;
use alloy::primitives::{Address, Bytes, U256, B256, hex};
use alloy::eips::{
    eip4844::BlobTransactionSidecar,
    eip7702::{Authorization, SignedAuthorization},
};
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

    /// EIP-2930 access list (optional)
    #[serde(default, rename = "accessList")]
    pub access_list: Option<Vec<AccessListItemRpc>>,

    /// EIP-2718 transaction type (optional)
    /// Typically an 8-bit integer in hex or decimal
    #[serde(default, rename = "type")]
    pub transaction_type: Option<String>,

    /// EIP-4844 fields
    #[serde(default, rename = "blobVersionedHashes")]
    pub blob_versioned_hashes: Option<Vec<String>>,

    #[serde(default, rename = "maxFeePerBlobGas")]
    pub max_fee_per_blob_gas: Option<String>,

    #[serde(default)]
    pub sidecar: Option<BlobTransactionSidecar>,

    /// EIP-7702
    #[serde(default, rename = "authorizationList")]
    pub authorization_list: Option<Vec<AuthorizationRpc>>,
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
///
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

/// Parse a hexadecimal string into a 32-byte array or B256.
///
/// Expects a string starting with "0x", followed by exactly 64 hex characters.
/// Returns an error if the length is incorrect or it cannot decode the hex.
pub fn parse_hex_b256(hex_str: &str) -> Result<B256, String> {
    // 1) Strip "0x" prefix
    let hex_str = hex_str
        .strip_prefix("0x")
        .ok_or_else(|| "Hex value must start with \"0x\"".to_string())?;

    // 2) Decode into raw bytes
    let bytes = hex::decode(hex_str)
        .map_err(|e| format!("Failed to decode hex: {e}"))?;

    // 3) Check for 32 bytes
    if bytes.len() != 32 {
        return Err(format!(
            "Expected 32 bytes (64 hex characters), got {}",
            bytes.len()
        ));
    }

    // 4) Convert into B256 (or [u8; 32])
    Ok(B256::from_slice(&bytes))
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

/// Parse a decimal/hexadecimal string into a `u8` value.
///
/// Expects a string starting with "0x". If the hex string contains no data (i.e. "0x"),
/// an empty `u` value is returned.
///
/// # Arguments
///
/// * `hex` - The hexadecimal string
///
/// # Returns
///
/// * `Result<u8, String>` - Parsed u8 or error message
pub fn parse_hex_or_dec_u8(s: &str) -> Result<u8, String> {
    if let Some(stripped) = s.strip_prefix("0x") {
        u8::from_str_radix(stripped, 16).map_err(|e| format!("Invalid hex: {e}"))
    } else {
        s.parse::<u8>().map_err(|e| format!("Invalid decimal: {e}"))
    }
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

#[derive(Debug, Deserialize)]
pub struct AccessListItemRpc {
    pub address: String,
    #[serde(rename = "storageKeys")]
    pub storage_keys: Vec<String>,  // These hex strings should be parsed into B256 values for Alloy TransactionReceipt
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AuthorizationRpc {
    #[serde(rename = "chainId")]
    pub chain_id: String,       // e.g. "0x1"
    #[serde(rename = "address")]
    pub contract_address: String,
    pub nonce: String,          // e.g. "0x42" or decimal
    #[serde(rename = "yParity")]
    pub y_parity: String,       // "0x0" or "0x1"
    pub r: String,              // "0x..." 32-byte hex
    pub s: String,              // "0x..." 32-byte hex
}

impl AuthorizationRpc {
    pub fn to_authorization(&self) -> Result<SignedAuthorization, String> {
        // 1) Parse chain ID as a u64, then wrap in `ChainId`.
        let chain_id_u256 = parse_hex_u256(&self.chain_id)?;

        // 2) Parse the contract address
        let contract_address = parse_hex_address(&self.contract_address)?;

        // 3) Parse the nonce
        let nonce_u64 = parse_hex_u64(&self.nonce)?;

        // 4) Parse yParity (0 or 1)
        let parity_val = parse_hex_u64(&self.y_parity)?;
        let y_parity = match parity_val {
            0 => 0u8,
            1 => 1u8,
            _ => return Err("Invalid y_parity, must be 0 or 1".to_string()),
        };

        // 5) Parse r, s (256-bit hex -> `U256`)
        let r_val = parse_hex_u256(&self.r)?;
        let s_val = parse_hex_u256(&self.s)?;

        // 6) Build the "inner" authorization
        let inner = Authorization {
            chain_id: chain_id_u256,
            address: contract_address,
            nonce: nonce_u64,
        };

        // 7) Finally, call `new_unchecked`
        Ok(SignedAuthorization::new_unchecked(inner, y_parity, r_val, s_val))
    }
}