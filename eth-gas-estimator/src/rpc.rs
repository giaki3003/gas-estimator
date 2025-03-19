use std::sync::Arc;

use alloy::{
    // Import the pre-defined typed Ethereum network
    network::Ethereum,
    providers::{Provider, ProviderBuilder},
    // The typed RPC request / block / transaction types
    rpc::types::{BlockId, BlockNumberOrTag, Block},
};
use eyre::Result;

/// Ethereum RPC client for blockchain interactions
///
/// This client provides a typed interface for communicating with Ethereum nodes.
/// It uses the Alloy typed providers to ensure type safety in RPC interactions.
#[derive(Clone)]
pub struct EthereumClient {
    /// Typed provider for Ethereum network
    pub provider: Arc<dyn Provider<Ethereum>>,
}

impl EthereumClient {
    /// Create a new Ethereum client with an HTTP provider
    ///
    /// This constructor establishes a connection to an Ethereum node and
    /// verifies the connection is working by fetching the latest block number.
    ///
    /// # Arguments
    ///
    /// * `rpc_url` - URL of the Ethereum RPC endpoint
    ///
    /// # Returns
    ///
    /// * `Result<Self>` - New client instance or an error
    pub async fn new(rpc_url: &str) -> Result<Self> {
        // Create a provider for the Ethereum network at the specified URL
        let provider = ProviderBuilder::new()
            .network::<Ethereum>()
            .on_http(rpc_url.parse()?);

        // Test the connection by fetching the latest block number
        let block_number = provider.get_block_number().await?;
        println!("Connected! Latest block number: {block_number}");

        Ok(Self {
            provider: Arc::new(provider),
        })
    }

    /// Fetch the latest block from the Ethereum network
    ///
    /// # Returns
    ///
    /// * `Result<Block>` - The latest block or an error
    pub async fn get_latest_block(&self) -> Result<Block> {
        // Request the latest block from the provider
        let maybe_block = self
            .provider
            .get_block(BlockId::Number(BlockNumberOrTag::Latest))
            .await?;

        // Ensure a block was returned
        let block = maybe_block.ok_or_else(|| eyre::eyre!("No latest block returned"))?;
        Ok(block)
    }
}