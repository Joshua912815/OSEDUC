use std::{fmt, sync::Arc};

use async_trait::async_trait;
use oseduc_core::{SafetyFlag, TutorChatPrompt, TutorChatRequest, TutorResponse};

use crate::{LlmConfig, LlmProviderKind, SecretString};

#[async_trait]
pub trait LlmProvider: Send + Sync {
    fn name(&self) -> &'static str;

    async fn chat(&self, prompt: TutorChatPrompt) -> Result<TutorResponse, LlmError>;
}

#[derive(Clone)]
pub struct LlmGateway {
    provider: Arc<dyn LlmProvider>,
}

impl LlmGateway {
    pub fn new(provider: Arc<dyn LlmProvider>) -> Self {
        Self { provider }
    }

    pub fn mock() -> Self {
        Self::new(Arc::new(MockLlmProvider::new()))
    }

    pub fn from_config(config: LlmConfig) -> Result<Self, LlmError> {
        match config.provider {
            LlmProviderKind::Mock => Ok(Self::mock()),
            LlmProviderKind::OpenAiCompatible => {
                Ok(Self::new(Arc::new(OpenAiCompatibleProvider::new(config)?)))
            }
        }
    }

    pub fn provider_name(&self) -> &'static str {
        self.provider.name()
    }

    pub async fn chat(&self, request: TutorChatRequest) -> Result<TutorResponse, LlmError> {
        self.chat_with_context(request, Vec::new()).await
    }

    pub async fn chat_with_context(
        &self,
        request: TutorChatRequest,
        context_chunks: Vec<oseduc_core::TutorContextChunk>,
    ) -> Result<TutorResponse, LlmError> {
        self.provider
            .chat(TutorChatPrompt::new(request, context_chunks))
            .await
    }
}

#[derive(Clone, Debug, Default)]
pub struct MockLlmProvider;

impl MockLlmProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl LlmProvider for MockLlmProvider {
    fn name(&self) -> &'static str {
        "mock"
    }

    async fn chat(&self, prompt: TutorChatPrompt) -> Result<TutorResponse, LlmError> {
        let mut response = TutorResponse::mock(format!(
            "Mock tutor response for: {}",
            prompt.request.message.trim()
        ));
        response.provider = self.name().to_owned();
        response.citations = prompt.citations();
        if !response.citations.is_empty() {
            response
                .safety_flags
                .push(SafetyFlag::SourceGroundedContext);
        }
        Ok(response)
    }
}

#[derive(Clone, Debug)]
pub struct OpenAiCompatibleProvider {
    config: LlmConfig,
    client: reqwest::Client,
}

impl OpenAiCompatibleProvider {
    pub fn new(config: LlmConfig) -> Result<Self, LlmError> {
        if config.provider != LlmProviderKind::OpenAiCompatible {
            return Err(LlmError::InvalidProviderConfig);
        }
        if config.api_key.is_none() {
            return Err(LlmError::MissingApiKey);
        }
        Ok(Self {
            client: reqwest::Client::builder()
                .timeout(config.timeout)
                .build()
                .map_err(|error| LlmError::HttpClient(error.to_string()))?,
            config,
        })
    }

    fn endpoint(&self) -> String {
        format!(
            "{}/chat/completions",
            self.config.base_url.trim_end_matches('/')
        )
    }

    fn request_body(&self, prompt: &TutorChatPrompt) -> serde_json::Value {
        serde_json::json!({
            "model": self.config.model,
            "messages": [
                {
                    "role": "system",
                    "content": SYSTEM_PROMPT
                },
                {
                    "role": "system",
                    "content": format_reference_context(prompt)
                },
                {
                    "role": "user",
                    "content": prompt.request.message
                }
            ],
            "temperature": 0.2
        })
    }
}

#[async_trait]
impl LlmProvider for OpenAiCompatibleProvider {
    fn name(&self) -> &'static str {
        "openai_compatible"
    }

    async fn chat(&self, prompt: TutorChatPrompt) -> Result<TutorResponse, LlmError> {
        let api_key = self
            .config
            .api_key
            .as_ref()
            .ok_or(LlmError::MissingApiKey)?;
        let response = self
            .client
            .post(self.endpoint())
            .header(
                reqwest::header::AUTHORIZATION,
                authorization_header(api_key)?,
            )
            .json(&self.request_body(&prompt))
            .send()
            .await
            .map_err(|error| LlmError::ProviderRequest(error.to_string()))?;

        if !response.status().is_success() {
            return Err(LlmError::ProviderStatus(response.status().as_u16()));
        }

        let payload: OpenAiChatResponse = response
            .json()
            .await
            .map_err(|error| LlmError::ProviderResponse(error.to_string()))?;
        let answer = payload
            .choices
            .first()
            .map(|choice| choice.message.content.trim().to_owned())
            .filter(|content| !content.is_empty())
            .ok_or(LlmError::EmptyResponse)?;

        let citations = prompt.citations();
        let safety_flags = if citations.is_empty() {
            vec![SafetyFlag::MissingCitation]
        } else {
            vec![SafetyFlag::SourceGroundedContext]
        };

        Ok(TutorResponse {
            answer,
            provider: self.name().to_owned(),
            citations,
            safety_flags,
        })
    }
}

const SYSTEM_PROMPT: &str = "You are OSeduc's controlled operating-system teaching tutor. Use the provided rCore reference context as the source of truth for knowledge explanations. Explain in your own words, cite the provided citation labels, and say when the context is insufficient. Do not provide complete assignment solutions or present source-derived material as OSeduc-original text.";

fn format_reference_context(prompt: &TutorChatPrompt) -> String {
    if prompt.context_chunks.is_empty() {
        return "No source-grounded reference context was provided for this request.".to_owned();
    }

    prompt
        .context_chunks
        .iter()
        .map(|chunk| {
            format!(
                "[{}]\nNode: {} ({})\nSource: {} <{}>\nLicense/provenance: {}\nTeaching context:\n{}",
                chunk.citation_label,
                chunk.node_title,
                chunk.node_id,
                chunk.source_title,
                chunk.source_url,
                chunk.license_note,
                chunk.teaching_context
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

#[derive(serde::Deserialize)]
struct OpenAiChatResponse {
    choices: Vec<OpenAiChoice>,
}

#[derive(serde::Deserialize)]
struct OpenAiChoice {
    message: OpenAiMessage,
}

#[derive(serde::Deserialize)]
struct OpenAiMessage {
    content: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LlmError {
    InvalidProviderConfig,
    MissingApiKey,
    InvalidAuthorizationHeader,
    HttpClient(String),
    ProviderRequest(String),
    ProviderStatus(u16),
    ProviderResponse(String),
    EmptyResponse,
}

impl fmt::Display for LlmError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidProviderConfig => {
                formatter.write_str("LLM config does not match the requested provider")
            }
            Self::MissingApiKey => formatter.write_str("LLM provider API key is missing"),
            Self::InvalidAuthorizationHeader => {
                formatter.write_str("LLM provider authorization header is invalid")
            }
            Self::HttpClient(message) => write!(formatter, "LLM HTTP client error: {message}"),
            Self::ProviderRequest(message) => {
                write!(formatter, "LLM provider request failed: {message}")
            }
            Self::ProviderStatus(status) => {
                write!(formatter, "LLM provider returned HTTP status {status}")
            }
            Self::ProviderResponse(message) => {
                write!(formatter, "LLM provider response was invalid: {message}")
            }
            Self::EmptyResponse => formatter.write_str("LLM provider returned an empty response"),
        }
    }
}

impl std::error::Error for LlmError {}

fn authorization_header(api_key: &SecretString) -> Result<reqwest::header::HeaderValue, LlmError> {
    reqwest::header::HeaderValue::from_str(&format!("Bearer {}", api_key.expose_secret()))
        .map_err(|_| LlmError::InvalidAuthorizationHeader)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LlmProviderKind;

    #[tokio::test]
    async fn mock_provider_returns_stable_response() {
        let provider = MockLlmProvider::new();
        let response = provider
            .chat(TutorChatPrompt::new(
                TutorChatRequest {
                    message: "explain traps".to_owned(),
                    student_id: None,
                    knowledge_node_ids: Vec::new(),
                },
                Vec::new(),
            ))
            .await
            .expect("mock provider should respond");

        assert_eq!(response.provider, "mock");
        assert!(response.answer.contains("explain traps"));
        assert_eq!(response.safety_flags, vec![SafetyFlag::MockResponse]);
    }

    #[tokio::test]
    async fn mock_provider_returns_context_citations() {
        let provider = MockLlmProvider::new();
        let response = provider
            .chat(TutorChatPrompt::new(
                TutorChatRequest {
                    message: "explain address spaces".to_owned(),
                    student_id: None,
                    knowledge_node_ids: vec!["ch4-address-space".to_owned()],
                },
                vec![sample_context_chunk()],
            ))
            .await
            .expect("mock provider should respond");

        assert_eq!(response.citations.len(), 1);
        assert_eq!(response.citations[0].label, "rCore v3 ch4");
        assert!(response
            .safety_flags
            .contains(&SafetyFlag::SourceGroundedContext));
    }

    #[test]
    fn openai_provider_rejects_mock_config() {
        let config = LlmConfig::from_getter(|_| None).expect("mock config should load");

        let error = OpenAiCompatibleProvider::new(config).expect_err("mock config should fail");

        assert_eq!(error, LlmError::InvalidProviderConfig);
    }

    #[test]
    fn authorization_header_uses_bearer_token() {
        let header = authorization_header(&SecretString::new("token")).expect("valid header");

        assert_eq!(
            header.to_str().expect("header should be visible"),
            "Bearer token"
        );
    }

    #[test]
    fn errors_do_not_include_api_key() {
        let error = LlmError::ProviderRequest("connection refused".to_owned());

        assert!(!error.to_string().contains("token"));
    }

    #[test]
    fn openai_request_body_uses_model_and_user_message() {
        let config = LlmConfig {
            provider: LlmProviderKind::OpenAiCompatible,
            base_url: "https://example.test/v1".to_owned(),
            model: "example-model".to_owned(),
            api_key: Some(SecretString::new("token")),
            timeout: std::time::Duration::from_secs(5),
        };
        let provider = OpenAiCompatibleProvider::new(config).expect("config should be valid");
        let body = provider.request_body(&TutorChatPrompt::new(
            TutorChatRequest {
                message: "what is fork?".to_owned(),
                student_id: None,
                knowledge_node_ids: Vec::new(),
            },
            vec![sample_context_chunk()],
        ));

        assert_eq!(body["model"], "example-model");
        assert!(body["messages"][0]["content"]
            .as_str()
            .expect("system prompt should be a string")
            .contains("rCore reference context"));
        assert!(body["messages"][1]["content"]
            .as_str()
            .expect("context prompt should be a string")
            .contains("rCore v3 ch4"));
        assert_eq!(body["messages"][2]["content"], "what is fork?");
    }

    fn sample_context_chunk() -> oseduc_core::TutorContextChunk {
        oseduc_core::TutorContextChunk {
            node_id: "ch4-address-space".to_owned(),
            node_title: "Address Space".to_owned(),
            source_id: "rcore-v3-ch4".to_owned(),
            source_title: "rCore Chapter 4".to_owned(),
            source_url: "https://rcore-os.cn/rCore-Tutorial-Book-v3/chapter4/index.html".to_owned(),
            license_note: "GPL-3.0; cite rCore".to_owned(),
            teaching_context: "Address-space teaching context".to_owned(),
            citation_label: "rCore v3 ch4".to_owned(),
        }
    }
}
