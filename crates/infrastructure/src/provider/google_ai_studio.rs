use application::port::AiProvider;
use async_trait::async_trait;
use domain::model::completion::{ChatCompletionRequest, ChatCompletionResponse};
use domain::model::provider::ProviderId;
use domain::DomainError;

// 先頭の `_` は「未使用警告を抑える」プレフィックス。実装時に `_` を外す。
pub struct GoogleAiStudioClient {
    _api_key: String,
    _http: reqwest::Client,
}

impl GoogleAiStudioClient {
    pub fn new(api_key: String) -> Self {
        Self {
            _api_key: api_key,
            _http: reqwest::Client::new(),
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
        _req: &ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, DomainError> {
        Err(DomainError::InvalidRequest(
            "google_ai_studio: not implemented yet".to_string(),
        ))
    }
}
