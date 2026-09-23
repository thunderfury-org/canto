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
    pub default_prefix: String,
    pub category: String, // "domain_and_ip" | "geosite" | "geoip"
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
            default_prefix: "".to_string(),
            category: "domain_and_ip".to_string(),
        },
        RulesetPreset {
            id: "sagernet-geosite".to_string(),
            name: "SagerNet 官方 GeoSite 规则集".to_string(),
            repo: "SagerNet/sing-geosite".to_string(),
            tag: "rule-set".to_string(),
            description: "SagerNet 官方维护的最新域名分流规则集 (branch: rule-set)".to_string(),
            url_pattern: "https://raw.githubusercontent.com/SagerNet/sing-geosite/rule-set/{tag}.srs".to_string(),
            format: "binary".to_string(),
            default_prefix: "geosite-".to_string(),
            category: "geosite".to_string(),
        },
        RulesetPreset {
            id: "sagernet-geoip".to_string(),
            name: "SagerNet 官方 GeoIP 规则集".to_string(),
            repo: "SagerNet/sing-geoip".to_string(),
            tag: "rule-set".to_string(),
            description: "SagerNet 官方维护的 IP 分流规则集 (branch: rule-set)".to_string(),
            url_pattern: "https://raw.githubusercontent.com/SagerNet/sing-geoip/rule-set/{tag}.srs".to_string(),
            format: "binary".to_string(),
            default_prefix: "geoip-".to_string(),
            category: "geoip".to_string(),
        },
        RulesetPreset {
            id: "metacubex-geosite".to_string(),
            name: "MetaCubeX GeoSite (域名规则)".to_string(),
            repo: "MetaCubeX/meta-rules-dat".to_string(),
            tag: "sing/geo/geosite".to_string(),
            description: "MetaCubeX 维护的完整 GeoSite 域名分流规则".to_string(),
            url_pattern: "https://raw.githubusercontent.com/MetaCubeX/meta-rules-dat/sing/geo/geosite/{tag}.srs".to_string(),
            format: "binary".to_string(),
            default_prefix: "geosite-".to_string(),
            category: "geosite".to_string(),
        },
        RulesetPreset {
            id: "metacubex-geoip".to_string(),
            name: "MetaCubeX GeoIP (IP 规则)".to_string(),
            repo: "MetaCubeX/meta-rules-dat".to_string(),
            tag: "sing/geo/geoip".to_string(),
            description: "MetaCubeX 维护的 GeoIP 规则".to_string(),
            url_pattern: "https://raw.githubusercontent.com/MetaCubeX/meta-rules-dat/sing/geo/geoip/{tag}.srs".to_string(),
            format: "binary".to_string(),
            default_prefix: "geoip-".to_string(),
            category: "geoip".to_string(),
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
    pub raw_name: String,
    pub formats: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub srs_size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub json_size: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InspectReleaseResponse {
    pub source_type: String, // "release" | "branch"
    pub owner: String,
    pub repo: String,
    pub tag: String,
    pub name: String,
    pub html_url: String,
    pub published_at: Option<String>,
    pub download_url_template_srs: String,
    pub download_url_template_json: String,
    pub suggested_prefix: String,
    pub category: String,
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

    let target = match parse_ruleset_source_target(input) {
        Ok(res) => res,
        Err(err) => return json_error(StatusCode::BAD_REQUEST, &err),
    };

    match target {
        RulesetSourceTarget::Release { owner, repo, tag } => {
            inspect_github_release(&state, owner, repo, tag).await
        }
        RulesetSourceTarget::Branch {
            owner,
            repo,
            branch,
            path,
        } => inspect_github_branch(&state, owner, repo, branch, path).await,
    }
}

async fn inspect_github_release(
    state: &WebState,
    owner: String,
    repo: String,
    tag_opt: Option<String>,
) -> Response {
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

    type AssetMeta = (String, BTreeSet<String>, Option<u64>, Option<u64>);
    let mut rules_map: BTreeMap<String, AssetMeta> = BTreeMap::new();

    for asset in assets {
        let Some(asset_name) = asset.get("name").and_then(|v| v.as_str()) else {
            continue;
        };
        let size = asset.get("size").and_then(|v| v.as_u64());

        if let Some(tag) = asset_name.strip_suffix(".srs") {
            let entry = rules_map
                .entry(tag.to_string())
                .or_insert_with(|| (asset_name.to_string(), BTreeSet::new(), None, None));
            entry.1.insert("srs".to_string());
            entry.2 = size;
        } else if let Some(tag) = asset_name.strip_suffix(".json") {
            let entry = rules_map
                .entry(tag.to_string())
                .or_insert_with(|| (asset_name.to_string(), BTreeSet::new(), None, None));
            entry.1.insert("json".to_string());
            entry.3 = size;
        }
    }

    let rules: Vec<RuleAssetItem> = rules_map
        .into_iter()
        .map(
            |(tag, (raw_name, formats, srs_size, json_size))| RuleAssetItem {
                tag,
                raw_name,
                formats: formats.into_iter().collect(),
                srs_size,
                json_size,
            },
        )
        .collect();

    let download_url_template_srs =
        format!("https://github.com/{owner}/{repo}/releases/download/{effective_tag}/{{tag}}.srs");
    let download_url_template_json =
        format!("https://github.com/{owner}/{repo}/releases/download/{effective_tag}/{{tag}}.json");

    (
        StatusCode::OK,
        Json(InspectReleaseResponse {
            source_type: "release".to_string(),
            owner,
            repo,
            tag: effective_tag,
            name,
            html_url,
            published_at,
            download_url_template_srs,
            download_url_template_json,
            suggested_prefix: "".to_string(),
            category: "domain_and_ip".to_string(),
            rules,
        }),
    )
        .into_response()
}

async fn inspect_github_branch(
    state: &WebState,
    owner: String,
    repo: String,
    branch: String,
    path: String,
) -> Response {
    let clean_path = path.trim_matches('/').to_string();

    // 1. Prefer Git Trees API: /repos/{owner}/{repo}/git/trees/{branch}:{clean_path}
    // Git Trees API returns up to 100,000 files without the 1,000 files truncation limit of /contents!
    let tree_ref = if clean_path.is_empty() {
        branch.clone()
    } else {
        format!("{branch}:{clean_path}")
    };
    let git_trees_url = format!("https://api.github.com/repos/{owner}/{repo}/git/trees/{tree_ref}");

    let mut res = state
        .http
        .get(&git_trees_url)
        .header("User-Agent", "canto-web-studio")
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await;

    // Fallback to /contents API if git trees returns non-success
    let is_trees_success = match &res {
        Ok(r) => r.status().is_success(),
        Err(_) => false,
    };

    if !is_trees_success {
        let fallback_contents_url = if clean_path.is_empty() {
            format!("https://api.github.com/repos/{owner}/{repo}/contents?ref={branch}")
        } else {
            format!(
                "https://api.github.com/repos/{owner}/{repo}/contents/{clean_path}?ref={branch}"
            )
        };
        if let Ok(fallback_res) = state
            .http
            .get(&fallback_contents_url)
            .header("User-Agent", "canto-web-studio")
            .header("Accept", "application/vnd.github.v3+json")
            .send()
            .await
            && fallback_res.status().is_success()
        {
            res = Ok(fallback_res);
        }
    }

    let response = match res {
        Ok(r) => r,
        Err(err) => {
            return json_error(
                StatusCode::BAD_GATEWAY,
                &format!("failed to fetch github branch items: {err}"),
            );
        }
    };

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return json_error(
            StatusCode::BAD_GATEWAY,
            &format!("GitHub API returned {status}: {body}"),
        );
    }

    let bytes = match response.bytes().await {
        Ok(b) => b,
        Err(err) => {
            return json_error(
                StatusCode::BAD_GATEWAY,
                &format!("failed to read GitHub response: {err}"),
            );
        }
    };

    let parsed_json: serde_json::Value = match serde_json::from_slice(&bytes) {
        Ok(v) => v,
        Err(err) => {
            return json_error(
                StatusCode::BAD_GATEWAY,
                &format!("failed to parse GitHub JSON: {err}"),
            );
        }
    };

    // Git Trees API returns { "tree": [...] }, while /contents returns [...]
    let empty_vec = Vec::new();
    let items: &[serde_json::Value] =
        if let Some(tree_arr) = parsed_json.get("tree").and_then(|v| v.as_array()) {
            tree_arr
        } else if let Some(arr) = parsed_json.as_array() {
            arr
        } else {
            &empty_vec
        };

    type AssetMeta = (String, BTreeSet<String>, Option<u64>, Option<u64>);
    let mut rules_map: BTreeMap<String, AssetMeta> = BTreeMap::new();

    // Check if majority of files share a prefix like "geosite-" or "geoip-"
    let mut prefix_geosite_count = 0;
    let mut prefix_geoip_count = 0;
    let mut total_rule_files = 0;

    for item in items {
        let typ = item.get("type").and_then(|v| v.as_str()).unwrap_or("");
        if typ != "file" && typ != "blob" {
            continue;
        }
        let Some(raw_path) = item
            .get("path")
            .or_else(|| item.get("name"))
            .and_then(|v| v.as_str())
        else {
            continue;
        };
        let file_name = raw_path.rsplit('/').next().unwrap_or(raw_path);
        let is_srs = file_name.ends_with(".srs");
        let is_json = file_name.ends_with(".json");
        if !is_srs && !is_json {
            continue;
        }
        total_rule_files += 1;
        if file_name.starts_with("geosite-") {
            prefix_geosite_count += 1;
        } else if file_name.starts_with("geoip-") {
            prefix_geoip_count += 1;
        }
    }

    let (file_prefix, suggested_prefix, category) =
        if total_rule_files > 0 && prefix_geosite_count >= total_rule_files / 2 {
            ("geosite-", "geosite-", "geosite")
        } else if total_rule_files > 0 && prefix_geoip_count >= total_rule_files / 2 {
            ("geoip-", "geoip-", "geoip")
        } else if clean_path.contains("geosite") || repo.to_lowercase().contains("geosite") {
            ("", "geosite-", "geosite")
        } else if clean_path.contains("geoip") || repo.to_lowercase().contains("geoip") {
            ("", "geoip-", "geoip")
        } else {
            ("", "", "general")
        };

    for item in items {
        let typ = item.get("type").and_then(|v| v.as_str()).unwrap_or("");
        if typ != "file" && typ != "blob" {
            continue;
        }
        let Some(raw_path) = item
            .get("path")
            .or_else(|| item.get("name"))
            .and_then(|v| v.as_str())
        else {
            continue;
        };
        let file_name = raw_path.rsplit('/').next().unwrap_or(raw_path);
        let size = item.get("size").and_then(|v| v.as_u64());

        let (base_name, fmt) = if let Some(stripped) = file_name.strip_suffix(".srs") {
            (stripped, "srs")
        } else if let Some(stripped) = file_name.strip_suffix(".json") {
            (stripped, "json")
        } else {
            continue;
        };

        // If file_prefix is "geosite-", strip it so tag is clean (e.g. "cn", "ai")
        let clean_tag = if !file_prefix.is_empty() && base_name.starts_with(file_prefix) {
            &base_name[file_prefix.len()..]
        } else {
            base_name
        };

        let entry = rules_map
            .entry(clean_tag.to_string())
            .or_insert_with(|| (file_name.to_string(), BTreeSet::new(), None, None));
        entry.1.insert(fmt.to_string());
        if fmt == "srs" {
            entry.2 = size;
        } else {
            entry.3 = size;
        }
    }

    let rules: Vec<RuleAssetItem> = rules_map
        .into_iter()
        .map(
            |(tag, (raw_name, formats, srs_size, json_size))| RuleAssetItem {
                tag,
                raw_name,
                formats: formats.into_iter().collect(),
                srs_size,
                json_size,
            },
        )
        .collect();

    let path_segment = if clean_path.is_empty() {
        "".to_string()
    } else {
        format!("{clean_path}/")
    };

    // Construct raw.githubusercontent.com template URL
    let download_url_template_srs = format!(
        "https://raw.githubusercontent.com/{owner}/{repo}/{branch}/{path_segment}{file_prefix}{{tag}}.srs"
    );
    let download_url_template_json = format!(
        "https://raw.githubusercontent.com/{owner}/{repo}/{branch}/{path_segment}{file_prefix}{{tag}}.json"
    );

    let html_url = if clean_path.is_empty() {
        format!("https://github.com/{owner}/{repo}/tree/{branch}")
    } else {
        format!("https://github.com/{owner}/{repo}/tree/{branch}/{clean_path}")
    };

    (
        StatusCode::OK,
        Json(InspectReleaseResponse {
            source_type: "branch".to_string(),
            owner,
            repo,
            tag: branch,
            name: format!("{clean_path} ({total_rule_files} files)"),
            html_url,
            published_at: None,
            download_url_template_srs,
            download_url_template_json,
            suggested_prefix: suggested_prefix.to_string(),
            category: category.to_string(),
            rules,
        }),
    )
        .into_response()
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum RulesetSourceTarget {
    Release {
        owner: String,
        repo: String,
        tag: Option<String>,
    },
    Branch {
        owner: String,
        repo: String,
        branch: String,
        path: String,
    },
}

pub fn parse_ruleset_source_target(input: &str) -> Result<RulesetSourceTarget, String> {
    let raw = input.trim();
    if raw.is_empty() {
        return Err("target cannot be empty".to_string());
    }

    // Case 1: https://github.com/... or http://github.com/...
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

        // Release download
        if segments.len() >= 5 && segments[2] == "releases" && segments[3] == "download" {
            let tag = segments[4].to_string();
            return Ok(RulesetSourceTarget::Release {
                owner,
                repo,
                tag: Some(tag),
            });
        }

        // Release tag
        if segments.len() >= 5 && segments[2] == "releases" && segments[3] == "tag" {
            let tag = segments[4].to_string();
            return Ok(RulesetSourceTarget::Release {
                owner,
                repo,
                tag: Some(tag),
            });
        }

        // Release latest
        if segments.len() >= 4 && segments[2] == "releases" && segments[3] == "latest" {
            return Ok(RulesetSourceTarget::Release {
                owner,
                repo,
                tag: None,
            });
        }

        // Branch / Tree URL: https://github.com/owner/repo/tree/branch/subpath...
        if segments.len() >= 4 && segments[2] == "tree" {
            let branch = segments[3].to_string();
            let subpath = if segments.len() > 4 {
                segments[4..].join("/")
            } else {
                "".to_string()
            };
            return Ok(RulesetSourceTarget::Branch {
                owner,
                repo,
                branch,
                path: subpath,
            });
        }

        return Ok(RulesetSourceTarget::Release {
            owner,
            repo,
            tag: None,
        });
    }

    // Case 2: raw.githubusercontent.com/owner/repo/branch/subpath...
    if raw.starts_with("https://raw.githubusercontent.com/")
        || raw.starts_with("http://raw.githubusercontent.com/")
    {
        let path = raw
            .trim_start_matches("https://raw.githubusercontent.com/")
            .trim_start_matches("http://raw.githubusercontent.com/")
            .trim_matches('/');
        let segments: Vec<&str> = path.split('/').collect();
        if segments.len() >= 3 {
            let owner = segments[0].to_string();
            let repo = segments[1].to_string();
            let branch = segments[2].to_string();
            let subpath = if segments.len() > 3 {
                // If last segment has extension like .srs, strip filename
                let last = segments[segments.len() - 1];
                if last.ends_with(".srs") || last.ends_with(".json") {
                    segments[3..segments.len() - 1].join("/")
                } else {
                    segments[3..].join("/")
                }
            } else {
                "".to_string()
            };
            return Ok(RulesetSourceTarget::Branch {
                owner,
                repo,
                branch,
                path: subpath,
            });
        }
    }

    // Case 3: owner/repo#branch/subpath or owner/repo#branch
    if let Some((repo_part, branch_part)) = raw.split_once('#') {
        let segments: Vec<&str> = repo_part.split('/').collect();
        if segments.len() == 2 && !segments[0].is_empty() && !segments[1].is_empty() {
            let (branch, subpath) = match branch_part.split_once('/') {
                Some((b, p)) => (b.to_string(), p.to_string()),
                None => (branch_part.to_string(), "".to_string()),
            };
            return Ok(RulesetSourceTarget::Branch {
                owner: segments[0].to_string(),
                repo: segments[1].to_string(),
                branch,
                path: subpath,
            });
        }
    }

    // Case 4: owner/repo@tag
    if let Some((repo_part, tag_part)) = raw.split_once('@') {
        let segments: Vec<&str> = repo_part.split('/').collect();
        if segments.len() == 2 && !segments[0].is_empty() && !segments[1].is_empty() {
            // Note: if tag_part contains '/', it might be a branch with subpath, e.g. sing/geo/geosite
            if tag_part.contains('/') {
                let (branch, subpath) = tag_part.split_once('/').unwrap();
                return Ok(RulesetSourceTarget::Branch {
                    owner: segments[0].to_string(),
                    repo: segments[1].to_string(),
                    branch: branch.to_string(),
                    path: subpath.to_string(),
                });
            }
            let tag = tag_part.trim();
            let tag_opt = if tag.is_empty() || tag == "latest" {
                None
            } else {
                Some(tag.to_string())
            };
            return Ok(RulesetSourceTarget::Release {
                owner: segments[0].to_string(),
                repo: segments[1].to_string(),
                tag: tag_opt,
            });
        }
    }

    // Case 5: owner/repo
    let segments: Vec<&str> = raw.split('/').collect();
    if segments.len() == 2 && !segments[0].is_empty() && !segments[1].is_empty() {
        return Ok(RulesetSourceTarget::Release {
            owner: segments[0].to_string(),
            repo: segments[1].to_string(),
            tag: None,
        });
    }

    Err(format!(
        "unrecognized GitHub source target '{raw}'. Expected 'owner/repo@tag', 'https://github.com/owner/repo/tree/branch/path' or 'owner/repo#branch/path'"
    ))
}

fn json_error(status: StatusCode, message: &str) -> Response {
    (status, Json(json!({ "error": message }))).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ruleset_source_target() {
        // Tag URL
        let target = parse_ruleset_source_target(
            "https://github.com/DustinWin/ruleset_geodata/releases/tag/sing-box-ruleset",
        )
        .unwrap();
        assert_eq!(
            target,
            RulesetSourceTarget::Release {
                owner: "DustinWin".to_string(),
                repo: "ruleset_geodata".to_string(),
                tag: Some("sing-box-ruleset".to_string()),
            }
        );

        // Branch Tree URL with subpath
        let target = parse_ruleset_source_target(
            "https://github.com/MetaCubeX/meta-rules-dat/tree/sing/geo/geosite",
        )
        .unwrap();
        assert_eq!(
            target,
            RulesetSourceTarget::Branch {
                owner: "MetaCubeX".to_string(),
                repo: "meta-rules-dat".to_string(),
                branch: "sing".to_string(),
                path: "geo/geosite".to_string(),
            }
        );

        // Branch Tree URL at root
        let target =
            parse_ruleset_source_target("https://github.com/SagerNet/sing-geosite/tree/rule-set")
                .unwrap();
        assert_eq!(
            target,
            RulesetSourceTarget::Branch {
                owner: "SagerNet".to_string(),
                repo: "sing-geosite".to_string(),
                branch: "rule-set".to_string(),
                path: "".to_string(),
            }
        );

        // Shorthand with #branch/path
        let target =
            parse_ruleset_source_target("MetaCubeX/meta-rules-dat#sing/geo/geoip").unwrap();
        assert_eq!(
            target,
            RulesetSourceTarget::Branch {
                owner: "MetaCubeX".to_string(),
                repo: "meta-rules-dat".to_string(),
                branch: "sing".to_string(),
                path: "geo/geoip".to_string(),
            }
        );

        // Shorthand with @tag
        let target =
            parse_ruleset_source_target("DustinWin/ruleset_geodata@sing-box-ruleset").unwrap();
        assert_eq!(
            target,
            RulesetSourceTarget::Release {
                owner: "DustinWin".to_string(),
                repo: "ruleset_geodata".to_string(),
                tag: Some("sing-box-ruleset".to_string()),
            }
        );
    }
}
