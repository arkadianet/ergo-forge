//! ergo-web binary. Config comes from the environment; there is no config file.

use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let bind: SocketAddr = std::env::var("BIND_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:8080".to_string())
        .parse()?;

    let listener = tokio::net::TcpListener::bind(bind).await?;
    tracing::info!("listening on {bind}");

    let cfg = ergo_web::app::AppConfig::from_env();
    match &cfg.explorer_url {
        Some(u) => tracing::info!("chain lookups enabled via {u}"),
        None => tracing::info!("chain lookups disabled (no EXPLORER_URL); no outbound calls"),
    }
    match cfg.rate_limit_per_minute {
        Some(n) => tracing::info!(
            "rate limiting engine routes at {n}/min per client (trust_proxy={})",
            cfg.trust_proxy
        ),
        None => tracing::info!("no rate limiting (leave it to the reverse proxy)"),
    }
    axum::serve(
        listener,
        ergo_web::app::router_with(cfg).into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;
    Ok(())
}

/// Resolve on SIGTERM or Ctrl-C so deploys do not cut live requests.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c().await.ok();
    };
    #[cfg(unix)]
    let term = async {
        if let Ok(mut s) = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            s.recv().await;
        }
    };
    #[cfg(not(unix))]
    let term = std::future::pending::<()>();
    tokio::select! { _ = ctrl_c => {}, _ = term => {} }
    tracing::info!("shutdown signal received");
}
