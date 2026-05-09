//! OpenRouter API クライアント
//! 仕様: <https://openrouter.ai/docs/api/reference/overview>

use application::port::AiProvider;
use async_trait::async_trait;
use domain::model::completion::{ChatCompletionRequest, ChatCompletionResponse, Role, Usage};
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
            match msg.role {
                Role::System => messages.push(OpenRouterMessage {
                    role: "system".into(),
                    content: msg.content.clone(),
                    name: None
                }),
                Role::User => messages.push(OpenRouterMessage {
                    role: "user".into(),
                    content: msg.content.clone(),
                    name: None
                }),
                Role::Assistant => messages.push(OpenRouterMessage {
                    role: "assistant".into(),
                    content: msg.content.clone(),
                    name: None
                }),
                Role::Tool  => messages.push(OpenRouterMessage {
                    role: "tool".into(),
                    content: msg.content.clone(),
                    name: None
                }),
            }
        }

        // クライアントつくる
        let body = OpenRouterRequest {
            model: req.model.clone(),
            messages,
            temperature: req.temperature,
            max_tokens: req.max_tokens,
            ..Default::default()
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

        // パース済レスポンスからchoiceを所有権ごと取り出す
        let content = parsed
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .ok_or_else(|| DomainError::ProviderError("empty choices".into()))?;

        // レスポンスを組み立てて返す
        // Usageはないパラメータは無視
        Ok(ChatCompletionResponse {
            provider: ProviderId::OpenRouter,
            model: req.model.clone(),
            content: content.unwrap_or_default(),
            usage: parsed.usage.map(|u| Usage {
                prompt_tokens: u.prompt_tokens,
                completion_tokens: u.completion_tokens,
                total_tokens: u.total_tokens,
            }),
        })
    }
}

// ===== OpenRouter API wire types =====
// roleは小文字 ("system" | "user" | "assistant")。
// snake_case フィールド名 (max_tokens / prompt_tokens 等) はそのまま使われる。

#[derive(Serialize, Default)]
pub struct OpenRouterRequest {
    pub model: String, // 未指定ならデフォルト
    pub messages: Vec<OpenRouterMessage>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,

    // 生成制御パラメータ
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,

    // 構造化出力
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,

    // OpenRouter 独自・詳細設定
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transforms: Option<Vec<String>>, // "plugins" の一部
    #[serde(skip_serializing_if = "Option::is_none")]
    pub models: Option<Vec<String>>,      // フォールバック用
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route: Option<String>,           // "fallback"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,            // 悪用検知用ID
}

#[derive(Serialize, Deserialize)]
pub struct OpenRouterMessage {
    pub role: String, // "system", "user", "assistant", "tool"
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum OpenRouterContentPart {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image_url")]
    Image { image_url: OpenRouterImageUrl },
}

// TODO:後で拡張するかも
#[derive(Serialize, Deserialize)]
pub struct OpenRouterImageUrl {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>, // "auto", "low", "high"
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ResponseFormat {
    #[serde(rename = "json_object")]
    JsonObject,
    #[serde(rename = "json_schema")]
    JsonSchema { json_schema: JsonSchemaDef },
}

#[derive(Serialize, Deserialize)]
pub struct JsonSchemaDef {
    pub name: String,
    pub strict: Option<bool>,
    pub schema: serde_json::Value,
}

// ===== Response types =====

#[derive(Deserialize)]
pub struct OpenRouterResponse {
    pub id: String,
    pub choices: Vec<Choice>,
    pub usage: Option<Usage>,
    pub model: String,
}

#[derive(Deserialize)]
pub struct Choice {
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
