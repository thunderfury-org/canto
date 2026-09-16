use axum::body::Body;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "web/dist/"]
pub struct WebAssets;

pub async fn static_handler(uri: axum::http::Uri) -> Response {
    let raw_path = uri.path();
    let path = raw_path.trim_start_matches('/');

    // API routes must never fall back to SPA index.html
    if path.starts_with("api/") || path == "api" {
        return StatusCode::NOT_FOUND.into_response();
    }

    if path.is_empty() || path == "index.html" {
        return serve_asset("index.html").unwrap_or_else(|| StatusCode::NOT_FOUND.into_response());
    }

    if let Some(resp) = serve_asset(path) {
        return resp;
    }

    // SPA fallback: client-side routes without file extension serve index.html
    if !path.contains('.') {
        return serve_asset("index.html").unwrap_or_else(|| StatusCode::NOT_FOUND.into_response());
    }

    StatusCode::NOT_FOUND.into_response()
}

fn serve_asset(path: &str) -> Option<Response> {
    WebAssets::get(path).map(|file| {
        let mime = mime_guess::from_path(path).first_or_octet_stream();
        Response::builder()
            .status(StatusCode::OK)
            .header(
                header::CONTENT_TYPE,
                HeaderValue::from_str(mime.as_ref())
                    .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")),
            )
            .body(Body::from(file.data))
            .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
    })
}
