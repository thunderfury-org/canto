use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use canto::config::WebSettings;
use canto::web::{WebState, create_app};
use tower::ServiceExt;

#[tokio::test]
async fn test_public_static_and_spa_fallback() {
    let settings = WebSettings {
        enabled: true,
        listen: "127.0.0.1:5800".to_string(),
        admin_token: "test_secret_token".to_string(),
        public_url: None,
    };
    let state = WebState::new(settings);
    let app = create_app(state);

    // 1. GET / serves index.html
    let response = app
        .clone()
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .unwrap()
        .to_str()
        .unwrap();
    assert!(content_type.contains("text/html"));
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8_lossy(&body);
    assert!(
        body_str.contains("canto")
            || body_str.contains("<html")
            || body_str.contains("<!doctype html>")
            || body_str.contains("<!DOCTYPE html>")
    );

    // 2. GET /favicon.svg serves favicon
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/favicon.svg")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .unwrap()
        .to_str()
        .unwrap();
    assert!(content_type.contains("image/svg+xml"));

    // 3. SPA fallback: GET /templates serves index.html
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/templates")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .unwrap()
        .to_str()
        .unwrap();
    assert!(content_type.contains("text/html"));
}

#[tokio::test]
async fn test_protected_api_auth_checks() {
    let settings = WebSettings {
        enabled: true,
        listen: "127.0.0.1:5800".to_string(),
        admin_token: "test_secret_token".to_string(),
        public_url: None,
    };
    let state = WebState::new(settings);
    let app = create_app(state);

    // 1. No auth -> 401 Unauthorized
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // 2. Wrong token -> 401 Unauthorized
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/status")
                .header(header::AUTHORIZATION, "Bearer wrong_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // 3. Valid Authorization Bearer header -> 200 OK
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/status")
                .header(header::AUTHORIZATION, "Bearer test_secret_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8_lossy(&body);
    assert!(body_str.contains(r#""status":"ok""#));

    // 4. Valid session cookie -> 200 OK
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/status")
                .header(header::COOKIE, "canto_admin_token=test_secret_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // 5. POST /api/auth/verify with wrong token -> 401
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/verify")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"token":"wrong"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // 6. POST /api/auth/verify with valid token -> 200 OK and Set-Cookie
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/verify")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"token":"test_secret_token"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let set_cookie = response
        .headers()
        .get(header::SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap();
    assert!(set_cookie.contains("canto_admin_token=test_secret_token"));
}

#[tokio::test]
async fn test_server_startup_and_graceful_shutdown() {
    use canto::web::WebServer;
    use std::time::Duration;

    let settings = WebSettings {
        enabled: true,
        listen: "127.0.0.1:0".to_string(),
        admin_token: "test_secret_token".to_string(),
        public_url: None,
    };

    let server = WebServer::new(settings);
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    let server_task = tokio::spawn(async move {
        server
            .run_with_signal(async {
                let _ = shutdown_rx.await;
            })
            .await
    });

    tokio::time::sleep(Duration::from_millis(100)).await;
    let _ = shutdown_tx.send(());

    let res = tokio::time::timeout(Duration::from_secs(3), server_task).await;
    assert!(res.is_ok(), "Server did not shut down within timeout");
    let inner_res = res.unwrap().unwrap();
    assert!(inner_res.is_ok());
}
