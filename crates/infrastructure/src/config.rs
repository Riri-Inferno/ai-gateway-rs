use std::env;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub bind: String,
    pub gateway_api_keys: Vec<String>,
    pub google_ai_studio_api_key: Option<String>,
    pub groq_api_key: Option<String>,
    pub openrouter_api_key: Option<String>,
}

impl AppConfig {
    pub fn from_env() -> Self {
        Self {
            bind: env::var("GATEWAY_BIND").unwrap_or_else(|_| "0.0.0.0:8080".to_string()),
            gateway_api_keys: env::var("GATEWAY_API_KEYS")
                .ok()
                .map(|s| {
                    s.split(',')
                        .map(|k| k.trim().to_string())
                        .filter(|k| !k.is_empty())
                        .collect()
                })
                .unwrap_or_default(),
            google_ai_studio_api_key: env::var("GOOGLE_AI_STUDIO_API_KEY").ok(),
            groq_api_key: env::var("GROQ_API_KEY").ok(),
            openrouter_api_key: env::var("OPENROUTER_API_KEY").ok(),
        }
    }
}
