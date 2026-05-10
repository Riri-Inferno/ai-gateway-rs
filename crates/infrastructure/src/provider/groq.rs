//! Groq API クライアント
//! OpenAI互換のchat/completionsエンドポイントを叩く想定。
//! 仕様: <https://console.groq.com/docs/api-reference#chat-create>

use application::port::AiProvider;
use async_trait::async_trait;
use domain::model::completion::{
    ChatCompletionRequest, ChatCompletionResponse, ContentPart, MessageContent, Role, Usage,
};
use domain::model::provider::ProviderId;
use domain::DomainError;
use serde::{Deserialize, Serialize};

const ENDPOINT: &str = "https://api.groq.com/openai/v1/chat/completions";

pub struct GroqClient {
    api_key: String,
    http: reqwest::Client,
}

impl GroqClient {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            http: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl AiProvider for GroqClient {
    fn id(&self) -> ProviderId {
        ProviderId::Groq
    }

    async fn chat_completion(
        &self,
        req: &ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, DomainError> {
        // メッセージを Groq の wire 型に詰め替える
        let mut messages: Vec<GroqMessage> = Vec::new();
        for msg in &req.messages {
            // domain の MessageContent → Groq の content（string or array）に変換
            // テキストのみなら文字列、画像混在なら配列で送る
            // （古いモデルは配列形式を受け付けないため、なるべく単純な形で送る）
            let content = domain_content_to_groq(&msg.content);
            let role = match msg.role {
                Role::System => "system",
                Role::User => "user",
                Role::Assistant => "assistant",
                Role::Tool => "tool",
            };
            messages.push(GroqMessage {
                role: role.into(),
                content,
            });
        }

        let body = GroqRequest {
            model: req.model.clone(),
            messages,
            temperature: req.temperature,
            max_tokens: req.max_tokens,
        };

        let resp = self
            .http
            .post(ENDPOINT)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| DomainError::ProviderError(format!("request failed: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(DomainError::ProviderError(format!(
                "groq call status={status} body={text}"
            )));
        }

        let parsed: GroqResponse = resp
            .json()
            .await
            .map_err(|e| DomainError::ProviderError(format!("decode failed: {e}")))?;

        let content = parsed
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .ok_or_else(|| DomainError::ProviderError("empty choices".into()))?;

        Ok(ChatCompletionResponse {
            provider: ProviderId::Groq,
            model: req.model.clone(),
            content,
            usage: parsed.usage.map(|u| Usage {
                prompt_tokens: u.prompt_tokens,
                completion_tokens: u.completion_tokens,
                total_tokens: u.total_tokens,
            }),
        })
    }
}

/// domain MessageContent → Groq wire content
/// テキスト単体なら GroqContent::Text、partsなら GroqContent::Parts に詰め替える
fn domain_content_to_groq(c: &MessageContent) -> GroqContent {
    match c {
        MessageContent::Text(s) => GroqContent::Text(s.clone()),
        MessageContent::Parts(parts) => GroqContent::Parts(
            parts
                .iter()
                .map(|p| match p {
                    ContentPart::Text { text } => GroqContentPart::Text { text: text.clone() },
                    ContentPart::ImageUrl { image_url } => GroqContentPart::ImageUrl {
                        image_url: GroqImageUrl {
                            url: image_url.url.clone(),
                            detail: image_url.detail.clone(),
                        },
                    },
                })
                .collect(),
        ),
    }
}

// ===== Groq API の wire型（OpenAI互換） =====

#[derive(Serialize)]
struct GroqRequest {
    model: String,
    messages: Vec<GroqMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
}

#[derive(Serialize)]
struct GroqMessage {
    role: String,
    content: GroqContent,
}

// `#[serde(untagged)]`: シリアライズ時にどっちのvariantかでJSON形が変わる。
// Text ならただの文字列、Parts なら配列。OpenAI互換APIの仕様。
#[derive(Serialize)]
#[serde(untagged)]
enum GroqContent {
    Text(String),
    Parts(Vec<GroqContentPart>),
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum GroqContentPart {
    Text { text: String },
    ImageUrl { image_url: GroqImageUrl },
}

#[derive(Serialize)]
struct GroqImageUrl {
    url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<String>,
}

#[derive(Deserialize)]
struct GroqResponse {
    choices: Vec<GroqChoice>,
    usage: Option<GroqUsage>,
}

#[derive(Deserialize)]
struct GroqChoice {
    message: GroqResponseMessage,
}

#[derive(Deserialize)]
struct GroqResponseMessage {
    content: String,
}

#[derive(Deserialize)]
struct GroqUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}
