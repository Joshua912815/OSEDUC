use std::collections::HashSet;

use oseduc_core::{
    Citation, KnowledgeEdge, KnowledgeEdgeDirection, KnowledgeNeighbor, KnowledgeNode,
    KnowledgeNodeDetail, RecordProgressRequest, RetrievalChunk, SafetyFlag, SourceReference,
    StudentNodeProgress, StudentProfile, TutorChatRequest, TutorContextChunk,
    TutorFeedbackDifficulty, TutorFeedbackRequest, TutorInteraction, TutorInteractionFeedback,
    TutorResponse, UpsertStudentProfileRequest,
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

    pub async fn list_edges(&self) -> Result<Vec<KnowledgeEdge>, StoreError> {
        sqlx::query_as::<_, KnowledgeEdgeRow>(
            "SELECT from_node_id, to_node_id, relation
             FROM knowledge_edges
             ORDER BY from_node_id, to_node_id, relation",
        )
        .fetch_all(self.pool())
        .await
        .map(|rows| rows.into_iter().map(KnowledgeEdge::from).collect())
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

    pub async fn get_or_create_student_profile(
        &self,
        student_id: &str,
    ) -> Result<StudentProfile, StoreError> {
        sqlx::query(
            "INSERT INTO student_profiles (student_id)
             VALUES ($1)
             ON CONFLICT (student_id) DO NOTHING",
        )
        .bind(student_id)
        .execute(self.pool())
        .await
        .map_err(database_error)?;

        self.get_student_profile(student_id).await
    }

    pub async fn upsert_student_profile(
        &self,
        student_id: &str,
        request: &UpsertStudentProfileRequest,
    ) -> Result<StudentProfile, StoreError> {
        sqlx::query_as::<_, StudentProfileRow>(
            "INSERT INTO student_profiles
                (student_id, display_name, learning_goal, preferred_depth, updated_at)
             VALUES ($1, $2, $3, $4, now())
             ON CONFLICT (student_id) DO UPDATE SET
                display_name = EXCLUDED.display_name,
                learning_goal = EXCLUDED.learning_goal,
                preferred_depth = EXCLUDED.preferred_depth,
                updated_at = now()
             RETURNING student_id, display_name, learning_goal, preferred_depth,
                       created_at::TEXT AS created_at, updated_at::TEXT AS updated_at",
        )
        .bind(student_id)
        .bind(&request.display_name)
        .bind(&request.learning_goal)
        .bind(request.preferred_depth.as_str())
        .fetch_one(self.pool())
        .await
        .map(StudentProfile::from)
        .map_err(database_error)
    }

    pub async fn get_student_profile(
        &self,
        student_id: &str,
    ) -> Result<StudentProfile, StoreError> {
        sqlx::query_as::<_, StudentProfileRow>(
            "SELECT student_id, display_name, learning_goal, preferred_depth,
                    created_at::TEXT AS created_at, updated_at::TEXT AS updated_at
             FROM student_profiles
             WHERE student_id = $1",
        )
        .bind(student_id)
        .fetch_optional(self.pool())
        .await
        .map_err(database_error)?
        .map(StudentProfile::from)
        .ok_or_else(|| StoreError::NotFound(student_id.to_owned()))
    }

    pub async fn list_student_progress(
        &self,
        student_id: &str,
    ) -> Result<Vec<StudentNodeProgress>, StoreError> {
        sqlx::query_as::<_, StudentNodeProgressRow>(
            "SELECT student_id, node_id, status, mastery_score::INT AS mastery_score,
                    notes, updated_at::TEXT AS updated_at
             FROM student_node_progress
             WHERE student_id = $1
             ORDER BY node_id",
        )
        .bind(student_id)
        .fetch_all(self.pool())
        .await
        .map(|rows| rows.into_iter().map(StudentNodeProgress::from).collect())
        .map_err(database_error)
    }

    pub async fn record_student_progress(
        &self,
        student_id: &str,
        node_id: &str,
        request: &RecordProgressRequest,
    ) -> Result<StudentNodeProgress, StoreError> {
        self.ensure_node_exists(node_id).await?;
        self.get_or_create_student_profile(student_id).await?;
        let mastery_score = request
            .validated_score()
            .map_err(|error| StoreError::InvalidInput(error.to_string()))?;

        sqlx::query_as::<_, StudentNodeProgressRow>(
            "INSERT INTO student_node_progress
                (student_id, node_id, status, mastery_score, notes, last_interaction_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, now(), now())
             ON CONFLICT (student_id, node_id) DO UPDATE SET
                status = EXCLUDED.status,
                mastery_score = EXCLUDED.mastery_score,
                notes = EXCLUDED.notes,
                last_interaction_at = now(),
                updated_at = now()
             RETURNING student_id, node_id, status, mastery_score::INT AS mastery_score,
                       notes, updated_at::TEXT AS updated_at",
        )
        .bind(student_id)
        .bind(node_id)
        .bind(request.status.as_str())
        .bind(i16::from(mastery_score))
        .bind(&request.notes)
        .fetch_one(self.pool())
        .await
        .map(StudentNodeProgress::from)
        .map_err(database_error)
    }

    pub async fn record_tutor_interaction(
        &self,
        request: &TutorChatRequest,
        response: &TutorResponse,
        log_student_message: bool,
    ) -> Result<TutorInteraction, StoreError> {
        let student_id = normalized_optional_text(request.student_id.as_deref());
        if let Some(student_id) = student_id.as_deref() {
            self.get_or_create_student_profile(student_id).await?;
        }

        let citation_labels = response
            .citations
            .iter()
            .map(|citation| citation.label.clone())
            .collect::<Vec<_>>();
        let citation_sources = response
            .citations
            .iter()
            .map(|citation| citation.source.clone())
            .collect::<Vec<_>>();
        let citation_node_ids = response
            .citations
            .iter()
            .map(|citation| citation.node_id.clone().unwrap_or_default())
            .collect::<Vec<_>>();
        let safety_flags = response
            .safety_flags
            .iter()
            .map(SafetyFlag::as_str)
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let message = log_student_message.then(|| request.message.clone());

        sqlx::query_as::<_, TutorInteractionRow>(
            "INSERT INTO tutor_interactions
                (student_id, provider, knowledge_node_ids, citation_labels, citation_sources,
                 citation_node_ids, safety_flags, message_logged, message)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
             RETURNING id, student_id, provider, knowledge_node_ids, citation_labels,
                       citation_sources, citation_node_ids, safety_flags, message_logged, message,
                       created_at::TEXT AS created_at,
                       NULL::BOOLEAN AS feedback_helpful,
                       NULL::TEXT AS feedback_difficulty,
                       NULL::TEXT AS feedback_text,
                       NULL::TEXT AS feedback_updated_at",
        )
        .bind(student_id.as_deref())
        .bind(&response.provider)
        .bind(&request.knowledge_node_ids)
        .bind(&citation_labels)
        .bind(&citation_sources)
        .bind(&citation_node_ids)
        .bind(&safety_flags)
        .bind(log_student_message)
        .bind(&message)
        .fetch_one(self.pool())
        .await
        .map(TutorInteraction::from)
        .map_err(database_error)
    }

    pub async fn list_tutor_interactions(
        &self,
        student_id: &str,
        limit: i64,
    ) -> Result<Vec<TutorInteraction>, StoreError> {
        let limit = limit.clamp(1, 100);

        sqlx::query_as::<_, TutorInteractionRow>(
            "SELECT ti.id, ti.student_id, ti.provider, ti.knowledge_node_ids,
                    ti.citation_labels, ti.citation_sources, ti.citation_node_ids,
                    ti.safety_flags, ti.message_logged, ti.message,
                    ti.created_at::TEXT AS created_at,
                    tf.helpful AS feedback_helpful,
                    tf.difficulty AS feedback_difficulty,
                    tf.feedback_text AS feedback_text,
                    tf.updated_at::TEXT AS feedback_updated_at
             FROM tutor_interactions ti
             LEFT JOIN tutor_interaction_feedback tf ON tf.interaction_id = ti.id
             WHERE ti.student_id = $1
             ORDER BY ti.created_at DESC, ti.id DESC
             LIMIT $2",
        )
        .bind(student_id)
        .bind(limit)
        .fetch_all(self.pool())
        .await
        .map(|rows| rows.into_iter().map(TutorInteraction::from).collect())
        .map_err(database_error)
    }

    pub async fn upsert_tutor_feedback(
        &self,
        interaction_id: i64,
        request: &TutorFeedbackRequest,
    ) -> Result<TutorInteractionFeedback, StoreError> {
        self.ensure_tutor_interaction_exists(interaction_id).await?;
        let difficulty = request
            .difficulty
            .as_ref()
            .map(TutorFeedbackDifficulty::as_str);
        let feedback_text = normalized_optional_text(request.feedback_text.as_deref());

        sqlx::query_as::<_, TutorFeedbackRow>(
            "INSERT INTO tutor_interaction_feedback
                (interaction_id, helpful, difficulty, feedback_text, updated_at)
             VALUES ($1, $2, $3, $4, now())
             ON CONFLICT (interaction_id) DO UPDATE SET
                helpful = EXCLUDED.helpful,
                difficulty = EXCLUDED.difficulty,
                feedback_text = EXCLUDED.feedback_text,
                updated_at = now()
             RETURNING interaction_id, helpful, difficulty, feedback_text,
                       updated_at::TEXT AS updated_at",
        )
        .bind(interaction_id)
        .bind(request.helpful)
        .bind(difficulty)
        .bind(&feedback_text)
        .fetch_one(self.pool())
        .await
        .map(TutorInteractionFeedback::from)
        .map_err(database_error)
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

    async fn ensure_tutor_interaction_exists(&self, id: i64) -> Result<(), StoreError> {
        let exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM tutor_interactions WHERE id = $1)",
        )
        .bind(id)
        .fetch_one(self.pool())
        .await
        .map_err(database_error)?;
        if exists {
            Ok(())
        } else {
            Err(StoreError::NotFound(id.to_string()))
        }
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
struct KnowledgeEdgeRow {
    from_node_id: String,
    to_node_id: String,
    relation: String,
}

impl From<KnowledgeEdgeRow> for KnowledgeEdge {
    fn from(row: KnowledgeEdgeRow) -> Self {
        Self {
            from_node_id: row.from_node_id,
            to_node_id: row.to_node_id,
            relation: row.relation,
        }
    }
}

#[derive(sqlx::FromRow)]
struct StudentProfileRow {
    student_id: String,
    display_name: Option<String>,
    learning_goal: Option<String>,
    preferred_depth: String,
    created_at: String,
    updated_at: String,
}

impl From<StudentProfileRow> for StudentProfile {
    fn from(row: StudentProfileRow) -> Self {
        Self {
            student_id: row.student_id,
            display_name: row.display_name,
            learning_goal: row.learning_goal,
            preferred_depth: oseduc_core::LearningDepth::parse(&row.preferred_depth),
            created_at: Some(row.created_at),
            updated_at: Some(row.updated_at),
        }
    }
}

#[derive(sqlx::FromRow)]
struct StudentNodeProgressRow {
    student_id: String,
    node_id: String,
    status: String,
    mastery_score: i32,
    notes: Option<String>,
    updated_at: String,
}

impl From<StudentNodeProgressRow> for StudentNodeProgress {
    fn from(row: StudentNodeProgressRow) -> Self {
        Self {
            student_id: row.student_id,
            node_id: row.node_id,
            status: oseduc_core::ProgressStatus::parse(&row.status),
            mastery_score: row.mastery_score.clamp(0, 100) as u8,
            notes: row.notes,
            updated_at: Some(row.updated_at),
        }
    }
}

#[derive(sqlx::FromRow)]
struct TutorInteractionRow {
    id: i64,
    student_id: Option<String>,
    provider: String,
    knowledge_node_ids: Vec<String>,
    citation_labels: Vec<String>,
    citation_sources: Vec<String>,
    citation_node_ids: Vec<String>,
    safety_flags: Vec<String>,
    message_logged: bool,
    message: Option<String>,
    created_at: String,
    feedback_helpful: Option<bool>,
    feedback_difficulty: Option<String>,
    feedback_text: Option<String>,
    feedback_updated_at: Option<String>,
}

impl From<TutorInteractionRow> for TutorInteraction {
    fn from(row: TutorInteractionRow) -> Self {
        let feedback = row
            .feedback_updated_at
            .map(|updated_at| TutorInteractionFeedback {
                interaction_id: row.id,
                helpful: row.feedback_helpful,
                difficulty: row
                    .feedback_difficulty
                    .as_deref()
                    .and_then(TutorFeedbackDifficulty::parse),
                feedback_text: row.feedback_text,
                updated_at: Some(updated_at),
            });

        Self {
            id: row.id,
            student_id: row.student_id,
            knowledge_node_ids: row.knowledge_node_ids,
            provider: row.provider,
            citations: citations_from_arrays(
                row.citation_labels,
                row.citation_sources,
                row.citation_node_ids,
            ),
            safety_flags: row
                .safety_flags
                .into_iter()
                .filter_map(|flag| SafetyFlag::parse(&flag))
                .collect(),
            message_logged: row.message_logged,
            message: row.message,
            created_at: Some(row.created_at),
            feedback,
        }
    }
}

#[derive(sqlx::FromRow)]
struct TutorFeedbackRow {
    interaction_id: i64,
    helpful: Option<bool>,
    difficulty: Option<String>,
    feedback_text: Option<String>,
    updated_at: String,
}

impl From<TutorFeedbackRow> for TutorInteractionFeedback {
    fn from(row: TutorFeedbackRow) -> Self {
        Self {
            interaction_id: row.interaction_id,
            helpful: row.helpful,
            difficulty: row
                .difficulty
                .as_deref()
                .and_then(TutorFeedbackDifficulty::parse),
            feedback_text: row.feedback_text,
            updated_at: Some(row.updated_at),
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

fn normalized_optional_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn citations_from_arrays(
    labels: Vec<String>,
    sources: Vec<String>,
    node_ids: Vec<String>,
) -> Vec<Citation> {
    labels
        .into_iter()
        .enumerate()
        .map(|(index, label)| Citation {
            label,
            source: sources.get(index).cloned().unwrap_or_default(),
            node_id: node_ids
                .get(index)
                .filter(|node_id| !node_id.is_empty())
                .cloned(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconstructs_citations_from_parallel_arrays() {
        let citations = citations_from_arrays(
            vec!["rCore v3 ch4".to_owned()],
            vec!["https://example.test/ch4".to_owned()],
            vec!["ch4-address-space".to_owned()],
        );

        assert_eq!(citations.len(), 1);
        assert_eq!(citations[0].label, "rCore v3 ch4");
        assert_eq!(citations[0].source, "https://example.test/ch4");
        assert_eq!(citations[0].node_id, Some("ch4-address-space".to_owned()));
    }

    #[test]
    fn normalizes_optional_text() {
        assert_eq!(
            normalized_optional_text(Some("  hello  ")),
            Some("hello".to_owned())
        );
        assert_eq!(normalized_optional_text(Some("  ")), None);
        assert_eq!(normalized_optional_text(None), None);
    }
}
