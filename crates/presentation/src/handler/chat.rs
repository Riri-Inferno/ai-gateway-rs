use axum::extract::State;
use axum::Json;
use domain::model::completion::{ChatCompletionRequest, ChatCompletionResponse};

use crate::error::ApiError;
use crate::state::AppState;

#[utoipa::path(
    post,
    path = "/v1/chat/completions",
    request_body = ChatCompletionRequest,
    responses(
        (status = 200, description = "Completion result", body = ChatCompletionResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Provider not found"),
        (status = 502, description = "Upstream provider error"),
    ),
    tag = "chat",
    security(("api_key" = []))
)]
pub async fn chat_completion(
    State(state): State<AppState>,
    Json(body): Json<ChatCompletionRequest>,
) -> Result<Json<ChatCompletionResponse>, ApiError> {
    let resp = state.chat.execute(body).await?;
    Ok(Json(resp))
}
