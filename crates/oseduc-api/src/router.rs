use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use oseduc_core::{SafetyFlag, TutorChatRequest, TutorResponse};
use oseduc_llm::{LlmError, LlmGateway};
use serde::Serialize;

use crate::PublicConfig;

#[derive(Clone)]
pub struct AppState {
    pub gateway: LlmGateway,
    pub public_config: PublicConfig,
}

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/v1/config/public", get(public_config))
        .route("/v1/tutor/chat", post(tutor_chat))
        .with_state(state)
}

async fn healthz() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

async fn public_config(State(state): State<AppState>) -> Json<PublicConfig> {
    Json(state.public_config)
}

async fn tutor_chat(
    State(state): State<AppState>,
    Json(request): Json<TutorChatRequest>,
) -> Result<Json<TutorResponse>, AppError> {
    let response = state.gateway.chat(request).await?;
    Ok(Json(response))
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{to_bytes, Body},
        http::{Method, Request},
    };
    use oseduc_llm::LlmConfig;
    use tower::ServiceExt;

    fn test_state() -> AppState {
        let config = LlmConfig::from_getter(|_| None).expect("mock config should load");
        AppState {
            gateway: LlmGateway::mock(),
            public_config: PublicConfig::from(&config),
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
    }

    #[tokio::test]
    async fn tutor_chat_uses_mock_gateway() {
        let app = build_router(test_state());
        let body = serde_json::json!({
            "message": "Explain trap handling",
            "knowledge_node_ids": ["trap"]
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
        assert!(body.contains("mock_response"));
    }
}
