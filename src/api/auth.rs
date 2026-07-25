use axum::{
    body::Body,
    extract::Request,
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};

/// Environment variable holding the bearer token required for mutating operations.
pub const API_TOKEN_ENV: &str = "ATROPOS_API_TOKEN";

/// Returns the configured API token, if any.
pub fn configured_token() -> Option<String> {
    match std::env::var(API_TOKEN_ENV) {
        Ok(t) if !t.is_empty() => Some(t),
        _ => None,
    }
}

/// Axum middleware enforcing a static bearer token on protected routes.
///
/// If `ATROPOS_API_TOKEN` is unset the middleware allows all requests (developer
/// mode), so local development and existing integration tests keep working. A
/// startup warning is emitted in that case. When the variable is set, requests
/// must carry `Authorization: Bearer <token>` or they receive `401 Unauthorized`.
pub async fn require_bearer_token(req: Request<Body>, next: Next) -> Result<Response, StatusCode> {
    let expected = match configured_token() {
        Some(t) => t,
        None => return Ok(next.run(req).await),
    };

    let provided = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));

    match provided {
        Some(token) if constant_time_eq(token.as_bytes(), expected.as_bytes()) => {
            Ok(next.run(req).await)
        }
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}

/// Length-independent, constant-time byte comparison to avoid leaking the token
/// via response-timing side channels.
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}
