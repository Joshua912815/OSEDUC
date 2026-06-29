use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SourceReference {
    pub id: String,
    pub title: String,
    pub url: String,
    pub source_kind: String,
    pub license_note: String,
    pub retrieved_at: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct KnowledgeNode {
    pub id: String,
    pub title: String,
    pub chapter: String,
    pub kind: String,
    pub summary: String,
    #[serde(default)]
    pub learning_objectives: Vec<String>,
    #[serde(default)]
    pub common_misconceptions: Vec<String>,
    pub source_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct KnowledgeEdge {
    pub from_node_id: String,
    pub to_node_id: String,
    pub relation: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RetrievalChunk {
    pub id: String,
    pub node_id: String,
    pub source_id: String,
    pub original_summary: String,
    pub teaching_context: String,
    pub citation_label: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct KnowledgeNodeDetail {
    pub node: KnowledgeNode,
    pub source: SourceReference,
    #[serde(default)]
    pub retrieval_chunks: Vec<RetrievalChunk>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct KnowledgeNeighbor {
    pub node: KnowledgeNode,
    pub relation: String,
    pub direction: KnowledgeEdgeDirection,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeEdgeDirection {
    Prerequisite,
    Dependent,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TutorContextChunk {
    pub node_id: String,
    pub node_title: String,
    pub source_id: String,
    pub source_title: String,
    pub source_url: String,
    pub license_note: String,
    pub teaching_context: String,
    pub citation_label: String,
}

impl TutorContextChunk {
    pub fn citation(&self) -> crate::Citation {
        crate::Citation {
            label: self.citation_label.clone(),
            source: self.source_url.clone(),
            node_id: Some(self.node_id.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tutor_context_chunk_builds_citation() {
        let chunk = TutorContextChunk {
            node_id: "ch4-address-space".to_owned(),
            node_title: "Address Space".to_owned(),
            source_id: "rcore-v3-ch4".to_owned(),
            source_title: "rCore Chapter 4".to_owned(),
            source_url: "https://example.test/chapter4".to_owned(),
            license_note: "GPL-3.0; cite source".to_owned(),
            teaching_context: "Address-space context".to_owned(),
            citation_label: "rCore v3 ch4".to_owned(),
        };

        let citation = chunk.citation();

        assert_eq!(citation.label, "rCore v3 ch4");
        assert_eq!(citation.source, "https://example.test/chapter4");
        assert_eq!(citation.node_id, Some("ch4-address-space".to_owned()));
    }
}
