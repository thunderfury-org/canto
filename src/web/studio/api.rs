use crate::web::state::WebState;
use crate::web::studio::compiler::CompileError;
use crate::web::studio::model::{
    NodeSource, Profile, SourceKind, Template, new_profile_id, new_source_id, new_template_id,
    now_rfc3339, validate_template_content,
};
use axum::Json;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use serde_json::{Value, json};

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

    if let Some(nodes) = req.nodes.filter(|nodes| !nodes.is_empty()) {
        source.mark_active(nodes);
    } else if let Err(err) = state.source_manager.ingest(&mut source).await {
        return json_error(StatusCode::BAD_REQUEST, &err);
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
    } else if (content_updated || source.kind == SourceKind::Subscription)
        && let Err(err) = state.source_manager.ingest(&mut source).await
    {
        return json_error(StatusCode::BAD_REQUEST, &err);
    }

    match state.sources.replace(source).await {
        Ok(Some(saved)) => (StatusCode::OK, Json(saved)).into_response(),
        Ok(None) => json_error(StatusCode::NOT_FOUND, "source not found"),
        Err(err) => json_error(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string()),
    }
}

pub async fn delete_source(State(state): State<WebState>, Path(id): Path<String>) -> Response {
    let empty_risk = state.profiles.check_source_removal(&id).await;
    if !empty_risk.is_empty() {
        let names: Vec<String> = empty_risk
            .into_iter()
            .map(|p| format!("\"{}\"", p.name))
            .collect();
        return json_error(
            StatusCode::CONFLICT,
            &format!(
                "cannot delete source: would leave profile(s) with no sources: {}",
                names.join(", ")
            ),
        );
    }

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
    match state.source_manager.refresh(&id).await {
        Ok(Some(saved)) => (StatusCode::OK, Json(saved)).into_response(),
        Ok(None) => json_error(StatusCode::NOT_FOUND, "source not found"),
        Err(err) => json_error(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string()),
    }
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
    let used_by = state.profiles.find_by_template(&id).await;
    if !used_by.is_empty() {
        let names: Vec<String> = used_by
            .into_iter()
            .map(|p| format!("\"{}\"", p.name))
            .collect();
        return json_error(
            StatusCode::CONFLICT,
            &format!(
                "cannot delete template: currently in use by profile(s): {}",
                names.join(", ")
            ),
        );
    }

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
    if let Err(err) = state
        .compiler
        .validate_refs(&req.template_id, &source_ids)
        .await
    {
        let (status, msg) = compile_error_response(err);
        return json_error(status, &msg);
    }

    let token = match state.compiler.allocate_unique_token().await {
        Ok(token) => token,
        Err(err) => {
            let (status, msg) = compile_error_response(err);
            return json_error(status, &msg);
        }
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
    if let Err(err) = state
        .compiler
        .validate_refs(&profile.template_id, &profile.source_ids)
        .await
    {
        let (status, msg) = compile_error_response(err);
        return json_error(status, &msg);
    }
    if req.rotate_token {
        match state.compiler.allocate_unique_token().await {
            Ok(token) => profile.token = token,
            Err(err) => {
                let (status, msg) = compile_error_response(err);
                return json_error(status, &msg);
            }
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
    match state.compiler.compile_by_id(&id).await {
        Ok(compiled) => (StatusCode::OK, Json(compiled)).into_response(),
        Err(err) => {
            let (status, msg) = compile_error_response(err);
            json_error(status, &msg)
        }
    }
}

pub async fn get_subscription(
    State(state): State<WebState>,
    Path(token): Path<String>,
    headers: HeaderMap,
) -> Response {
    match state.compiler.compile_by_token(&token).await {
        Ok(compiled) => {
            if headers
                .get(header::IF_NONE_MATCH)
                .and_then(|value| value.to_str().ok())
                == Some(compiled.etag.as_str())
            {
                return StatusCode::NOT_MODIFIED.into_response();
            }
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::ETAG, compiled.etag)
                .body(Body::from(compiled.raw_bytes))
                .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
        }
        Err(err) => {
            let (status, msg) = compile_error_response(err);
            json_error(status, &msg)
        }
    }
}

fn compile_error_response(err: CompileError) -> (StatusCode, String) {
    match err {
        CompileError::ProfileNotFound(_) => {
            (StatusCode::NOT_FOUND, "profile not found".to_string())
        }
        CompileError::TokenNotFound(_) => {
            (StatusCode::NOT_FOUND, "subscription not found".to_string())
        }
        CompileError::TemplateNotFound(_) => {
            (StatusCode::BAD_REQUEST, "template not found".to_string())
        }
        CompileError::SourceNotFound(id) => {
            (StatusCode::BAD_REQUEST, format!("source '{id}' not found"))
        }
        CompileError::EmptySources => (
            StatusCode::BAD_REQUEST,
            "at least one source is required".to_string(),
        ),
        CompileError::TokenAllocationFailed => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to allocate a unique profile token".to_string(),
        ),
        CompileError::SerializationError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
    }
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

fn json_error(status: StatusCode, message: &str) -> Response {
    (status, Json(json!({ "error": message }))).into_response()
}
