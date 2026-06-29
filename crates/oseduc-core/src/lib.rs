#![forbid(unsafe_code)]

mod knowledge;
mod tutor;

pub use knowledge::{
    KnowledgeEdge, KnowledgeEdgeDirection, KnowledgeNeighbor, KnowledgeNode, KnowledgeNodeDetail,
    RetrievalChunk, SourceReference, TutorContextChunk,
};
pub use tutor::{Citation, SafetyFlag, TutorChatRequest, TutorResponse};

pub fn crate_name() -> &'static str {
    "oseduc-core"
}

#[cfg(test)]
mod tests {
    #[test]
    fn exposes_crate_name() {
        assert_eq!(super::crate_name(), "oseduc-core");
    }
}
