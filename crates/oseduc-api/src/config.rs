use std::{env, net::SocketAddr};

use oseduc_llm::{LlmConfig, LlmProviderKind};
use serde::Serialize;

const DEFAULT_BIND_ADDR: &str = "127.0.0.1:3000";

#[derive(Clone, Debug)]
pub struct ApiConfig {
    pub bind_addr: SocketAddr,
    pub llm: LlmConfig,
}

impl ApiConfig {
    pub fn from_env() -> Result<Self, ApiConfigError> {
        let bind_addr = env::var("OSEDUC_BIND_ADDR")
            .unwrap_or_else(|_| DEFAULT_BIND_ADDR.to_owned())
            .parse::<SocketAddr>()
            .map_err(|_| ApiConfigError::InvalidBindAddr)?;
        let llm = LlmConfig::from_env().map_err(ApiConfigError::Llm)?;

        Ok(Self { bind_addr, llm })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PublicConfig {
    pub llm_provider: String,
    pub llm_model: String,
    pub live_llm_enabled: bool,
    pub llm_timeout_secs: u64,
}

impl From<&LlmConfig> for PublicConfig {
    fn from(config: &LlmConfig) -> Self {
        Self {
            llm_provider: config.provider.as_str().to_owned(),
            llm_model: config.model.clone(),
            live_llm_enabled: config.provider != LlmProviderKind::Mock,
            llm_timeout_secs: config.timeout.as_secs(),
        }
    }
}

#[derive(Debug)]
pub enum ApiConfigError {
    InvalidBindAddr,
    Llm(oseduc_llm::ConfigError),
}

impl std::fmt::Display for ApiConfigError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidBindAddr => formatter.write_str("OSEDUC_BIND_ADDR is invalid"),
            Self::Llm(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for ApiConfigError {}

#[cfg(test)]
mod tests {
    use super::*;
    use oseduc_llm::{LlmProviderKind, SecretString};
    use std::time::Duration;

    #[test]
    fn public_config_does_not_include_api_key() {
        let config = LlmConfig {
            provider: LlmProviderKind::OpenAiCompatible,
            base_url: "https://example.test/v1".to_owned(),
            model: "example-model".to_owned(),
            api_key: Some(SecretString::new("token")),
            timeout: Duration::from_secs(42),
        };

        let public = PublicConfig::from(&config);
        let json = serde_json::to_string(&public).expect("serialize public config");

        assert!(public.live_llm_enabled);
        assert!(!json.contains("api_key"));
        assert!(!json.contains("token"));
    }
}
