use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::{Value, json};
use std::collections::HashMap;

pub(super) struct CommonLink {
    pub(super) userinfo: Option<String>,
    pub(super) host: String,
    pub(super) port: u16,
    pub(super) query: HashMap<String, String>,
    pub(super) fragment: Option<String>,
}

pub(super) fn parse_common_link_full(rest: &str) -> Result<CommonLink, String> {
    let (body, fragment) = split_fragment(rest);
    parse_common_link(&body, fragment)
}

pub(super) fn parse_common_link(
    body: &str,
    fragment: Option<String>,
) -> Result<CommonLink, String> {
    let (without_query, query) = split_query(body);
    let (userinfo, hostport) = split_userinfo_host(without_query.trim_end_matches('/'));
    let (host, port) = split_host_port(&hostport)?;
    if host.is_empty() {
        return Err("missing server host".to_string());
    }
    Ok(CommonLink {
        userinfo,
        host,
        port,
        query,
        fragment,
    })
}

pub(super) fn tag_of(link: &CommonLink) -> String {
    link.fragment
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| format!("{}:{}", link.host, link.port))
}

pub(super) fn transport_from_query(query: &HashMap<String, String>) -> Option<Value> {
    let net = qget(query, &["type", "net", "network", "obfs"]).unwrap_or("tcp");
    let header = qget(query, &["headerType", "header_type"]).unwrap_or("none");
    let path = qget(query, &["path"]);
    let mut host = qget(query, &["host", "Host"]);
    if host.is_none() && matches!(net, "ws" | "websocket") {
        host = qget(query, &["sni", "peer"]);
    }
    let service = qget(query, &["serviceName", "service_name"]);
    build_transport(net, header, path, host, service)
}

pub(super) fn tls_from_query(query: &HashMap<String, String>, default_on: bool) -> Option<Value> {
    let security = qget(query, &["security"])
        .unwrap_or("")
        .to_ascii_lowercase();
    let public_key = qget(query, &["pbk", "public_key", "publicKey"]).map(ToString::to_string);
    let is_reality = security == "reality" || public_key.is_some();
    if security == "none" && !is_reality {
        return None;
    }

    let enabled =
        default_on || is_reality || security == "tls" || qget(query, &["sni", "peer"]).is_some();
    if !enabled {
        return None;
    }

    let sni = qget(query, &["sni", "peer", "server_name", "serverName"]).map(ToString::to_string);
    let insecure = qget(
        query,
        &[
            "insecure",
            "allowInsecure",
            "allow_insecure",
            "skip-cert-verify",
        ],
    )
    .map(truthy)
    .unwrap_or(false);
    let fingerprint = qget(query, &["fp", "fingerprint"]).map(ToString::to_string);
    let alpn = qget(query, &["alpn"]).map(split_csv);
    let short_id = qget(query, &["sid", "short_id", "shortId"]).map(ToString::to_string);

    Some(build_tls(
        sni,
        alpn,
        insecure,
        fingerprint,
        public_key,
        short_id,
    ))
}

pub(super) fn build_transport(
    net: &str,
    header_type: &str,
    path: Option<&str>,
    host: Option<&str>,
    service_name: Option<&str>,
) -> Option<Value> {
    let net = net.to_ascii_lowercase();
    let header = header_type.to_ascii_lowercase();
    let transport_type = match net.as_str() {
        "ws" | "websocket" => "ws",
        "httpupgrade" | "http_upgrade" => "httpupgrade",
        "grpc" => "grpc",
        "quic" => "quic",
        "h2" | "http" => "http",
        "tcp" | "raw" | "none" | "" => {
            if header == "http" {
                "http"
            } else {
                return None;
            }
        }
        other => other,
    };

    let mut obj = serde_json::Map::new();
    obj.insert("type".into(), json!(transport_type));
    match transport_type {
        "ws" => {
            let (path, early_data) = split_early_data(path);
            if let Some(path) = path {
                obj.insert("path".into(), json!(normalize_path(&path)));
            }
            if let Some(early_data) = early_data {
                obj.insert(
                    "early_data_header_name".into(),
                    json!("Sec-WebSocket-Protocol"),
                );
                obj.insert("max_early_data".into(), json!(early_data));
            }
            if let Some(host) = host {
                obj.insert("headers".into(), json!({ "Host": host }));
            }
        }
        "httpupgrade" => {
            if let Some(path) = path {
                obj.insert("path".into(), json!(normalize_path(path)));
            }
            if let Some(host) = host {
                obj.insert("host".into(), json!(host));
            }
        }
        "grpc" => {
            if let Some(name) = service_name.or(path) {
                obj.insert("service_name".into(), json!(name));
            }
        }
        "http" => {
            if let Some(path) = path {
                obj.insert("path".into(), json!(normalize_path(path)));
            }
            if let Some(host) = host {
                let hosts: Vec<String> = split_csv(host);
                obj.insert("host".into(), json!(hosts));
            }
        }
        _ => {}
    }
    Some(Value::Object(obj))
}

pub(super) fn build_tls(
    sni: Option<String>,
    alpn: Option<Vec<String>>,
    insecure: bool,
    fingerprint: Option<String>,
    public_key: Option<String>,
    short_id: Option<String>,
) -> Value {
    let mut tls = serde_json::Map::new();
    tls.insert("enabled".into(), json!(true));
    if let Some(sni) = sni.filter(|s| !s.is_empty()) {
        tls.insert("server_name".into(), json!(sni));
    }
    if insecure {
        tls.insert("insecure".into(), json!(true));
    }
    if let Some(alpn) = alpn.filter(|v| !v.is_empty()) {
        tls.insert("alpn".into(), json!(alpn));
    }
    let has_reality = public_key.as_ref().is_some_and(|s| !s.is_empty());
    if let Some(fingerprint) = fingerprint.filter(|s| !s.is_empty()) {
        tls.insert(
            "utls".into(),
            json!({ "enabled": true, "fingerprint": fingerprint }),
        );
    } else if has_reality {
        tls.insert("utls".into(), json!({ "enabled": true }));
    }
    if let Some(public_key) = public_key.filter(|s| !s.is_empty()) {
        let mut reality = serde_json::Map::new();
        reality.insert("enabled".into(), json!(true));
        reality.insert("public_key".into(), json!(public_key));
        if let Some(short_id) = short_id {
            reality.insert("short_id".into(), json!(short_id));
        }
        tls.insert("reality".into(), Value::Object(reality));
    }
    Value::Object(tls)
}

pub(super) fn outbound_base(
    typ: &str,
    tag: &str,
    host: &str,
    port: u16,
) -> serde_json::Map<String, Value> {
    let mut obj = serde_json::Map::new();
    obj.insert("type".into(), json!(typ));
    obj.insert("tag".into(), json!(tag));
    obj.insert("server".into(), json!(host));
    obj.insert("server_port".into(), json!(port));
    obj
}

pub(super) fn uniquify_tags(mut nodes: Vec<Value>) -> Vec<Value> {
    let mut seen: HashMap<String, usize> = HashMap::new();
    for node in &mut nodes {
        let fallback = node
            .get("server")
            .and_then(Value::as_str)
            .unwrap_or("node")
            .to_string();
        let tag = node
            .get("tag")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(fallback.as_str())
            .to_string();
        let count = seen.entry(tag.clone()).or_insert(0);
        *count += 1;
        let final_tag = if *count == 1 {
            tag
        } else {
            format!("{tag}-{}", *count)
        };
        if let Some(obj) = node.as_object_mut() {
            obj.insert("tag".into(), json!(final_tag));
        }
    }
    nodes
}

pub(super) fn split_scheme(raw: &str) -> Result<(String, String), String> {
    let idx = raw
        .find("://")
        .ok_or_else(|| "missing URI scheme".to_string())?;
    let scheme = raw[..idx].to_ascii_lowercase();
    if scheme.is_empty() {
        return Err("missing URI scheme".to_string());
    }
    Ok((scheme, raw[idx + 3..].to_string()))
}

pub(super) fn split_fragment(rest: &str) -> (String, Option<String>) {
    match rest.rfind('#') {
        Some(idx) => (
            rest[..idx].to_string(),
            Some(percent_decode(&rest[idx + 1..])),
        ),
        None => (rest.to_string(), None),
    }
}

pub(super) fn split_query(rest: &str) -> (&str, HashMap<String, String>) {
    match rest.find('?') {
        Some(idx) => (&rest[..idx], parse_query(&rest[idx + 1..])),
        None => (rest, HashMap::new()),
    }
}

pub(super) fn parse_query(query: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for part in query.split('&') {
        if part.is_empty() {
            continue;
        }
        let (key, value) = match part.split_once('=') {
            Some((k, v)) => (percent_decode(k), percent_decode(v)),
            None => (percent_decode(part), String::new()),
        };
        map.insert(key, value);
    }
    map
}

pub(super) fn split_userinfo_host(body: &str) -> (Option<String>, String) {
    match body.rfind('@') {
        Some(idx) => (Some(body[..idx].to_string()), body[idx + 1..].to_string()),
        None => (None, body.to_string()),
    }
}

pub(super) fn split_host_port(hostport: &str) -> Result<(String, u16), String> {
    let hostport = hostport.trim();
    let hostport = hostport.split('/').next().unwrap_or(hostport);
    if hostport.starts_with('[') {
        let end = hostport
            .find(']')
            .ok_or_else(|| "invalid IPv6 host".to_string())?;
        let host = hostport[1..end].to_string();
        let port = match hostport[end + 1..].strip_prefix(':') {
            Some(port) if !port.is_empty() => parse_port(port)?,
            _ => 443,
        };
        return Ok((host, port));
    }
    match hostport.rfind(':') {
        Some(idx) => {
            let host = hostport[..idx].to_string();
            let port = parse_port(&hostport[idx + 1..])?;
            if host.is_empty() {
                return Err("missing server host".to_string());
            }
            Ok((host, port))
        }
        None => Ok((hostport.to_string(), 443)),
    }
}

pub(super) fn parse_port(raw: &str) -> Result<u16, String> {
    let digits: String = raw
        .trim()
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect();
    digits
        .parse::<u16>()
        .map_err(|_| format!("invalid port '{raw}'"))
        .and_then(|port| {
            if port == 0 {
                Err("invalid port '0'".to_string())
            } else {
                Ok(port)
            }
        })
}

pub(super) fn qget<'a>(query: &'a HashMap<String, String>, keys: &[&str]) -> Option<&'a str> {
    for key in keys {
        for (candidate, value) in query {
            if candidate.eq_ignore_ascii_case(key) && !value.is_empty() {
                return Some(value.as_str());
            }
        }
    }
    None
}

pub(super) fn truthy(value: &str) -> bool {
    matches!(
        value.to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

pub(super) fn split_csv(value: impl AsRef<str>) -> Vec<String> {
    value
        .as_ref()
        .trim()
        .trim_matches(|ch| ch == '{' || ch == '}')
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .collect()
}

pub(super) fn normalize_ss_method(method: &str) -> String {
    match method {
        "chacha20-poly1305" => "chacha20-ietf-poly1305".to_string(),
        "xchacha20-poly1305" => "xchacha20-ietf-poly1305".to_string(),
        other => other.to_string(),
    }
}

pub(super) fn split_early_data(path: Option<&str>) -> (Option<String>, Option<u64>) {
    let Some(path) = path else {
        return (None, None);
    };
    if let Some((base, ed)) = path.rsplit_once("?ed=")
        && let Ok(value) = ed.parse::<u64>()
    {
        return (Some(base.to_string()), Some(value));
    }
    (Some(path.to_string()), None)
}

pub(super) fn parse_leading_int(value: &str) -> Option<u64> {
    let digits: String = value.chars().take_while(|ch| ch.is_ascii_digit()).collect();
    if digits.is_empty() {
        None
    } else {
        digits.parse().ok()
    }
}

pub(super) fn hy2_server_ports(
    hostport: &str,
    query: &HashMap<String, String>,
) -> Option<Vec<String>> {
    let mut ranges = Vec::new();
    if let Some(extra) = extra_port_ranges(hostport) {
        ranges.extend(extra);
    }
    if let Some(mport) = qget(query, &["mport", "server_ports"]) {
        ranges.extend(normalize_port_ranges(mport));
    }
    if ranges.is_empty() {
        None
    } else {
        ranges.dedup();
        Some(ranges)
    }
}

fn extra_port_ranges(hostport: &str) -> Option<Vec<String>> {
    let extra = if hostport.starts_with('[') {
        let end = hostport.find(']')?;
        hostport[end + 1..].strip_prefix(':')?.split_once(',')?.1
    } else {
        hostport.rsplit_once(':')?.1.split_once(',')?.1
    };
    let ranges = normalize_port_ranges(extra);
    if ranges.is_empty() {
        None
    } else {
        Some(ranges)
    }
}

pub(super) fn normalize_port_ranges(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(|part| part.replace('-', ":"))
        .collect()
}

pub(super) fn multiplex_from_query(query: &HashMap<String, String>) -> Option<Value> {
    multiplex_from_fields(
        qget(query, &["protocol"]),
        qget(query, &["max-streams", "max_streams"]),
        qget(query, &["max-connections", "max_connections"]),
        qget(query, &["min-streams", "min_streams"]),
        qget(query, &["padding"]),
    )
}

pub(super) fn multiplex_from_value(value: &Value) -> Option<Value> {
    multiplex_from_fields(
        json_string(value, &["protocol"]).as_deref(),
        json_string(value, &["max_streams", "max-streams"]).as_deref(),
        json_string(value, &["max_connections", "max-connections"]).as_deref(),
        json_string(value, &["min_streams", "min-streams"]).as_deref(),
        json_string(value, &["padding"]).as_deref(),
    )
}

fn multiplex_from_fields(
    protocol: Option<&str>,
    max_streams: Option<&str>,
    max_connections: Option<&str>,
    min_streams: Option<&str>,
    padding: Option<&str>,
) -> Option<Value> {
    let protocol = protocol?;
    if !matches!(protocol, "smux" | "yamux" | "h2mux") {
        return None;
    }
    let mut obj = serde_json::Map::new();
    obj.insert("enabled".into(), json!(true));
    obj.insert("protocol".into(), json!(protocol));
    if let Some(max_streams) = max_streams.and_then(|s| s.parse::<u64>().ok()) {
        obj.insert("max_streams".into(), json!(max_streams));
    } else {
        if let Some(max_connections) = max_connections.and_then(|s| s.parse::<u64>().ok()) {
            obj.insert("max_connections".into(), json!(max_connections));
        }
        if let Some(min_streams) = min_streams.and_then(|s| s.parse::<u64>().ok()) {
            obj.insert("min_streams".into(), json!(min_streams));
        }
    }
    if padding.is_some_and(truthy) {
        obj.insert("padding".into(), json!(true));
    }
    Some(Value::Object(obj))
}

pub(super) fn fill_tls_server_name_from_ws_host(obj: &mut serde_json::Map<String, Value>) {
    let sni_missing = obj
        .get("tls")
        .and_then(Value::as_object)
        .map(|tls| {
            tls.get("server_name")
                .and_then(Value::as_str)
                .unwrap_or("")
                .is_empty()
        })
        .unwrap_or(true);
    if !sni_missing {
        return;
    }
    let Some(host) = obj
        .get("transport")
        .and_then(|transport| transport.get("headers"))
        .and_then(|headers| headers.get("Host"))
        .and_then(Value::as_str)
        .filter(|host| !host.is_empty())
        .map(ToString::to_string)
    else {
        return;
    };
    if let Some(tls) = obj.get_mut("tls").and_then(Value::as_object_mut) {
        tls.insert("server_name".into(), json!(host));
    }
}

pub(super) fn normalize_path(path: &str) -> String {
    if path.is_empty() {
        "/".to_string()
    } else if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    }
}

pub(super) fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let (Some(high), Some(low)) = (from_hex(bytes[i + 1]), from_hex(bytes[i + 2]))
        {
            out.push((high << 4) | low);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(out)
        .unwrap_or_else(|err| String::from_utf8_lossy(err.as_bytes()).into_owned())
}

fn from_hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

pub(super) fn decode_base64_to_string(input: &str) -> Option<String> {
    let bytes = decode_base64_bytes(input)?;
    let text = String::from_utf8(bytes).ok()?;
    if text
        .chars()
        .any(|ch| ch.is_control() && !matches!(ch, '\n' | '\r' | '\t'))
    {
        return None;
    }
    Some(text)
}

fn decode_base64_bytes(input: &str) -> Option<Vec<u8>> {
    let compact: String = input.chars().filter(|ch| !ch.is_whitespace()).collect();
    if compact.is_empty() {
        return None;
    }
    let mut padded = compact.replace('-', "+").replace('_', "/");
    while !padded.len().is_multiple_of(4) {
        padded.push('=');
    }
    STANDARD.decode(padded).ok()
}

pub(super) fn json_string(value: &Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(item) = value.get(*key) {
            if let Some(text) = item.as_str() {
                if !text.is_empty() {
                    return Some(text.to_string());
                }
            } else if let Some(n) = item.as_u64() {
                return Some(n.to_string());
            } else if let Some(n) = item.as_i64() {
                return Some(n.to_string());
            }
        }
    }
    None
}

pub(super) fn json_u16(value: &Value, keys: &[&str]) -> Option<u16> {
    for key in keys {
        if let Some(item) = value.get(*key) {
            if let Some(n) = item.as_u64() {
                return u16::try_from(n).ok().filter(|port| *port > 0);
            }
            if let Some(text) = item.as_str() {
                return text.parse::<u16>().ok().filter(|port| *port > 0);
            }
        }
    }
    None
}

pub(super) fn json_bool(value: &Value, keys: &[&str]) -> Option<bool> {
    for key in keys {
        if let Some(item) = value.get(*key) {
            if let Some(b) = item.as_bool() {
                return Some(b);
            }
            if let Some(text) = item.as_str() {
                return Some(truthy(text));
            }
        }
    }
    None
}
