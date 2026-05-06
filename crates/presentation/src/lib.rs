//! プレゼンテーション層: HTTP境界。axumハンドラ・ルーティング・OpenAPI定義を持つ。

pub mod error;
pub mod handler;
pub mod middleware;
pub mod openapi;
pub mod router;
pub mod state;

pub use router::build_router;
pub use state::AppState;
