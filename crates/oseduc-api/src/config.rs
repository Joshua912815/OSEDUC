use std::{env, net::SocketAddr};

use oseduc_llm::{LlmConfig, LlmProviderKind};
use oseduc_store::DatabaseConfig;
use serde::Serialize;

const DEFAULT_BIND_ADDR: &str = "127.0.0.1:3000";

#[derive(Clone, Debug)]
pub struct ApiConfig {
    pub bind_addr: SocketAddr,
    pub llm: LlmConfig,
    pub database: DatabaseConfig,
}

impl ApiConfig {
    pub fn from_env() -> Result<Self, ApiConfigError> {
        let bind_addr = env::var("OSEDUC_BIND_ADDR")
            .unwrap_or_else(|_| DEFAULT_BIND_ADDR.to_owned())
            .parse::<SocketAddr>()
            .map_err(|_| ApiConfigError::InvalidBindAddr)?;
        let llm = LlmConfig::from_env().map_err(ApiConfigError::Llm)?;
        let database = DatabaseConfig::from_env().map_err(ApiConfigError::Database)?;

        Ok(Self {
            bind_addr,
            llm,
            database,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PublicConfig {
    pub llm_provider: String,
    pub llm_model: String,
    pub live_llm_enabled: bool,
    pub llm_timeout_secs: u64,
    pub knowledge_store: String,
    pub admin_seed_enabled: bool,
}

impl PublicConfig {
    pub fn new(llm: &LlmConfig, database: &DatabaseConfig) -> Self {
        Self {
            llm_provider: llm.provider.as_str().to_owned(),
            llm_model: llm.model.clone(),
            live_llm_enabled: llm.provider != LlmProviderKind::Mock,
            llm_timeout_secs: llm.timeout.as_secs(),
            knowledge_store: "postgres".to_owned(),
            admin_seed_enabled: database.enable_admin_seed,
        }
    }
}

#[derive(Debug)]
pub enum ApiConfigError {
    Database(oseduc_store::DatabaseConfigError),
    InvalidBindAddr,
    Llm(oseduc_llm::ConfigError),
}

impl std::fmt::Display for ApiConfigError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Database(error) => write!(formatter, "{error}"),
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
    use oseduc_store::SecretDatabaseUrl;
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

        let database = DatabaseConfig {
            database_url: SecretDatabaseUrl::new(
                "postgres://oseduc:dev-password@127.0.0.1:5432/oseduc",
            ),
            auto_migrate: false,
            enable_admin_seed: true,
        };

        let public = PublicConfig::new(&config, &database);
        let json = serde_json::to_string(&public).expect("serialize public config");

        assert!(public.live_llm_enabled);
        assert_eq!(public.knowledge_store, "postgres");
        assert!(public.admin_seed_enabled);
        assert!(!json.contains("api_key"));
        assert!(!json.contains("token"));
        assert!(!json.contains("dev-password"));
    }
}
