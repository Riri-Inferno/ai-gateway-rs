use axum::Json;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct HealthResponse {
    pub status: &'static str,
}

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
