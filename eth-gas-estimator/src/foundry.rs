use crate::{
    estimator::{GWEI, DEFAULT_GAS_LIMIT},
    error::ServiceError,
};
use alloy::{
    network::AnyNetwork,
    primitives::{Address, Bytes, U256, TxKind, B256},
    providers:: { Provider as AlloyProvider, ProviderBuilder },
    rpc::types::{BlockNumberOrTag, TransactionRequest},
};
use foundry_fork_db::{cache::BlockchainDbMeta, BlockchainDb, SharedBackend};
use revm::{
    db::CacheDB,
    primitives::{
        BlockEnv, Bytes as RevmBytes, ExecutionResult, OptimismFields,
        TransactTo, TxEnv, Address as RevmAddress, U256 as RevmU256, B256 as RevmB256, AccessListItem, AccessList, BlobExcessGasAndPrice, AuthorizationList,
    },
    Evm,
};
use tracing::{debug, info, error};

/// Build a concrete AnyNetwork provider for simulation purposes.
/// 
/// This creates a provider specifically for use with the Foundry fork system, allowing
/// us to simulate transactions against a fork of the current network state.
/// 
/// # Arguments
///
/// * `rpc_url` - URL of the Ethereum RPC endpoint to connect to
///
/// # Returns
///
/// * A provider that can be used for blockchain interactions, or an error
fn build_any_provider(rpc_url: &str) -> Result<impl AlloyProvider<AnyNetwork> + Clone + Unpin + 'static, ServiceError> {
    // Parse the URL and handle errors
    let parsed = rpc_url.parse().map_err(|e| ServiceError::RPCConnection(format!("Bad URL: {e}")))?;

    // Create a new provider using the AnyNetwork type for flexibility
    let provider = ProviderBuilder::new()
        .network::<AnyNetwork>()
        .on_http(parsed);

    Ok(provider)
}

/// Estimate gas usage for a transaction by simulating it using Foundry's fork database
///
/// This function creates a fork of the blockchain at the latest block and simulates
/// the transaction execution to determine the exact gas required.
///
/// # Arguments
///
/// * `rpc_url` - The Ethereum RPC URL to use for forking
/// * `tx_request` - The transaction request to simulate
///
/// # Returns
///
/// * `Result<U256, ServiceError>` - The estimated gas on success, or an error
pub async fn estimate_gas_from_request_foundry(
    rpc_url: &str,
    tx_request: &TransactionRequest,
) -> Result<U256, ServiceError> {
    debug!("Building provider for RPC URL: {}", rpc_url);
    let provider = build_any_provider(rpc_url)?;

    debug!("Fetching the latest block");
    // Get the latest block for the fork point
    let block = provider
        .get_block(alloy::eips::BlockId::Number(BlockNumberOrTag::Latest))
        .await
        .map_err(|e| ServiceError::RPCConnection(format!("Failed to get latest block: {}", e)))?
        .ok_or_else(|| ServiceError::RPCConnection("Failed to get latest block".to_string()))?;
    debug!("Latest block fetched: number: {:?}, hash: {:?}", block.header.number, block.header.hash);

    debug!("Setting up fork at block {}", block.header.number);
    info!("Estimating gas with local fork DB at block: {:?}", block.header.number);

    // Create BlockchainDbMeta identifier for the fork
    let chain_id = provider.get_chain_id().await.unwrap_or(1);
    debug!("Using chain id: {}", chain_id);
    let meta = BlockchainDbMeta::default()
        .with_chain_id(chain_id)
        .with_block(&block);

    // Create a new blockchain database (or reuse from cache)
    debug!("Initializing blockchain database");
    let db = BlockchainDb::new(meta, None);

    // Spawn the backend with the database instance
    // This creates a shared backend that can fetch missing data from the RPC provider
    debug!("Spawning shared backend");
    let shared_backend = SharedBackend::spawn_backend(provider, db, None).await;
    debug!("Shared backend spawned successfully");

    // Configure EVM environment using the latest block's parameters
    let basefee = block.header.base_fee_per_gas.map(U256::from).unwrap_or_default();
    debug!("Block base fee: {:?}", basefee);
    
    // Create the block environment from the latest block data
    let block_env = BlockEnv {
        number: convert_u256(U256::from(block.header.number)),
        coinbase: convert_address(block.header.beneficiary),
        timestamp: convert_u256(U256::from(block.header.timestamp)),
        gas_limit: convert_u256(U256::from(30_000_000)), // High gas limit for simulation
        basefee: convert_u256(basefee),
        prevrandao: {
            let pr = block.header.mix_hash.expect("Block missing randao - are you on some esoteric chain or old pow block?");
            debug!("Block prevrandao (mix_hash): {:?}", pr);
            Some(pr)
        },
        difficulty: convert_u256(block.header.difficulty),
        blob_excess_gas_and_price: block
            .header
            .blob_gas_used
            .zip(block.header.excess_blob_gas)
            .map(|(used, excess)| {
                debug!("Block blob gas used: {}, excess blob gas: {}", used, excess);
                BlobExcessGasAndPrice {
                    blob_gasprice: used as u128,
                    excess_blob_gas: excess,
                }
            }),
    };
    debug!("EVM block environment configured: {:?}", block_env);

    // Create transaction environment from request
    debug!("Converting transaction request into EVM transaction environment");
    let tx_env = convert_tx_request_to_tx_env(tx_request)
        .map_err(|e| ServiceError::Simulation(e.to_string()))?;
    debug!("Transaction environment configured: {:?}", tx_env);

    // Execute the simulation in a blocking task to avoid blocking the async runtime
    debug!("Starting blocking REVM simulation");
    let gas_used = tokio::task::spawn_blocking(move || {
        debug!("Inside spawn_blocking: creating CacheDB and EVM instance");
        // The internal REVM call is synchronous, so keep it in blocking code
        let db = CacheDB::new(shared_backend);

        let mut evm = Evm::builder()
            .with_db(db)
            .with_block_env(block_env)
            .with_tx_env(tx_env)
            .build();
        debug!("EVM instance built, starting transaction simulation");

        // Execute the transaction simulation
        let result = evm
            .transact()
            .map_err(|e| {
                error!("EVM simulation failed: {:?}", e);
                ServiceError::Simulation(format!("EVM simulation failed: {:?}", e))
            })?;

        // Extract the gas used based on the execution result
        let gas_used = match result.result {
            ExecutionResult::Success { gas_used, .. } => {
                // For success, just log debug (or info)
                debug!("EVM simulation SUCCESS with gas_used: {}", gas_used);
                U256::from(gas_used)
            }
            ExecutionResult::Revert { gas_used, .. } => {
                // For revert, log an error
                error!("EVM simulation REVERTED with gas_used: {}", gas_used);
                U256::from(gas_used)
            }
            ExecutionResult::Halt { gas_used, .. } => {
                // For halt, also log an error
                error!("EVM simulation HALTED with gas_used: {}", gas_used);
                U256::from(gas_used)
            }
        };

        Ok::<U256, ServiceError>(gas_used)
    })
    .await
    .map_err(|e| {
        error!("spawn_blocking task failed: {:?}", e);
        ServiceError::Simulation(format!("spawn_blocking failed: {e:?}"))
    })??;
    
    debug!("Gas estimation completed successfully: {:?}", gas_used);
    Ok(gas_used)
}

/// Converts an Alloy TransactionRequest to REVM's TxEnv
///
/// This function translates between the Alloy and REVM type systems to prepare
/// a transaction for simulation in the EVM.
///
/// # Arguments
///
/// * `request` - The Alloy transaction request to convert
///
/// # Returns
///
/// * `Result<TxEnv, eyre::Error>` - The converted transaction environment or an error
pub fn convert_tx_request_to_tx_env(request: &TransactionRequest) -> Result<TxEnv, eyre::Error> {
    debug!("Starting conversion of TransactionRequest to TxEnv: {:?}", request);

    // 1) 'from' => caller
    let caller = match request.from {
        Some(addr) => {
            debug!("Using 'from' address: {:?}", addr);
            addr
        }
        None => {
            error!("Transaction request missing 'from' field");
            eyre::bail!("Transaction request missing 'from' field")
        }
    };

    // 2) 'to' => TxKind::Call(...) or TxKind::Create
    let transact_to = match request.to {
        Some(tx_kind) => match tx_kind {
            TxKind::Call(addr) => {
                debug!("Transaction type: Call, destination: {:?}", addr);
                TransactTo::Call(convert_address(addr))
            }
            TxKind::Create => {
                debug!("Transaction type: Create");
                TransactTo::Create
            }
        },
        None => {
            debug!("Transaction 'to' field missing, defaulting to Create");
            TransactTo::Create
        }
    };

    // 3) value
    let value = request.value.unwrap_or_default();
    debug!("Transaction value: {:?}", value);

    // 4) data from request.input
    let data = match request.input.input() {
        Some(bytes) => {
            debug!("Transaction input data found, length: {}", bytes.len());
            convert_bytes(bytes.clone())
        }
        None => {
            debug!("No transaction input data found, using empty Bytes");
            RevmBytes::default()
        }
    };

    // 5) gas limit
    let gas_limit = request.gas.unwrap_or(DEFAULT_GAS_LIMIT);
    debug!("Transaction gas limit: {}", gas_limit);

    // 6) gas pricing
    let gas_price = if let Some(max_fee) = request.max_fee_per_gas {
        debug!("EIP-1559 transaction detected, using max_fee_per_gas: {:?}", max_fee);
        convert_u256(U256::from(max_fee))
    } else if let Some(price) = request.gas_price {
        debug!("Legacy transaction detected, using gas_price: {:?}", price);
        convert_u256(U256::from(price))
    } else {
        // default
        debug!("No gas price specified, defaulting to 1 gwei");
        RevmU256::from(GWEI) // 1 gwei
    };

    let gas_priority_fee = request.max_priority_fee_per_gas.map(|fee| {
        debug!("Using max_priority_fee_per_gas: {:?}", fee);
        convert_u256(U256::from(fee))
    });

    // 7) Access list
    let access_list = match &request.access_list {
        Some(alist) => {
            debug!("Access list provided with {} entries", alist.len());
            convert_access_list(alist)
        }
        None => {
            debug!("No access list provided, using empty list");
            Vec::new()
        }
    };

    // 8) EIP-4844
    let blob_hashes = request
        .blob_versioned_hashes
        .clone()
        .unwrap_or_default()
        .into_iter()
        .map(|hash| {
            debug!("Converting blob versioned hash: {:?}", hash);
            convert_b256(hash)
        })
        .collect();

    let max_fee_per_blob_gas = request.max_fee_per_blob_gas.map(|fee| {
        debug!("Using max_fee_per_blob_gas: {:?}", fee);
        convert_u256(U256::from(fee))
    });

    // 9) EIP-7702 authorization
    let authorization_list = match &request.authorization_list {
        Some(list) => {
            debug!("Found EIP-7702 authorization list with {} items", list.len());
            let revm_auth_list = AuthorizationList::Signed(list.to_vec());
            Some(revm_auth_list)
        }
        None => {
            debug!("No authorization list provided");
            None
        }
    };

    // 10) Build the final TxEnv
    let tx_env = TxEnv {
        caller: convert_address(caller),
        gas_limit,
        gas_price,
        transact_to,
        value: convert_u256(value),
        data,
        nonce: request.nonce, // Option<u64>
        chain_id: request.chain_id,
        access_list,
        gas_priority_fee,
        blob_hashes,
        max_fee_per_blob_gas,
        authorization_list,
        optimism: OptimismFields::default(),
    };

    debug!("TxEnv conversion complete: {:?}", tx_env);
    Ok(tx_env)
}

// ----- Helper functions for type conversion -----

/// Convert an Alloy Address to a REVM Address
fn convert_address(address: Address) -> RevmAddress {
    let mut bytes = [0u8; 20];
    bytes.copy_from_slice(address.as_slice());
    RevmAddress::from(bytes)
}

/// Convert an Alloy U256 to a REVM U256
fn convert_u256(value: U256) -> RevmU256 {
    let bytes = value.to_be_bytes::<32>();
    RevmU256::from_be_bytes(bytes)
}

/// Convert Alloy Bytes to REVM Bytes
fn convert_bytes(bytes: Bytes) -> RevmBytes {
    RevmBytes::from(bytes.to_vec())
}

/// Convert an Alloy B256 to a REVM B256
fn convert_b256(hash: B256) -> RevmB256 {
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(hash.as_slice());
    RevmB256::from(bytes)
}

/// Convert an Alloy AccessList to a REVM AccessList
fn convert_access_list(access_list: &AccessList) -> Vec<AccessListItem> {
    access_list.0.iter().map(|item| {
        AccessListItem {
            address: convert_address(item.address),
            storage_keys: item.storage_keys.iter().map(|key| {
                let mut bytes = [0u8; 32];
                bytes.copy_from_slice(key.as_slice());
                RevmB256::from(bytes)
            }).collect(),
        }
    }).collect()
}