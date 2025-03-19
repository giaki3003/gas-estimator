use eyre::Result;
use serde::Deserialize;
use std::env;

/// Service configuration structure
///
/// This structure contains all the configuration parameters for the gas estimation service.
/// It handles loading values from environment variables with appropriate defaults.
#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    /// Host address to bind the server to (default: 127.0.0.1)
    pub host: String,
    
    /// Port to listen on (default: 8080)
    pub port: u16,
    
    /// Ethereum RPC endpoint URL for communicating with the blockchain
    pub ethereum_rpc_url: String,
}

impl Config {
    /// Load configuration from environment variables
    ///
    /// This method reads configuration from environment variables,
    /// using default values when variables are not defined.
    ///
    /// # Returns
    ///
    /// * `Result<Self>` - Configuration structure or error
    ///
    /// # Environment Variables
    ///
    /// * `HOST` - Server host address (default: "127.0.0.1")
    /// * `PORT` - Server port (default: 8080)
    /// * `ETHEREUM_RPC_URL` - Ethereum RPC URL (default: "http://localhost:8545")
    pub fn from_env() -> Result<Self> {
        // Load .env file if it exists (useful for development)
        let _ = dotenv::dotenv();
        
        // Create configuration with values from environment or defaults
        Ok(Config {
            host: env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            port: env::var("PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse::<u16>()?,
            ethereum_rpc_url: env::var("ETHEREUM_RPC_URL")
                .unwrap_or_else(|_| "http://localhost:8545".to_string()),
        })
    }
}