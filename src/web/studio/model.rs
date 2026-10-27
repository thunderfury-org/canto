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
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        alias = "exclude_protocols"
    )]
    pub exclude_protocols: Vec<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        alias = "include_pattern"
    )]
    pub include_pattern: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        alias = "exclude_pattern"
    )]
    pub exclude_pattern: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty", alias = "rename_rules")]
    pub rename_rules: Vec<RenameRule>,
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "prefix")]
    pub prefix: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameRule {
    pub pattern: String,
    pub replacement: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FilterReason {
    #[serde(rename = "协议黑名单")]
    ProtocolBlacklisted,
    #[serde(rename = "未命中包含")]
    UnmatchedInclude,
    #[serde(rename = "命中排除")]
    MatchedExclude,
}

impl FilterReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ProtocolBlacklisted => "协议黑名单",
            Self::UnmatchedInclude => "未命中包含",
            Self::MatchedExclude => "命中排除",
        }
    }
}

pub fn strip_leading_case_flag(mut s: &str) -> &str {
    s = s.trim();
    while s.starts_with("(?i)") {
        s = s[4..].trim();
    }
    s
}

pub fn validate_filter_regex(pattern: &str) -> std::result::Result<(), String> {
    let p = strip_leading_case_flag(pattern);
    if p.is_empty() {
        return Ok(());
    }
    regex::RegexBuilder::new(p)
        .case_insensitive(true)
        .build()
        .map(|_| ())
        .map_err(|err| err.to_string())
}

pub fn normalize_pattern(val: Option<String>) -> Option<String> {
    val.and_then(|raw| {
        let stripped = strip_leading_case_flag(&raw);
        if stripped.is_empty() {
            None
        } else {
            Some(stripped.to_string())
        }
    })
}

pub fn normalize_protocols(protocols: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    for p in protocols {
        let trimmed = p.trim().to_ascii_lowercase();
        if !trimmed.is_empty() && !out.contains(&trimmed) {
            out.push(trimmed);
        }
    }
    out
}

pub fn validate_rename_regex(pattern: &str) -> std::result::Result<(), String> {
    let p = strip_leading_case_flag(pattern);
    if p.is_empty() {
        return Err("pattern cannot be empty".to_string());
    }
    regex::RegexBuilder::new(p)
        .case_insensitive(true)
        .build()
        .map(|_| ())
        .map_err(|err| err.to_string())
}

pub fn normalize_prefix(val: Option<String>) -> Option<String> {
    match val {
        Some(s) if !s.is_empty() => Some(s),
        _ => None,
    }
}

pub fn normalize_rename_rules(rules: Vec<RenameRule>) -> Vec<RenameRule> {
    rules
        .into_iter()
        .map(|rule| RenameRule {
            pattern: strip_leading_case_flag(&rule.pattern).to_string(),
            replacement: rule.replacement,
        })
        .collect()
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

    pub fn classify_node(&self, node: &Value) -> Option<FilterReason> {
        let exclude_protocols: std::collections::HashSet<String> = self
            .exclude_protocols
            .iter()
            .map(|p| p.trim().to_ascii_lowercase())
            .filter(|p| !p.is_empty())
            .collect();

        let typ = node
            .get("type")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or("")
            .to_ascii_lowercase();
        if exclude_protocols.contains(&typ) {
            return Some(FilterReason::ProtocolBlacklisted);
        }

        let tag = node.get("tag").and_then(Value::as_str).unwrap_or("");

        if let Some(p) = self.include_pattern.as_deref() {
            let stripped = strip_leading_case_flag(p);
            if !stripped.is_empty()
                && let Ok(re) = regex::RegexBuilder::new(stripped)
                    .case_insensitive(true)
                    .build()
                && !re.is_match(tag)
            {
                return Some(FilterReason::UnmatchedInclude);
            }
        }

        if let Some(p) = self.exclude_pattern.as_deref() {
            let stripped = strip_leading_case_flag(p);
            if !stripped.is_empty()
                && let Ok(re) = regex::RegexBuilder::new(stripped)
                    .case_insensitive(true)
                    .build()
                && re.is_match(tag)
            {
                return Some(FilterReason::MatchedExclude);
            }
        }

        None
    }

    pub fn effective_nodes(&self) -> Vec<Value> {
        // Step 0: Filter determination on raw nodes
        let filtered_nodes: Vec<&Value> = self
            .nodes
            .iter()
            .filter(|node| self.classify_node(node).is_none())
            .collect();

        // Precompile valid rename rules
        let compiled_rules: Vec<(regex::Regex, &str)> = self
            .rename_rules
            .iter()
            .filter_map(|rule| {
                let p = strip_leading_case_flag(&rule.pattern);
                if p.is_empty() {
                    return None;
                }
                regex::RegexBuilder::new(p)
                    .case_insensitive(true)
                    .build()
                    .ok()
                    .map(|re| (re, rule.replacement.as_str()))
            })
            .collect();

        // Step 1: Sequential batch regex replacement
        // Step 2: Prefix prepending (preserving trailing spaces)
        let mut staged_tags: Vec<String> = Vec::with_capacity(filtered_nodes.len());
        for node in &filtered_nodes {
            let mut tag = node
                .get("tag")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            for (re, replacement) in &compiled_rules {
                let replaced = re.replace_all(&tag, *replacement);
                if !replaced.trim().is_empty() {
                    tag = replaced.into_owned();
                }
            }
            if let Some(prefix) = self.prefix.as_deref()
                && !prefix.is_empty()
            {
                tag = format!("{prefix}{tag}");
            }
            staged_tags.push(tag);
        }

        // Step 3: Intra-source duplicate disambiguation in original node order
        let mut seen_counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        let mut used_tags: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut out = Vec::with_capacity(filtered_nodes.len());

        for (node, base_tag) in filtered_nodes.into_iter().zip(staged_tags) {
            let count = seen_counts.entry(base_tag.clone()).or_insert(0);
            *count += 1;

            let final_tag = if *count == 1 && !used_tags.contains(&base_tag) {
                used_tags.insert(base_tag.clone());
                base_tag
            } else {
                let mut suffix_idx = if *count == 1 { 2 } else { *count };
                loop {
                    let candidate = format!("{base_tag}-{suffix_idx}");
                    if !used_tags.contains(&candidate) {
                        used_tags.insert(candidate.clone());
                        break candidate;
                    }
                    suffix_idx += 1;
                }
            };

            let mut node_clone = node.clone();
            if let Some(obj) = node_clone.as_object_mut() {
                obj.insert("tag".to_string(), Value::String(final_tag));
            }
            out.push(node_clone);
        }

        out
    }

    pub fn effective_node_count(&self) -> usize {
        self.effective_nodes().len()
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateMeta {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
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

        // Pass 1: Validate tags, duplicate tags, types, and collect direct/block base tags
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
            if typ == "block" {
                return Err(format!(
                    "policy group '{tag}' uses removed type 'block'; reject is a route action"
                ));
            }
            if typ == "direct" {
                base_tags.insert(tag.to_string());
            } else if !matches!(typ, "selector" | "urltest") {
                return Err(format!(
                    "policy group '{tag}' has unsupported type '{typ}'; must be selector, urltest, or direct"
                ));
            }
        }

        // Pass 2: Validate selector/urltest outbounds targets
        for group in policy_groups {
            let Some(item) = group.as_object() else {
                continue;
            };
            let typ = item
                .get("type")
                .and_then(Value::as_str)
                .map(str::trim)
                .unwrap_or("");
            if matches!(typ, "direct" | "block") {
                continue;
            }
            let tag = item
                .get("tag")
                .and_then(Value::as_str)
                .map(str::trim)
                .unwrap_or("");
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
                    || matches!(target, "直连" | "direct");
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
            validate_rule_source_http_client(item)?;
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

fn validate_rule_source_http_client(
    item: &serde_json::Map<String, Value>,
) -> std::result::Result<(), String> {
    if item.contains_key("download_detour") {
        return Err("rule source uses removed field 'download_detour'".to_string());
    }
    let Some(client) = item.get("http_client") else {
        return Ok(());
    };
    let detour = client
        .as_object()
        .and_then(|obj| obj.get("detour"))
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or("");
    if detour.is_empty() || client.as_object().is_none_or(|obj| obj.len() != 1) {
        return Err("rule source http_client must be { \"detour\": \"<outbound>\" }".to_string());
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
        assert!(
            validate_template_content(&json!({
                "policy_groups": [
                    { "type": "direct", "tag": "直连" },
                    { "type": "block", "tag": "block" }
                ]
            }))
            .is_err()
        );
        assert!(
            validate_template_content(&json!({
                "policy_groups": [
                    { "type": "selector", "tag": "默认策略", "outbounds": ["block"] }
                ]
            }))
            .is_err()
        );
        assert!(
            validate_template_content(&json!({
                "rule_sets": [{
                    "type": "remote",
                    "tag": "cn",
                    "http_client": { "detour": "" }
                }]
            }))
            .is_err()
        );
        assert!(
            validate_template_content(&json!({
                "rule_sets": [{
                    "type": "remote",
                    "tag": "cn",
                    "http_client": { "detour": "ALL" }
                }]
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

    #[test]
    fn test_node_source_backward_compatibility() {
        let raw_json = r#"{
            "id": "src_legacy",
            "name": "Legacy Source",
            "type": "subscription",
            "url": "https://example.com/sub",
            "status": "active",
            "nodeCount": 2,
            "nodes": [
                { "tag": "HK-01", "type": "vless" },
                { "tag": "JP-01", "type": "vmess" }
            ]
        }"#;
        let source: NodeSource = serde_json::from_str(raw_json).unwrap();
        assert!(source.exclude_protocols.is_empty());
        assert!(source.include_pattern.is_none());
        assert!(source.exclude_pattern.is_none());
        assert!(source.rename_rules.is_empty());
        assert!(source.prefix.is_none());
        assert_eq!(source.effective_nodes().len(), 2);
    }

    #[test]
    fn test_validate_filter_regex() {
        assert!(validate_filter_regex("").is_ok());
        assert!(validate_filter_regex("(?i)(港|hk)").is_ok());
        assert!(validate_filter_regex("(hk").is_err());
        // Lookaround rejected
        assert!(validate_filter_regex("^(?!.*(官网)).*").is_err());
        assert!(validate_filter_regex("(?=.*hk)").is_err());
        // Backreference rejected
        assert!(validate_filter_regex(r"(a)\1").is_err());
    }

    #[test]
    fn test_node_filtering_priority() {
        let source = NodeSource {
            id: "src_test".to_string(),
            name: "Test".to_string(),
            kind: SourceKind::Manual,
            url: None,
            content: None,
            status: Default::default(),
            last_updated: None,
            last_error: None,
            node_count: 4,
            nodes: vec![
                serde_json::json!({ "tag": "HK-01 [官网]", "type": "vmess" }),
                serde_json::json!({ "tag": "JP-01", "type": "trojan" }),
                serde_json::json!({ "tag": "HK-02 [官网]", "type": "vless" }),
                serde_json::json!({ "tag": "HK-03 专线", "type": "vless" }),
            ],
            exclude_protocols: vec!["vmess".to_string()],
            include_pattern: Some("(?i)hk".to_string()),
            exclude_pattern: Some("(?i)官网".to_string()),
            rename_rules: Vec::new(),
            prefix: None,
        };
        // 1. HK-01 [官网] (vmess): matches exclude_pattern AND matches include, but vmess is blacklisted!
        assert_eq!(
            source.classify_node(&source.nodes[0]),
            Some(FilterReason::ProtocolBlacklisted)
        );
        // 2. JP-01 (trojan): not in blacklist, but tag doesn't match "hk"
        assert_eq!(
            source.classify_node(&source.nodes[1]),
            Some(FilterReason::UnmatchedInclude)
        );
        // 3. HK-02 [官网] (vless): matches "hk", but contains "官网"
        assert_eq!(
            source.classify_node(&source.nodes[2]),
            Some(FilterReason::MatchedExclude)
        );
        // 4. HK-03 专线 (vless): passes all
        assert_eq!(source.classify_node(&source.nodes[3]), None);

        let effective = source.effective_nodes();
        assert_eq!(effective.len(), 1);
        assert_eq!(effective[0]["tag"], "HK-03 专线");
        assert_eq!(source.effective_node_count(), 1);
    }

    #[test]
    fn test_validate_rename_regex() {
        assert!(validate_rename_regex("(?i)hk").is_ok());
        assert!(validate_rename_regex("(hk").is_err());
        assert!(validate_rename_regex("").is_err());
        assert!(validate_rename_regex("   ").is_err());
        assert!(validate_rename_regex("(?i)").is_err());
        // Lookaround rejected by regex crate
        assert!(validate_rename_regex("^(?!.*(官网)).*").is_err());
        // Backreference rejected
        assert!(validate_rename_regex(r"(a)\1").is_err());
    }

    #[test]
    fn test_pipeline_order_filter_rename_prefix_disambiguate() {
        let source = NodeSource {
            id: "src_pipeline".to_string(),
            name: "Pipeline Test".to_string(),
            kind: SourceKind::Manual,
            url: None,
            content: None,
            status: Default::default(),
            last_updated: None,
            last_error: None,
            node_count: 5,
            nodes: vec![
                serde_json::json!({ "tag": "HK-01 [官网]", "type": "vmess", "server": "1.1.1.1" }), // filtered by protocol
                serde_json::json!({ "tag": "JP-01", "type": "vless", "server": "2.2.2.2" }), // filtered by include pattern
                serde_json::json!({ "tag": "HK-01", "type": "vless", "server": "3.3.3.3" }), // passes filter
                serde_json::json!({ "tag": "HK-02", "type": "vless", "server": "4.4.4.4" }), // passes filter
                serde_json::json!({ "tag": "HK-03", "type": "vless", "server": "5.5.5.5" }), // passes filter
            ],
            exclude_protocols: vec!["vmess".to_string()],
            include_pattern: Some("(?i)hk".to_string()),
            exclude_pattern: None,
            rename_rules: vec![
                // 1. Rename all HK-\d+ to "香港"
                RenameRule {
                    pattern: r"(?i)hk-\d+".to_string(),
                    replacement: "香港".to_string(),
                },
                // 2. Empty replacement check: this rule tries to blank out the string, must fall back!
                RenameRule {
                    pattern: ".*".to_string(),
                    replacement: "   ".to_string(),
                },
            ],
            // Prefix with trailing space
            prefix: Some("[主力] ".to_string()),
        };

        let effective = source.effective_nodes();
        assert_eq!(effective.len(), 3);

        // Verification of Step 1 (rename to "香港"), Step 2 (prefix "[主力] "), and Step 3 (disambiguate -2, -3)
        assert_eq!(effective[0]["tag"], "[主力] 香港");
        assert_eq!(effective[1]["tag"], "[主力] 香港-2");
        assert_eq!(effective[2]["tag"], "[主力] 香港-3");

        // Raw nodes must remain untouched
        assert_eq!(source.nodes[2]["tag"], "HK-01");
        assert_eq!(source.nodes[3]["tag"], "HK-02");
        assert_eq!(source.nodes[4]["tag"], "HK-03");
    }

    #[test]
    fn test_pipeline_disambiguation_with_existing_suffix() {
        let source = NodeSource {
            id: "src_collision".to_string(),
            name: "Collision Test".to_string(),
            kind: SourceKind::Manual,
            url: None,
            content: None,
            status: Default::default(),
            last_updated: None,
            last_error: None,
            node_count: 3,
            nodes: vec![
                serde_json::json!({ "tag": "HK", "type": "vless" }),
                serde_json::json!({ "tag": "HK-2", "type": "vless" }),
                serde_json::json!({ "tag": "HK", "type": "vless" }),
            ],
            exclude_protocols: Vec::new(),
            include_pattern: None,
            exclude_pattern: None,
            rename_rules: Vec::new(),
            prefix: None,
        };

        let effective = source.effective_nodes();
        assert_eq!(effective.len(), 3);
        // Node 0 -> "HK"
        // Node 1 -> "HK-2"
        // Node 2 -> "HK" duplicate, attempts "HK-2" (already used), advances to "HK-3"
        assert_eq!(effective[0]["tag"], "HK");
        assert_eq!(effective[1]["tag"], "HK-2");
        assert_eq!(effective[2]["tag"], "HK-3");
    }

    #[test]
    fn test_capture_group_replacement() {
        let source = NodeSource {
            id: "src_cg".to_string(),
            name: "Capture Group".to_string(),
            kind: SourceKind::Manual,
            url: None,
            content: None,
            status: Default::default(),
            last_updated: None,
            last_error: None,
            node_count: 2,
            nodes: vec![
                serde_json::json!({ "tag": "HK 01", "type": "vless" }),
                serde_json::json!({ "tag": "US - 05", "type": "vless" }),
            ],
            exclude_protocols: Vec::new(),
            include_pattern: None,
            exclude_pattern: None,
            rename_rules: vec![RenameRule {
                pattern: r"(?i)([a-z]+)\s*[-]?\s*(\d+)".to_string(),
                replacement: "[$1] #$2".to_string(),
            }],
            prefix: Some("Node: ".to_string()),
        };

        let effective = source.effective_nodes();
        assert_eq!(effective.len(), 2);
        assert_eq!(effective[0]["tag"], "Node: [HK] #01");
        assert_eq!(effective[1]["tag"], "Node: [US] #05");
    }
}
