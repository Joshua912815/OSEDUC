use std::collections::HashSet;

use oseduc_core::{
    KnowledgeEdgeDirection, KnowledgeNeighbor, KnowledgeNode, KnowledgeNodeDetail, RetrievalChunk,
    SourceReference, TutorContextChunk,
};

use crate::{KnowledgeSeed, KnowledgeSeedSummary, PostgresStore, StoreError};

impl PostgresStore {
    pub async fn list_sources(&self) -> Result<Vec<SourceReference>, StoreError> {
        sqlx::query_as::<_, SourceReferenceRow>(
            "SELECT id, title, url, source_kind, license_note, retrieved_at::TEXT AS retrieved_at
             FROM source_references
             ORDER BY id",
        )
        .fetch_all(self.pool())
        .await
        .map(|rows| rows.into_iter().map(SourceReference::from).collect())
        .map_err(database_error)
    }

    pub async fn list_nodes(&self) -> Result<Vec<KnowledgeNode>, StoreError> {
        sqlx::query_as::<_, KnowledgeNodeRow>(
            "SELECT id, title, chapter, kind, summary, learning_objectives,
                    common_misconceptions, source_id
             FROM knowledge_nodes
             ORDER BY chapter, id",
        )
        .fetch_all(self.pool())
        .await
        .map(|rows| rows.into_iter().map(KnowledgeNode::from).collect())
        .map_err(database_error)
    }

    pub async fn get_node_detail(&self, id: &str) -> Result<KnowledgeNodeDetail, StoreError> {
        let node = self.get_node(id).await?;
        let source = self.get_source(&node.source_id).await?;
        let retrieval_chunks = self.list_retrieval_chunks_for_node(id).await?;

        Ok(KnowledgeNodeDetail {
            node,
            source,
            retrieval_chunks,
        })
    }

    pub async fn get_neighbors(&self, id: &str) -> Result<Vec<KnowledgeNeighbor>, StoreError> {
        self.ensure_node_exists(id).await?;
        sqlx::query_as::<_, KnowledgeNeighborRow>(
            "SELECT n.id, n.title, n.chapter, n.kind, n.summary, n.learning_objectives,
                    n.common_misconceptions, n.source_id, e.relation,
                    'dependent' AS direction
             FROM knowledge_edges e
             JOIN knowledge_nodes n ON n.id = e.to_node_id
             WHERE e.from_node_id = $1
             UNION ALL
             SELECT n.id, n.title, n.chapter, n.kind, n.summary, n.learning_objectives,
                    n.common_misconceptions, n.source_id, e.relation,
                    'prerequisite' AS direction
             FROM knowledge_edges e
             JOIN knowledge_nodes n ON n.id = e.from_node_id
             WHERE e.to_node_id = $1
             ORDER BY chapter, id",
        )
        .bind(id)
        .fetch_all(self.pool())
        .await
        .map(|rows| rows.into_iter().map(KnowledgeNeighbor::from).collect())
        .map_err(database_error)
    }

    pub async fn tutor_context_for_node_ids(
        &self,
        node_ids: &[String],
    ) -> Result<Vec<TutorContextChunk>, StoreError> {
        if node_ids.is_empty() {
            return Ok(Vec::new());
        }

        let rows = sqlx::query_as::<_, TutorContextChunkRow>(
            "SELECT rc.node_id, n.title AS node_title, rc.source_id,
                    s.title AS source_title, s.url AS source_url, s.license_note,
                    rc.teaching_context, rc.citation_label
             FROM retrieval_chunks rc
             JOIN knowledge_nodes n ON n.id = rc.node_id
             JOIN source_references s ON s.id = rc.source_id
             WHERE rc.node_id = ANY($1::TEXT[])
             ORDER BY array_position($1::TEXT[], rc.node_id), rc.id",
        )
        .bind(node_ids)
        .fetch_all(self.pool())
        .await
        .map_err(database_error)?;

        let found_node_ids = rows
            .iter()
            .map(|row| row.node_id.as_str())
            .collect::<HashSet<_>>();
        if let Some(missing_id) = node_ids
            .iter()
            .find(|node_id| !found_node_ids.contains(node_id.as_str()))
        {
            return Err(StoreError::NotFound(missing_id.clone()));
        }

        Ok(rows.into_iter().map(TutorContextChunk::from).collect())
    }

    pub async fn seed_knowledge_graph(
        &self,
        seed: &KnowledgeSeed,
    ) -> Result<KnowledgeSeedSummary, StoreError> {
        seed.validate()
            .map_err(|error| StoreError::InvalidSeed(error.to_string()))?;

        let mut transaction = self.pool().begin().await.map_err(database_error)?;

        for source in &seed.sources {
            sqlx::query(
                "INSERT INTO source_references
                    (id, title, url, source_kind, license_note, retrieved_at)
                 VALUES ($1, $2, $3, $4, $5, $6::TIMESTAMPTZ)
                 ON CONFLICT (id) DO UPDATE SET
                    title = EXCLUDED.title,
                    url = EXCLUDED.url,
                    source_kind = EXCLUDED.source_kind,
                    license_note = EXCLUDED.license_note,
                    retrieved_at = EXCLUDED.retrieved_at",
            )
            .bind(&source.id)
            .bind(&source.title)
            .bind(&source.url)
            .bind(&source.source_kind)
            .bind(&source.license_note)
            .bind(&source.retrieved_at)
            .execute(&mut *transaction)
            .await
            .map_err(database_error)?;
        }

        for node in &seed.nodes {
            sqlx::query(
                "INSERT INTO knowledge_nodes
                    (id, title, chapter, kind, summary, learning_objectives,
                     common_misconceptions, source_id)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                 ON CONFLICT (id) DO UPDATE SET
                    title = EXCLUDED.title,
                    chapter = EXCLUDED.chapter,
                    kind = EXCLUDED.kind,
                    summary = EXCLUDED.summary,
                    learning_objectives = EXCLUDED.learning_objectives,
                    common_misconceptions = EXCLUDED.common_misconceptions,
                    source_id = EXCLUDED.source_id",
            )
            .bind(&node.id)
            .bind(&node.title)
            .bind(&node.chapter)
            .bind(&node.kind)
            .bind(&node.summary)
            .bind(&node.learning_objectives)
            .bind(&node.common_misconceptions)
            .bind(&node.source_id)
            .execute(&mut *transaction)
            .await
            .map_err(database_error)?;
        }

        for edge in &seed.edges {
            sqlx::query(
                "INSERT INTO knowledge_edges (from_node_id, to_node_id, relation)
                 VALUES ($1, $2, $3)
                 ON CONFLICT (from_node_id, to_node_id, relation) DO NOTHING",
            )
            .bind(&edge.from_node_id)
            .bind(&edge.to_node_id)
            .bind(&edge.relation)
            .execute(&mut *transaction)
            .await
            .map_err(database_error)?;
        }

        for chunk in &seed.retrieval_chunks {
            sqlx::query(
                "INSERT INTO retrieval_chunks
                    (id, node_id, source_id, original_summary, teaching_context, citation_label)
                 VALUES ($1, $2, $3, $4, $5, $6)
                 ON CONFLICT (id) DO UPDATE SET
                    node_id = EXCLUDED.node_id,
                    source_id = EXCLUDED.source_id,
                    original_summary = EXCLUDED.original_summary,
                    teaching_context = EXCLUDED.teaching_context,
                    citation_label = EXCLUDED.citation_label",
            )
            .bind(&chunk.id)
            .bind(&chunk.node_id)
            .bind(&chunk.source_id)
            .bind(&chunk.original_summary)
            .bind(&chunk.teaching_context)
            .bind(&chunk.citation_label)
            .execute(&mut *transaction)
            .await
            .map_err(database_error)?;
        }

        transaction.commit().await.map_err(database_error)?;

        Ok(seed.summary())
    }

    async fn ensure_node_exists(&self, id: &str) -> Result<(), StoreError> {
        let exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM knowledge_nodes WHERE id = $1)",
        )
        .bind(id)
        .fetch_one(self.pool())
        .await
        .map_err(database_error)?;
        if exists {
            Ok(())
        } else {
            Err(StoreError::NotFound(id.to_owned()))
        }
    }

    async fn get_node(&self, id: &str) -> Result<KnowledgeNode, StoreError> {
        sqlx::query_as::<_, KnowledgeNodeRow>(
            "SELECT id, title, chapter, kind, summary, learning_objectives,
                    common_misconceptions, source_id
             FROM knowledge_nodes
             WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(self.pool())
        .await
        .map_err(database_error)?
        .map(KnowledgeNode::from)
        .ok_or_else(|| StoreError::NotFound(id.to_owned()))
    }

    async fn get_source(&self, id: &str) -> Result<SourceReference, StoreError> {
        sqlx::query_as::<_, SourceReferenceRow>(
            "SELECT id, title, url, source_kind, license_note, retrieved_at::TEXT AS retrieved_at
             FROM source_references
             WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(self.pool())
        .await
        .map_err(database_error)?
        .map(SourceReference::from)
        .ok_or_else(|| StoreError::NotFound(id.to_owned()))
    }

    async fn list_retrieval_chunks_for_node(
        &self,
        node_id: &str,
    ) -> Result<Vec<RetrievalChunk>, StoreError> {
        sqlx::query_as::<_, RetrievalChunkRow>(
            "SELECT id, node_id, source_id, original_summary, teaching_context, citation_label
             FROM retrieval_chunks
             WHERE node_id = $1
             ORDER BY id",
        )
        .bind(node_id)
        .fetch_all(self.pool())
        .await
        .map(|rows| rows.into_iter().map(RetrievalChunk::from).collect())
        .map_err(database_error)
    }
}

#[derive(sqlx::FromRow)]
struct SourceReferenceRow {
    id: String,
    title: String,
    url: String,
    source_kind: String,
    license_note: String,
    retrieved_at: String,
}

impl From<SourceReferenceRow> for SourceReference {
    fn from(row: SourceReferenceRow) -> Self {
        Self {
            id: row.id,
            title: row.title,
            url: row.url,
            source_kind: row.source_kind,
            license_note: row.license_note,
            retrieved_at: row.retrieved_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct KnowledgeNodeRow {
    id: String,
    title: String,
    chapter: String,
    kind: String,
    summary: String,
    learning_objectives: Vec<String>,
    common_misconceptions: Vec<String>,
    source_id: String,
}

impl From<KnowledgeNodeRow> for KnowledgeNode {
    fn from(row: KnowledgeNodeRow) -> Self {
        Self {
            id: row.id,
            title: row.title,
            chapter: row.chapter,
            kind: row.kind,
            summary: row.summary,
            learning_objectives: row.learning_objectives,
            common_misconceptions: row.common_misconceptions,
            source_id: row.source_id,
        }
    }
}

#[derive(sqlx::FromRow)]
struct RetrievalChunkRow {
    id: String,
    node_id: String,
    source_id: String,
    original_summary: String,
    teaching_context: String,
    citation_label: String,
}

impl From<RetrievalChunkRow> for RetrievalChunk {
    fn from(row: RetrievalChunkRow) -> Self {
        Self {
            id: row.id,
            node_id: row.node_id,
            source_id: row.source_id,
            original_summary: row.original_summary,
            teaching_context: row.teaching_context,
            citation_label: row.citation_label,
        }
    }
}

#[derive(sqlx::FromRow)]
struct KnowledgeNeighborRow {
    id: String,
    title: String,
    chapter: String,
    kind: String,
    summary: String,
    learning_objectives: Vec<String>,
    common_misconceptions: Vec<String>,
    source_id: String,
    relation: String,
    direction: String,
}

impl From<KnowledgeNeighborRow> for KnowledgeNeighbor {
    fn from(row: KnowledgeNeighborRow) -> Self {
        let direction = match row.direction.as_str() {
            "prerequisite" => KnowledgeEdgeDirection::Prerequisite,
            _ => KnowledgeEdgeDirection::Dependent,
        };
        Self {
            node: KnowledgeNode {
                id: row.id,
                title: row.title,
                chapter: row.chapter,
                kind: row.kind,
                summary: row.summary,
                learning_objectives: row.learning_objectives,
                common_misconceptions: row.common_misconceptions,
                source_id: row.source_id,
            },
            relation: row.relation,
            direction,
        }
    }
}

#[derive(sqlx::FromRow)]
struct TutorContextChunkRow {
    node_id: String,
    node_title: String,
    source_id: String,
    source_title: String,
    source_url: String,
    license_note: String,
    teaching_context: String,
    citation_label: String,
}

impl From<TutorContextChunkRow> for TutorContextChunk {
    fn from(row: TutorContextChunkRow) -> Self {
        Self {
            node_id: row.node_id,
            node_title: row.node_title,
            source_id: row.source_id,
            source_title: row.source_title,
            source_url: row.source_url,
            license_note: row.license_note,
            teaching_context: row.teaching_context,
            citation_label: row.citation_label,
        }
    }
}

fn database_error(error: sqlx::Error) -> StoreError {
    StoreError::Database(error.to_string())
}
