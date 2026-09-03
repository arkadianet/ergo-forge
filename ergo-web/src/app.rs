//! Router assembly. Separate from `main` so integration tests build the same
//! app without going through binary startup.

use std::sync::Arc;

use axum::{extract::DefaultBodyLimit, routing::get, routing::post, Router};
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;

use crate::engine::EngineBudget;

/// Largest accepted request body. Real inputs are a few KiB.
pub const MAX_BODY_BYTES: usize = 64 * 1024;

/// Engine jobs on the blocking pool at once, one shared budget. Each one
/// holds a large-stack thread, so this is also the bound on those threads.
/// Excess requests wait for a permit rather than fail. The permit lives
/// inside the blocking job (`crate::engine`), so a client that disconnects
/// does not release it early. Health checks and the static UI never take a
/// permit.
pub const MAX_ENGINE_IN_FLIGHT: usize = 64;

/// Shared state: configuration and the engine budget.
pub struct AppState {
    pub cfg: AppConfig,
    pub engine: EngineBudget,
}

/// Runtime configuration. `explorer_url` is the ONE outbound dependency the
/// service can have; `None` (the default) keeps the "nothing leaves this
/// host" promise and turns `/api/v1/lookup` into a 501.
#[derive(Debug, Clone, Default)]
pub struct AppConfig {
    /// Base URL of an Ergo explorer API (e.g. `https://api.ergoplatform.com`).
    pub explorer_url: Option<String>,
    /// Static folder for non-API paths.
    pub ui_dir: Option<String>,
}

impl AppConfig {
    /// From the environment: `EXPLORER_URL`, `UI_DIR`.
    pub fn from_env() -> Self {
        AppConfig {
            explorer_url: std::env::var("EXPLORER_URL")
                .ok()
                .map(|u| u.trim_end_matches('/').to_string())
                .filter(|u| !u.is_empty()),
            ui_dir: std::env::var("UI_DIR").ok(),
        }
    }
}

/// The complete application router, configured from the environment.
pub fn router() -> Router {
    router_with(AppConfig::from_env())
}

/// The complete application router with an explicit configuration.
pub fn router_with(cfg: AppConfig) -> Router {
    let state = Arc::new(AppState {
        engine: EngineBudget::new(MAX_ENGINE_IN_FLIGHT),
        cfg,
    });
    Router::new()
        .route("/api/v1/health", get(crate::routes::health::health))
        .route("/api/v1/inspect", post(crate::routes::inspect::inspect))
        .route("/api/v1/hunt", post(crate::routes::hunt::hunt_route))
        .route("/api/v1/eval", post(crate::routes::eval::eval_route))
        .route(
            "/api/v1/compile",
            post(crate::routes::compile::compile_route),
        )
        .route("/api/v1/test", post(crate::routes::test::test_route))
        .route("/api/v1/examples", get(crate::routes::examples::list))
        .route(
            "/api/v1/examples/{*id}",
            get(crate::routes::examples::fetch),
        )
        .route("/api/v1/config", get(crate::routes::lookup::config))
        .route("/api/v1/lookup", post(crate::routes::lookup::lookup))
        .fallback_service(ServeDir::new(
            state.cfg.ui_dir.clone().unwrap_or_else(|| "ui".into()),
        ))
        .with_state(state)
        // The limit is enforced inside the `Json` extractor, so the rejection
        // flows through `ApiJson` and comes back as JSON.
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .layer(TraceLayer::new_for_http())
}
