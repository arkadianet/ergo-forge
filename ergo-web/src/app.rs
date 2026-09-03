//! Router assembly. Separate from `main` so integration tests build the same
//! app without going through binary startup.

use std::sync::Arc;

use axum::{extract::DefaultBodyLimit, routing::get, routing::post, Router};
use tokio::sync::Semaphore;
use tower::limit::GlobalConcurrencyLimitLayer;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;

/// Largest accepted request body. Real inputs are a few KiB.
pub const MAX_BODY_BYTES: usize = 64 * 1024;

/// Engine requests (inspect, hunt, eval) in flight at once, one shared budget.
/// Each one holds a large-stack thread, so this is also the bound on those
/// threads. Excess requests queue rather than fail. Scoped to the engine
/// routes only: health checks and the static UI must stay answerable while
/// the engine is saturated.
pub const MAX_ENGINE_IN_FLIGHT: usize = 64;

/// The complete application router.
pub fn router() -> Router {
    // One semaphore shared by every engine route (a per-route layer would
    // give each its own budget).
    let engine_limit =
        GlobalConcurrencyLimitLayer::with_semaphore(Arc::new(Semaphore::new(MAX_ENGINE_IN_FLIGHT)));
    Router::new()
        .route("/api/v1/health", get(crate::routes::health::health))
        .route(
            "/api/v1/inspect",
            post(crate::routes::inspect::inspect).layer(engine_limit.clone()),
        )
        .route(
            "/api/v1/hunt",
            post(crate::routes::hunt::hunt_route).layer(engine_limit.clone()),
        )
        .route(
            "/api/v1/eval",
            post(crate::routes::eval::eval_route).layer(engine_limit.clone()),
        )
        .route(
            "/api/v1/compile",
            post(crate::routes::compile::compile_route).layer(engine_limit.clone()),
        )
        .route(
            "/api/v1/test",
            post(crate::routes::test::test_route).layer(engine_limit),
        )
        .route("/api/v1/examples", get(crate::routes::examples::list))
        .route(
            "/api/v1/examples/{*id}",
            get(crate::routes::examples::fetch),
        )
        .fallback_service(ServeDir::new(
            std::env::var("UI_DIR").unwrap_or_else(|_| "ui".into()),
        ))
        // The limit is enforced inside the `Json` extractor, so the rejection
        // flows through `ApiJson` and comes back as JSON.
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .layer(TraceLayer::new_for_http())
}
