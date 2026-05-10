//! OpenRouter API クライアント
//! 仕様: <https://openrouter.ai/docs/api/reference/overview>

use application::port::AiProvider;
use async_trait::async_trait;
use domain::model::completion::{
    ChatCompletionRequest, ChatCompletionResponse, ContentPart, MessageContent, Role, Usage,
};
use domain::model::provider::ProviderId;
use domain::DomainError;
use serde::{Deserialize, Serialize};

const ENDPOINT: &str = "https://openrouter.ai/api/v1/chat/completions";

pub struct OpenRouterClient {
    api_key: String,
    http: reqwest::Client,
}

impl OpenRouterClient {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            http: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl AiProvider for OpenRouterClient {
    fn id(&self) -> ProviderId {
        ProviderId::OpenRouter
    }

    async fn chat_completion(
        &self,
        req: &ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, DomainError> {
        // メッセージインスタンスを宣言
        let mut messages: Vec<OpenRouterMessage> = Vec::new();

        // リクエストに含まれるるロールを元にメッセージを作る
        for msg in &req.messages {
            let content = domain_content_to_openrouter(&msg.content);

            let role = match msg.role {
                Role::System => "system",
                Role::User => "user",
                Role::Assistant => "assistant",
                Role::Tool => "tool",
            };
            messages.push(OpenRouterMessage {
                role: role.into(),
                content,
                name: None
            });
        }

        // クライアントつくる
        let body = OpenRouterRequest {
            model: req.model.clone(),
            messages,
            temperature: req.temperature,
            max_tokens: req.max_tokens,
        };

        // リクエストを飛ばす
        let resp = self
            .http
            .post(ENDPOINT)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| DomainError::ProviderError(format!("request failed: {e}")))?;

        // OpenRouterAPI呼び出しが成功じゃないときはエラーを返して終了
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(DomainError::ProviderError(format!(
                "OpenRouter call status={status} body={text}"
            )));
        }

        // APIからのレスポンスをデコード
        let parsed: OpenRouterResponse = resp
            .json()
            .await
            .map_err(|e| DomainError::ProviderError(format!("decode failed: {e}")))?;

        // パース済レスポンスからcontentを取り出す。choices無し or content無しでエラー
        let content = parsed
            .choices
            .into_iter()
            .next()
            .and_then(|c| c.message.content)
            .ok_or_else(|| DomainError::ProviderError("empty content".into()))?;

        // レスポンスを組み立てて返す
        Ok(ChatCompletionResponse {
            provider: ProviderId::OpenRouter,
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

/// domain MessageContent → OpenRouter wire content
/// テキスト単体なら OpenRouter::Text、partsなら OpenRouterContent::Parts に詰め替える
fn domain_content_to_openrouter(c: &MessageContent) -> OpenRouterContent {
    match c {
        MessageContent::Text(s) => OpenRouterContent::Text(s.clone()),
        MessageContent::Parts(parts) => OpenRouterContent::Parts(
            parts
                .iter()
                .map(|p| match p {
                    ContentPart::Text { text } => OpenRouterContentPart::Text { text: text.clone() },
                    ContentPart::ImageUrl { image_url } => OpenRouterContentPart::ImageUrl {
                        image_url: OpenRouterImageUrl {
                            url: image_url.url.clone(),
                            detail: image_url.detail.clone(),
                        },
                    },
                })
                .collect(),
        ),
    }
}

// ===== OpenRouter API wire types =====
// roleは小文字 ("system" | "user" | "assistant")。
// snake_case フィールド名 (max_tokens / prompt_tokens 等) はそのまま使われる。

#[derive(Serialize, Default)]
pub struct OpenRouterRequest {
    pub model: String, // 未指定ならデフォルト
    pub messages: Vec<OpenRouterMessage>,

    // 生成制御パラメータ
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
}

#[derive(Serialize)]
pub struct OpenRouterMessage {
    pub role: String, // "system", "user", "assistant", "tool"
    pub content: OpenRouterContent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

// `#[serde(untagged)]`: シリアライズ時に variant に応じてJSON形が変わる。
// Text ならただの文字列、Parts なら配列。OpenAI互換APIの仕様。
#[derive(Serialize)]
#[serde(untagged)]
pub enum OpenRouterContent {
    Text(String),
    Parts(Vec<OpenRouterContentPart>),
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OpenRouterContentPart {
    Text { text: String },
    ImageUrl { image_url: OpenRouterImageUrl },
}

#[derive(Serialize)]
pub struct OpenRouterImageUrl {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

// ===== Response types =====

#[derive(Deserialize)]
pub struct OpenRouterResponse {
    pub id: String,
    pub choices: Vec<OpenRouterChoice>,
    pub usage: Option<OpenRouterUsage>,
    pub model: String,
}

#[derive(Deserialize)]
pub struct OpenRouterChoice {
    pub message: OpenRouterResponseMessage,
    pub finish_reason: Option<String>,
}

#[derive(Deserialize)]
pub struct OpenRouterResponseMessage {
    pub role: String,
    pub content: Option<String>,
}

#[derive(Deserialize)]
pub struct OpenRouterUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}
