use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{CantoError, Result};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceKind {
    Subscription,
    Manual,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SourceStatus {
    #[default]
    Idle,
    Active,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeSource {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub kind: SourceKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(default)]
    pub status: SourceStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_updated: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    #[serde(default)]
    pub node_count: usize,
    #[serde(default)]
    pub nodes: Vec<Value>,
}

impl NodeSource {
    pub fn sync_counts(&mut self) {
        self.node_count = self.nodes.len();
    }

    pub fn mark_active(&mut self, nodes: Vec<Value>) {
        self.nodes = nodes;
        self.status = SourceStatus::Active;
        self.last_error = None;
        self.last_updated = Some(now_rfc3339());
        self.sync_counts();
    }

    pub fn mark_error(&mut self, error: String) {
        self.status = SourceStatus::Error;
        self.last_error = Some(error);
        self.sync_counts();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Template {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    pub content: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub template_id: String,
    #[serde(default)]
    pub source_ids: Vec<String>,
    pub token: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public_url: Option<String>,
}

impl Profile {
    pub fn with_public_url(mut self, base: &str) -> Self {
        let base = base.trim_end_matches('/');
        self.public_url = Some(format!("{base}/sub/{}", self.token));
        self
    }

    pub fn strip_public_url(&mut self) {
        self.public_url = None;
    }
}

pub fn new_template_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("tpl_{nanos:x}")
}

pub fn new_profile_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("prof_{nanos:x}")
}

pub fn new_profile_token() -> Result<String> {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes)
        .map_err(|err| CantoError::Web(format!("failed to generate profile token: {err}")))?;
    Ok(format!("tok_{}", hex_encode(&bytes)))
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0xf) as usize] as char);
    }
    out
}

pub fn is_safe_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

pub fn validate_template_content(content: &Value) -> std::result::Result<(), String> {
    let Some(obj) = content.as_object() else {
        return Err("content must be a JSON object".to_string());
    };

    validate_optional_object(obj, "log")?;
    validate_optional_object(obj, "experimental")?;
    validate_optional_object(obj, "dns")?;
    validate_optional_object(obj, "route")?;
    validate_optional_array_of_objects(obj, "inbounds")?;
    validate_optional_array_of_objects(obj, "endpoints")?;
    validate_optional_array_of_objects(obj, "outbounds")?;
    validate_optional_array_of_objects(obj, "node_groups")?;
    validate_optional_array_of_objects(obj, "policy_groups")?;
    validate_optional_array_of_objects(obj, "rule_sets")?;

    let mut base_tags: std::collections::HashSet<String> = std::collections::HashSet::new();
    if let Some(outbounds) = obj.get("outbounds").and_then(Value::as_array) {
        for outbound in outbounds {
            let Some(item) = outbound.as_object() else {
                return Err("content.outbounds entries must be objects".to_string());
            };
            let tag = item
                .get("tag")
                .and_then(Value::as_str)
                .map(str::trim)
                .unwrap_or("");
            if tag.is_empty() {
                return Err("content.outbounds entries must have non-empty tag".to_string());
            }
            let typ = item
                .get("type")
                .and_then(Value::as_str)
                .map(str::trim)
                .unwrap_or("");
            if typ.is_empty() {
                return Err("content.outbounds entries must have non-empty type".to_string());
            }
            if matches!(typ, "selector" | "urltest") {
                return Err(
                    "selector and urltest groups must be defined in policy_groups or node_groups"
                        .to_string(),
                );
            }
            base_tags.insert(tag.to_string());
        }
    }

    let mut node_group_tags: std::collections::HashSet<String> = std::collections::HashSet::new();
    if let Some(node_groups) = obj.get("node_groups").and_then(Value::as_array) {
        for group in node_groups {
            let Some(item) = group.as_object() else {
                return Err("content.node_groups entries must be objects".to_string());
            };
            let tag = item
                .get("tag")
                .and_then(Value::as_str)
                .map(str::trim)
                .unwrap_or("");
            if tag.is_empty() {
                return Err("node group must have a non-empty tag".to_string());
            }
            if !node_group_tags.insert(tag.to_string()) {
                return Err(format!("duplicate node group tag '{tag}'"));
            }
            let typ = item
                .get("type")
                .and_then(Value::as_str)
                .map(str::trim)
                .unwrap_or("");
            if typ.is_empty() {
                return Err(format!("node group '{tag}' must have a non-empty type"));
            }
            let Some(targets) = item.get("outbounds").and_then(Value::as_array) else {
                return Err(format!(
                    "node group '{tag}' outbounds must be an array of strings"
                ));
            };
            for target_val in targets {
                let Some(target) = target_val.as_str() else {
                    return Err(format!(
                        "node group '{tag}' outbounds must be an array of strings"
                    ));
                };
                let trimmed = target.trim();
                if !trimmed.starts_with('{') || !trimmed.ends_with('}') || trimmed.len() < 2 {
                    return Err(format!(
                        "node group '{tag}' outbound '{target}' must be a regex pattern enclosed in '{{...}}'"
                    ));
                }
                let pattern = trimmed[1..trimmed.len() - 1].trim();
                if pattern.is_empty() {
                    return Err(format!(
                        "node group '{tag}' outbound pattern cannot be empty"
                    ));
                }
                regex::RegexBuilder::new(pattern)
                    .case_insensitive(true)
                    .build()
                    .map_err(|err| {
                        format!("node group '{tag}' has invalid regex pattern '{pattern}': {err}")
                    })?;
            }
        }
    }

    if let Some(policy_groups) = obj.get("policy_groups").and_then(Value::as_array) {
        let mut policy_group_tags: std::collections::HashSet<String> =
            std::collections::HashSet::new();
        for group in policy_groups {
            let Some(item) = group.as_object() else {
                return Err("content.policy_groups entries must be objects".to_string());
            };
            let tag = item
                .get("tag")
                .and_then(Value::as_str)
                .map(str::trim)
                .unwrap_or("");
            if tag.is_empty() {
                return Err("policy group must have a non-empty tag".to_string());
            }
            if node_group_tags.contains(tag) {
                return Err(format!(
                    "policy group tag '{tag}' collides with node group tag"
                ));
            }
            if !policy_group_tags.insert(tag.to_string()) {
                return Err(format!("duplicate policy group tag '{tag}'"));
            }
            let typ = item
                .get("type")
                .and_then(Value::as_str)
                .map(str::trim)
                .unwrap_or("");
            if typ.is_empty() {
                return Err(format!("policy group '{tag}' must have a non-empty type"));
            }
            let Some(targets) = item.get("outbounds").and_then(Value::as_array) else {
                return Err(format!(
                    "policy group '{tag}' outbounds must be an array of strings"
                ));
            };
            if targets.is_empty() {
                return Err(format!("policy group '{tag}' outbounds cannot be empty"));
            }
            for target_val in targets {
                let Some(target) = target_val.as_str() else {
                    return Err(format!(
                        "policy group '{tag}' outbounds must be an array of strings"
                    ));
                };
                if target.starts_with('{') && target.ends_with('}') {
                    return Err(format!(
                        "policy group '{tag}' cannot contain regex pattern '{target}'; candidates must be node groups or basic outbounds"
                    ));
                }
                let is_allowed = node_group_tags.contains(target)
                    || base_tags.contains(target)
                    || matches!(target, "直连" | "direct" | "reject" | "block");
                if !is_allowed {
                    return Err(format!(
                        "policy group '{tag}' references unknown target '{target}'; candidates must be node groups or basic outbounds"
                    ));
                }
            }
        }
    }

    let mut defined_rule_set_tags: std::collections::HashSet<String> =
        std::collections::HashSet::new();

    if let Some(rule_sets) = obj.get("rule_sets").and_then(Value::as_array) {
        for rs in rule_sets {
            let Some(item) = rs.as_object() else {
                return Err("content.rule_sets entries must be objects".to_string());
            };
            let typ = item
                .get("type")
                .and_then(Value::as_str)
                .map(str::trim)
                .unwrap_or("");
            if typ.is_empty() {
                return Err("rule_set entry must have a non-empty type".to_string());
            }

            let mut tags = Vec::new();
            if let Some(tag_str) = item.get("tag").and_then(Value::as_str) {
                let trimmed = tag_str.trim();
                if trimmed.is_empty() {
                    return Err("rule_set tag cannot be empty".to_string());
                }
                tags.push(trimmed.to_string());
            } else if let Some(tag_arr) = item.get("tag").and_then(Value::as_array) {
                if tag_arr.is_empty() {
                    return Err("rule_set tag array cannot be empty".to_string());
                }
                for t in tag_arr {
                    let Some(s) = t.as_str().map(str::trim) else {
                        return Err("rule_set tag array elements must be strings".to_string());
                    };
                    if s.is_empty() {
                        return Err("rule_set tag element cannot be empty".to_string());
                    }
                    tags.push(s.to_string());
                }
            } else {
                return Err("rule_set entry must have a tag string or array of tags".to_string());
            }

            for tag in tags {
                if !defined_rule_set_tags.insert(tag.clone()) {
                    return Err(format!("duplicate rule_set tag '{tag}'"));
                }
            }
        }
    }

    if let Some(route_obj) = obj.get("route").and_then(Value::as_object)
        && let Some(rule_sets) = route_obj.get("rule_set").and_then(Value::as_array)
    {
        for rs in rule_sets {
            if let Some(tag) = rs.get("tag").and_then(Value::as_str).map(str::trim)
                && !tag.is_empty()
            {
                defined_rule_set_tags.insert(tag.to_string());
            } else if let Some(tag_arr) = rs.get("tag").and_then(Value::as_array) {
                for t in tag_arr {
                    if let Some(tag) = t.as_str().map(str::trim)
                        && !tag.is_empty()
                    {
                        defined_rule_set_tags.insert(tag.to_string());
                    }
                }
            }
        }
    }

    if let Some(route_obj) = obj.get("route").and_then(Value::as_object)
        && let Some(rules) = route_obj.get("rules").and_then(Value::as_array)
    {
        for rule in rules {
            if let Some(rule_obj) = rule.as_object()
                && let Some(target_rs) = rule_obj.get("rule_set")
            {
                validate_rule_set_reference(target_rs, &defined_rule_set_tags, "route rule")?;
            }
        }
    }

    if let Some(dns_obj) = obj.get("dns").and_then(Value::as_object)
        && let Some(rules) = dns_obj.get("rules").and_then(Value::as_array)
    {
        for rule in rules {
            if let Some(rule_obj) = rule.as_object()
                && let Some(target_rs) = rule_obj.get("rule_set")
            {
                validate_rule_set_reference(target_rs, &defined_rule_set_tags, "dns rule")?;
            }
        }
    }

    Ok(())
}

fn validate_rule_set_reference(
    val: &Value,
    defined: &std::collections::HashSet<String>,
    scope: &str,
) -> std::result::Result<(), String> {
    if let Some(s) = val.as_str() {
        let tag = s.trim();
        if !tag.is_empty() && !defined.contains(tag) {
            return Err(format!("{scope} references undefined rule_set '{tag}'"));
        }
    } else if let Some(arr) = val.as_array() {
        for item in arr {
            if let Some(s) = item.as_str() {
                let tag = s.trim();
                if !tag.is_empty() && !defined.contains(tag) {
                    return Err(format!("{scope} references undefined rule_set '{tag}'"));
                }
            }
        }
    }
    Ok(())
}

fn validate_optional_object(
    obj: &serde_json::Map<String, Value>,
    key: &str,
) -> std::result::Result<(), String> {
    match obj.get(key) {
        None | Some(Value::Null) => Ok(()),
        Some(Value::Object(_)) => Ok(()),
        Some(_) => Err(format!("content.{key} must be an object")),
    }
}

fn validate_optional_array_of_objects(
    obj: &serde_json::Map<String, Value>,
    key: &str,
) -> std::result::Result<(), String> {
    match obj.get(key) {
        None | Some(Value::Null) => Ok(()),
        Some(Value::Array(arr)) => {
            if arr.iter().any(|item| !item.is_object()) {
                return Err(format!("content.{key} entries must be objects"));
            }
            Ok(())
        }
        Some(_) => Err(format!("content.{key} must be an array")),
    }
}

pub fn new_source_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("src_{nanos:x}")
}

pub fn now_rfc3339() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format_unix_utc(secs)
}

fn format_unix_utc(secs: u64) -> String {
    let days = (secs / 86400) as i64;
    let rem = secs % 86400;
    let hour = rem / 3600;
    let min = (rem % 3600) / 60;
    let sec = rem % 60;
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{min:02}:{sec:02}Z")
}

/// Unix epoch day 0 = 1970-01-01. Algorithm by Howard Hinnant.
fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m as u32, d as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_formats_unix_epoch() {
        assert_eq!(format_unix_utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(format_unix_utc(1_704_067_200), "2024-01-01T00:00:00Z");
        assert_eq!(format_unix_utc(1_704_067_261), "2024-01-01T00:01:01Z");
    }

    #[test]
    fn test_validates_template_content_shape() {
        use serde_json::json;

        assert!(validate_template_content(&json!([])).is_err());
        assert!(validate_template_content(&json!({})).is_ok());
        assert!(
            validate_template_content(&json!({
                "log": { "level": "warn" },
                "node_groups": [{
                    "type": "urltest",
                    "tag": "香港节点",
                    "outbounds": ["{(?i)(港|hk)}"]
                }],
                "policy_groups": [{
                    "type": "selector",
                    "tag": "默认策略",
                    "outbounds": ["香港节点", "直连"]
                }],
                "outbounds": [{ "type": "direct", "tag": "直连" }]
            }))
            .is_ok()
        );
        // selector in base outbounds must be rejected
        assert!(
            validate_template_content(&json!({
                "outbounds": [{ "type": "selector", "tag": "g", "outbounds": ["direct"] }]
            }))
            .is_err()
        );
        // policy group with non-string target must be rejected
        assert!(
            validate_template_content(&json!({
                "policy_groups": [{ "type": "selector", "tag": "g", "outbounds": [1] }]
            }))
            .is_err()
        );
        // policy group with regex pattern must be rejected
        assert!(
            validate_template_content(&json!({
                "policy_groups": [{ "type": "selector", "tag": "g", "outbounds": ["{(?i)hk}"] }]
            }))
            .is_err()
        );
        // policy group referencing unknown node group must be rejected
        assert!(
            validate_template_content(&json!({
                "policy_groups": [{ "type": "selector", "tag": "g", "outbounds": ["unknown_group"] }]
            }))
            .is_err()
        );
        // tag collision between policy group and node group must be rejected
        assert!(
            validate_template_content(&json!({
                "node_groups": [{ "type": "urltest", "tag": "hk", "outbounds": ["{(?i)hk}"] }],
                "policy_groups": [{ "type": "selector", "tag": "hk", "outbounds": ["direct"] }]
            }))
            .is_err()
        );
        // node group with non-braced target must be rejected
        assert!(
            validate_template_content(&json!({
                "node_groups": [{ "type": "urltest", "tag": "hk", "outbounds": ["hk-01"] }]
            }))
            .is_err()
        );
        // node group with invalid regex pattern must be rejected
        assert!(
            validate_template_content(&json!({
                "node_groups": [{ "type": "urltest", "tag": "hk", "outbounds": ["{[}"] }]
            }))
            .is_err()
        );
        // node group with empty pattern must be rejected
        assert!(
            validate_template_content(&json!({
                "node_groups": [{ "type": "urltest", "tag": "hk", "outbounds": ["{}"] }]
            }))
            .is_err()
        );
        // valid rule_sets with multi-tag and route references
        assert!(
            validate_template_content(&json!({
                "rule_sets": [
                    {
                        "tag": ["cn", "apple"],
                        "type": "remote",
                        "format": "binary",
                        "url": "https://example.com/{tag}.srs"
                    },
                    {
                        "tag": "ai",
                        "type": "remote",
                        "format": "binary",
                        "url": "https://example.com/ai.srs"
                    }
                ],
                "route": {
                    "rules": [
                        { "rule_set": ["cn", "ai"], "outbound": "direct" }
                    ]
                },
                "dns": {
                    "rules": [
                        { "rule_set": "apple", "server": "dns_direct" }
                    ]
                }
            }))
            .is_ok()
        );

        // duplicate rule_set tag rejected
        assert!(
            validate_template_content(&json!({
                "rule_sets": [
                    { "tag": ["cn", "ai"], "type": "remote" },
                    { "tag": "cn", "type": "remote" }
                ]
            }))
            .is_err()
        );

        // route rule referencing undefined rule_set rejected
        assert!(
            validate_template_content(&json!({
                "rule_sets": [
                    { "tag": "cn", "type": "remote" }
                ],
                "route": {
                    "rules": [
                        { "rule_set": "unknown_tag", "outbound": "direct" }
                    ]
                }
            }))
            .is_err()
        );

        // dns rule referencing undefined rule_set rejected
        assert!(
            validate_template_content(&json!({
                "rule_sets": [
                    { "tag": "cn", "type": "remote" }
                ],
                "dns": {
                    "rules": [
                        { "rule_set": ["unknown_tag"], "server": "dns_direct" }
                    ]
                }
            }))
            .is_err()
        );

        assert!(!is_safe_id("../etc/passwd"));
        assert!(is_safe_id("tpl_abc-1"));
    }
}
