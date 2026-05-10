use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// derive: 値型として複製可・HashMapのキーに使える・JSONと相互変換・OpenAPIスキーマ生成
// `rename_all = "snake_case"` で JSON上は "google_ai_studio" 等になる
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProviderId {
    GoogleAiStudio,
    Groq,
    #[serde(rename = "openrouter")]
    OpenRouter,
    Vertex,
}

impl ProviderId {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::GoogleAiStudio => "google_ai_studio",
            Self::Groq => "groq",
            Self::OpenRouter => "openrouter",
            Self::Vertex => "vertex",
        }
    }
}
