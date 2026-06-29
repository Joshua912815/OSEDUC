use serde::{Deserialize, Serialize};

use crate::TutorContextChunk;

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
    #[serde(default)]
    pub interaction_id: Option<i64>,
    pub answer: String,
    pub provider: String,
    #[serde(default)]
    pub citations: Vec<Citation>,
    #[serde(default)]
    pub safety_flags: Vec<SafetyFlag>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TutorChatPrompt {
    pub request: TutorChatRequest,
    #[serde(default)]
    pub context_chunks: Vec<TutorContextChunk>,
}

impl TutorChatPrompt {
    pub fn new(request: TutorChatRequest, context_chunks: Vec<TutorContextChunk>) -> Self {
        Self {
            request,
            context_chunks,
        }
    }

    pub fn citations(&self) -> Vec<Citation> {
        self.context_chunks
            .iter()
            .map(TutorContextChunk::citation)
            .collect()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Citation {
    pub label: String,
    pub source: String,
    #[serde(default)]
    pub node_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TutorInteraction {
    pub id: i64,
    #[serde(default)]
    pub student_id: Option<String>,
    #[serde(default)]
    pub knowledge_node_ids: Vec<String>,
    pub provider: String,
    #[serde(default)]
    pub citations: Vec<Citation>,
    #[serde(default)]
    pub safety_flags: Vec<SafetyFlag>,
    pub message_logged: bool,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub feedback: Option<TutorInteractionFeedback>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TutorInteractionFeedback {
    pub interaction_id: i64,
    #[serde(default)]
    pub helpful: Option<bool>,
    #[serde(default)]
    pub difficulty: Option<TutorFeedbackDifficulty>,
    #[serde(default)]
    pub feedback_text: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TutorFeedbackRequest {
    #[serde(default)]
    pub helpful: Option<bool>,
    #[serde(default)]
    pub difficulty: Option<TutorFeedbackDifficulty>,
    #[serde(default)]
    pub feedback_text: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TutorFeedbackDifficulty {
    TooEasy,
    JustRight,
    TooHard,
}

impl TutorFeedbackDifficulty {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::TooEasy => "too_easy",
            Self::JustRight => "just_right",
            Self::TooHard => "too_hard",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "too_easy" => Some(Self::TooEasy),
            "just_right" => Some(Self::JustRight),
            "too_hard" => Some(Self::TooHard),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SafetyFlag {
    MockResponse,
    MissingCitation,
    SourceGroundedContext,
    ProviderError,
    AcademicIntegrityBoundary,
}

impl SafetyFlag {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::MockResponse => "mock_response",
            Self::MissingCitation => "missing_citation",
            Self::SourceGroundedContext => "source_grounded_context",
            Self::ProviderError => "provider_error",
            Self::AcademicIntegrityBoundary => "academic_integrity_boundary",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "mock_response" => Some(Self::MockResponse),
            "missing_citation" => Some(Self::MissingCitation),
            "source_grounded_context" => Some(Self::SourceGroundedContext),
            "provider_error" => Some(Self::ProviderError),
            "academic_integrity_boundary" => Some(Self::AcademicIntegrityBoundary),
            _ => None,
        }
    }
}

impl TutorResponse {
    pub fn mock(answer: impl Into<String>) -> Self {
        Self {
            interaction_id: None,
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
        assert_eq!(response.interaction_id, None);
        assert_eq!(response.safety_flags, vec![SafetyFlag::MockResponse]);
    }

    #[test]
    fn prompt_citations_come_from_context_chunks() {
        let prompt = TutorChatPrompt::new(
            TutorChatRequest {
                message: "Explain address spaces".to_owned(),
                student_id: None,
                knowledge_node_ids: vec!["ch4-address-space".to_owned()],
            },
            vec![TutorContextChunk {
                node_id: "ch4-address-space".to_owned(),
                node_title: "Address Space".to_owned(),
                source_id: "rcore-v3-ch4".to_owned(),
                source_title: "rCore Chapter 4".to_owned(),
                source_url: "https://example.test/chapter4".to_owned(),
                license_note: "GPL-3.0; cite source".to_owned(),
                teaching_context: "Address-space context".to_owned(),
                citation_label: "rCore v3 ch4".to_owned(),
            }],
        );

        assert_eq!(prompt.citations().len(), 1);
        assert_eq!(prompt.citations()[0].label, "rCore v3 ch4");
    }

    #[test]
    fn storage_enums_round_trip_with_strings() {
        assert_eq!(
            SafetyFlag::parse(SafetyFlag::SourceGroundedContext.as_str()),
            Some(SafetyFlag::SourceGroundedContext)
        );
        assert_eq!(
            TutorFeedbackDifficulty::parse(TutorFeedbackDifficulty::JustRight.as_str()),
            Some(TutorFeedbackDifficulty::JustRight)
        );
    }
}
