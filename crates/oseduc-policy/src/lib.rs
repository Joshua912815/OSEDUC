#![forbid(unsafe_code)]

use std::collections::{HashMap, HashSet};

use oseduc_core::{
    KnowledgeEdge, KnowledgeNode, LearningPath, LearningPathRecommendation, ProgressStatus,
    StudentNodeProgress,
};

const DEFAULT_LIMIT: usize = 5;
const MASTERED_THRESHOLD: u8 = 80;

pub fn recommend_learning_path(
    student_id: impl Into<String>,
    nodes: &[KnowledgeNode],
    edges: &[KnowledgeEdge],
    progress: &[StudentNodeProgress],
    limit: Option<usize>,
) -> LearningPath {
    let student_id = student_id.into();
    let progress_by_node = progress
        .iter()
        .map(|entry| (entry.node_id.as_str(), entry))
        .collect::<HashMap<_, _>>();
    let prerequisites = prerequisite_map(edges);
    let completed_nodes = nodes
        .iter()
        .filter(|node| is_mastered(&progress_by_node, &node.id))
        .count();
    let max_items = limit.unwrap_or(DEFAULT_LIMIT).max(1);

    let mut recommendations = nodes
        .iter()
        .filter(|node| !is_mastered(&progress_by_node, &node.id))
        .filter_map(|node| {
            let blocked_by = prerequisites
                .get(node.id.as_str())
                .into_iter()
                .flatten()
                .filter(|prerequisite| !is_mastered(&progress_by_node, prerequisite))
                .cloned()
                .collect::<Vec<_>>();
            if !blocked_by.is_empty() {
                return None;
            }

            let progress = progress_by_node.get(node.id.as_str()).copied();
            let current_status = progress
                .map(|entry| entry.status.clone())
                .unwrap_or(ProgressStatus::NotStarted);
            let mastery_score = progress.map(|entry| entry.mastery_score).unwrap_or(0);
            let priority = recommendation_priority(&current_status, mastery_score);

            Some(LearningPathRecommendation {
                node: node.clone(),
                reason: recommendation_reason(&current_status, mastery_score),
                priority,
                blocked_by,
                current_status,
                mastery_score,
            })
        })
        .collect::<Vec<_>>();

    recommendations.sort_by(|left, right| {
        left.priority
            .cmp(&right.priority)
            .then_with(|| left.node.chapter.cmp(&right.node.chapter))
            .then_with(|| left.node.id.cmp(&right.node.id))
    });
    recommendations.truncate(max_items);

    LearningPath {
        student_id,
        total_nodes: nodes.len(),
        completed_nodes,
        recommendations,
    }
}

fn prerequisite_map(edges: &[KnowledgeEdge]) -> HashMap<&str, Vec<String>> {
    let mut prerequisites: HashMap<&str, Vec<String>> = HashMap::new();
    for edge in edges {
        if edge.relation == "prerequisite_for" {
            prerequisites
                .entry(edge.to_node_id.as_str())
                .or_default()
                .push(edge.from_node_id.clone());
        }
    }
    prerequisites
}

fn is_mastered(progress_by_node: &HashMap<&str, &StudentNodeProgress>, node_id: &str) -> bool {
    progress_by_node.get(node_id).is_some_and(|entry| {
        entry.status == ProgressStatus::Mastered || entry.mastery_score >= MASTERED_THRESHOLD
    })
}

fn recommendation_priority(status: &ProgressStatus, mastery_score: u8) -> u16 {
    match status {
        ProgressStatus::NeedsReview => 10,
        ProgressStatus::InProgress => 20,
        ProgressStatus::NotStarted => 40,
        ProgressStatus::Mastered => {
            if mastery_score < MASTERED_THRESHOLD {
                30
            } else {
                100
            }
        }
    }
}

fn recommendation_reason(status: &ProgressStatus, mastery_score: u8) -> String {
    match status {
        ProgressStatus::NeedsReview => {
            "This node was marked as needing review and its prerequisites are satisfied.".to_owned()
        }
        ProgressStatus::InProgress => {
            "Continue this in-progress node before moving further along the OS mainline.".to_owned()
        }
        ProgressStatus::Mastered if mastery_score < MASTERED_THRESHOLD => {
            "The status is mastered, but the mastery score is below the review threshold."
                .to_owned()
        }
        _ => "This is the next available node after the mastered prerequisites.".to_owned(),
    }
}

pub fn blocked_nodes(
    nodes: &[KnowledgeNode],
    edges: &[KnowledgeEdge],
    progress: &[StudentNodeProgress],
) -> HashSet<String> {
    let progress_by_node = progress
        .iter()
        .map(|entry| (entry.node_id.as_str(), entry))
        .collect::<HashMap<_, _>>();
    let prerequisites = prerequisite_map(edges);

    nodes
        .iter()
        .filter(|node| {
            prerequisites
                .get(node.id.as_str())
                .into_iter()
                .flatten()
                .any(|prerequisite| !is_mastered(&progress_by_node, prerequisite))
        })
        .map(|node| node.id.clone())
        .collect()
}

pub fn crate_name() -> &'static str {
    "oseduc-policy"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recommends_first_unmastered_available_node() {
        let path = recommend_learning_path(
            "student-1",
            &sample_nodes(),
            &sample_edges(),
            &[progress("ch1", ProgressStatus::Mastered, 95)],
            Some(3),
        );

        assert_eq!(path.completed_nodes, 1);
        assert_eq!(path.recommendations[0].node.id, "ch2");
        assert_eq!(path.recommendations[0].blocked_by, Vec::<String>::new());
    }

    #[test]
    fn does_not_recommend_nodes_with_unmastered_prerequisites() {
        let path =
            recommend_learning_path("student-1", &sample_nodes(), &sample_edges(), &[], Some(5));

        assert_eq!(path.recommendations.len(), 1);
        assert_eq!(path.recommendations[0].node.id, "ch1");
        assert!(blocked_nodes(&sample_nodes(), &sample_edges(), &[]).contains("ch2"));
    }

    #[test]
    fn prioritizes_review_over_new_nodes() {
        let progress = vec![
            progress("ch1", ProgressStatus::Mastered, 95),
            progress("ch2", ProgressStatus::NeedsReview, 55),
        ];
        let path = recommend_learning_path(
            "student-1",
            &sample_nodes(),
            &sample_edges(),
            &progress,
            None,
        );

        assert_eq!(path.recommendations[0].node.id, "ch2");
        assert_eq!(path.recommendations[0].priority, 10);
    }

    fn sample_nodes() -> Vec<KnowledgeNode> {
        ["ch1", "ch2", "ch3"]
            .into_iter()
            .map(|id| KnowledgeNode {
                id: id.to_owned(),
                title: id.to_owned(),
                chapter: id.to_owned(),
                kind: "rust_os_mainline".to_owned(),
                summary: format!("{id} summary"),
                learning_objectives: Vec::new(),
                common_misconceptions: Vec::new(),
                source_id: format!("source-{id}"),
            })
            .collect()
    }

    fn sample_edges() -> Vec<KnowledgeEdge> {
        vec![
            KnowledgeEdge {
                from_node_id: "ch1".to_owned(),
                to_node_id: "ch2".to_owned(),
                relation: "prerequisite_for".to_owned(),
            },
            KnowledgeEdge {
                from_node_id: "ch2".to_owned(),
                to_node_id: "ch3".to_owned(),
                relation: "prerequisite_for".to_owned(),
            },
        ]
    }

    fn progress(node_id: &str, status: ProgressStatus, mastery_score: u8) -> StudentNodeProgress {
        StudentNodeProgress {
            student_id: "student-1".to_owned(),
            node_id: node_id.to_owned(),
            status,
            mastery_score,
            notes: None,
            updated_at: None,
        }
    }
}
