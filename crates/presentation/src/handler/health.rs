use axum::Json;
use serde::Serialize;
use utoipa::ToSchema;

// `ToSchema` 派生で OpenAPI のスキーマ定義をutoipaに認識させる
#[derive(Debug, Serialize, ToSchema)]
pub struct HealthResponse {
    pub status: &'static str,
}

// `#[utoipa::path(...)]`: このハンドラのOpenAPI仕様を宣言。
// `paths(health::healthz)` を ApiDoc 側に列挙すると Swagger UI に出る
#[utoipa::path(
    get,
    path = "/healthz",
    responses(
        (status = 200, description = "Liveness OK", body = HealthResponse)
    ),
    tag = "system"
)]
pub async fn healthz() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

#[utoipa::path(
    get,
    path = "/readyz",
    responses(
        (status = 200, description = "Readiness OK", body = HealthResponse)
    ),
    tag = "system"
)]
pub async fn readyz() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ready" })
}
