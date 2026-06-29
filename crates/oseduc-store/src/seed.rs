use std::collections::HashSet;

use oseduc_core::{KnowledgeEdge, KnowledgeNode, RetrievalChunk, SourceReference};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct KnowledgeSeed {
    #[serde(default)]
    pub sources: Vec<SourceReference>,
    #[serde(default)]
    pub nodes: Vec<KnowledgeNode>,
    #[serde(default)]
    pub edges: Vec<KnowledgeEdge>,
    #[serde(default)]
    pub retrieval_chunks: Vec<RetrievalChunk>,
}

impl KnowledgeSeed {
    pub fn from_json_str(value: &str) -> Result<Self, KnowledgeSeedError> {
        let seed: Self = serde_json::from_str(value).map_err(|error| KnowledgeSeedError::Json {
            message: error.to_string(),
        })?;
        seed.validate()?;
        Ok(seed)
    }

    pub fn validate(&self) -> Result<(), KnowledgeSeedError> {
        if self.sources.is_empty() {
            return Err(KnowledgeSeedError::Empty("sources"));
        }
        if self.nodes.is_empty() {
            return Err(KnowledgeSeedError::Empty("nodes"));
        }
        if self.retrieval_chunks.is_empty() {
            return Err(KnowledgeSeedError::Empty("retrieval_chunks"));
        }

        let mut source_ids = HashSet::new();
        for source in &self.sources {
            require_non_empty("source.id", &source.id)?;
            require_non_empty("source.title", &source.title)?;
            require_non_empty("source.url", &source.url)?;
            require_non_empty("source.license_note", &source.license_note)?;
            if !source_ids.insert(source.id.as_str()) {
                return Err(KnowledgeSeedError::DuplicateId(source.id.clone()));
            }
        }

        let mut node_ids = HashSet::new();
        for node in &self.nodes {
            require_non_empty("node.id", &node.id)?;
            require_non_empty("node.title", &node.title)?;
            require_non_empty("node.summary", &node.summary)?;
            if !source_ids.contains(node.source_id.as_str()) {
                return Err(KnowledgeSeedError::MissingSource {
                    owner_id: node.id.clone(),
                    source_id: node.source_id.clone(),
                });
            }
            if !node_ids.insert(node.id.as_str()) {
                return Err(KnowledgeSeedError::DuplicateId(node.id.clone()));
            }
        }

        for edge in &self.edges {
            if !node_ids.contains(edge.from_node_id.as_str()) {
                return Err(KnowledgeSeedError::MissingNode(edge.from_node_id.clone()));
            }
            if !node_ids.contains(edge.to_node_id.as_str()) {
                return Err(KnowledgeSeedError::MissingNode(edge.to_node_id.clone()));
            }
            require_non_empty("edge.relation", &edge.relation)?;
        }

        let mut chunk_ids = HashSet::new();
        for chunk in &self.retrieval_chunks {
            require_non_empty("retrieval_chunk.id", &chunk.id)?;
            require_non_empty("retrieval_chunk.original_summary", &chunk.original_summary)?;
            require_non_empty("retrieval_chunk.teaching_context", &chunk.teaching_context)?;
            require_non_empty("retrieval_chunk.citation_label", &chunk.citation_label)?;
            if !node_ids.contains(chunk.node_id.as_str()) {
                return Err(KnowledgeSeedError::MissingNode(chunk.node_id.clone()));
            }
            if !source_ids.contains(chunk.source_id.as_str()) {
                return Err(KnowledgeSeedError::MissingSource {
                    owner_id: chunk.id.clone(),
                    source_id: chunk.source_id.clone(),
                });
            }
            if !chunk_ids.insert(chunk.id.as_str()) {
                return Err(KnowledgeSeedError::DuplicateId(chunk.id.clone()));
            }
        }

        Ok(())
    }

    pub fn summary(&self) -> KnowledgeSeedSummary {
        KnowledgeSeedSummary {
            sources: self.sources.len(),
            nodes: self.nodes.len(),
            edges: self.edges.len(),
            retrieval_chunks: self.retrieval_chunks.len(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct KnowledgeSeedSummary {
    pub sources: usize,
    pub nodes: usize,
    pub edges: usize,
    pub retrieval_chunks: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum KnowledgeSeedError {
    Json { message: String },
    Empty(&'static str),
    MissingField(&'static str),
    DuplicateId(String),
    MissingSource { owner_id: String, source_id: String },
    MissingNode(String),
}

impl std::fmt::Display for KnowledgeSeedError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Json { .. } => formatter.write_str("knowledge seed JSON is invalid"),
            Self::Empty(field) => write!(formatter, "knowledge seed must include {field}"),
            Self::MissingField(field) => {
                write!(formatter, "knowledge seed field is empty: {field}")
            }
            Self::DuplicateId(id) => write!(formatter, "knowledge seed id is duplicated: {id}"),
            Self::MissingSource {
                owner_id,
                source_id,
            } => write!(
                formatter,
                "knowledge seed item {owner_id} references missing source {source_id}"
            ),
            Self::MissingNode(id) => {
                write!(formatter, "knowledge seed references missing node {id}")
            }
        }
    }
}

impl std::error::Error for KnowledgeSeedError {}

fn require_non_empty(field: &'static str, value: &str) -> Result<(), KnowledgeSeedError> {
    if value.trim().is_empty() {
        return Err(KnowledgeSeedError::MissingField(field));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const RCORE_SEED: &str = include_str!("../../../data/knowledge/rcore-v3-rust-seed.json");

    #[test]
    fn rcore_seed_is_valid() {
        let seed = KnowledgeSeed::from_json_str(RCORE_SEED).expect("seed should parse");

        assert_eq!(seed.summary().nodes, 8);
        assert_eq!(seed.summary().sources, 8);
        assert_eq!(seed.summary().retrieval_chunks, 8);
        assert!(seed
            .retrieval_chunks
            .iter()
            .all(|chunk| !chunk.teaching_context.trim().is_empty()));
    }

    #[test]
    fn rejects_chunk_without_citation_label() {
        let value = serde_json::json!({
            "sources": [{
                "id": "source",
                "title": "Source",
                "url": "https://example.test",
                "source_kind": "tutorial_chapter",
                "license_note": "test",
                "retrieved_at": "2026-06-29T00:00:00Z"
            }],
            "nodes": [{
                "id": "node",
                "title": "Node",
                "chapter": "chapter",
                "kind": "concept",
                "summary": "summary",
                "learning_objectives": [],
                "common_misconceptions": [],
                "source_id": "source"
            }],
            "edges": [],
            "retrieval_chunks": [{
                "id": "chunk",
                "node_id": "node",
                "source_id": "source",
                "original_summary": "summary",
                "teaching_context": "context",
                "citation_label": ""
            }]
        });

        let error = KnowledgeSeed::from_json_str(&value.to_string())
            .expect_err("empty citation label should fail");

        assert_eq!(
            error,
            KnowledgeSeedError::MissingField("retrieval_chunk.citation_label")
        );
    }
}
