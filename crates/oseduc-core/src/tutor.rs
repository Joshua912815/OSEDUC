use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TutorChatRequest {
    pub message: String,
    #[serde(default)]
    pub student_id: Option<String>,
    #[serde(default)]
    pub knowledge_node_ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TutorResponse {
    pub answer: String,
    pub provider: String,
    #[serde(default)]
    pub citations: Vec<Citation>,
    #[serde(default)]
    pub safety_flags: Vec<SafetyFlag>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Citation {
    pub label: String,
    pub source: String,
    #[serde(default)]
    pub node_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SafetyFlag {
    MockResponse,
    MissingCitation,
    ProviderError,
    AcademicIntegrityBoundary,
}

impl TutorResponse {
    pub fn mock(answer: impl Into<String>) -> Self {
        Self {
            answer: answer.into(),
            provider: "mock".to_owned(),
            citations: Vec::new(),
            safety_flags: vec![SafetyFlag::MockResponse],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_response_marks_mock_safety_flag() {
        let response = TutorResponse::mock("hello");

        assert_eq!(response.answer, "hello");
        assert_eq!(response.provider, "mock");
        assert_eq!(response.safety_flags, vec![SafetyFlag::MockResponse]);
    }
}
