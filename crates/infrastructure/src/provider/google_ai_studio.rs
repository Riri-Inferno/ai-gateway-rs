//! Google AI Studio (Gemini) クライアント。
//! `models/{model}:generateContent` を叩いてChat補完を行う。
//! 別プロバイダ（Groq/OpenRouter等）を実装する際の **リファレンス** として使う。

use application::port::AiProvider;
use async_trait::async_trait;
use domain::model::completion::{
    ChatCompletionRequest, ChatCompletionResponse, ContentPart, MessageContent, Role, Usage,
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
                Role::System => {
                    // system は画像不可。テキストのみ抽出
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
                        "google_ai_studio: tool role is not supported yet".into(),
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

        // レスポンスの parts のうち最初の text を取り出す
        let content = parsed
            .candidates
            .into_iter()
            .next()
            .and_then(|c| c.content.parts.into_iter().find_map(|p| p.text))
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

/// `data:image/jpeg;base64,XXXXX` 形式の data URL を (mime_type, base64データ) に分解
/// http(s) URL は Gemini が直接受け付けないので未サポートとしてエラー
fn parse_data_url(url: &str) -> Result<(String, String), DomainError> {
    let suffix = url.strip_prefix("data:").ok_or_else(|| {
        DomainError::InvalidRequest(
            "google_ai_studio: only data: URLs are supported for image input".into(),
        )
    })?;

    let (header, data) = suffix.split_once(',').ok_or_else(|| {
        DomainError::InvalidRequest("google_ai_studio: malformed data: URL".into())
    })?;

    if !header.ends_with(";base64") {
        return Err(DomainError::InvalidRequest(
            "google_ai_studio: only base64-encoded data: URLs are supported".into(),
        ));
    }

    let mime_type = header.trim_end_matches(";base64").to_string();
    Ok((mime_type, data.to_string()))
}

// ===== Gemini API のリクエスト/レスポンス型（このファイル内専用） =====

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

// Gemini の part は text または inlineData の片方を持つ。
// 単一struct + Option二つで両用、不要なフィールドは serde で skip する設計。
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
