use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};

use crate::web::state::WebState;

#[derive(Debug, Deserialize)]
pub struct VerifyAuthRequest {
    pub token: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct VerifyAuthResponse {
    pub authenticated: bool,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct StatusResponse {
    pub status: &'static str,
    pub version: &'static str,
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
        for item in cookie_str.split(';') {
            let item = item.trim();
            if let Some(token) = item.strip_prefix("canto_admin_token=") {
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
        return token == configured;
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

pub async fn handle_status() -> Response {
    let resp = StatusResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    };
    let json = serde_json::to_string(&resp).unwrap_or_default();
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

pub async fn handle_verify_auth(
    State(state): State<WebState>,
    headers: HeaderMap,
    body: Option<axum::Json<VerifyAuthRequest>>,
) -> Response {
    let configured = state.settings.admin_token.trim();
    if configured.is_empty() {
        return Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                r#"{"authenticated":false,"message":"Admin token is not configured on server"}"#,
            ))
            .unwrap_or_else(|_| StatusCode::UNAUTHORIZED.into_response());
    }

    let candidate = body
        .and_then(|b| b.token.clone())
        .or_else(|| extract_token(&headers));

    if let Some(token) = candidate
        && token == configured
    {
        let cookie = format!(
            "canto_admin_token={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}",
            30 * 24 * 3600
        );
        return Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/json")
            .header(
                header::SET_COOKIE,
                HeaderValue::from_str(&cookie).unwrap_or_else(|_| HeaderValue::from_static("")),
            )
            .body(Body::from(
                r#"{"authenticated":true,"message":"Admin authentication successful"}"#,
            ))
            .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response());
    }

    Response::builder()
        .status(StatusCode::UNAUTHORIZED)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            r#"{"authenticated":false,"message":"Invalid admin token"}"#,
        ))
        .unwrap_or_else(|_| StatusCode::UNAUTHORIZED.into_response())
}

pub async fn handle_logout() -> Response {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .header(
            header::SET_COOKIE,
            HeaderValue::from_static(
                "canto_admin_token=; Path=/; Expires=Thu, 01 Jan 1970 00:00:00 GMT; HttpOnly; SameSite=Lax",
            ),
        )
        .body(Body::from(r#"{"authenticated":false,"message":"Logged out"}"#))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}
