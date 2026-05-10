use axum::extract::DefaultBodyLimit;
use axum::routing::{get, post};
use axum::Router;
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::handler::{chat, health};
use crate::middleware::api_key::require_api_key;
use crate::openapi::ApiDoc;
use crate::state::AppState;

// 画像入力で数MBのbodyが普通に飛んでくるため、axum既定の2MBを引き上げる
const MAX_BODY_BYTES: usize = 16 * 1024 * 1024;

// axumのRouterはビルダーパターン: メソッドチェーンで構築する。
// `route_layer` は「直前までに追加されたroute群にだけmiddlewareを適用」する。
// → /v1/chat/completions のみ認証、/healthzやSwaggerUIは素通り。
pub fn build_router(state: AppState) -> Router {
    let v1 = Router::new()
        .route("/chat/completions", post(chat::chat_completion))
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            require_api_key,
        ));

    Router::new()
        .nest("/v1", v1)
        .route("/healthz", get(health::healthz))
        .route("/readyz", get(health::readyz))
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
