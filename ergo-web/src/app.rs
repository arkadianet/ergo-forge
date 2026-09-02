//! Router assembly. Separate from `main` so integration tests build the same
//! app without going through binary startup.

use axum::{extract::DefaultBodyLimit, routing::get, routing::post, Router};
use tower::limit::ConcurrencyLimitLayer;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;

/// Largest accepted request body. Real inputs are a few KiB.
pub const MAX_BODY_BYTES: usize = 64 * 1024;

/// Inspect requests in flight at once. Each one holds a large-stack thread,
/// so this is also the bound on those threads. Excess requests queue rather
/// than fail. Scoped to the inspect route only: health checks and the static
/// UI must stay answerable while inspection is saturated.
pub const MAX_INSPECT_IN_FLIGHT: usize = 64;

/// The complete application router.
pub fn router() -> Router {
    Router::new()
        .route("/api/v1/health", get(crate::routes::health::health))
        .route(
            "/api/v1/inspect",
            post(crate::routes::inspect::inspect)
                .layer(ConcurrencyLimitLayer::new(MAX_INSPECT_IN_FLIGHT)),
        )
        .fallback_service(ServeDir::new(
            std::env::var("UI_DIR").unwrap_or_else(|_| "ui".into()),
        ))
        // The limit is enforced inside the `Json` extractor, so the rejection
        // flows through `ApiJson` and comes back as JSON.
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .layer(TraceLayer::new_for_http())
}
