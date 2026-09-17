use std::time::Duration;

use axum::Json;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use serde_json::{Value, json};
use tracing::warn;

use crate::web::state::WebState;
use crate::web::studio::expand::{Expansion, expand_profile};
use crate::web::studio::model::{
    NodeSource, Profile, SourceKind, Template, new_profile_id, new_profile_token, new_source_id,
    new_template_id, now_rfc3339, validate_template_content,
};
use crate::web::studio::parser::parse_subscription;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSourceRequest {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: SourceKind,
    pub url: Option<String>,
    pub content: Option<String>,
    pub nodes: Option<Vec<Value>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSourceRequest {
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub kind: Option<SourceKind>,
    pub url: Option<String>,
    pub content: Option<String>,
    pub nodes: Option<Vec<Value>>,
}

pub async fn list_sources(State(state): State<WebState>) -> Response {
    (StatusCode::OK, Json(state.sources.list().await)).into_response()
}

pub async fn get_source(State(state): State<WebState>, Path(id): Path<String>) -> Response {
    match state.sources.get(&id).await {
        Some(source) => (StatusCode::OK, Json(source)).into_response(),
        None => json_error(StatusCode::NOT_FOUND, "source not found"),
    }
}

pub async fn create_source(
    State(state): State<WebState>,
    Json(req): Json<CreateSourceRequest>,
) -> Response {
    let name = req.name.trim();
    if name.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "name is required");
    }

    let mut source = NodeSource {
        id: new_source_id(),
        name: name.to_string(),
        kind: req.kind,
        url: normalize_optional(req.url),
        content: normalize_optional(req.content),
        status: Default::default(),
        last_updated: None,
        last_error: None,
        node_count: 0,
        nodes: Vec::new(),
    };

    match source.kind {
        SourceKind::Subscription => {
            let Some(url) = source.url.clone() else {
                return json_error(
                    StatusCode::BAD_REQUEST,
                    "subscription source requires a url",
                );
            };
            if !is_http_url(&url) {
                return json_error(
                    StatusCode::BAD_REQUEST,
                    "subscription url must start with http:// or https://",
                );
            }
            apply_fetched_nodes(&state, &mut source, &url).await;
        }
        SourceKind::Manual => {
            if let Some(nodes) = req.nodes.filter(|nodes| !nodes.is_empty()) {
                source.mark_active(nodes);
            } else {
                let Some(content) = source.content.clone() else {
                    return json_error(
                        StatusCode::BAD_REQUEST,
                        "manual source requires content or nodes",
                    );
                };
                match parse_subscription(&content) {
                    Ok(nodes) => source.mark_active(nodes),
                    Err(err) => {
                        return json_error(
                            StatusCode::BAD_REQUEST,
                            &format!("failed to parse node definitions: {err}"),
                        );
                    }
                }
            }
        }
    }

    match state.sources.insert(source).await {
        Ok(saved) => (StatusCode::CREATED, Json(saved)).into_response(),
        Err(err) => json_error(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string()),
    }
}

pub async fn update_source(
    State(state): State<WebState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateSourceRequest>,
) -> Response {
    let Some(mut source) = state.sources.get(&id).await else {
        return json_error(StatusCode::NOT_FOUND, "source not found");
    };

    if let Some(name) = req.name {
        let name = name.trim();
        if name.is_empty() {
            return json_error(StatusCode::BAD_REQUEST, "name is required");
        }
        source.name = name.to_string();
    }
    if let Some(kind) = req.kind {
        source.kind = kind;
    }
    if let Some(url) = req.url {
        source.url = normalize_optional(Some(url));
    }
    let content_updated = req.content.is_some();
    if let Some(content) = req.content {
        source.content = normalize_optional(Some(content));
    }

    if let Some(nodes) = req.nodes {
        source.mark_active(nodes);
    } else if source.kind == SourceKind::Manual
        && let Some(content) = source.content.clone()
        && content_updated
    {
        match parse_subscription(&content) {
            Ok(nodes) => source.mark_active(nodes),
            Err(err) => {
                return json_error(
                    StatusCode::BAD_REQUEST,
                    &format!("failed to parse node definitions: {err}"),
                );
            }
        }
    }

    if source.kind == SourceKind::Subscription
        && let Some(url) = source.url.as_deref()
        && !is_http_url(url)
    {
        return json_error(
            StatusCode::BAD_REQUEST,
            "subscription url must start with http:// or https://",
        );
    }

    match state.sources.replace(source).await {
        Ok(Some(saved)) => (StatusCode::OK, Json(saved)).into_response(),
        Ok(None) => json_error(StatusCode::NOT_FOUND, "source not found"),
        Err(err) => json_error(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string()),
    }
}

pub async fn delete_source(State(state): State<WebState>, Path(id): Path<String>) -> Response {
    match state.sources.delete(&id).await {
        Ok(true) => {
            if let Err(err) = state.profiles.unbind_source(&id).await {
                return json_error(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string());
            }
            StatusCode::NO_CONTENT.into_response()
        }
        Ok(false) => json_error(StatusCode::NOT_FOUND, "source not found"),
        Err(err) => json_error(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string()),
    }
}

pub async fn refresh_source(State(state): State<WebState>, Path(id): Path<String>) -> Response {
    let Some(mut source) = state.sources.get(&id).await else {
        return json_error(StatusCode::NOT_FOUND, "source not found");
    };

    match source.kind {
        SourceKind::Subscription => {
            let Some(url) = source.url.clone() else {
                return json_error(
                    StatusCode::BAD_REQUEST,
                    "subscription source requires a url",
                );
            };
            apply_fetched_nodes(&state, &mut source, &url).await;
        }
        SourceKind::Manual => {
            if let Some(content) = source.content.clone() {
                match parse_subscription(&content) {
                    Ok(nodes) => source.mark_active(nodes),
                    Err(err) => source.mark_error(err),
                }
            } else if source.nodes.is_empty() {
                source.mark_error("manual source has no content to refresh".to_string());
            } else {
                source.mark_active(source.nodes.clone());
            }
        }
    }

    match state.sources.replace(source).await {
        Ok(Some(saved)) => (StatusCode::OK, Json(saved)).into_response(),
        Ok(None) => json_error(StatusCode::NOT_FOUND, "source not found"),
        Err(err) => json_error(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string()),
    }
}

async fn apply_fetched_nodes(state: &WebState, source: &mut NodeSource, url: &str) {
    match fetch_subscription(&state.http, url).await {
        Ok(body) => match parse_subscription(&body) {
            Ok(nodes) => source.mark_active(nodes),
            Err(err) => {
                warn!("Failed to parse subscription from '{url}': {err}");
                source.mark_error(err);
            }
        },
        Err(err) => {
            warn!("{err}");
            source.mark_error(err);
        }
    }
}

async fn fetch_subscription(client: &reqwest::Client, url: &str) -> Result<String, String> {
    let response = client
        .get(url)
        .header(
            "User-Agent",
            format!("canto/{} (sing-box)", env!("CARGO_PKG_VERSION")),
        )
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .map_err(|err| format!("failed to fetch '{url}': {err}"))?;

    let status = response.status();
    if !status.is_success() {
        return Err(format!("failed to fetch '{url}': HTTP {status}"));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|err| format!("failed to read '{url}': {err}"))?;
    if bytes.len() > 10 * 1024 * 1024 {
        return Err(format!("subscription from '{url}' exceeds 10MB"));
    }
    String::from_utf8(bytes.to_vec())
        .map_err(|err| format!("subscription from '{url}' is not valid UTF-8: {err}"))
}

fn is_http_url(url: &str) -> bool {
    let lower = url.trim().to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://")
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value.and_then(|raw| {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTemplateRequest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub content: Value,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTemplateRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub content: Option<Value>,
}

pub async fn list_templates(State(state): State<WebState>) -> Response {
    (StatusCode::OK, Json(state.templates.list().await)).into_response()
}

pub async fn get_template(State(state): State<WebState>, Path(id): Path<String>) -> Response {
    match state.templates.get(&id).await {
        Some(template) => (StatusCode::OK, Json(template)).into_response(),
        None => json_error(StatusCode::NOT_FOUND, "template not found"),
    }
}

pub async fn create_template(
    State(state): State<WebState>,
    Json(req): Json<CreateTemplateRequest>,
) -> Response {
    let name = req.name.trim();
    if name.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "name is required");
    }
    if let Err(err) = validate_template_content(&req.content) {
        return json_error(StatusCode::BAD_REQUEST, &err);
    }

    let template = Template {
        id: new_template_id(),
        name: name.to_string(),
        description: req.description.trim().to_string(),
        updated_at: Some(now_rfc3339()),
        content: req.content,
    };

    match state.templates.insert(template).await {
        Ok(saved) => (StatusCode::CREATED, Json(saved)).into_response(),
        Err(err) => json_error(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string()),
    }
}

pub async fn update_template(
    State(state): State<WebState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateTemplateRequest>,
) -> Response {
    let Some(mut template) = state.templates.get(&id).await else {
        return json_error(StatusCode::NOT_FOUND, "template not found");
    };

    if let Some(name) = req.name {
        let name = name.trim();
        if name.is_empty() {
            return json_error(StatusCode::BAD_REQUEST, "name is required");
        }
        template.name = name.to_string();
    }
    if let Some(description) = req.description {
        template.description = description.trim().to_string();
    }
    if let Some(content) = req.content {
        if let Err(err) = validate_template_content(&content) {
            return json_error(StatusCode::BAD_REQUEST, &err);
        }
        template.content = content;
    }
    template.updated_at = Some(now_rfc3339());

    match state.templates.replace(template).await {
        Ok(Some(saved)) => (StatusCode::OK, Json(saved)).into_response(),
        Ok(None) => json_error(StatusCode::NOT_FOUND, "template not found"),
        Err(err) => json_error(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string()),
    }
}

pub async fn delete_template(State(state): State<WebState>, Path(id): Path<String>) -> Response {
    match state.templates.delete(&id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => json_error(StatusCode::NOT_FOUND, "template not found"),
        Err(err) => json_error(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string()),
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProfileRequest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub template_id: String,
    pub source_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProfileRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub template_id: Option<String>,
    pub source_ids: Option<Vec<String>>,
    #[serde(default)]
    pub rotate_token: bool,
}

pub async fn list_profiles(State(state): State<WebState>) -> Response {
    let base = state.settings.resolve_public_url();
    let profiles: Vec<Profile> = state
        .profiles
        .list()
        .await
        .into_iter()
        .map(|profile| profile.with_public_url(&base))
        .collect();
    (StatusCode::OK, Json(profiles)).into_response()
}

pub async fn create_profile(
    State(state): State<WebState>,
    Json(req): Json<CreateProfileRequest>,
) -> Response {
    let name = req.name.trim();
    if name.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "name is required");
    }
    let source_ids = unique_ids(&req.source_ids);
    if let Some(err) = validate_profile_refs(&state, &req.template_id, &source_ids).await {
        return err;
    }

    let token = match unique_token(&state).await {
        Ok(token) => token,
        Err(err) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string()),
    };

    let profile = Profile {
        id: new_profile_id(),
        name: name.to_string(),
        description: req.description.trim().to_string(),
        template_id: req.template_id,
        source_ids,
        token,
        updated_at: Some(now_rfc3339()),
        public_url: None,
    };

    match state.profiles.insert(profile).await {
        Ok(saved) => (
            StatusCode::CREATED,
            Json(saved.with_public_url(&state.settings.resolve_public_url())),
        )
            .into_response(),
        Err(err) => json_error(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string()),
    }
}

pub async fn update_profile(
    State(state): State<WebState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateProfileRequest>,
) -> Response {
    let Some(mut profile) = state.profiles.get(&id).await else {
        return json_error(StatusCode::NOT_FOUND, "profile not found");
    };

    if let Some(name) = req.name {
        let name = name.trim();
        if name.is_empty() {
            return json_error(StatusCode::BAD_REQUEST, "name is required");
        }
        profile.name = name.to_string();
    }
    if let Some(description) = req.description {
        profile.description = description.trim().to_string();
    }
    if let Some(template_id) = req.template_id {
        profile.template_id = template_id;
    }
    if let Some(source_ids) = req.source_ids {
        profile.source_ids = unique_ids(&source_ids);
    }
    if let Some(err) =
        validate_profile_refs(&state, &profile.template_id, &profile.source_ids).await
    {
        return err;
    }
    if req.rotate_token {
        match unique_token(&state).await {
            Ok(token) => profile.token = token,
            Err(err) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string()),
        }
    }
    profile.updated_at = Some(now_rfc3339());

    match state.profiles.replace(profile).await {
        Ok(Some(saved)) => (
            StatusCode::OK,
            Json(saved.with_public_url(&state.settings.resolve_public_url())),
        )
            .into_response(),
        Ok(None) => json_error(StatusCode::NOT_FOUND, "profile not found"),
        Err(err) => json_error(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string()),
    }
}

pub async fn delete_profile(State(state): State<WebState>, Path(id): Path<String>) -> Response {
    match state.profiles.delete(&id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => json_error(StatusCode::NOT_FOUND, "profile not found"),
        Err(err) => json_error(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string()),
    }
}

pub async fn preview_profile(State(state): State<WebState>, Path(id): Path<String>) -> Response {
    let Some(profile) = state.profiles.get(&id).await else {
        return json_error(StatusCode::NOT_FOUND, "profile not found");
    };
    match compile_profile(&state, &profile).await {
        Ok(expansion) => (
            StatusCode::OK,
            Json(json!({
                "config": expansion.config,
                "matchedMap": expansion.matched_map,
                "totalNodes": expansion.total_nodes,
                "usedCount": expansion.used_count,
            })),
        )
            .into_response(),
        Err(CompileError::BadRequest(message)) => json_error(StatusCode::BAD_REQUEST, &message),
    }
}

pub async fn get_subscription(
    State(state): State<WebState>,
    Path(token): Path<String>,
    headers: HeaderMap,
) -> Response {
    let Some(profile) = state.profiles.get_by_token(&token).await else {
        return json_error(StatusCode::NOT_FOUND, "subscription not found");
    };
    match compile_profile(&state, &profile).await {
        Ok(expansion) => match serde_json::to_vec(&expansion.config) {
            Ok(body) => {
                let etag = etag_for(&body);
                if headers
                    .get(header::IF_NONE_MATCH)
                    .and_then(|value| value.to_str().ok())
                    == Some(etag.as_str())
                {
                    return StatusCode::NOT_MODIFIED.into_response();
                }
                Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::ETAG, etag)
                    .body(Body::from(body))
                    .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
            }
            Err(err) => json_error(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string()),
        },
        Err(_) => json_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to compile profile",
        ),
    }
}

enum CompileError {
    BadRequest(String),
}

async fn compile_profile(state: &WebState, profile: &Profile) -> Result<Expansion, CompileError> {
    let Some(template) = state.templates.get(&profile.template_id).await else {
        return Err(CompileError::BadRequest("template not found".to_string()));
    };
    let mut nodes = Vec::new();
    for source_id in &profile.source_ids {
        let Some(source) = state.sources.get(source_id).await else {
            continue;
        };
        nodes.extend(source.nodes);
    }
    Ok(expand_profile(&template.content, &nodes))
}

async fn validate_profile_refs(
    state: &WebState,
    template_id: &str,
    source_ids: &[String],
) -> Option<Response> {
    if source_ids.is_empty() {
        return Some(json_error(
            StatusCode::BAD_REQUEST,
            "at least one source is required",
        ));
    }
    if state.templates.get(template_id).await.is_none() {
        return Some(json_error(StatusCode::BAD_REQUEST, "template not found"));
    }
    for source_id in source_ids {
        if state.sources.get(source_id).await.is_none() {
            return Some(json_error(
                StatusCode::BAD_REQUEST,
                &format!("source '{source_id}' not found"),
            ));
        }
    }
    None
}

async fn unique_token(state: &WebState) -> crate::error::Result<String> {
    for _ in 0..8 {
        let token = new_profile_token()?;
        if state.profiles.get_by_token(&token).await.is_none() {
            return Ok(token);
        }
    }
    Err(crate::error::CantoError::Web(
        "failed to allocate a unique profile token".to_string(),
    ))
}

fn unique_ids(ids: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for id in ids {
        let id = id.trim();
        if id.is_empty() || out.iter().any(|existing| existing == id) {
            continue;
        }
        out.push(id.to_string());
    }
    out
}

fn etag_for(body: &[u8]) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    body.hash(&mut hasher);
    format!("\"{:016x}\"", hasher.finish())
}

fn json_error(status: StatusCode, message: &str) -> Response {
    (status, Json(json!({ "error": message }))).into_response()
}
