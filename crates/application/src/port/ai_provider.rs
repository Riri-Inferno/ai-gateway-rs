use async_trait::async_trait;
use domain::model::completion::{ChatCompletionRequest, ChatCompletionResponse};
use domain::model::provider::ProviderId;
use domain::DomainError;

#[async_trait]
pub trait AiProvider: Send + Sync {
    fn id(&self) -> ProviderId;

    async fn chat_completion(
        &self,
        req: &ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, DomainError>;
}
