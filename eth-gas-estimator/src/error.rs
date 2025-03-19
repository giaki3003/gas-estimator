use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use serde::Serialize;
use thiserror::Error;

/// Service-specific error types
///
/// This enum defines all possible errors that can occur in the gas estimation service.
/// Each variant represents a specific error case and includes relevant details.
#[derive(Error, Debug)]
pub enum ServiceError {
    #[error("RPC connection error: {0}")]
    RPCConnection(String),

    #[error("Transaction simulation failed: {0}")]
    Simulation(String),

    #[error("Gas estimation failed: {0}")]
    Estimation(String),
}

/// Structured error response for the API
///
/// This structure defines the JSON format of error responses returned by the API.
#[derive(Serialize)]
struct ErrorResponse {
    /// Human-readable error message
    error: String,
    
    /// Machine-readable error code
    error_code: String,
    
    /// Optional detailed error information
    details: Option<String>,
}

impl ResponseError for ServiceError {
    /// Convert the error to an HTTP response
    ///
    /// This method generates an appropriate HTTP response based on the error type,
    /// including status code and a JSON error body.
    fn error_response(&self) -> HttpResponse {
        let (status_code, error_code, details) = match self {
            ServiceError::RPCConnection(details) => (
                StatusCode::BAD_GATEWAY,
                "RPC_CONNECTION_ERROR",
                Some(details.clone()),
            ),
            ServiceError::Simulation(details) => (
                StatusCode::BAD_REQUEST,
                "SIMULATION_ERROR",
                Some(details.clone()),
            ),
            ServiceError::Estimation(details) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "ESTIMATION_ERROR",
                Some(details.clone()),
            ),
        };

        HttpResponse::build(status_code).json(ErrorResponse {
            error: self.to_string(),
            error_code: error_code.to_string(),
            details,
        })
    }

    /// Get the HTTP status code for this error
    fn status_code(&self) -> StatusCode {
        match *self {
            ServiceError::RPCConnection(_) => StatusCode::BAD_GATEWAY,
            ServiceError::Simulation(_) => StatusCode::BAD_REQUEST,
            ServiceError::Estimation(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}