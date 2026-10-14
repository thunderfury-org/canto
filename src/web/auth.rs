use axum::Json;
use axum::body::{Body, Bytes};
use axum::extract::State;
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};

use crate::web::state::WebState;

pub const COOKIE_NAME: &str = "canto_admin_token";
pub const COOKIE_MAX_AGE_SECS: u64 = 30 * 24 * 3600;

#[derive(Debug, Deserialize)]
pub struct VerifyAuthRequest {
    pub token: Option<String>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct VerifyAuthResponse {
    pub authenticated: bool,
    pub message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusResponse {
    pub status: &'static str,
    pub version: &'static str,
    pub started_at: String,
    pub uptime_secs: u64,
}

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

pub fn extract_token(headers: &HeaderMap) -> Option<String> {
    if let Some(auth_header) = headers.get(header::AUTHORIZATION)
        && let Ok(auth_str) = auth_header.to_str()
    {
        let trimmed = auth_str.trim();
        if let Some(bearer) = trimmed.strip_prefix("Bearer ") {
            return Some(bearer.trim().to_string());
        }
        return Some(trimmed.to_string());
    }

    if let Some(cookie_header) = headers.get(header::COOKIE)
        && let Ok(cookie_str) = cookie_header.to_str()
    {
        let cookie_prefix = format!("{COOKIE_NAME}=");
        for item in cookie_str.split(';') {
            let item = item.trim();
            if let Some(token) = item.strip_prefix(&cookie_prefix) {
                return Some(token.trim().to_string());
            }
        }
    }

    None
}

pub fn is_authorized(headers: &HeaderMap, configured_token: &str) -> bool {
    let configured = configured_token.trim();
    if configured.is_empty() {
        return false;
    }
    if let Some(token) = extract_token(headers) {
        return constant_time_eq(token.as_bytes(), configured.as_bytes());
    }
    false
}

pub async fn require_admin_auth(
    State(state): State<WebState>,
    request: axum::extract::Request,
    next: Next,
) -> Response {
    if !is_authorized(request.headers(), &state.settings.admin_token) {
        return Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                r#"{"error":"Unauthorized: invalid or missing admin token"}"#,
            ))
            .unwrap_or_else(|_| StatusCode::UNAUTHORIZED.into_response());
    }

    next.run(request).await
}

pub async fn handle_status(State(state): State<WebState>) -> Response {
    let resp = StatusResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        started_at: state.started_at_rfc3339.clone(),
        uptime_secs: state.uptime_secs(),
    };
    let json = serde_json::to_string(&resp).unwrap_or_default();
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

pub fn submitted_password(body: &[u8]) -> Option<String> {
    let trimmed = body.trim_ascii();
    if trimmed.is_empty() {
        return None;
    }
    serde_json::from_slice::<VerifyAuthRequest>(trimmed)
        .ok()
        .and_then(|request| request.token)
        .filter(|token| !token.is_empty())
}

pub async fn handle_verify_auth(
    State(state): State<WebState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let configured = state.settings.admin_token.trim();
    if configured.is_empty() {
        return (
            StatusCode::UNAUTHORIZED,
            Json(VerifyAuthResponse {
                authenticated: false,
                message: "Admin token is not configured on server".to_string(),
            }),
        )
            .into_response();
    }

    let candidate = submitted_password(&body).or_else(|| extract_token(&headers));

    if let Some(token) = candidate
        && constant_time_eq(token.as_bytes(), configured.as_bytes())
    {
        let cookie = format!(
            "{COOKIE_NAME}={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age={COOKIE_MAX_AGE_SECS}"
        );
        let resp = Json(VerifyAuthResponse {
            authenticated: true,
            message: "Admin authentication successful".to_string(),
        });
        let mut response = (StatusCode::OK, resp).into_response();
        if let Ok(header_val) = HeaderValue::from_str(&cookie) {
            response
                .headers_mut()
                .insert(header::SET_COOKIE, header_val);
        }
        return response;
    }

    (
        StatusCode::UNAUTHORIZED,
        Json(VerifyAuthResponse {
            authenticated: false,
            message: "Invalid admin token".to_string(),
        }),
    )
        .into_response()
}

pub async fn handle_logout() -> Response {
    let expired_cookie = format!(
        "{COOKIE_NAME}=; Path=/; Expires=Thu, 01 Jan 1970 00:00:00 GMT; HttpOnly; SameSite=Lax"
    );
    let resp = Json(VerifyAuthResponse {
        authenticated: false,
        message: "Logged out".to_string(),
    });
    let mut response = (StatusCode::OK, resp).into_response();
    if let Ok(header_val) = HeaderValue::from_str(&expired_cookie) {
        response
            .headers_mut()
            .insert(header::SET_COOKIE, header_val);
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constant_time_eq() {
        assert!(constant_time_eq(b"secret", b"secret"));
        assert!(!constant_time_eq(b"secret", b"secret1"));
        assert!(!constant_time_eq(b"secret1", b"secret"));
        assert!(!constant_time_eq(b"secret", b"wrong!"));
    }

    #[test]
    fn test_extract_token() {
        let mut headers = HeaderMap::new();
        assert_eq!(extract_token(&headers), None);

        headers.insert(header::AUTHORIZATION, "Bearer tok_123".parse().unwrap());
        assert_eq!(extract_token(&headers), Some("tok_123".to_string()));

        headers.insert(header::AUTHORIZATION, "raw_tok_456".parse().unwrap());
        assert_eq!(extract_token(&headers), Some("raw_tok_456".to_string()));

        headers.remove(header::AUTHORIZATION);
        headers.insert(
            header::COOKIE,
            "session=abc; canto_admin_token=cookie_tok; other=1"
                .parse()
                .unwrap(),
        );
        assert_eq!(extract_token(&headers), Some("cookie_tok".to_string()));
    }

    #[test]
    fn test_is_authorized() {
        let mut headers = HeaderMap::new();
        assert!(!is_authorized(&headers, "my_token"));
        assert!(!is_authorized(&headers, ""));

        headers.insert(header::AUTHORIZATION, "Bearer my_token".parse().unwrap());
        assert!(is_authorized(&headers, "my_token"));
        assert!(!is_authorized(&headers, "wrong_token"));
        assert!(!is_authorized(&headers, ""));
    }

    #[test]
    fn test_empty_verify_body_is_not_a_submitted_password() {
        assert_eq!(submitted_password(b""), None);
        assert_eq!(submitted_password(b"{}"), None);
        assert_eq!(submitted_password(b"{\n}"), None);
    }

    #[tokio::test]
    async fn test_empty_verify_body_uses_cookie() {
        let dir = std::env::temp_dir().join(format!("canto-auth-empty-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let settings = crate::config::WebSettings {
            admin_token: "secret".to_string(),
            ..Default::default()
        };
        let state = WebState::new(settings, dir.clone());
        let app = crate::web::router::create_app(state);

        let empty = axum::http::Request::builder()
            .method("POST")
            .uri("/api/auth/verify")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::empty())
            .unwrap();
        let response = tower::ServiceExt::oneshot(app.clone(), empty)
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        let cookie = axum::http::Request::builder()
            .method("POST")
            .uri("/api/auth/verify")
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::COOKIE, "canto_admin_token=secret")
            .body(Body::empty())
            .unwrap();
        let response = tower::ServiceExt::oneshot(app, cookie).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let set_cookie = response
            .headers()
            .get(header::SET_COOKIE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("");
        assert!(set_cookie.starts_with("canto_admin_token=secret;"));
        assert!(set_cookie.contains("HttpOnly"));
        assert!(set_cookie.contains("SameSite=Lax"));
        assert!(!set_cookie.contains("Secure"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn test_submitted_password_reads_only_a_present_token() {
        assert_eq!(
            submitted_password(br#"{"token":"secret"}"#),
            Some("secret".to_string())
        );
        assert_eq!(submitted_password(br#"{"token":""}"#), None);
        assert_eq!(submitted_password(b"not-json"), None);
    }
}
