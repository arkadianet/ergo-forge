//! Router assembly. Separate from `main` so integration tests build the same
//! app without going through binary startup.

use axum::{routing::get, routing::post, Router};

/// The complete application router.
pub fn router() -> Router {
    Router::new()
        .route("/api/v1/health", get(crate::routes::health::health))
        .route("/api/v1/inspect", post(crate::routes::inspect::inspect))
}
