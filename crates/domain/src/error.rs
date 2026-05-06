use thiserror::Error;

// `#[derive(Error)]` で `std::error::Error` 実装を自動生成（thiserrorクレートのマクロ）
#[derive(Debug, Error)]
pub enum DomainError {
    #[error("provider not found: {0}")]
    ProviderNotFound(String),

    #[error("invalid request: {0}")]
    InvalidRequest(String),

    #[error("provider error: {0}")]
    ProviderError(String),
}
