use crate::{
    error::ServiceError,
    rpc::EthereumClient,
    foundry::estimate_gas_from_request_foundry,
};
use alloy::{
    primitives::U256,
    rpc::types::TransactionRequest,
};
use eyre::Result;
use std::sync::Arc;
use tracing::{debug, instrument, error};

/// Gas unit constants
pub const GWEI: u64 = 1_000_000_000;

/// Default gas limit for simple Ethereum transfers (21,000 gas)
pub const DEFAULT_GAS_LIMIT: u64 = 21_000;

/// Default gas price in gwei (10 gwei)
pub const DEFAULT_GAS_PRICE: u64 = 10 * GWEI;

/// Gas estimator service that calculates gas requirements for Ethereum transactions
///
/// This service provides methods for estimating gas usage of Ethereum transactions
/// using either local simulation with REVM or by falling back to RPC methods.
#[derive(Clone)]
pub struct GasEstimator {
    /// Ethereum client for interacting with the blockchain
    pub eth_client: Arc<EthereumClient>,
    /// RPC URL used for creating simulation forks
    rpc_url: String,
}

impl GasEstimator {
    /// Creates a new gas estimator with the provided client and RPC URL
    pub fn new(eth_client: Arc<EthereumClient>, rpc_url: &str) -> Self {
        Self {
            eth_client,
            rpc_url: rpc_url.to_string(),
        }
    }

    /// Estimate gas for a transaction using fork-based simulation
    ///
    /// This method attempts to simulate the transaction execution using a forked
    /// state of the blockchain and returns the estimated gas limit required.
    ///
    /// # Arguments
    ///
    /// * `tx_request` - The transaction request parameters
    ///
    /// # Returns
    ///
    /// * `Result<U256>` - The estimated gas limit on success, or an error
    #[instrument(skip(self, tx_request), err)]
    pub async fn estimate_raw_gas(&self, tx_request: &TransactionRequest) -> Result<U256> {
        debug!("Starting gas estimation for transaction request: {:?}", tx_request);

        // Attempt to estimate gas using local simulation with REVM
        match estimate_gas_from_request_foundry(&self.rpc_url, tx_request).await {
            Ok(gas) => {
                debug!("Simulation succeeded, estimated gas: {}", gas);
                Ok(gas)
            },
            Err(e) => {
                error!("Simulation failed with error: {}", e);
                Err(ServiceError::Estimation("Failed to estimate gas".to_string()).into())
            }
        }
    }
}