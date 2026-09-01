//! Integration tests over a real server on an ephemeral port.

use std::net::SocketAddr;

/// Start the app on a random free port; returns its base URL.
async fn spawn() -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, ergo_web::app::router())
            .await
            .unwrap();
    });
    format!("http://{addr}")
}

#[tokio::test]
async fn health_returns_ok() {
    let base = spawn().await;
    let body = reqwest::get(format!("{base}/api/v1/health"))
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(body.contains("\"status\":\"ok\""), "body: {body}");
}
