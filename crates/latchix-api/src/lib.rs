#![forbid(unsafe_code)]

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;
use tower_http::trace::TraceLayer;

/// Shared state for health endpoints.
#[derive(Clone, Debug)]
pub struct AppState {
    version: &'static str,
    ready: bool,
}

impl AppState {
    /// Creates health endpoint state.
    #[must_use]
    pub const fn new(version: &'static str, ready: bool) -> Self {
        Self { version, ready }
    }

    /// Returns the server version.
    #[must_use]
    pub const fn version(&self) -> &'static str {
        self.version
    }

    /// Returns whether the process considers itself ready.
    #[must_use]
    pub const fn is_ready(&self) -> bool {
        self.ready
    }
}

/// Response body for health and readiness probes.
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    status: &'static str,
    version: &'static str,
}

/// Builds the HTTP router.
///
/// Phase 0 exposes health endpoints only. The tenant-scoped control plane
/// arrives in Phase 3.
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn healthz(State(state): State<AppState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: state.version(),
    })
}

async fn readyz(State(state): State<AppState>) -> (StatusCode, Json<HealthResponse>) {
    if state.is_ready() {
        (
            StatusCode::OK,
            Json(HealthResponse {
                status: "ready",
                version: state.version(),
            }),
        )
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(HealthResponse {
                status: "not_ready",
                version: state.version(),
            }),
        )
    }
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    use super::{AppState, router};

    #[tokio::test]
    async fn healthz_is_ok() {
        let response = router(AppState::new("test", false))
            .oneshot(
                Request::builder()
                    .uri("/healthz")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        let body = response
            .into_body()
            .collect()
            .await
            .expect("body")
            .to_bytes();
        assert!(body.windows(4).any(|window| window == b"\"ok\""));
    }

    #[tokio::test]
    async fn readyz_reports_not_ready() {
        let response = router(AppState::new("test", false))
            .oneshot(
                Request::builder()
                    .uri("/readyz")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn readyz_reports_ready() {
        let response = router(AppState::new("test", true))
            .oneshot(
                Request::builder()
                    .uri("/readyz")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
    }
}
