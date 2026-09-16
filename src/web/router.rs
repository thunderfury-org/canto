use axum::Router;
use axum::middleware;
use axum::routing::{get, post};
use tower_http::cors::{Any, CorsLayer};

use crate::web::auth::{handle_logout, handle_status, handle_verify_auth, require_admin_auth};
use crate::web::state::WebState;
use crate::web::static_files::static_handler;

pub fn create_app(state: WebState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let protected_api =
        Router::new()
            .route("/status", get(handle_status))
            .layer(middleware::from_fn_with_state(
                state.clone(),
                require_admin_auth,
            ));

    let public_api = Router::new()
        .route("/auth/verify", post(handle_verify_auth))
        .route("/auth/logout", post(handle_logout));

    let api = Router::new().merge(protected_api).merge(public_api);

    Router::new()
        .nest("/api", api)
        .fallback(static_handler)
        .layer(cors)
        .with_state(state)
}
