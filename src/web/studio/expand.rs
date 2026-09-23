use std::collections::{BTreeMap, HashMap, HashSet};

use regex::RegexBuilder;
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

    let policy_groups = template_content
        .get("policy_groups")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let node_groups = template_content
        .get("node_groups")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let base_outbounds = template_content
        .get("outbounds")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let fallback_tag = policy_groups
        .iter()
        .chain(base_outbounds.iter())
        .find_map(|outbound| {
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
    let mut expanded_node_groups: Vec<Value> = Vec::with_capacity(node_groups.len());

    for mut group in node_groups {
        let group_tag = node_tag(&group).unwrap_or("").to_string();
        let targets: Vec<String> = group
            .get("outbounds")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();

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
        if let Some(obj) = group.as_object_mut() {
            obj.insert(
                "outbounds".to_string(),
                Value::Array(new_targets.into_iter().map(Value::String).collect()),
            );
        }
        expanded_node_groups.push(group);
    }

    let (terminal_policies, selector_policies): (Vec<Value>, Vec<Value>) =
        policy_groups.into_iter().partition(|o| {
            matches!(
                o.get("type").and_then(Value::as_str),
                Some("direct" | "block")
            )
        });

    let mut assembled_outbounds: Vec<Value> = Vec::new();
    assembled_outbounds.extend(selector_policies);
    assembled_outbounds.extend(expanded_node_groups);
    assembled_outbounds.extend(terminal_policies);
    assembled_outbounds.extend(base_outbounds);

    if needs_synthetic_direct
        || assembled_outbounds
            .iter()
            .all(|o| o.get("type").and_then(Value::as_str) != Some("direct"))
    {
        ensure_direct_outbound(&mut assembled_outbounds);
    }

    let mut existing_tags: HashSet<String> = HashSet::new();
    for outbound in &assembled_outbounds {
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
            assembled_outbounds.push(node.clone());
            existing_tags.insert(tag);
        }
    }

    let mut config = template_content.clone();
    if let Some(obj) = config.as_object_mut() {
        obj.remove("policy_groups");
        obj.remove("node_groups");
        obj.insert("outbounds".to_string(), Value::Array(assembled_outbounds));

        if let Some(mut rule_sets_val) = obj.remove("rule_sets") {
            if let Some(rule_sets_arr) = rule_sets_val.as_array_mut() {
                rule_sets_arr.retain(|rs| {
                    if let Some(arr) = rs.get("tag").and_then(Value::as_array) {
                        !arr.is_empty()
                    } else if let Some(s) = rs.get("tag").and_then(Value::as_str) {
                        !s.trim().is_empty()
                    } else {
                        false
                    }
                });
                for rs in rule_sets_arr.iter_mut() {
                    if let Some(rs_obj) = rs.as_object_mut() {
                        rs_obj.remove("name");
                        rs_obj.remove("source_url");
                        rs_obj.remove("tag_prefix");
                        rs_obj.remove("category");
                        rs_obj.remove("preset_id");
                    }
                }
            }
            let route_entry = obj
                .entry("route".to_string())
                .or_insert_with(|| Value::Object(serde_json::Map::new()));
            if let Some(route_obj) = route_entry.as_object_mut() {
                route_obj.insert("rule_set".to_string(), rule_sets_val);
            }
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
    let Ok(re) = RegexBuilder::new(pattern).case_insensitive(true).build() else {
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
            "node_groups": [
                { "type": "urltest", "tag": "香港节点", "outbounds": ["{(?i)(港|hk)}"] }
            ],
            "outbounds": [
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
            "node_groups": [
                { "type": "selector", "tag": "My 节点", "outbounds": ["{My-}"] },
                { "type": "urltest", "tag": "ALL", "outbounds": ["{.*}"] }
            ],
            "outbounds": [
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
            "node_groups": [
                { "type": "selector", "tag": "香港节点", "outbounds": ["{(?i)hk}"] }
            ],
            "outbounds": [
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
            "node_groups": [
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
            "node_groups": [
                { "type": "urltest", "tag": "g", "outbounds": ["{[}"] }
            ],
            "outbounds": [
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
            "policy_groups": [
                { "type": "selector", "tag": "默认策略", "outbounds": ["香港节点", "直连"] }
            ],
            "node_groups": [
                { "type": "selector", "tag": "香港节点", "outbounds": ["HK-01"] }
            ],
            "outbounds": [
                { "type": "direct", "tag": "直连" }
            ]
        });
        let expansion = expand_profile(&template, &[vless("HK-01"), vless("JP-01")]);
        assert_eq!(
            tags_of(&expansion.config["outbounds"][0]),
            vec!["香港节点", "直连"]
        );
        assert_eq!(tags_of(&expansion.config["outbounds"][1]), vec!["HK-01"]);
        assert_eq!(
            outbound_tags(&expansion.config),
            vec!["默认策略", "香港节点", "直连", "HK-01"]
        );
        assert_eq!(expansion.used_count, 0);
    }

    #[test]
    fn test_does_not_overwrite_existing_outbound_with_same_tag() {
        let template = json!({
            "node_groups": [
                { "type": "selector", "tag": "HK-01", "outbounds": ["{(?i)hk}"] }
            ],
            "outbounds": [
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
            "node_groups": [
                { "type": "selector", "tag": "g", "outbounds": ["{(?i)hk}", "直连"] }
            ],
            "outbounds": [
                { "type": "direct", "tag": "直连" }
            ]
        });
        let expansion = expand_profile(&template, &[vless("JP-01")]);
        assert_eq!(tags_of(&expansion.config["outbounds"][0]), vec!["直连"]);
    }

    #[test]
    fn test_first_seen_node_tag_wins() {
        let template = json!({
            "node_groups": [
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

    #[test]
    fn test_expands_rule_sets_into_route_rule_set() {
        let template = json!({
            "outbounds": [{ "type": "direct", "tag": "直连" }],
            "rule_sets": [
                {
                    "type": "remote",
                    "tag": ["cn", "ai", "netflix"],
                    "format": "binary",
                    "url": "https://github.com/DustinWin/ruleset_geodata/releases/download/sing-box-ruleset/{tag}.srs",
                    "download_detour": "ALL"
                }
            ],
            "route": {
                "rules": [
                    { "rule_set": ["cn"], "outbound": "直连" }
                ]
            }
        });
        let expansion = expand_profile(&template, &[]);
        assert!(expansion.config.get("rule_sets").is_none());
        assert_eq!(
            expansion.config["route"]["rule_set"][0]["tag"],
            json!(["cn", "ai", "netflix"])
        );
        assert_eq!(expansion.config["route"]["rule_set"][0]["format"], "binary");
    }
    #[test]
    fn test_expands_with_direct_in_policy_groups_and_no_base_outbounds() {
        let template = json!({
            "node_groups": [
                { "type": "urltest", "tag": "香港节点", "outbounds": ["{(?i)(港|hk)}"] }
            ],
            "policy_groups": [
                { "type": "direct", "tag": "直连" },
                { "type": "block", "tag": "block" },
                { "type": "selector", "tag": "默认策略", "outbounds": ["香港节点", "直连"] }
            ]
        });
        let nodes = vec![vless("HK-01")];
        let expansion = expand_profile(&template, &nodes);

        assert_eq!(
            outbound_tags(&expansion.config),
            vec!["默认策略", "香港节点", "直连", "block", "HK-01"]
        );
        assert_eq!(expansion.config["outbounds"][0]["type"], "selector");
        assert_eq!(expansion.config["outbounds"][1]["type"], "urltest");
        assert_eq!(expansion.config["outbounds"][2]["type"], "direct");
        assert_eq!(expansion.config["outbounds"][3]["type"], "block");
    }
}
