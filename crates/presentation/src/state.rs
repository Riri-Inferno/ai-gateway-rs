use std::sync::Arc;

use application::usecase::chat_completion::ChatCompletionUseCase;

// axumのハンドラに `State<AppState>` で注入される共有状態。
// `Clone` 必須（axumがハンドラ毎にクローンするため）。Arcで包んでいるので実体コピーは発生しない。
#[derive(Clone)]
pub struct AppState {
    pub chat: Arc<ChatCompletionUseCase>,
    pub allowed_api_keys: Arc<Vec<String>>,
}
