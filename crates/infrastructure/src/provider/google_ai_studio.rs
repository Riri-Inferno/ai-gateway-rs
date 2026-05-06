//! Google AI Studio (Gemini) クライアント。
//! `models/{model}:generateContent` を叩いてChat補完を行う。
//! 別プロバイダ（Groq/OpenRouter等）を実装する際の **リファレンス** として使う。

use application::port::AiProvider;
use async_trait::async_trait;
use domain::model::completion::{
    ChatCompletionRequest, ChatCompletionResponse, Role, Usage,
};
use domain::model::provider::ProviderId;
use domain::DomainError;
use serde::{Deserialize, Serialize};

const ENDPOINT_BASE: &str = "https://generativelanguage.googleapis.com/v1beta/models";

pub struct GoogleAiStudioClient {
    api_key: String,
    http: reqwest::Client,
}

impl GoogleAiStudioClient {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            http: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl AiProvider for GoogleAiStudioClient {
    fn id(&self) -> ProviderId {
        ProviderId::GoogleAiStudio
    }

    async fn chat_completion(
        &self,
        req: &ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, DomainError> {
        // Gemini は roleが "user"/"model" の二択、systemは別フィールド `systemInstruction`
        let mut system_parts: Vec<GeminiPart> = Vec::new();
        let mut contents: Vec<GeminiContent> = Vec::new();

        for msg in &req.messages {
            match msg.role {
                Role::System => system_parts.push(GeminiPart {
                    text: msg.content.clone(),
                }),
                Role::User => contents.push(GeminiContent {
                    role: "user".into(),
                    parts: vec![GeminiPart {
                        text: msg.content.clone(),
                    }],
                }),
                Role::Assistant => contents.push(GeminiContent {
                    role: "model".into(),
                    parts: vec![GeminiPart {
                        text: msg.content.clone(),
                    }],
                }),
            }
        }

        let body = GeminiRequest {
            contents,
            system_instruction: (!system_parts.is_empty()).then_some(GeminiSystemInstruction {
                parts: system_parts,
            }),
            generation_config: Some(GeminiGenerationConfig {
                temperature: req.temperature,
                max_output_tokens: req.max_tokens,
            }),
        };

        let url = format!("{ENDPOINT_BASE}/{}:generateContent", req.model);
        let resp = self
            .http
            .post(&url)
            .query(&[("key", &self.api_key)])
            .json(&body)
            .send()
            .await
            .map_err(|e| DomainError::ProviderError(format!("request failed: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(DomainError::ProviderError(format!(
                "gemini status={status} body={text}"
            )));
        }

        let parsed: GeminiResponse = resp
            .json()
            .await
            .map_err(|e| DomainError::ProviderError(format!("decode failed: {e}")))?;

        let content = parsed
            .candidates
            .into_iter()
            .next()
            .and_then(|c| c.content.parts.into_iter().next())
            .map(|p| p.text)
            .ok_or_else(|| DomainError::ProviderError("empty response".into()))?;

        Ok(ChatCompletionResponse {
            provider: ProviderId::GoogleAiStudio,
            model: req.model.clone(),
            content,
            usage: parsed.usage_metadata.map(|u| Usage {
                prompt_tokens: u.prompt_token_count.unwrap_or(0),
                completion_tokens: u.candidates_token_count.unwrap_or(0),
                total_tokens: u.total_token_count.unwrap_or(0),
            }),
        })
    }
}

// ===== Gemini API のリクエスト/レスポンス型（このファイル内専用） =====
// 外に出さないので `pub` を付けない（モジュール内のみ可視）

#[derive(Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
    #[serde(rename = "systemInstruction", skip_serializing_if = "Option::is_none")]
    system_instruction: Option<GeminiSystemInstruction>,
    #[serde(rename = "generationConfig", skip_serializing_if = "Option::is_none")]
    generation_config: Option<GeminiGenerationConfig>,
}

#[derive(Serialize)]
struct GeminiContent {
    role: String,
    parts: Vec<GeminiPart>,
}

#[derive(Serialize)]
struct GeminiSystemInstruction {
    parts: Vec<GeminiPart>,
}

#[derive(Serialize, Deserialize)]
struct GeminiPart {
    text: String,
}

#[derive(Serialize)]
struct GeminiGenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(rename = "maxOutputTokens", skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<u32>,
}

#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Vec<GeminiCandidate>,
    #[serde(rename = "usageMetadata")]
    usage_metadata: Option<GeminiUsage>,
}

#[derive(Deserialize)]
struct GeminiCandidate {
    content: GeminiResponseContent,
}

#[derive(Deserialize)]
struct GeminiResponseContent {
    parts: Vec<GeminiPart>,
}

#[derive(Deserialize)]
struct GeminiUsage {
    #[serde(rename = "promptTokenCount")]
    prompt_token_count: Option<u32>,
    #[serde(rename = "candidatesTokenCount")]
    candidates_token_count: Option<u32>,
    #[serde(rename = "totalTokenCount")]
    total_token_count: Option<u32>,
}
