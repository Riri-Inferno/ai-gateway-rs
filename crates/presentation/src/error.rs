use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use domain::DomainError;
use serde_json::json;

// ドメインエラー → HTTPレスポンスに変換する境界の型。
// `IntoResponse` を実装することで、ハンドラの戻り値で `Result<_, ApiError>` が使える。
#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    message: String,
}

impl From<DomainError> for ApiError {
    fn from(e: DomainError) -> Self {
        let status = match &e {
            DomainError::ProviderNotFound(_) => StatusCode::NOT_FOUND,
            DomainError::InvalidRequest(_) => StatusCode::BAD_REQUEST,
            DomainError::ProviderError(_) => StatusCode::BAD_GATEWAY,
        };
        Self {
            status,
            message: e.to_string(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(json!({ "error": self.message }))).into_response()
    }
}
