use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::provider::ProviderId;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ChatMessage {
    pub role: Role,
    pub content: MessageContent,
}

// `#[serde(untagged)]`: JSONでは "content": "hello" でも "content": [...] でも受け付ける。
// 後方互換のため Text 単体も許容（既存の文字列リクエストを壊さない）
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(untagged)]
pub enum MessageContent {
    /// 単純な文字列（テキストのみ、後方互換用）
    Text(String),
    /// マルチモーダル: text/image を混ぜたパーツ列
    Parts(Vec<ContentPart>),
}

// `#[serde(tag = "type")]`: JSONの type フィールドでvariantを判定
//   {"type": "text", "text": "..."}
//   {"type": "image_url", "image_url": {...}}
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentPart {
    Text { text: String },
    ImageUrl { image_url: ImageUrl },
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ImageUrl {
    /// data: URL（base64エンコードされた画像データ）または http(s) URL
    pub url: String,
    /// "low" / "high" / "auto"（OpenAI互換、Geminiは無視）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl MessageContent {
    /// テキストのみ抽出（system instruction 等、画像を扱えない場面用）
    pub fn extract_text(&self) -> String {
        match self {
            Self::Text(s) => s.clone(),
            Self::Parts(parts) => parts
                .iter()
                .filter_map(|p| match p {
                    ContentPart::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n"),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool
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
