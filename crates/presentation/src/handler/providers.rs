use axum::extract::State;
use axum::Json;
use domain::model::provider::ProviderId;
use serde::Serialize;
use utoipa::ToSchema;

use crate::state::AppState;

#[derive(Debug, Serialize, ToSchema)]
pub struct ProvidersResponse {
    pub providers: Vec<ProviderId>,
}

#[utoipa::path(
    get,
    path = "/v1/providers",
    responses(
        (status = 200, description = "現在configureされているプロバイダ一覧", body = ProvidersResponse),
        (status = 401, description = "Unauthorized"),
    ),
    tag = "providers",
    security(("api_key" = []))
)]
pub async fn list_providers(State(state): State<AppState>) -> Json<ProvidersResponse> {
    // HashMapのキー順は不定なので、出力安定化のために sort
    let mut providers = state.chat.available_providers();
    providers.sort_by_key(|p| p.as_str());

    Json(ProvidersResponse { providers })
}
