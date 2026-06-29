use std::sync::Arc;

use async_trait::async_trait;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post, put},
    Json, Router,
};
use oseduc_core::{
    KnowledgeEdge, KnowledgeNeighbor, KnowledgeNode, KnowledgeNodeDetail, LearningPath,
    RecordProgressRequest, SafetyFlag, SourceReference, StudentNodeProgress, StudentProfile,
    TutorChatRequest, TutorContextChunk, TutorResponse, UpsertStudentProfileRequest,
};
use oseduc_llm::{LlmError, LlmGateway};
use oseduc_policy::recommend_learning_path;
use oseduc_store::{KnowledgeSeed, KnowledgeSeedSummary, PostgresStore, StoreError};
use serde::{Deserialize, Serialize};

use crate::PublicConfig;

#[derive(Clone)]
pub struct AppState {
    pub gateway: LlmGateway,
    pub public_config: PublicConfig,
    pub knowledge: Arc<dyn KnowledgeCatalog>,
    pub admin_seed_enabled: bool,
}

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/v1/config/public", get(public_config))
        .route("/v1/knowledge/nodes", get(list_knowledge_nodes))
        .route("/v1/knowledge/nodes/{id}", get(get_knowledge_node))
        .route(
            "/v1/knowledge/nodes/{id}/neighbors",
            get(get_knowledge_neighbors),
        )
        .route("/v1/sources", get(list_sources))
        .route("/v1/admin/knowledge/seed", post(seed_knowledge))
        .route(
            "/v1/students/{student_id}/profile",
            get(get_student_profile).put(upsert_student_profile),
        )
        .route(
            "/v1/students/{student_id}/progress",
            get(list_student_progress),
        )
        .route(
            "/v1/students/{student_id}/progress/{node_id}",
            put(record_student_progress),
        )
        .route(
            "/v1/students/{student_id}/learning-path",
            get(get_learning_path),
        )
        .route("/v1/tutor/chat", post(tutor_chat))
        .with_state(state)
}

#[async_trait]
pub trait KnowledgeCatalog: Send + Sync {
    async fn list_nodes(&self) -> Result<Vec<KnowledgeNode>, StoreError>;

    async fn list_edges(&self) -> Result<Vec<KnowledgeEdge>, StoreError>;

    async fn get_node_detail(&self, id: &str) -> Result<KnowledgeNodeDetail, StoreError>;

    async fn get_neighbors(&self, id: &str) -> Result<Vec<KnowledgeNeighbor>, StoreError>;

    async fn list_sources(&self) -> Result<Vec<SourceReference>, StoreError>;

    async fn seed_knowledge_graph(
        &self,
        seed: &KnowledgeSeed,
    ) -> Result<KnowledgeSeedSummary, StoreError>;

    async fn tutor_context_for_node_ids(
        &self,
        node_ids: &[String],
    ) -> Result<Vec<TutorContextChunk>, StoreError>;

    async fn get_or_create_student_profile(
        &self,
        student_id: &str,
    ) -> Result<StudentProfile, StoreError>;

    async fn upsert_student_profile(
        &self,
        student_id: &str,
        request: &UpsertStudentProfileRequest,
    ) -> Result<StudentProfile, StoreError>;

    async fn list_student_progress(
        &self,
        student_id: &str,
    ) -> Result<Vec<StudentNodeProgress>, StoreError>;

    async fn record_student_progress(
        &self,
        student_id: &str,
        node_id: &str,
        request: &RecordProgressRequest,
    ) -> Result<StudentNodeProgress, StoreError>;
}

#[async_trait]
impl KnowledgeCatalog for PostgresStore {
    async fn list_nodes(&self) -> Result<Vec<KnowledgeNode>, StoreError> {
        PostgresStore::list_nodes(self).await
    }

    async fn list_edges(&self) -> Result<Vec<KnowledgeEdge>, StoreError> {
        PostgresStore::list_edges(self).await
    }

    async fn get_node_detail(&self, id: &str) -> Result<KnowledgeNodeDetail, StoreError> {
        PostgresStore::get_node_detail(self, id).await
    }

    async fn get_neighbors(&self, id: &str) -> Result<Vec<KnowledgeNeighbor>, StoreError> {
        PostgresStore::get_neighbors(self, id).await
    }

    async fn list_sources(&self) -> Result<Vec<SourceReference>, StoreError> {
        PostgresStore::list_sources(self).await
    }

    async fn seed_knowledge_graph(
        &self,
        seed: &KnowledgeSeed,
    ) -> Result<KnowledgeSeedSummary, StoreError> {
        PostgresStore::seed_knowledge_graph(self, seed).await
    }

    async fn tutor_context_for_node_ids(
        &self,
        node_ids: &[String],
    ) -> Result<Vec<TutorContextChunk>, StoreError> {
        PostgresStore::tutor_context_for_node_ids(self, node_ids).await
    }

    async fn get_or_create_student_profile(
        &self,
        student_id: &str,
    ) -> Result<StudentProfile, StoreError> {
        PostgresStore::get_or_create_student_profile(self, student_id).await
    }

    async fn upsert_student_profile(
        &self,
        student_id: &str,
        request: &UpsertStudentProfileRequest,
    ) -> Result<StudentProfile, StoreError> {
        PostgresStore::upsert_student_profile(self, student_id, request).await
    }

    async fn list_student_progress(
        &self,
        student_id: &str,
    ) -> Result<Vec<StudentNodeProgress>, StoreError> {
        PostgresStore::list_student_progress(self, student_id).await
    }

    async fn record_student_progress(
        &self,
        student_id: &str,
        node_id: &str,
        request: &RecordProgressRequest,
    ) -> Result<StudentNodeProgress, StoreError> {
        PostgresStore::record_student_progress(self, student_id, node_id, request).await
    }
}

async fn healthz() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

async fn public_config(State(state): State<AppState>) -> Json<PublicConfig> {
    Json(state.public_config)
}

async fn list_knowledge_nodes(
    State(state): State<AppState>,
) -> Result<Json<Vec<KnowledgeNode>>, AppError> {
    Ok(Json(state.knowledge.list_nodes().await?))
}

async fn get_knowledge_node(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<KnowledgeNodeDetail>, AppError> {
    Ok(Json(state.knowledge.get_node_detail(&id).await?))
}

async fn get_knowledge_neighbors(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Vec<KnowledgeNeighbor>>, AppError> {
    Ok(Json(state.knowledge.get_neighbors(&id).await?))
}

async fn list_sources(
    State(state): State<AppState>,
) -> Result<Json<Vec<SourceReference>>, AppError> {
    Ok(Json(state.knowledge.list_sources().await?))
}

async fn seed_knowledge(
    State(state): State<AppState>,
) -> Result<Json<KnowledgeSeedSummary>, AppError> {
    if !state.admin_seed_enabled {
        return Err(AppError::new(
            StatusCode::NOT_FOUND,
            "not_found",
            "admin seed endpoint is disabled",
        ));
    }
    let seed = KnowledgeSeed::from_json_str(RCORE_V3_RUST_SEED)
        .map_err(|error| AppError::bad_request("invalid_seed", error.to_string()))?;
    Ok(Json(state.knowledge.seed_knowledge_graph(&seed).await?))
}

async fn get_student_profile(
    State(state): State<AppState>,
    Path(student_id): Path<String>,
) -> Result<Json<StudentProfile>, AppError> {
    Ok(Json(
        state
            .knowledge
            .get_or_create_student_profile(&student_id)
            .await?,
    ))
}

async fn upsert_student_profile(
    State(state): State<AppState>,
    Path(student_id): Path<String>,
    Json(request): Json<UpsertStudentProfileRequest>,
) -> Result<Json<StudentProfile>, AppError> {
    Ok(Json(
        state
            .knowledge
            .upsert_student_profile(&student_id, &request)
            .await?,
    ))
}

async fn list_student_progress(
    State(state): State<AppState>,
    Path(student_id): Path<String>,
) -> Result<Json<Vec<StudentNodeProgress>>, AppError> {
    Ok(Json(
        state.knowledge.list_student_progress(&student_id).await?,
    ))
}

async fn record_student_progress(
    State(state): State<AppState>,
    Path((student_id, node_id)): Path<(String, String)>,
    Json(request): Json<RecordProgressRequest>,
) -> Result<Json<StudentNodeProgress>, AppError> {
    Ok(Json(
        state
            .knowledge
            .record_student_progress(&student_id, &node_id, &request)
            .await?,
    ))
}

async fn get_learning_path(
    State(state): State<AppState>,
    Path(student_id): Path<String>,
    Query(query): Query<LearningPathQuery>,
) -> Result<Json<LearningPath>, AppError> {
    let nodes = state.knowledge.list_nodes().await?;
    let edges = state.knowledge.list_edges().await?;
    let progress = state.knowledge.list_student_progress(&student_id).await?;

    Ok(Json(recommend_learning_path(
        student_id,
        &nodes,
        &edges,
        &progress,
        query.limit,
    )))
}

async fn tutor_chat(
    State(state): State<AppState>,
    Json(request): Json<TutorChatRequest>,
) -> Result<Json<TutorResponse>, AppError> {
    let context_chunks = match state
        .knowledge
        .tutor_context_for_node_ids(&request.knowledge_node_ids)
        .await
    {
        Ok(context_chunks) => context_chunks,
        Err(StoreError::NotFound(id)) => {
            return Err(AppError::new(
                StatusCode::NOT_FOUND,
                "knowledge_context_missing",
                format!("knowledge context not found for node {id}"),
            ));
        }
        Err(error) => return Err(error.into()),
    };
    let response = state
        .gateway
        .chat_with_context(request, context_chunks)
        .await?;
    Ok(Json(response))
}

#[derive(Clone, Debug, Default, Deserialize)]
struct LearningPathQuery {
    limit: Option<usize>,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

#[derive(Debug)]
struct AppError {
    status: StatusCode,
    code: &'static str,
    message: String,
}

impl From<LlmError> for AppError {
    fn from(error: LlmError) -> Self {
        Self {
            status: StatusCode::BAD_GATEWAY,
            code: "llm_error",
            message: error.to_string(),
        }
    }
}

impl From<StoreError> for AppError {
    fn from(error: StoreError) -> Self {
        match error {
            StoreError::NotFound(message) => Self::new(StatusCode::NOT_FOUND, "not_found", message),
            StoreError::InvalidInput(message) => {
                Self::new(StatusCode::BAD_REQUEST, "invalid_input", message)
            }
            StoreError::InvalidSeed(_) => Self::new(
                StatusCode::BAD_REQUEST,
                "invalid_seed",
                "knowledge seed validation failed",
            ),
            StoreError::Database(_) | StoreError::Migration(_) => Self::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "knowledge_store_error",
                error.to_string(),
            ),
        }
    }
}

impl AppError {
    fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
        }
    }

    fn bad_request(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, code, message)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ErrorResponse {
                error: self.code,
                message: self.message,
                safety_flags: vec![SafetyFlag::ProviderError],
            }),
        )
            .into_response()
    }
}

#[derive(Serialize)]
struct ErrorResponse {
    error: &'static str,
    message: String,
    safety_flags: Vec<SafetyFlag>,
}

const RCORE_V3_RUST_SEED: &str = include_str!("../../../data/knowledge/rcore-v3-rust-seed.json");

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{to_bytes, Body},
        http::{Method, Request},
    };
    use oseduc_core::{KnowledgeEdgeDirection, LearningDepth, ProgressStatus, RetrievalChunk};
    use oseduc_llm::LlmConfig;
    use oseduc_store::{DatabaseConfig, SecretDatabaseUrl};
    use tower::ServiceExt;

    fn test_state() -> AppState {
        test_state_with_admin_seed(false)
    }

    fn test_state_with_admin_seed(admin_seed_enabled: bool) -> AppState {
        let config = LlmConfig::from_getter(|_| None).expect("mock config should load");
        let database = DatabaseConfig {
            database_url: SecretDatabaseUrl::new(
                "postgres://oseduc:dev-password@127.0.0.1:5432/oseduc",
            ),
            auto_migrate: false,
            enable_admin_seed: admin_seed_enabled,
        };
        AppState {
            gateway: LlmGateway::mock(),
            public_config: PublicConfig::new(&config, &database),
            knowledge: Arc::new(MemoryKnowledgeCatalog),
            admin_seed_enabled,
        }
    }

    #[derive(Clone)]
    struct MemoryKnowledgeCatalog;

    #[async_trait]
    impl KnowledgeCatalog for MemoryKnowledgeCatalog {
        async fn list_nodes(&self) -> Result<Vec<KnowledgeNode>, StoreError> {
            Ok(sample_nodes())
        }

        async fn list_edges(&self) -> Result<Vec<KnowledgeEdge>, StoreError> {
            Ok(sample_edges())
        }

        async fn get_node_detail(&self, id: &str) -> Result<KnowledgeNodeDetail, StoreError> {
            if id == sample_node().id {
                Ok(sample_detail())
            } else {
                Err(StoreError::NotFound(id.to_owned()))
            }
        }

        async fn get_neighbors(&self, id: &str) -> Result<Vec<KnowledgeNeighbor>, StoreError> {
            if id == sample_node().id {
                Ok(vec![KnowledgeNeighbor {
                    node: KnowledgeNode {
                        id: "ch5-process".to_owned(),
                        title: "Process".to_owned(),
                        chapter: "chapter5".to_owned(),
                        kind: "rust_os_mainline".to_owned(),
                        summary: "Process model".to_owned(),
                        learning_objectives: Vec::new(),
                        common_misconceptions: Vec::new(),
                        source_id: "rcore-v3-ch5".to_owned(),
                    },
                    relation: "prerequisite_for".to_owned(),
                    direction: KnowledgeEdgeDirection::Dependent,
                }])
            } else {
                Err(StoreError::NotFound(id.to_owned()))
            }
        }

        async fn list_sources(&self) -> Result<Vec<SourceReference>, StoreError> {
            Ok(vec![sample_source()])
        }

        async fn seed_knowledge_graph(
            &self,
            seed: &KnowledgeSeed,
        ) -> Result<KnowledgeSeedSummary, StoreError> {
            Ok(seed.summary())
        }

        async fn tutor_context_for_node_ids(
            &self,
            node_ids: &[String],
        ) -> Result<Vec<TutorContextChunk>, StoreError> {
            if node_ids.is_empty() {
                return Ok(Vec::new());
            }
            if node_ids.iter().all(|node_id| node_id == &sample_node().id) {
                Ok(vec![TutorContextChunk {
                    node_id: sample_node().id,
                    node_title: sample_node().title,
                    source_id: sample_source().id,
                    source_title: sample_source().title,
                    source_url: sample_source().url,
                    license_note: sample_source().license_note,
                    teaching_context: "Address-space context".to_owned(),
                    citation_label: "rCore v3 ch4".to_owned(),
                }])
            } else {
                Err(StoreError::NotFound("missing-node".to_owned()))
            }
        }

        async fn get_or_create_student_profile(
            &self,
            student_id: &str,
        ) -> Result<StudentProfile, StoreError> {
            Ok(StudentProfile {
                student_id: student_id.to_owned(),
                display_name: None,
                learning_goal: None,
                preferred_depth: LearningDepth::Balanced,
                created_at: Some("2026-06-29 00:00:00+00".to_owned()),
                updated_at: Some("2026-06-29 00:00:00+00".to_owned()),
            })
        }

        async fn upsert_student_profile(
            &self,
            student_id: &str,
            request: &UpsertStudentProfileRequest,
        ) -> Result<StudentProfile, StoreError> {
            Ok(StudentProfile {
                student_id: student_id.to_owned(),
                display_name: request.display_name.clone(),
                learning_goal: request.learning_goal.clone(),
                preferred_depth: request.preferred_depth.clone(),
                created_at: Some("2026-06-29 00:00:00+00".to_owned()),
                updated_at: Some("2026-06-29 00:00:01+00".to_owned()),
            })
        }

        async fn list_student_progress(
            &self,
            student_id: &str,
        ) -> Result<Vec<StudentNodeProgress>, StoreError> {
            Ok(vec![StudentNodeProgress {
                student_id: student_id.to_owned(),
                node_id: "ch4-address-space".to_owned(),
                status: ProgressStatus::Mastered,
                mastery_score: 90,
                notes: Some("comfortable with page tables".to_owned()),
                updated_at: Some("2026-06-29 00:00:00+00".to_owned()),
            }])
        }

        async fn record_student_progress(
            &self,
            student_id: &str,
            node_id: &str,
            request: &RecordProgressRequest,
        ) -> Result<StudentNodeProgress, StoreError> {
            if !sample_nodes().iter().any(|node| node.id == node_id) {
                return Err(StoreError::NotFound(node_id.to_owned()));
            }
            let mastery_score = request
                .validated_score()
                .map_err(|error| StoreError::InvalidInput(error.to_string()))?;
            Ok(StudentNodeProgress {
                student_id: student_id.to_owned(),
                node_id: node_id.to_owned(),
                status: request.status.clone(),
                mastery_score,
                notes: request.notes.clone(),
                updated_at: Some("2026-06-29 00:00:02+00".to_owned()),
            })
        }
    }

    fn sample_source() -> SourceReference {
        SourceReference {
            id: "rcore-v3-ch4".to_owned(),
            title: "rCore Chapter 4".to_owned(),
            url: "https://rcore-os.cn/rCore-Tutorial-Book-v3/chapter4/index.html".to_owned(),
            source_kind: "tutorial_chapter".to_owned(),
            license_note: "GPL-3.0; cite rCore".to_owned(),
            retrieved_at: "2026-06-29T00:00:00Z".to_owned(),
        }
    }

    fn sample_node() -> KnowledgeNode {
        KnowledgeNode {
            id: "ch4-address-space".to_owned(),
            title: "Address Space".to_owned(),
            chapter: "chapter4".to_owned(),
            kind: "rust_os_mainline".to_owned(),
            summary: "Address-space model".to_owned(),
            learning_objectives: vec!["Explain page tables".to_owned()],
            common_misconceptions: Vec::new(),
            source_id: sample_source().id,
        }
    }

    fn sample_nodes() -> Vec<KnowledgeNode> {
        vec![
            sample_node(),
            KnowledgeNode {
                id: "ch5-process".to_owned(),
                title: "Process".to_owned(),
                chapter: "chapter5".to_owned(),
                kind: "rust_os_mainline".to_owned(),
                summary: "Process model".to_owned(),
                learning_objectives: Vec::new(),
                common_misconceptions: Vec::new(),
                source_id: "rcore-v3-ch5".to_owned(),
            },
        ]
    }

    fn sample_edges() -> Vec<KnowledgeEdge> {
        vec![KnowledgeEdge {
            from_node_id: "ch4-address-space".to_owned(),
            to_node_id: "ch5-process".to_owned(),
            relation: "prerequisite_for".to_owned(),
        }]
    }

    fn sample_detail() -> KnowledgeNodeDetail {
        KnowledgeNodeDetail {
            node: sample_node(),
            source: sample_source(),
            retrieval_chunks: vec![RetrievalChunk {
                id: "chunk-ch4".to_owned(),
                node_id: sample_node().id,
                source_id: sample_source().id,
                original_summary: "summary".to_owned(),
                teaching_context: "Address-space context".to_owned(),
                citation_label: "rCore v3 ch4".to_owned(),
            }],
        }
    }

    #[tokio::test]
    async fn health_check_returns_ok() {
        let app = build_router(test_state());

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/healthz")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn public_config_does_not_return_secret_fields() {
        let app = build_router(test_state());

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/config/public")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let body = String::from_utf8(body.to_vec()).expect("body should be utf8");

        assert!(body.contains("mock"));
        assert!(!body.contains("api_key"));
        assert!(!body.contains("token"));
        assert!(!body.contains("dev-password"));
    }

    #[tokio::test]
    async fn knowledge_nodes_endpoint_returns_nodes() {
        let app = build_router(test_state());

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/knowledge/nodes")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let body = String::from_utf8(body.to_vec()).expect("body should be utf8");

        assert!(body.contains("ch4-address-space"));
        assert!(body.contains("Address Space"));
    }

    #[tokio::test]
    async fn knowledge_node_detail_endpoint_returns_source_and_chunks() {
        let app = build_router(test_state());

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/knowledge/nodes/ch4-address-space")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let body = String::from_utf8(body.to_vec()).expect("body should be utf8");

        assert!(body.contains("rCore Chapter 4"));
        assert!(body.contains("teaching_context"));
        assert!(body.contains("rCore v3 ch4"));
    }

    #[tokio::test]
    async fn knowledge_neighbors_endpoint_returns_neighbors() {
        let app = build_router(test_state());

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/knowledge/nodes/ch4-address-space/neighbors")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let body = String::from_utf8(body.to_vec()).expect("body should be utf8");

        assert!(body.contains("ch5-process"));
        assert!(body.contains("dependent"));
    }

    #[tokio::test]
    async fn sources_endpoint_returns_source_references() {
        let app = build_router(test_state());

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/sources")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let body = String::from_utf8(body.to_vec()).expect("body should be utf8");

        assert!(body.contains("rcore-v3-ch4"));
        assert!(body.contains("GPL-3.0"));
    }

    #[tokio::test]
    async fn student_profile_endpoint_upserts_profile() {
        let app = build_router(test_state());
        let body = serde_json::json!({
            "display_name": "Alice",
            "learning_goal": "finish rCore ch1-ch8",
            "preferred_depth": "deep"
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::PUT)
                    .uri("/v1/students/student-1/profile")
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let body = String::from_utf8(body.to_vec()).expect("body should be utf8");

        assert!(body.contains("student-1"));
        assert!(body.contains("Alice"));
        assert!(body.contains("deep"));
    }

    #[tokio::test]
    async fn student_progress_endpoint_records_progress() {
        let app = build_router(test_state());
        let body = serde_json::json!({
            "status": "needs_review",
            "mastery_score": 55,
            "notes": "confused by fork and exec"
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::PUT)
                    .uri("/v1/students/student-1/progress/ch5-process")
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let body = String::from_utf8(body.to_vec()).expect("body should be utf8");

        assert!(body.contains("ch5-process"));
        assert!(body.contains("needs_review"));
        assert!(body.contains("55"));
    }

    #[tokio::test]
    async fn student_progress_endpoint_rejects_invalid_score() {
        let app = build_router(test_state());
        let body = serde_json::json!({
            "status": "in_progress",
            "mastery_score": 101
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::PUT)
                    .uri("/v1/students/student-1/progress/ch5-process")
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let body = String::from_utf8(body.to_vec()).expect("body should be utf8");

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(body.contains("invalid_input"));
    }

    #[tokio::test]
    async fn learning_path_endpoint_recommends_next_node() {
        let app = build_router(test_state());

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/students/student-1/learning-path?limit=3")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let body = String::from_utf8(body.to_vec()).expect("body should be utf8");

        assert!(body.contains("ch5-process"));
        assert!(body.contains("recommendations"));
        assert!(body.contains("completed_nodes"));
    }

    #[tokio::test]
    async fn admin_seed_endpoint_is_disabled_by_default() {
        let app = build_router(test_state());

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/v1/admin/knowledge/seed")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn admin_seed_endpoint_imports_builtin_seed_when_enabled() {
        let app = build_router(test_state_with_admin_seed(true));

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/v1/admin/knowledge/seed")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let body = String::from_utf8(body.to_vec()).expect("body should be utf8");

        assert!(body.contains("\"nodes\":8"));
        assert!(body.contains("\"retrieval_chunks\":8"));
    }

    #[tokio::test]
    async fn tutor_chat_uses_mock_gateway() {
        let app = build_router(test_state());
        let body = serde_json::json!({
            "message": "Explain address spaces",
            "knowledge_node_ids": ["ch4-address-space"]
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/v1/tutor/chat")
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let body = String::from_utf8(body.to_vec()).expect("body should be utf8");

        assert!(body.contains("Mock tutor response"));
        assert!(body.contains("rCore v3 ch4"));
        assert!(body.contains("mock_response"));
        assert!(body.contains("source_grounded_context"));
    }

    #[tokio::test]
    async fn tutor_chat_rejects_missing_knowledge_context() {
        let app = build_router(test_state());
        let body = serde_json::json!({
            "message": "Explain a missing node",
            "knowledge_node_ids": ["missing-node"]
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/v1/tutor/chat")
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let body = String::from_utf8(body.to_vec()).expect("body should be utf8");

        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(body.contains("knowledge_context_missing"));
        assert!(!body.contains("Address-space context"));
    }
}
