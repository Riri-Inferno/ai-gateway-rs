use utoipa::OpenApi;

use crate::handler::health;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "ai-gatewey-rs",
        version = "0.1.0",
        description = "AI Gateway for relaying multiple AI APIs (homelab use)"
    ),
    paths(health::healthz, health::readyz),
    components(schemas(health::HealthResponse))
)]
pub struct ApiDoc;
