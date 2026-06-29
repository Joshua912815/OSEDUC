use std::{env, fmt, time::Duration};

const DEFAULT_BASE_URL: &str = "https://api.openai.com/v1";
const DEFAULT_TIMEOUT_SECS: u64 = 30;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LlmProviderKind {
    Mock,
    OpenAiCompatible,
}

impl LlmProviderKind {
    fn parse(value: &str) -> Result<Self, ConfigError> {
        match value.trim().to_ascii_lowercase().as_str() {
            "" | "mock" => Ok(Self::Mock),
            "openai_compatible" | "openai-compatible" => Ok(Self::OpenAiCompatible),
            other => Err(ConfigError::UnknownProvider(other.to_owned())),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Mock => "mock",
            Self::OpenAiCompatible => "openai_compatible",
        }
    }
}

#[derive(Clone, Eq, PartialEq)]
pub struct SecretString(String);

impl SecretString {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn expose_secret(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("<redacted>")
    }
}

impl fmt::Display for SecretString {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("<redacted>")
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LlmConfig {
    pub provider: LlmProviderKind,
    pub base_url: String,
    pub model: String,
    pub api_key: Option<SecretString>,
    pub timeout: Duration,
}

impl LlmConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_getter(|key| env::var(key).ok())
    }

    pub fn from_getter(get: impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let provider = LlmProviderKind::parse(
            get("OSEDUC_LLM_PROVIDER")
                .as_deref()
                .unwrap_or(LlmProviderKind::Mock.as_str()),
        )?;
        let base_url = non_empty(get("OSEDUC_LLM_BASE_URL")).unwrap_or(DEFAULT_BASE_URL.to_owned());
        let model = non_empty(get("OSEDUC_LLM_MODEL")).unwrap_or_else(|| match provider {
            LlmProviderKind::Mock => "mock-model".to_owned(),
            LlmProviderKind::OpenAiCompatible => String::new(),
        });
        let api_key = non_empty(get("OSEDUC_LLM_API_KEY")).map(SecretString::new);
        let timeout = parse_timeout(get("OSEDUC_LLM_TIMEOUT_SECS"))?;

        if provider == LlmProviderKind::OpenAiCompatible && api_key.is_none() {
            return Err(ConfigError::MissingApiKey);
        }
        if provider == LlmProviderKind::OpenAiCompatible && model.is_empty() {
            return Err(ConfigError::MissingModel);
        }

        Ok(Self {
            provider,
            base_url,
            model,
            api_key,
            timeout,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConfigError {
    UnknownProvider(String),
    MissingApiKey,
    MissingModel,
    InvalidTimeout(String),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownProvider(provider) => {
                write!(formatter, "unknown LLM provider: {provider}")
            }
            Self::MissingApiKey => formatter.write_str(
                "OSEDUC_LLM_API_KEY is required when OSEDUC_LLM_PROVIDER=openai_compatible",
            ),
            Self::MissingModel => formatter.write_str(
                "OSEDUC_LLM_MODEL is required when OSEDUC_LLM_PROVIDER=openai_compatible",
            ),
            Self::InvalidTimeout(value) => {
                write!(
                    formatter,
                    "OSEDUC_LLM_TIMEOUT_SECS must be a positive integer, got {value}"
                )
            }
        }
    }
}

impl std::error::Error for ConfigError {}

fn non_empty(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn parse_timeout(value: Option<String>) -> Result<Duration, ConfigError> {
    let Some(value) = non_empty(value) else {
        return Ok(Duration::from_secs(DEFAULT_TIMEOUT_SECS));
    };
    let secs = value
        .parse::<u64>()
        .map_err(|_| ConfigError::InvalidTimeout(value.clone()))?;
    if secs == 0 {
        return Err(ConfigError::InvalidTimeout(value));
    }
    Ok(Duration::from_secs(secs))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn getter<'a>(entries: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        |key| {
            entries
                .iter()
                .find_map(|(entry_key, value)| (*entry_key == key).then(|| (*value).to_owned()))
        }
    }

    #[test]
    fn mock_provider_does_not_require_api_key() {
        let config = LlmConfig::from_getter(getter(&[])).expect("mock config should load");

        assert_eq!(config.provider, LlmProviderKind::Mock);
        assert_eq!(config.model, "mock-model");
        assert!(config.api_key.is_none());
    }

    #[test]
    fn openai_compatible_requires_api_key() {
        let error = LlmConfig::from_getter(getter(&[
            ("OSEDUC_LLM_PROVIDER", "openai_compatible"),
            ("OSEDUC_LLM_MODEL", "example-model"),
        ]))
        .expect_err("missing key should fail");

        assert_eq!(error, ConfigError::MissingApiKey);
    }

    #[test]
    fn openai_compatible_requires_model() {
        let error = LlmConfig::from_getter(getter(&[
            ("OSEDUC_LLM_PROVIDER", "openai_compatible"),
            ("OSEDUC_LLM_API_KEY", "secret-key-value"),
        ]))
        .expect_err("missing model should fail");

        assert_eq!(error, ConfigError::MissingModel);
    }

    #[test]
    fn secret_debug_and_display_are_redacted() {
        let secret = SecretString::new("secret-key-value");

        assert_eq!(format!("{secret:?}"), "<redacted>");
        assert_eq!(secret.to_string(), "<redacted>");
        assert!(!format!("{secret:?}").contains("secret-key-value"));
    }

    #[test]
    fn config_debug_does_not_expose_api_key() {
        let config = LlmConfig::from_getter(getter(&[
            ("OSEDUC_LLM_PROVIDER", "openai_compatible"),
            ("OSEDUC_LLM_MODEL", "example-model"),
            ("OSEDUC_LLM_API_KEY", "secret-key-value"),
        ]))
        .expect("config should load");

        let debug = format!("{config:?}");
        assert!(debug.contains("<redacted>"));
        assert!(!debug.contains("secret-key-value"));
    }

    #[test]
    fn rejects_invalid_timeout() {
        let error = LlmConfig::from_getter(getter(&[("OSEDUC_LLM_TIMEOUT_SECS", "0")]))
            .expect_err("zero timeout should fail");

        assert_eq!(error, ConfigError::InvalidTimeout("0".to_owned()));
    }
}
