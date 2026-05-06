use axum::routing::get;
use axum::Router;
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::handler::health;
use crate::openapi::ApiDoc;

// axumのRouterはビルダーパターン: メソッドチェーンで構築する。
// `merge` で別のRouter（ここではSwaggerUi）を統合、`layer` でmiddlewareを追加。
pub fn build_router() -> Router {
    Router::new()
        .route("/healthz", get(health::healthz))
        .route("/readyz", get(health::readyz))
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .layer(TraceLayer::new_for_http())
}
