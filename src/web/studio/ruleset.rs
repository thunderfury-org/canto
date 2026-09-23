use std::collections::{BTreeMap, BTreeSet};

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::web::state::WebState;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RulesetPreset {
    pub id: String,
    pub name: String,
    pub repo: String,
    pub tag: String,
    pub description: String,
    pub url_pattern: String,
    pub format: String,
}

pub fn get_builtin_presets() -> Vec<RulesetPreset> {
    vec![
        RulesetPreset {
            id: "dustinwin-ruleset".to_string(),
            name: "DustinWin 规则集".to_string(),
            repo: "DustinWin/ruleset_geodata".to_string(),
            tag: "sing-box-ruleset".to_string(),
            description: "主流 DNS 与路由分流规则（cn, ai, netflix, youtube, proxy, private 等）".to_string(),
            url_pattern: "https://github.com/DustinWin/ruleset_geodata/releases/download/sing-box-ruleset/{tag}.srs".to_string(),
            format: "binary".to_string(),
        },
        RulesetPreset {
            id: "loyalsoldier-rules".to_string(),
            name: "Loyalsoldier 规则集".to_string(),
            repo: "Loyalsoldier/sing-box-rules".to_string(),
            tag: "release".to_string(),
            description: "经典社区全量 GeoSite / GeoIP 域名与 IP 规则集".to_string(),
            url_pattern: "https://github.com/Loyalsoldier/sing-box-rules/releases/download/release/{tag}.srs".to_string(),
            format: "binary".to_string(),
        },
        RulesetPreset {
            id: "metacubex-rules".to_string(),
            name: "MetaCubeX sing-box 规则集".to_string(),
            repo: "MetaCubeX/meta-rules-dat".to_string(),
            tag: "sing".to_string(),
            description: "MetaCubeX 维护的 GeoSite / GeoIP 兼容规则集".to_string(),
            url_pattern: "https://github.com/MetaCubeX/meta-rules-dat/releases/download/sing/{tag}.srs".to_string(),
            format: "binary".to_string(),
        },
    ]
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InspectReleaseRequest {
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleAssetItem {
    pub tag: String,
    pub formats: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub srs_size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub json_size: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InspectReleaseResponse {
    pub owner: String,
    pub repo: String,
    pub tag: String,
    pub name: String,
    pub html_url: String,
    pub published_at: Option<String>,
    pub download_url_template_srs: String,
    pub download_url_template_json: String,
    pub rules: Vec<RuleAssetItem>,
}

pub async fn list_ruleset_presets() -> Response {
    (StatusCode::OK, Json(get_builtin_presets())).into_response()
}

pub async fn inspect_release(
    State(state): State<WebState>,
    Json(req): Json<InspectReleaseRequest>,
) -> Response {
    let input = req.url.trim();
    if input.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "url is required");
    }

    let (owner, repo, tag_opt) = match parse_github_release_target(input) {
        Ok(res) => res,
        Err(err) => return json_error(StatusCode::BAD_REQUEST, &err),
    };

    let gh_api_url = match &tag_opt {
        Some(t) => format!("https://api.github.com/repos/{owner}/{repo}/releases/tags/{t}"),
        None => format!("https://api.github.com/repos/{owner}/{repo}/releases/latest"),
    };

    let res = match state
        .http
        .get(&gh_api_url)
        .header("User-Agent", "canto-web-studio")
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await
    {
        Ok(res) => res,
        Err(err) => {
            return json_error(
                StatusCode::BAD_GATEWAY,
                &format!("failed to fetch github release: {err}"),
            );
        }
    };

    if !res.status().is_success() {
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        return json_error(
            StatusCode::BAD_GATEWAY,
            &format!("GitHub API returned {status}: {body}"),
        );
    }

    let bytes = match res.bytes().await {
        Ok(b) => b,
        Err(err) => {
            return json_error(
                StatusCode::BAD_GATEWAY,
                &format!("failed to read GitHub release response: {err}"),
            );
        }
    };

    let release_json: serde_json::Value = match serde_json::from_slice(&bytes) {
        Ok(v) => v,
        Err(err) => {
            return json_error(
                StatusCode::BAD_GATEWAY,
                &format!("failed to parse GitHub release JSON: {err}"),
            );
        }
    };

    let tag_name = release_json
        .get("tag_name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let effective_tag = if !tag_name.is_empty() {
        tag_name
    } else {
        tag_opt.unwrap_or_else(|| "latest".to_string())
    };

    let name = release_json
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or(&effective_tag)
        .to_string();
    let html_url = release_json
        .get("html_url")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let published_at = release_json
        .get("published_at")
        .and_then(|v| v.as_str())
        .map(str::to_string);

    let empty_assets = Vec::new();
    let assets = release_json
        .get("assets")
        .and_then(|v| v.as_array())
        .unwrap_or(&empty_assets);

    type AssetMeta = (BTreeSet<String>, Option<u64>, Option<u64>);
    let mut rules_map: BTreeMap<String, AssetMeta> = BTreeMap::new();

    for asset in assets {
        let Some(asset_name) = asset.get("name").and_then(|v| v.as_str()) else {
            continue;
        };
        let size = asset.get("size").and_then(|v| v.as_u64());

        if let Some(tag) = asset_name.strip_suffix(".srs") {
            let entry = rules_map.entry(tag.to_string()).or_default();
            entry.0.insert("srs".to_string());
            entry.1 = size;
        } else if let Some(tag) = asset_name.strip_suffix(".json") {
            let entry = rules_map.entry(tag.to_string()).or_default();
            entry.0.insert("json".to_string());
            entry.2 = size;
        }
    }

    let rules: Vec<RuleAssetItem> = rules_map
        .into_iter()
        .map(|(tag, (formats, srs_size, json_size))| RuleAssetItem {
            tag,
            formats: formats.into_iter().collect(),
            srs_size,
            json_size,
        })
        .collect();

    let download_url_template_srs =
        format!("https://github.com/{owner}/{repo}/releases/download/{effective_tag}/{{tag}}.srs");
    let download_url_template_json =
        format!("https://github.com/{owner}/{repo}/releases/download/{effective_tag}/{{tag}}.json");

    (
        StatusCode::OK,
        Json(InspectReleaseResponse {
            owner,
            repo,
            tag: effective_tag,
            name,
            html_url,
            published_at,
            download_url_template_srs,
            download_url_template_json,
            rules,
        }),
    )
        .into_response()
}

pub fn parse_github_release_target(
    input: &str,
) -> Result<(String, String, Option<String>), String> {
    let raw = input.trim();
    if raw.is_empty() {
        return Err("target cannot be empty".to_string());
    }

    // Case 1: https://github.com/owner/repo/...
    if raw.starts_with("https://github.com/") || raw.starts_with("http://github.com/") {
        let path = raw
            .trim_start_matches("https://github.com/")
            .trim_start_matches("http://github.com/")
            .trim_matches('/');
        let segments: Vec<&str> = path.split('/').collect();
        if segments.len() < 2 {
            return Err("invalid GitHub repository URL".to_string());
        }
        let owner = segments[0].to_string();
        let repo = segments[1].trim_end_matches(".git").to_string();

        if segments.len() >= 5 && segments[2] == "releases" && segments[3] == "download" {
            // https://github.com/owner/repo/releases/download/tag/...
            let tag = segments[4].to_string();
            return Ok((owner, repo, Some(tag)));
        }

        if segments.len() >= 5 && segments[2] == "releases" && segments[3] == "tag" {
            // https://github.com/owner/repo/releases/tag/tag
            let tag = segments[4].to_string();
            return Ok((owner, repo, Some(tag)));
        }

        if segments.len() >= 4 && segments[2] == "releases" && segments[3] == "latest" {
            return Ok((owner, repo, None));
        }

        return Ok((owner, repo, None));
    }

    // Case 2: owner/repo@tag or owner/repo
    if let Some((repo_part, tag_part)) = raw.split_once('@') {
        let segments: Vec<&str> = repo_part.split('/').collect();
        if segments.len() == 2 && !segments[0].is_empty() && !segments[1].is_empty() {
            let tag = tag_part.trim();
            let tag_opt = if tag.is_empty() || tag == "latest" {
                None
            } else {
                Some(tag.to_string())
            };
            return Ok((segments[0].to_string(), segments[1].to_string(), tag_opt));
        }
    }

    let segments: Vec<&str> = raw.split('/').collect();
    if segments.len() == 2 && !segments[0].is_empty() && !segments[1].is_empty() {
        return Ok((segments[0].to_string(), segments[1].to_string(), None));
    }

    Err(format!(
        "unrecognized GitHub release target '{raw}'. Expected 'owner/repo@tag' or 'https://github.com/owner/repo/releases/tag/tag'"
    ))
}

fn json_error(status: StatusCode, message: &str) -> Response {
    (status, Json(json!({ "error": message }))).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_github_release_target() {
        // Tag URL
        let (o, r, t) = parse_github_release_target(
            "https://github.com/DustinWin/ruleset_geodata/releases/tag/sing-box-ruleset",
        )
        .unwrap();
        assert_eq!(o, "DustinWin");
        assert_eq!(r, "ruleset_geodata");
        assert_eq!(t, Some("sing-box-ruleset".to_string()));

        // Download URL with asset
        let (o, r, t) = parse_github_release_target(
            "https://github.com/DustinWin/ruleset_geodata/releases/download/sing-box-ruleset/cn.srs",
        )
        .unwrap();
        assert_eq!(o, "DustinWin");
        assert_eq!(r, "ruleset_geodata");
        assert_eq!(t, Some("sing-box-ruleset".to_string()));

        // Shorthand with @tag
        let (o, r, t) =
            parse_github_release_target("DustinWin/ruleset_geodata@sing-box-ruleset").unwrap();
        assert_eq!(o, "DustinWin");
        assert_eq!(r, "ruleset_geodata");
        assert_eq!(t, Some("sing-box-ruleset".to_string()));

        // Shorthand without tag (latest)
        let (o, r, t) = parse_github_release_target("DustinWin/ruleset_geodata").unwrap();
        assert_eq!(o, "DustinWin");
        assert_eq!(r, "ruleset_geodata");
        assert_eq!(t, None);

        // Repo root URL
        let (o, r, t) =
            parse_github_release_target("https://github.com/Loyalsoldier/sing-box-rules").unwrap();
        assert_eq!(o, "Loyalsoldier");
        assert_eq!(r, "sing-box-rules");
        assert_eq!(t, None);
    }
}
