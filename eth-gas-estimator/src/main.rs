use tracing_subscriber::EnvFilter;
use crate::estimator::GasEstimator;
use actix_web::{web, App, HttpServer};
use std::sync::Arc;
use tracing_actix_web::TracingLogger;

mod api;
mod config;
mod error;
mod estimator;
mod models;
mod rpc;
mod foundry;

/// Application entry point
/// 
/// This is the main function that:
/// 1. Sets up logging
/// 2. Loads configuration
/// 3. Establishes connection to Ethereum node
/// 4. Creates the gas estimator service
/// 5. Starts the HTTP server with all endpoints
#[actix_web::main] // Actix will build a multithreaded runtime
async fn main() -> std::io::Result<()> {
    // Configure logging with appropriate log levels for different components
    // - Debug level for our service
    // - Lower levels for dependencies to reduce noise
    let filter = EnvFilter::from_default_env()
        .add_directive("eth_gas_estimator=info".parse().unwrap())
        .add_directive("actix_web=error".parse().unwrap())
        .add_directive("hyper=warn".parse().unwrap())
        .add_directive("reqwest=warn".parse().unwrap());
    
    // Initialize the tracing subscriber with our filter
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .init();

    // Load configuration from environment variables
    let config = config::Config::from_env().expect("Failed to load config");

    // Create Ethereum RPC client and handle potential connection errors
    let eth_client = rpc::EthereumClient::new(&config.ethereum_rpc_url)
        .await
        .expect("Failed to connect to Ethereum");

    // Build GasEstimator and wrap it in Arc for thread-safe sharing
    let estimator = Arc::new(
        GasEstimator::new(eth_client.into(), &config.ethereum_rpc_url),
    );

    // Create and start HTTP server
    HttpServer::new(move || {
        App::new()
            // Add logging middleware
            .wrap(TracingLogger::default())
            // Register the estimator as application data (shared between requests)
            .app_data(web::Data::new(estimator.clone())) 
            // Configure API routes
            .configure(api::configure)
    })
    // Set number of worker threads
    .workers(4)
    // Bind to host/port from configuration
    .bind(format!("{}:{}", config.host, config.port))?
    // Start the server
    .run()
    .await
}