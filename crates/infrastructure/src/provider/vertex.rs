//! Vertex AI (GCP) クライアント。
//! Gemini系モデルを `publishers/google/models/{model}:generateContent` で叩く。
//!
//! 認証は ADC (Application Default Credentials) 一本化:
//!   - ローカル: `gcloud auth application-default login`
//!   - k3s    : WIF JSON マウント + `GOOGLE_APPLICATION_CREDENTIALS` でパス指定
//!
//! ワイヤフォーマット自体は Google AI Studio (Gemini) と同形だが、
//! エンドポイントとBearerトークン取得が異なる。

use std::sync::Arc;

use application::port::AiProvider;
use async_trait::async_trait;
use domain::model::completion::{
    ChatCompletionRequest, ChatCompletionResponse, ContentPart, MessageContent, Role, Usage,
};
use domain::model::provider::ProviderId;
use domain::DomainError;
use gcp_auth::TokenProvider;
use serde::{Deserialize, Serialize};

const SCOPE: &str = "https://www.googleapis.com/auth/cloud-platform";

pub struct VertexClient {
    // `Arc<dyn TokenProvider>`: gcp_auth が返す「ADCのトークン発行者」。
    // 内部でトークンキャッシュを持つので、毎回呼んでも実HTTPは期限切れ時のみ。
    token_provider: Arc<dyn TokenProvider>,
    project_id: String,
    location: String,
    http: reqwest::Client,
}

impl VertexClient {
    pub fn new(
        token_provider: Arc<dyn TokenProvider>,
        project_id: String,
        location: String,
    ) -> Self {
        Self {
            token_provider,
            project_id,
            location,
            http: reqwest::Client::new(),
        }
    }

    fn endpoint(&self, model: &str) -> String {
        format!(
            "https://{loc}-aiplatform.googleapis.com/v1/projects/{proj}/locations/{loc}/publishers/google/models/{model}:generateContent",
            loc = self.location,
            proj = self.project_id,
            model = model,
        )
    }

    async fn fetch_access_token(&self) -> Result<String, DomainError> {
        let token = self
            .token_provider
            .token(&[SCOPE])
            .await
            .map_err(|e| DomainError::ProviderError(format!("vertex adc token failed: {e}")))?;
        Ok(token.as_str().to_string())
    }
}

#[async_trait]
impl AiProvider for VertexClient {
    fn id(&self) -> ProviderId {
        ProviderId::Vertex
    }

    async fn chat_completion(
        &self,
        req: &ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, DomainError> {
        // Gemini は role が "user"/"model" のみ。system は別フィールド `systemInstruction`
        let mut system_parts: Vec<GeminiPart> = Vec::new();
        let mut contents: Vec<GeminiContent> = Vec::new();

        for msg in &req.messages {
            match msg.role {
                Role::System => {
                    system_parts.push(GeminiPart::text(msg.content.extract_text()));
                }
                Role::User => contents.push(GeminiContent {
                    role: "user".into(),
                    parts: domain_content_to_gemini_parts(&msg.content)?,
                }),
                Role::Assistant => contents.push(GeminiContent {
                    role: "model".into(),
                    parts: domain_content_to_gemini_parts(&msg.content)?,
                }),
                Role::Tool => {
                    return Err(DomainError::InvalidRequest(
                        "vertex: tool role is not supported yet".into(),
                    ));
                }
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

        let access_token = self.fetch_access_token().await?;
        let url = self.endpoint(&req.model);

        let resp = self
            .http
            .post(&url)
            .bearer_auth(&access_token)
            .json(&body)
            .send()
            .await
            .map_err(|e| DomainError::ProviderError(format!("request failed: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(DomainError::ProviderError(format!(
                "vertex status={status} body={text}"
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
            .and_then(|c| c.content.parts.into_iter().find_map(|p| p.text))
            .ok_or_else(|| DomainError::ProviderError("empty response".into()))?;

        Ok(ChatCompletionResponse {
            provider: ProviderId::Vertex,
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

/// domain MessageContent → Gemini の parts 配列に変換
fn domain_content_to_gemini_parts(c: &MessageContent) -> Result<Vec<GeminiPart>, DomainError> {
    match c {
        MessageContent::Text(s) => Ok(vec![GeminiPart::text(s.clone())]),
        MessageContent::Parts(parts) => parts
            .iter()
            .map(|p| match p {
                ContentPart::Text { text } => Ok(GeminiPart::text(text.clone())),
                ContentPart::ImageUrl { image_url } => {
                    let (mime_type, data) = parse_data_url(&image_url.url)?;
                    Ok(GeminiPart::inline_data(mime_type, data))
                }
            })
            .collect(),
    }
}

/// `data:image/jpeg;base64,XXXXX` 形式の data URL を (mime_type, base64データ) に分解。
/// http(s) URL は Vertex (Gemini) が直接受け付けないので未サポートとしてエラー。
fn parse_data_url(url: &str) -> Result<(String, String), DomainError> {
    let suffix = url.strip_prefix("data:").ok_or_else(|| {
        DomainError::InvalidRequest("vertex: only data: URLs are supported for image input".into())
    })?;

    let (header, data) = suffix
        .split_once(',')
        .ok_or_else(|| DomainError::InvalidRequest("vertex: malformed data: URL".into()))?;

    if !header.ends_with(";base64") {
        return Err(DomainError::InvalidRequest(
            "vertex: only base64-encoded data: URLs are supported".into(),
        ));
    }

    let mime_type = header.trim_end_matches(";base64").to_string();
    Ok((mime_type, data.to_string()))
}

// ===== Gemini API のリクエスト/レスポンス型（このファイル内専用） =====
// 形は Google AI Studio と同じ。違いはホストと認証ヘッダのみ。

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

// part は text または inlineData の片方を持つ。
// 単一struct + Option二つで両用、不要なフィールドは serde で skip する。
#[derive(Serialize, Deserialize, Default)]
struct GeminiPart {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(
        rename = "inlineData",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    inline_data: Option<GeminiInlineData>,
}

impl GeminiPart {
    fn text(s: String) -> Self {
        Self {
            text: Some(s),
            inline_data: None,
        }
    }

    fn inline_data(mime_type: String, data: String) -> Self {
        Self {
            text: None,
            inline_data: Some(GeminiInlineData { mime_type, data }),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct GeminiInlineData {
    #[serde(rename = "mimeType")]
    mime_type: String,
    data: String,
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
