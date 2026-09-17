use std::collections::{BTreeMap, HashMap, HashSet};

use regex::Regex;
use serde_json::{Value, json};

#[derive(Debug, Clone, PartialEq)]
pub struct Expansion {
    pub config: Value,
    pub matched_map: BTreeMap<String, Vec<String>>,
    pub total_nodes: usize,
    pub used_count: usize,
}

pub fn expand_profile(template_content: &Value, nodes: &[Value]) -> Expansion {
    let mut nodes_by_tag: HashMap<String, Value> = HashMap::new();
    let mut tag_order: Vec<String> = Vec::new();
    for node in nodes {
        let Some(tag) = node_tag(node) else {
            continue;
        };
        if nodes_by_tag.contains_key(tag) {
            continue;
        }
        nodes_by_tag.insert(tag.to_string(), node.clone());
        tag_order.push(tag.to_string());
    }
    let total_nodes = tag_order.len();

    let mut config = template_content.clone();
    let Some(outbounds) = config.get_mut("outbounds").and_then(Value::as_array_mut) else {
        return Expansion {
            config,
            matched_map: BTreeMap::new(),
            total_nodes,
            used_count: 0,
        };
    };

    let fallback_tag = outbounds.iter().find_map(|outbound| {
        if outbound.get("type").and_then(Value::as_str) == Some("direct") {
            node_tag(outbound).map(str::to_string)
        } else {
            None
        }
    });

    let mut matched_map = BTreeMap::new();
    let mut regex_hits: HashSet<String> = HashSet::new();
    let mut referenced_tags: HashSet<String> = HashSet::new();
    let mut needs_synthetic_direct = false;

    for outbound in outbounds.iter_mut() {
        let kind = outbound
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        if !matches!(kind.as_str(), "selector" | "urltest") {
            continue;
        }
        let group_tag = outbound
            .get("tag")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let Some(targets) = outbound.get("outbounds").and_then(Value::as_array) else {
            continue;
        };
        let targets: Vec<String> = targets
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect();

        let mut new_targets: Vec<String> = Vec::new();
        let mut group_hits: Vec<String> = Vec::new();
        let mut seen_in_group: HashSet<String> = HashSet::new();

        for target in targets {
            if let Some(pattern) = placeholder_pattern(&target) {
                let mut hits = match_tags(pattern, &tag_order);
                hits.sort();
                hits.dedup();
                for tag in hits {
                    regex_hits.insert(tag.clone());
                    if seen_in_group.insert(tag.clone()) {
                        group_hits.push(tag.clone());
                        new_targets.push(tag);
                    }
                }
            } else if seen_in_group.insert(target.clone()) {
                new_targets.push(target);
            }
        }

        if new_targets.is_empty() {
            match fallback_tag.clone() {
                Some(tag) => new_targets.push(tag),
                None => {
                    needs_synthetic_direct = true;
                    new_targets.push("direct".to_string());
                }
            }
        }

        for tag in &new_targets {
            if nodes_by_tag.contains_key(tag) {
                referenced_tags.insert(tag.clone());
            }
        }

        group_hits.sort();
        matched_map.insert(group_tag, group_hits);
        if let Some(obj) = outbound.as_object_mut() {
            obj.insert(
                "outbounds".to_string(),
                Value::Array(new_targets.into_iter().map(Value::String).collect()),
            );
        }
    }

    if needs_synthetic_direct {
        ensure_direct_outbound(outbounds);
    }

    let mut existing_tags: HashSet<String> = HashSet::new();
    for outbound in outbounds.iter() {
        if let Some(tag) = node_tag(outbound) {
            existing_tags.insert(tag.to_string());
        }
    }

    let mut to_append: Vec<String> = referenced_tags.into_iter().collect();
    to_append.sort();
    for tag in to_append {
        if existing_tags.contains(&tag) {
            continue;
        }
        if let Some(node) = nodes_by_tag.get(&tag) {
            outbounds.push(node.clone());
            existing_tags.insert(tag);
        }
    }

    Expansion {
        config,
        matched_map,
        total_nodes,
        used_count: regex_hits.len(),
    }
}

fn node_tag(node: &Value) -> Option<&str> {
    node.get("tag")
        .and_then(Value::as_str)
        .filter(|tag| !tag.is_empty())
}

fn placeholder_pattern(target: &str) -> Option<&str> {
    if target.len() >= 2 && target.starts_with('{') && target.ends_with('}') {
        Some(&target[1..target.len() - 1])
    } else {
        None
    }
}

fn match_tags(pattern: &str, tags: &[String]) -> Vec<String> {
    if pattern.is_empty() {
        return Vec::new();
    }
    let Ok(re) = Regex::new(pattern) else {
        return Vec::new();
    };
    tags.iter()
        .filter(|tag| re.is_match(tag))
        .cloned()
        .collect()
}

fn ensure_direct_outbound(outbounds: &mut Vec<Value>) {
    if outbounds
        .iter()
        .any(|outbound| outbound.get("type").and_then(Value::as_str) == Some("direct"))
    {
        return;
    }
    outbounds.push(json!({ "type": "direct", "tag": "direct" }));
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn vless(tag: &str) -> Value {
        json!({ "type": "vless", "tag": tag, "server": "example.com", "server_port": 443 })
    }

    fn tags_of(group: &Value) -> Vec<String> {
        group["outbounds"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect()
    }

    fn outbound_tags(config: &Value) -> Vec<String> {
        config["outbounds"]
            .as_array()
            .unwrap()
            .iter()
            .map(|o| o["tag"].as_str().unwrap().to_string())
            .collect()
    }

    #[test]
    fn test_expands_case_insensitive_region_placeholder() {
        let template = json!({
            "outbounds": [
                { "type": "urltest", "tag": "香港节点", "outbounds": ["{(?i)(港|hk)}"] },
                { "type": "direct", "tag": "直连" }
            ]
        });
        let nodes = vec![vless("JP-01"), vless("HK-01"), vless("港-02")];
        let expansion = expand_profile(&template, &nodes);

        assert_eq!(
            tags_of(&expansion.config["outbounds"][0]),
            vec!["HK-01", "港-02"]
        );
        assert_eq!(
            expansion.matched_map.get("香港节点").unwrap(),
            &vec!["HK-01".to_string(), "港-02".to_string()]
        );
        assert_eq!(expansion.total_nodes, 3);
        assert_eq!(expansion.used_count, 2);
        assert_eq!(
            outbound_tags(&expansion.config),
            vec!["香港节点", "直连", "HK-01", "港-02"]
        );
        assert_eq!(expansion.config["outbounds"][2]["type"], "vless");
        assert_eq!(expansion.config["outbounds"][2]["server"], "example.com");
        assert!(
            expansion.config["outbounds"]
                .as_array()
                .unwrap()
                .iter()
                .all(|o| o["tag"].as_str().unwrap() != "JP-01")
        );
    }

    #[test]
    fn test_expands_wildcard_and_prefix_placeholders() {
        let template = json!({
            "outbounds": [
                { "type": "selector", "tag": "My 节点", "outbounds": ["{My-}"] },
                { "type": "urltest", "tag": "ALL", "outbounds": ["{.*}"] },
                { "type": "direct", "tag": "直连" }
            ]
        });
        let nodes = vec![vless("World-US-01"), vless("My-VPS"), vless("HK-01")];
        let expansion = expand_profile(&template, &nodes);

        assert_eq!(tags_of(&expansion.config["outbounds"][0]), vec!["My-VPS"]);
        assert_eq!(
            tags_of(&expansion.config["outbounds"][1]),
            vec!["HK-01", "My-VPS", "World-US-01"]
        );
        assert_eq!(expansion.used_count, 3);
    }

    #[test]
    fn test_falls_back_to_existing_direct_tag_when_regex_matches_nothing() {
        let template = json!({
            "outbounds": [
                { "type": "selector", "tag": "香港节点", "outbounds": ["{(?i)hk}"] },
                { "type": "direct", "tag": "直连" }
            ]
        });
        let expansion = expand_profile(&template, &[vless("JP-01")]);
        assert_eq!(tags_of(&expansion.config["outbounds"][0]), vec!["直连"]);
        assert_eq!(expansion.used_count, 0);
        assert_eq!(
            expansion.matched_map.get("香港节点").unwrap(),
            &Vec::<String>::new()
        );
    }

    #[test]
    fn test_synthesizes_direct_when_no_direct_outbound_exists() {
        let template = json!({
            "outbounds": [
                { "type": "selector", "tag": "g", "outbounds": ["{nomatch}"] }
            ]
        });
        let expansion = expand_profile(&template, &[vless("HK-01")]);
        assert_eq!(tags_of(&expansion.config["outbounds"][0]), vec!["direct"]);
        assert_eq!(outbound_tags(&expansion.config), vec!["g", "direct"]);
        assert_eq!(expansion.config["outbounds"][1]["type"], "direct");
    }

    #[test]
    fn test_invalid_regex_is_treated_as_zero_matches() {
        let template = json!({
            "outbounds": [
                { "type": "urltest", "tag": "g", "outbounds": ["{[}"] },
                { "type": "direct", "tag": "直连" }
            ]
        });
        let expansion = expand_profile(&template, &[vless("HK-01")]);
        assert_eq!(tags_of(&expansion.config["outbounds"][0]), vec!["直连"]);
        assert_eq!(expansion.used_count, 0);
    }

    #[test]
    fn test_keeps_literals_and_appends_referenced_source_nodes() {
        let template = json!({
            "outbounds": [
                { "type": "selector", "tag": "默认策略", "outbounds": ["HK-01", "直连"] },
                { "type": "direct", "tag": "直连" }
            ]
        });
        let expansion = expand_profile(&template, &[vless("HK-01"), vless("JP-01")]);
        assert_eq!(
            tags_of(&expansion.config["outbounds"][0]),
            vec!["HK-01", "直连"]
        );
        assert_eq!(
            outbound_tags(&expansion.config),
            vec!["默认策略", "直连", "HK-01"]
        );
        assert_eq!(expansion.used_count, 0);
    }

    #[test]
    fn test_does_not_overwrite_existing_outbound_with_same_tag() {
        let template = json!({
            "outbounds": [
                { "type": "selector", "tag": "HK-01", "outbounds": ["{(?i)hk}"] },
                { "type": "direct", "tag": "直连" }
            ]
        });
        let expansion = expand_profile(&template, &[vless("HK-01")]);
        assert_eq!(tags_of(&expansion.config["outbounds"][0]), vec!["HK-01"]);
        assert_eq!(outbound_tags(&expansion.config), vec!["HK-01", "直连"]);
        assert_eq!(expansion.config["outbounds"][0]["type"], "selector");
    }

    #[test]
    fn test_mixed_placeholder_keeps_literal_without_extra_fallback() {
        let template = json!({
            "outbounds": [
                { "type": "selector", "tag": "g", "outbounds": ["{(?i)hk}", "直连"] },
                { "type": "direct", "tag": "直连" }
            ]
        });
        let expansion = expand_profile(&template, &[vless("JP-01")]);
        assert_eq!(tags_of(&expansion.config["outbounds"][0]), vec!["直连"]);
    }

    #[test]
    fn test_first_seen_node_tag_wins() {
        let template = json!({
            "outbounds": [
                { "type": "urltest", "tag": "ALL", "outbounds": ["{.*}"] }
            ]
        });
        let first = json!({ "type": "vless", "tag": "HK-01", "server": "first.example" });
        let second = json!({ "type": "trojan", "tag": "HK-01", "server": "second.example" });
        let expansion = expand_profile(&template, &[first, second]);
        let appended = expansion.config["outbounds"]
            .as_array()
            .unwrap()
            .iter()
            .find(|o| o["tag"] == "HK-01")
            .unwrap();
        assert_eq!(appended["type"], "vless");
        assert_eq!(appended["server"], "first.example");
        assert_eq!(expansion.total_nodes, 1);
    }
}
