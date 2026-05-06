//! プレゼンテーション層: HTTP境界。axumハンドラ・ルーティング・OpenAPI定義を持つ。

pub mod handler;
pub mod openapi;
pub mod router;

pub use router::build_router;
