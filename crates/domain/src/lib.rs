//! ドメイン層: ビジネス上の概念のみ。外部crate（axum/reqwest等）に依存しないこと。

pub mod error;
pub mod model;

// 親モジュールから `domain::DomainError` で参照できるよう再エクスポート
pub use error::DomainError;
