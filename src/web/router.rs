use axum::Router;
use axum::body::Body;
use axum::http::{StatusCode, header};
use axum::middleware;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use tower_http::cors::{Any, CorsLayer};

use crate::web::auth::{handle_logout, handle_status, handle_verify_auth, require_admin_auth};
use crate::web::state::WebState;
use crate::web::static_files::static_handler;
use crate::web::studio::api::{
    create_source, create_template, delete_source, delete_template, get_source, get_template,
    list_sources, list_templates, refresh_source, update_source, update_template,
};

pub fn create_app(state: WebState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let auth_api = Router::new()
        .route("/auth/verify", post(handle_verify_auth))
        .route("/auth/logout", post(handle_logout));

    let protected_api = Router::new()
        .route("/status", get(handle_status))
        .route("/sources", get(list_sources).post(create_source))
        .route(
            "/sources/{id}",
            get(get_source).put(update_source).delete(delete_source),
        )
        .route("/sources/{id}/refresh", post(refresh_source))
        .route("/templates", get(list_templates).post(create_template))
        .route(
            "/templates/{id}",
            get(get_template)
                .put(update_template)
                .delete(delete_template),
        )
        .fallback(api_fallback_404)
        .layer(middleware::from_fn_with_state(
            state.clone(),
            require_admin_auth,
        ));

    let api = Router::new().merge(auth_api).merge(protected_api);

    Router::new()
        .nest("/api", api)
        .fallback(static_handler)
        .layer(cors)
        .with_state(state)
}

async fn api_fallback_404() -> Response {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"error":"Not Found"}"#))
        .unwrap_or_else(|_| StatusCode::NOT_FOUND.into_response())
}
