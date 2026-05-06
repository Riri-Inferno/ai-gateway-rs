use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::provider::ProviderId;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ChatMessage {
    pub role: Role,
    pub content: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ChatCompletionRequest {
    pub provider: ProviderId,
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ChatCompletionResponse {
    pub provider: ProviderId,
    pub model: String,
    pub content: String,
    pub usage: Option<Usage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}
