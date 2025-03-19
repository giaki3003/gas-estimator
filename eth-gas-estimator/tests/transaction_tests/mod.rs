//! Transaction type tests
//!
//! Tests for different Ethereum transaction types and their gas estimation.

use std::sync::Once;
use tracing_subscriber::EnvFilter;

pub mod eip1559_tests;
pub mod eip2930_tests;
pub mod eip4844_tests;
pub mod eip7702_tests;
pub mod legacy_tests;

static INIT: Once = Once::new();

/// Initializes the global logger (only once).
pub fn init_logger() {
    INIT.call_once(|| {
        let filter = EnvFilter::from_default_env()
            .add_directive("eth_gas_estimator=info".parse().unwrap())
            .add_directive("actix_web=error".parse().unwrap())
            .add_directive("hyper=warn".parse().unwrap())
            .add_directive("reqwest=warn".parse().unwrap());
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .init();
    });
}