//! インフラ層: 外部システム（HTTP API・環境変数等）の実装。
//! application層のtraitを実装する形で外側から差し込まれる。

pub mod config;
pub mod provider;
