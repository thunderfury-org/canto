use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use axum::response::IntoResponse;
use axum::routing::get;
use canto::config::WebSettings;
use canto::web::{WebState, create_app};
use serde_json::{Value, json};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tower::ServiceExt;

#[tokio::test]
async fn test_public_static_and_spa_fallback() {
    let settings = WebSettings {
        enabled: true,
        listen: "127.0.0.1:5800".to_string(),
        admin_token: "test_secret_token".to_string(),
        public_url: None,
    };
    let (state, _dir) = test_state(settings);
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
    let (state, _dir) = test_state(settings);
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

    // 6. Nonexistent /api/* endpoint without auth -> 401 Unauthorized (protected API subtree)
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/unknown_endpoint")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // 7. Nonexistent /api/* endpoint with auth -> 404 JSON, NOT SPA index.html fallback
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/unknown_endpoint")
                .header(header::AUTHORIZATION, "Bearer test_secret_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .unwrap()
        .to_str()
        .unwrap();
    assert!(content_type.contains("application/json"));

    // 8. POST /api/auth/verify with valid token -> 200 OK and Set-Cookie
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

    let server = WebServer::new(settings, temp_work_dir());
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

fn temp_work_dir() -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "canto_web_test_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&path).unwrap();
    path
}

fn test_settings() -> WebSettings {
    WebSettings {
        enabled: true,
        listen: "127.0.0.1:5800".to_string(),
        admin_token: "test_secret_token".to_string(),
        public_url: None,
    }
}

fn test_state(settings: WebSettings) -> (WebState, PathBuf) {
    let dir = temp_work_dir();
    (WebState::new(settings, dir.clone()), dir)
}

fn auth_req(method: &str, uri: &str, body: Body) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, "Bearer test_secret_token")
        .header(header::CONTENT_TYPE, "application/json")
        .body(body)
        .unwrap()
}

async fn json_body(response: axum::http::Response<Body>) -> (StatusCode, Value) {
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes)
            .unwrap_or(Value::String(String::from_utf8_lossy(&bytes).into_owned()))
    };
    (status, value)
}

#[derive(Clone)]
struct MockFeed {
    inner: Arc<Mutex<(StatusCode, String)>>,
}

async fn spawn_feed_server(initial_body: &str) -> (String, MockFeed) {
    let feed = MockFeed {
        inner: Arc::new(Mutex::new((StatusCode::OK, initial_body.to_string()))),
    };
    let app = Router::new()
        .route("/sub", get(mock_feed_handler))
        .with_state(feed.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (format!("http://{addr}/sub"), feed)
}

async fn mock_feed_handler(
    axum::extract::State(feed): axum::extract::State<MockFeed>,
) -> axum::response::Response {
    let (status, body) = feed.inner.lock().unwrap().clone();
    (status, body).into_response()
}

#[tokio::test]
async fn test_sources_crud_manual_and_persistence() {
    let (state, dir) = test_state(test_settings());
    let app = create_app(state);

    let unauthorized = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/sources")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);

    let (status, listed) = json_body(
        app.clone()
            .oneshot(auth_req("GET", "/api/sources", Body::empty()))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(listed, json!([]));

    let create_body = json!({
        "name": "自建节点",
        "type": "manual",
        "content": "vless://11111111-1111-1111-1111-111111111111@hk.example.com:443?type=ws&security=tls&sni=hk.example.com&path=/ws#HK-VLESS"
    });
    let (status, created) = json_body(
        app.clone()
            .oneshot(auth_req(
                "POST",
                "/api/sources",
                Body::from(create_body.to_string()),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(created["name"], "自建节点");
    assert_eq!(created["type"], "manual");
    assert_eq!(created["status"], "active");
    assert_eq!(created["nodeCount"], 1);
    assert_eq!(created["nodes"][0]["type"], "vless");
    assert_eq!(created["nodes"][0]["tag"], "HK-VLESS");
    let id = created["id"].as_str().unwrap().to_string();

    let persisted = std::fs::read_to_string(dir.join("studio").join("sources.json")).unwrap();
    assert!(persisted.contains("HK-VLESS"));
    assert!(persisted.contains(&id));

    let update_body = json!({
        "name": "自建节点-改",
        "content": "trojan://secret@tw.example.com:443?security=tls&sni=tw.example.com#TW-Trojan"
    });
    let (status, updated) = json_body(
        app.clone()
            .oneshot(auth_req(
                "PUT",
                &format!("/api/sources/{id}"),
                Body::from(update_body.to_string()),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(updated["name"], "自建节点-改");
    assert_eq!(updated["nodes"][0]["type"], "trojan");
    assert_eq!(updated["nodes"][0]["tag"], "TW-Trojan");

    let delete_res = app
        .clone()
        .oneshot(auth_req(
            "DELETE",
            &format!("/api/sources/{id}"),
            Body::empty(),
        ))
        .await
        .unwrap();
    assert_eq!(delete_res.status(), StatusCode::NO_CONTENT);

    let (status, listed) = json_body(
        app.oneshot(auth_req("GET", "/api/sources", Body::empty()))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(listed, json!([]));
}

#[tokio::test]
async fn test_sources_parse_mocked_subscription_and_keep_last_good() {
    let encoded = "dm1lc3M6Ly9leUoySWpvaU1pSXNJbkJ6SWpvaVZrMWxjM010TVNJc0ltRmtaQ0k2SW1ockxtVjRZVzF3YkdVdVkyOXRJaXdpY0c5eWRDSTZJalEwTXlJc0ltbGtJam9pTVRFeE1URXhNVEV0TVRFeE1TMHhNVEV4TFRFeE1URXRNVEV4TVRFeE1URXhNVEV4SWl3aVlXbGtJam9pTUNJc0ltNWxkQ0k2SW5keklpd2lkSGx3WlNJNkltNXZibVVpTENKMGJITWlPaUowYkhNaUxDSnpibWtpT2lKb2F5NWxlR0Z0Y0d4bExtTnZiU0o5CnNzOi8vWVdWekxUSTFOaTFuWTIwNmNHRnpjM2R2Y21RQDE5Mi4xNjguMTAwLjE6ODM4OCNTUy0xCnZsZXNzOi8vMTExMTExMTEtMTExMS0xMTExLTExMTEtMTExMTExMTExMTExQGhrLmV4YW1wbGUuY29tOjQ0Mz90eXBlPXRjcCZzZWN1cml0eT10bHMmc25pPWhrLmV4YW1wbGUuY29tI1ZMRVNTLTEKdHJvamFuOi8vc2VjcmV0QHR3LmV4YW1wbGUuY29tOjQ0Mz9zZWN1cml0eT10bHMmc25pPXR3LmV4YW1wbGUuY29tI1Ryb2phbi0xCmh5c3RlcmlhMjovL2h5MnBhc3NAanAuZXhhbXBsZS5jb206ODQ0Mz9zbmk9anAuZXhhbXBsZS5jb20jSFkyLTEK";
    let (url, feed) = spawn_feed_server(encoded).await;
    let (state, dir) = test_state(test_settings());
    let app = create_app(state);

    let create_body = json!({
        "name": "机场订阅",
        "type": "subscription",
        "url": url
    });
    let (status, created) = json_body(
        app.clone()
            .oneshot(auth_req(
                "POST",
                "/api/sources",
                Body::from(create_body.to_string()),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    assert_eq!(created["status"], "active");
    assert_eq!(created["nodeCount"], 5);
    let types: Vec<&str> = created["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["type"].as_str().unwrap())
        .collect();
    assert_eq!(
        types,
        ["vmess", "shadowsocks", "vless", "trojan", "hysteria2"]
    );
    let id = created["id"].as_str().unwrap().to_string();

    let persisted = std::fs::read_to_string(dir.join("studio").join("sources.json")).unwrap();
    assert!(persisted.contains("机场订阅"));
    assert!(persisted.contains("HY2-1"));

    let json_feed = json!([
        {"type":"vless","tag":"native-only","server":"a.example","server_port":443,"uuid":"u"},
        {"type":"direct","tag":"direct"}
    ])
    .to_string();
    *feed.inner.lock().unwrap() = (StatusCode::OK, json_feed);

    let (status, refreshed) = json_body(
        app.clone()
            .oneshot(auth_req(
                "POST",
                &format!("/api/sources/{id}/refresh"),
                Body::empty(),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(refreshed["nodeCount"], 1);
    assert_eq!(refreshed["nodes"][0]["tag"], "native-only");

    *feed.inner.lock().unwrap() = (StatusCode::BAD_GATEWAY, "upstream down".to_string());
    let (status, failed) = json_body(
        app.clone()
            .oneshot(auth_req(
                "POST",
                &format!("/api/sources/{id}/refresh"),
                Body::empty(),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(failed["status"], "error");
    assert_eq!(failed["nodeCount"], 1);
    assert_eq!(failed["nodes"][0]["tag"], "native-only");
    assert!(
        failed["lastError"].as_str().unwrap().contains("HTTP 502"),
        "{failed}"
    );

    let disk: Value = serde_json::from_str(
        &std::fs::read_to_string(dir.join("studio").join("sources.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(disk["sources"][0]["nodes"][0]["tag"], "native-only");
    assert_eq!(disk["sources"][0]["status"], "error");
}
