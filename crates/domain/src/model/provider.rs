use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderId {
    GoogleAiStudio,
    Groq,
    OpenRouter,
}

impl ProviderId {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::GoogleAiStudio => "google_ai_studio",
            Self::Groq => "groq",
            Self::OpenRouter => "openrouter",
        }
    }
}
