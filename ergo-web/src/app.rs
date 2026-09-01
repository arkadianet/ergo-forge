//! Router assembly. Separate from `main` so integration tests build the same
//! app without going through binary startup.

use axum::{routing::get, routing::post, Router};
use tower::limit::ConcurrencyLimitLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;

/// The complete application router.
pub fn router() -> Router {
    Router::new()
        .route("/api/v1/health", get(crate::routes::health::health))
        .route("/api/v1/inspect", post(crate::routes::inspect::inspect))
        .fallback_service(ServeDir::new(
            std::env::var("UI_DIR").unwrap_or_else(|_| "ui".into()),
        ))
        .layer(RequestBodyLimitLayer::new(64 * 1024))
        .layer(ConcurrencyLimitLayer::new(64))
        .layer(TraceLayer::new_for_http())
}
