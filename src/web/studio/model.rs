use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

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

pub fn new_template_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("tpl_{nanos:x}")
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

    if let Some(outbounds) = obj.get("outbounds") {
        let Some(arr) = outbounds.as_array() else {
            return Err("content.outbounds must be an array".to_string());
        };
        for outbound in arr {
            let Some(item) = outbound.as_object() else {
                return Err("content.outbounds entries must be objects".to_string());
            };
            let kind = item.get("type").and_then(Value::as_str).unwrap_or("");
            if matches!(kind, "selector" | "urltest")
                && let Some(targets) = item.get("outbounds")
            {
                let Some(targets) = targets.as_array() else {
                    return Err("strategy group outbounds must be an array of strings".to_string());
                };
                if targets.iter().any(|target| !target.is_string()) {
                    return Err("strategy group outbounds must be an array of strings".to_string());
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
                "outbounds": [{
                    "type": "urltest",
                    "tag": "香港节点",
                    "outbounds": ["{(?i)(港|hk)}"]
                }]
            }))
            .is_ok()
        );
        assert!(
            validate_template_content(&json!({
                "outbounds": [{ "type": "selector", "tag": "g", "outbounds": [1] }]
            }))
            .is_err()
        );
        assert!(!is_safe_id("../etc/passwd"));
        assert!(is_safe_id("tpl_abc-1"));
    }
}
