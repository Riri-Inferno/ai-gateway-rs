use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("provider not found: {0}")]
    ProviderNotFound(String),

    #[error("invalid request: {0}")]
    InvalidRequest(String),
}
