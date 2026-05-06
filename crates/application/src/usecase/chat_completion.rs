use std::collections::HashMap;
use std::sync::Arc;

use domain::model::completion::{ChatCompletionRequest, ChatCompletionResponse};
use domain::model::provider::ProviderId;
use domain::DomainError;

use crate::port::AiProvider;

// `Arc<dyn AiProvider>`: 「AiProvider trait を実装した何か」を動的ディスパッチで保持。
// Arcは複数スレッドから共有するためのスマートポインタ（参照カウント方式）。
pub struct ChatCompletionUseCase {
    providers: HashMap<ProviderId, Arc<dyn AiProvider>>,
}

impl ChatCompletionUseCase {
    pub fn new(providers: HashMap<ProviderId, Arc<dyn AiProvider>>) -> Self {
        Self { providers }
    }

    pub async fn execute(
        &self,
        req: ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, DomainError> {
        let provider = self
            .providers
            .get(&req.provider)
            .ok_or_else(|| DomainError::ProviderNotFound(req.provider.as_str().to_string()))?;
        provider.chat_completion(&req).await
    }

    pub fn available_providers(&self) -> Vec<ProviderId> {
        self.providers.keys().copied().collect()
    }
}
