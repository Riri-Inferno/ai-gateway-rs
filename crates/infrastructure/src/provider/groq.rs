//! Groq API クライアント
//! OpenAI互換のchat/completionsエンドポイントを叩く想定。
//! 仕様: <https://console.groq.com/docs/api-reference#chat-create>

use application::port::AiProvider;
use async_trait::async_trait;
use domain::model::completion::{ChatCompletionRequest, ChatCompletionResponse, Role, Usage};
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
        // メッセージインスタンスを宣言
        let mut messages: Vec<GroqMessage> = Vec::new();

        // リクエストに含まれるるロールを元にメッセージを作る
        for msg in &req.messages {
            match msg.role {
                Role::System => messages.push(GroqMessage {
                    role: "system".into(),
                    content: msg.content.clone()
                }),
                Role::User => messages.push(GroqMessage {
                    role: "user".into(),
                    content: msg.content.clone(),
                }),
                Role::Assistant => messages.push(GroqMessage {
                    role: "assistant".into(),
                    content: msg.content.clone(),
                })
            }
        }

        // クライアントつくる
        let body = GroqRequest {
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

        // GroqAPI呼び出しが成功じゃないときはエラーを返して終了
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(DomainError::ProviderError(format!(
                "groq call status={status} body={text}"
            )));
        }

        // APIからのレスポンスをデコード
        let parsed: GroqResponse = resp
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

// ===== Groq API の wire型（OpenAI互換） =====
// roleは小文字 ("system" | "user" | "assistant")。
// snake_case フィールド名 (max_tokens / prompt_tokens 等) はそのまま使われる。

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
    content: String,
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
