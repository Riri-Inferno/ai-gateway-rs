use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Context;
use application::port::AiProvider;
use application::usecase::chat_completion::ChatCompletionUseCase;
use domain::model::provider::ProviderId;
use infrastructure::config::AppConfig;
use infrastructure::provider::google_ai_studio::GoogleAiStudioClient;
use presentation::{build_router, AppState};
use tracing_subscriber::EnvFilter;

// `#[tokio::main]`: 通常 Rust の `fn main()` は同期だが、この属性で
// tokio非同期ランタイムを起動した上で `async fn main` を実行できる。
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();

    tracing_subscriber::fmt()
        .json()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    let config = AppConfig::from_env();

    // ===== DI: 利用可能なプロバイダを構築してUseCaseに注入 =====
    // 環境変数にAPIキーがあるものだけ登録。無ければそのプロバイダは404を返す。
    let mut providers: HashMap<ProviderId, Arc<dyn AiProvider>> = HashMap::new();
    if let Some(key) = config.google_ai_studio_api_key.clone() {
        providers.insert(
            ProviderId::GoogleAiStudio,
            Arc::new(GoogleAiStudioClient::new(key)),
        );
        tracing::info!("registered provider: google_ai_studio");
    } else {
        tracing::warn!("GOOGLE_AI_STUDIO_API_KEY not set; provider unavailable");
    }

    let state = AppState {
        chat: Arc::new(ChatCompletionUseCase::new(providers)),
        allowed_api_keys: Arc::new(config.gateway_api_keys.clone()),
    };

    if state.allowed_api_keys.is_empty() {
        tracing::warn!("GATEWAY_API_KEYS is empty; all /v1 requests will be rejected with 401");
    }

    let app = build_router(state);

    let listener = tokio::net::TcpListener::bind(&config.bind)
        .await
        .with_context(|| format!("failed to bind {}", config.bind))?;

    tracing::info!(addr = %config.bind, "ai-gatewey-rs listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

// k3sのrolling update時、Podには SIGTERM が送られる。これを捕捉して
// 進行中リクエストを終わらせてから停止するのが「graceful shutdown」。
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {}
        _ = terminate => {}
    }
}
