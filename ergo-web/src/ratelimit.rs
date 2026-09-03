//! Per-client rate limiting for a public instance: a token bucket per IP
//! over the engine routes. Off unless `RATE_LIMIT_PER_MINUTE` is set — a
//! self-hosted instance behind a reverse proxy that already limits does not
//! need two of them. Health and the static UI are never limited.
//!
//! The client key is the peer address, or the LAST `X-Forwarded-For` entry
//! when `TRUST_PROXY` is set (only the nearest proxy's appended entry can be
//! trusted; anything earlier is client-supplied). Buckets are in memory and
//! per process — this is a courtesy limiter, not an accounting system.

use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use axum::{
    extract::{ConnectInfo, Request, State},
    http::{header, HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};

use crate::app::AppState;

/// Token buckets per client. `per_minute` is both the refill rate and the
/// burst size, which is the simplest contract to explain in a README.
pub struct RateLimiter {
    per_minute: u32,
    buckets: Mutex<HashMap<IpAddr, (f64, Instant)>>,
}

impl RateLimiter {
    pub fn new(per_minute: u32) -> Self {
        RateLimiter {
            per_minute: per_minute.max(1),
            buckets: Mutex::new(HashMap::new()),
        }
    }

    /// Take one token for `who`; `Err(seconds)` when empty, with the wait
    /// until the next token.
    pub fn take(&self, who: IpAddr) -> Result<(), u32> {
        let now = Instant::now();
        let rate = self.per_minute as f64 / 60.0;
        let cap = self.per_minute as f64;
        let mut buckets = self.buckets.lock().unwrap_or_else(|e| e.into_inner());
        // Keep the map bounded on a long-running public instance.
        if buckets.len() > 10_000 {
            buckets.retain(|_, (_, last)| now.duration_since(*last).as_secs() < 120);
        }
        let entry = buckets.entry(who).or_insert((cap, now));
        let elapsed = now.duration_since(entry.1).as_secs_f64();
        entry.0 = (entry.0 + elapsed * rate).min(cap);
        entry.1 = now;
        if entry.0 >= 1.0 {
            entry.0 -= 1.0;
            Ok(())
        } else {
            Err(((1.0 - entry.0) / rate).ceil().max(1.0) as u32)
        }
    }
}

/// The client key for this request (see the module doc).
fn client_ip(req: &Request, trust_proxy: bool) -> Option<IpAddr> {
    if trust_proxy {
        if let Some(v) = req
            .headers()
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
        {
            if let Some(last) = v.split(',').next_back() {
                if let Ok(ip) = last.trim().parse() {
                    return Some(ip);
                }
            }
        }
    }
    req.extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|c| c.0.ip())
}

/// Middleware for the engine routes. A request from a client over budget
/// gets a JSON 429 with `Retry-After`; when the limiter is off, or the
/// client cannot be identified, the request passes.
pub async fn limit(State(state): State<Arc<AppState>>, req: Request, next: Next) -> Response {
    let Some(limiter) = state.limiter.as_ref() else {
        return next.run(req).await;
    };
    let Some(ip) = client_ip(&req, state.cfg.trust_proxy) else {
        return next.run(req).await;
    };
    match limiter.take(ip) {
        Ok(()) => next.run(req).await,
        Err(wait) => {
            let mut resp = (
                StatusCode::TOO_MANY_REQUESTS,
                Json(serde_json::json!({
                    "error": { "code": "rate_limited",
                               "message": format!("over the per-client budget; retry in {wait}s") }
                })),
            )
                .into_response();
            resp.headers_mut().insert(
                header::RETRY_AFTER,
                HeaderValue::from_str(&wait.to_string()).unwrap_or(HeaderValue::from_static("1")),
            );
            resp
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bucket_allows_a_burst_then_refuses_with_a_wait() {
        let l = RateLimiter::new(2);
        let ip: IpAddr = "10.0.0.1".parse().unwrap();
        assert!(l.take(ip).is_ok());
        assert!(l.take(ip).is_ok());
        let wait = l.take(ip).unwrap_err();
        assert!((1..=30).contains(&wait), "{wait}");
        // Another client has its own bucket.
        assert!(l.take("10.0.0.2".parse().unwrap()).is_ok());
    }
}
