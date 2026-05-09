use domain::model::completion::{
    ChatCompletionRequest, ChatCompletionResponse, ChatMessage, Role, Usage,
};
use domain::model::provider::ProviderId;
use utoipa::openapi::security::{ApiKey, ApiKeyValue, SecurityScheme};
use utoipa::{Modify, OpenApi};

use crate::handler::{chat, health};

// `#[derive(OpenApi)]` + `#[openapi(paths(...), components(schemas(...)))]` で
// 全ハンドラのOpenAPI仕様を集約。`ApiDoc::openapi()` で生成済みJSONを取り出せる
#[derive(OpenApi)]
#[openapi(
    info(
        title = "ai-gateway-rs",
        version = "0.1.0",
        description = "AI Gateway for relaying multiple AI APIs (homelab use)"
    ),
    paths(
        health::healthz,
        health::readyz,
        chat::chat_completion,
    ),
    components(schemas(
        health::HealthResponse,
        ChatCompletionRequest,
        ChatCompletionResponse,
        ChatMessage,
        Role,
        Usage,
        ProviderId,
    )),
    modifiers(&SecurityAddon),
    tags(
        (name = "system", description = "Liveness/Readiness probes"),
        (name = "chat", description = "Chat completion (provider-agnostic)"),
    )
)]
pub struct ApiDoc;

// Swagger UI に「X-API-Key を入れる欄」を出すための仕組み
struct SecurityAddon;
impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "api_key",
                SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("X-API-Key"))),
            );
        }
    }
}
