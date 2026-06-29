use serde::{Deserialize, Serialize};

use crate::KnowledgeNode;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StudentProfile {
    pub student_id: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub learning_goal: Option<String>,
    #[serde(default)]
    pub preferred_depth: LearningDepth,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct UpsertStudentProfileRequest {
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub learning_goal: Option<String>,
    #[serde(default)]
    pub preferred_depth: LearningDepth,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LearningDepth {
    Brief,
    #[default]
    Balanced,
    Deep,
}

impl LearningDepth {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Brief => "brief",
            Self::Balanced => "balanced",
            Self::Deep => "deep",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value {
            "brief" => Self::Brief,
            "deep" => Self::Deep,
            _ => Self::Balanced,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StudentNodeProgress {
    pub student_id: String,
    pub node_id: String,
    pub status: ProgressStatus,
    pub mastery_score: u8,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RecordProgressRequest {
    pub status: ProgressStatus,
    #[serde(default)]
    pub mastery_score: Option<u8>,
    #[serde(default)]
    pub notes: Option<String>,
}

impl RecordProgressRequest {
    pub fn validated_score(&self) -> Result<u8, ProgressValidationError> {
        let score = self
            .mastery_score
            .unwrap_or_else(|| self.status.default_score());
        if score > 100 {
            return Err(ProgressValidationError::InvalidMasteryScore(score));
        }
        Ok(score)
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProgressStatus {
    #[default]
    NotStarted,
    InProgress,
    NeedsReview,
    Mastered,
}

impl ProgressStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NotStarted => "not_started",
            Self::InProgress => "in_progress",
            Self::NeedsReview => "needs_review",
            Self::Mastered => "mastered",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value {
            "in_progress" => Self::InProgress,
            "needs_review" => Self::NeedsReview,
            "mastered" => Self::Mastered,
            _ => Self::NotStarted,
        }
    }

    pub fn default_score(&self) -> u8 {
        match self {
            Self::NotStarted => 0,
            Self::InProgress => 35,
            Self::NeedsReview => 55,
            Self::Mastered => 90,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProgressValidationError {
    InvalidMasteryScore(u8),
}

impl std::fmt::Display for ProgressValidationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidMasteryScore(score) => {
                write!(
                    formatter,
                    "mastery_score must be between 0 and 100, got {score}"
                )
            }
        }
    }
}

impl std::error::Error for ProgressValidationError {}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LearningPath {
    pub student_id: String,
    pub total_nodes: usize,
    pub completed_nodes: usize,
    #[serde(default)]
    pub recommendations: Vec<LearningPathRecommendation>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LearningPathRecommendation {
    pub node: KnowledgeNode,
    pub reason: String,
    pub priority: u16,
    #[serde(default)]
    pub blocked_by: Vec<String>,
    pub current_status: ProgressStatus,
    pub mastery_score: u8,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_request_defaults_score_from_status() {
        let request = RecordProgressRequest {
            status: ProgressStatus::InProgress,
            mastery_score: None,
            notes: None,
        };

        assert_eq!(request.validated_score(), Ok(35));
    }

    #[test]
    fn progress_request_rejects_invalid_score() {
        let request = RecordProgressRequest {
            status: ProgressStatus::Mastered,
            mastery_score: Some(101),
            notes: None,
        };

        assert_eq!(
            request.validated_score(),
            Err(ProgressValidationError::InvalidMasteryScore(101))
        );
    }
}
