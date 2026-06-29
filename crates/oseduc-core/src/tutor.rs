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
#[serde(rename_all = "snake_case")]
pub enum SafetyFlag {
    MockResponse,
    MissingCitation,
    SourceGroundedContext,
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
}
