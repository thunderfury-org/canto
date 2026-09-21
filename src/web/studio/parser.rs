use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::{Value, json};
use std::collections::HashMap;

const SKIP_OUTBOUND_TYPES: &[&str] = &[
    "selector",
    "urltest",
    "direct",
    "block",
    "dns",
    "compatible",
];

pub fn parse_subscription(input: &str) -> Result<Vec<Value>, String> {
    let input = input.trim().trim_start_matches('\u{feff}');
    if input.is_empty() {
        return Err("empty subscription content".to_string());
    }

    // 1. JSON
    if let Some(nodes) = try_parse_json(input) {
        return Ok(uniquify_tags(nodes));
    }

    // 2. Clash YAML (`proxies:` list)
    if let Some(res) = try_parse_clash_yaml(input) {
        return res.map(uniquify_tags);
    }

    // 3. Base64
    if let Some(decoded) = decode_base64_to_string(input)
        && decoded.trim() != input
    {
        if let Some(nodes) = try_parse_json(&decoded) {
            return Ok(uniquify_tags(nodes));
        }
        if let Some(res) = try_parse_clash_yaml(&decoded) {
            return res.map(uniquify_tags);
        }
        if let Ok(nodes) = parse_uri_list(&decoded)
            && !nodes.is_empty()
        {
            return Ok(uniquify_tags(nodes));
        }
    }

    // 4. Plaintext URI list
    parse_uri_list(input).map(uniquify_tags)
}

pub fn parse_uri(raw: &str) -> Result<Value, String> {
    let raw = raw.trim();
    let (scheme, rest) = split_scheme(raw)?;
    match scheme.as_str() {
        "ss" => parse_ss(&rest),
        "vmess" => parse_vmess(&rest),
        "vless" => parse_vless(&rest),
        "trojan" => parse_trojan(&rest),
        "hysteria2" | "hy2" => parse_hysteria2(&rest),
        other => Err(format!("unsupported protocol: {other}")),
    }
}

fn try_parse_clash_yaml(input: &str) -> Option<Result<Vec<Value>, String>> {
    let yaml: serde_yaml::Value = serde_yaml::from_str(input).ok()?;
    let proxies_val = match &yaml {
        serde_yaml::Value::Mapping(map) => {
            let key = serde_yaml::Value::String("proxies".to_string());
            map.get(&key)?
        }
        _ => return None,
    };
    let serde_yaml::Value::Sequence(proxies) = proxies_val else {
        return None;
    };
    let mut nodes = Vec::new();
    for item in proxies {
        let json_proxy = yaml_to_json_value(item.clone());
        if let Some(node) = parse_clash_proxy(&json_proxy) {
            nodes.push(node);
        }
    }
    if nodes.is_empty() {
        Some(Err("no valid proxy nodes found in Clash YAML".to_string()))
    } else {
        Some(Ok(nodes))
    }
}

fn yaml_to_json_value(val: serde_yaml::Value) -> Value {
    match val {
        serde_yaml::Value::Null => Value::Null,
        serde_yaml::Value::Bool(b) => Value::Bool(b),
        serde_yaml::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                json!(i)
            } else if let Some(u) = n.as_u64() {
                json!(u)
            } else if let Some(f) = n.as_f64() {
                json!(f)
            } else {
                Value::Null
            }
        }
        serde_yaml::Value::String(s) => Value::String(s),
        serde_yaml::Value::Sequence(seq) => {
            Value::Array(seq.into_iter().map(yaml_to_json_value).collect())
        }
        serde_yaml::Value::Mapping(map) => {
            let mut obj = serde_json::Map::new();
            for (k, v) in map {
                let key_str = match k {
                    serde_yaml::Value::String(s) => s,
                    serde_yaml::Value::Number(n) => n.to_string(),
                    serde_yaml::Value::Bool(b) => b.to_string(),
                    _ => continue,
                };
                obj.insert(key_str, yaml_to_json_value(v));
            }
            Value::Object(obj)
        }
        serde_yaml::Value::Tagged(tagged) => yaml_to_json_value(tagged.value),
    }
}

fn parse_clash_proxy(value: &Value) -> Option<Value> {
    let typ = json_string(value, &["type"])?.to_ascii_lowercase();
    match typ.as_str() {
        "ss" | "shadowsocks" => parse_clash_ss(value),
        "vmess" => parse_clash_vmess(value),
        "vless" => parse_clash_vless(value),
        "trojan" => parse_clash_trojan(value),
        "hysteria2" | "hy2" => parse_clash_hysteria2(value),
        _ => None,
    }
}

fn parse_clash_ss(value: &Value) -> Option<Value> {
    let server = json_string(value, &["server", "host"])?;
    let port = json_u16(value, &["port"])?;
    let tag = json_string(value, &["name", "tag"]).unwrap_or_else(|| format!("{server}:{port}"));
    let cipher = json_string(value, &["cipher", "method"])?;
    let password = json_string(value, &["password", "secret"])?;

    let mut obj = outbound_base("shadowsocks", &tag, &server, port);
    obj.insert("method".into(), json!(normalize_ss_method(&cipher)));
    obj.insert("password".into(), json!(password));

    if let Some(plugin) = json_string(value, &["plugin"]) {
        let plugin_lower = plugin.to_ascii_lowercase();
        let plugin_name = if plugin_lower == "obfs" || plugin_lower == "simple-obfs" {
            "obfs-local"
        } else {
            &plugin
        };
        obj.insert("plugin".into(), json!(plugin_name));

        if let Some(opts) = value
            .get("plugin-opts")
            .or_else(|| value.get("plugin_opts"))
        {
            if let Some(opts_str) = opts.as_str() {
                obj.insert("plugin_opts".into(), json!(opts_str));
            } else if let Some(opts_map) = opts.as_object() {
                let mut parts = Vec::new();
                if plugin_name == "obfs-local" {
                    if let Some(mode) = opts_map.get("mode").and_then(Value::as_str) {
                        parts.push(format!("obfs={mode}"));
                    }
                    if let Some(host) = opts_map.get("host").and_then(Value::as_str) {
                        parts.push(format!("obfs-host={host}"));
                    }
                } else if plugin_name == "v2ray-plugin" {
                    if let Some(mode) = opts_map.get("mode").and_then(Value::as_str) {
                        parts.push(format!("mode={mode}"));
                    }
                    if let Some(host) = opts_map.get("host").and_then(Value::as_str) {
                        parts.push(format!("host={host}"));
                    }
                    if let Some(path) = opts_map.get("path").and_then(Value::as_str) {
                        parts.push(format!("path={path}"));
                    }
                    if opts_map
                        .get("tls")
                        .and_then(Value::as_bool)
                        .unwrap_or(false)
                    {
                        parts.push("tls".to_string());
                    }
                } else {
                    for (k, v) in opts_map {
                        if let Some(s) = v.as_str() {
                            parts.push(format!("{k}={s}"));
                        }
                    }
                }
                if !parts.is_empty() {
                    obj.insert("plugin_opts".into(), json!(parts.join(";")));
                }
            }
        }
    }

    if json_bool(value, &["uot", "udp-over-tcp", "udp_over_tcp"]).unwrap_or(false) {
        obj.insert(
            "udp_over_tcp".into(),
            json!({ "enabled": true, "version": 2 }),
        );
    }
    if let Some(multiplex) = clash_multiplex(value) {
        obj.insert("multiplex".into(), multiplex);
    }

    Some(Value::Object(obj))
}

fn parse_clash_vmess(value: &Value) -> Option<Value> {
    let server = json_string(value, &["server", "host"])?;
    let port = json_u16(value, &["port"]).unwrap_or(443);
    let tag = json_string(value, &["name", "tag"]).unwrap_or_else(|| format!("{server}:{port}"));
    let uuid = json_string(value, &["uuid", "id"])?;
    let security = json_string(value, &["cipher", "security", "scy"])
        .filter(|s| !matches!(s.to_ascii_lowercase().as_str(), "none" | "zero" | ""))
        .unwrap_or_else(|| "auto".to_string());

    let mut obj = outbound_base("vmess", &tag, &server, port);
    obj.insert("uuid".into(), json!(uuid));
    obj.insert("security".into(), json!(security));
    obj.insert("packet_encoding".into(), json!("xudp"));

    if let Some(alter_id) = json_u16(value, &["alterId", "alter_id", "aid"])
        && alter_id > 0
    {
        obj.insert("alter_id".into(), json!(alter_id));
    }

    let network = json_string(value, &["network", "net"]).unwrap_or_else(|| "tcp".to_string());
    if let Some(transport) = clash_transport(value, &network) {
        obj.insert("transport".into(), transport);
    }

    if let Some(tls) = clash_tls(value, false) {
        obj.insert("tls".into(), tls);
    }
    fill_tls_server_name_from_ws_host(&mut obj);

    if let Some(multiplex) = clash_multiplex(value) {
        obj.insert("multiplex".into(), multiplex);
    }

    Some(Value::Object(obj))
}

fn parse_clash_vless(value: &Value) -> Option<Value> {
    let server = json_string(value, &["server", "host"])?;
    let port = json_u16(value, &["port"]).unwrap_or(443);
    let tag = json_string(value, &["name", "tag"]).unwrap_or_else(|| format!("{server}:{port}"));
    let uuid = json_string(value, &["uuid", "id"])?;

    let mut obj = outbound_base("vless", &tag, &server, port);
    obj.insert("uuid".into(), json!(uuid));
    obj.insert("packet_encoding".into(), json!("xudp"));

    if let Some(flow) = json_string(value, &["flow"])
        && !flow.is_empty()
        && flow != "none"
    {
        obj.insert("flow".into(), json!(flow));
    }

    let network = json_string(value, &["network", "net"]).unwrap_or_else(|| "tcp".to_string());
    if let Some(transport) = clash_transport(value, &network) {
        obj.insert("transport".into(), transport);
    }

    if let Some(tls) = clash_tls(value, false) {
        obj.insert("tls".into(), tls);
    }
    fill_tls_server_name_from_ws_host(&mut obj);

    if let Some(multiplex) = clash_multiplex(value) {
        obj.insert("multiplex".into(), multiplex);
    }

    Some(Value::Object(obj))
}

fn parse_clash_trojan(value: &Value) -> Option<Value> {
    let server = json_string(value, &["server", "host"])?;
    let port = json_u16(value, &["port"]).unwrap_or(443);
    let tag = json_string(value, &["name", "tag"]).unwrap_or_else(|| format!("{server}:{port}"));
    let password = json_string(value, &["password", "auth", "secret"])?;

    let mut obj = outbound_base("trojan", &tag, &server, port);
    obj.insert("password".into(), json!(password));

    let network = json_string(value, &["network", "net"]).unwrap_or_else(|| "tcp".to_string());
    if let Some(transport) = clash_transport(value, &network) {
        obj.insert("transport".into(), transport);
    }

    let tls_disabled = json_bool(value, &["tls"]).map(|b| !b).unwrap_or(false);
    if !tls_disabled && let Some(tls) = clash_tls(value, true) {
        obj.insert("tls".into(), tls);
    }
    fill_tls_server_name_from_ws_host(&mut obj);

    if let Some(multiplex) = clash_multiplex(value) {
        obj.insert("multiplex".into(), multiplex);
    }

    Some(Value::Object(obj))
}

fn parse_clash_hysteria2(value: &Value) -> Option<Value> {
    let server = json_string(value, &["server", "host"])?;
    let port = json_u16(value, &["port"]).unwrap_or(443);
    let tag = json_string(value, &["name", "tag"]).unwrap_or_else(|| format!("{server}:{port}"));
    let password = json_string(value, &["password", "auth"])?;

    let mut obj = outbound_base("hysteria2", &tag, &server, port);
    obj.insert("password".into(), json!(password));

    if let Some(ports_val) = value
        .get("ports")
        .or_else(|| value.get("mport"))
        .or_else(|| value.get("server_ports"))
    {
        let ranges = match ports_val {
            Value::String(s) => normalize_port_ranges(s),
            Value::Array(arr) => {
                let mut out = Vec::new();
                for item in arr {
                    if let Some(s) = item.as_str() {
                        out.extend(normalize_port_ranges(s));
                    } else if let Some(n) = item.as_u64() {
                        out.push(n.to_string());
                    }
                }
                out
            }
            Value::Number(n) => vec![n.to_string()],
            _ => Vec::new(),
        };
        if !ranges.is_empty() {
            obj.insert("server_ports".into(), json!(ranges));
        }
    }

    if let Some(up) = json_string(value, &["up", "up-mbps", "up_mbps", "upmbps"])
        .and_then(|s| parse_leading_int(&s))
        .or_else(|| value.get("up").and_then(Value::as_u64))
    {
        obj.insert("up_mbps".into(), json!(up));
    }

    if let Some(down) = json_string(value, &["down", "down-mbps", "down_mbps", "downmbps"])
        .and_then(|s| parse_leading_int(&s))
        .or_else(|| value.get("down").and_then(Value::as_u64))
    {
        obj.insert("down_mbps".into(), json!(down));
    }

    if let Some(obfs) = json_string(value, &["obfs"])
        && !obfs.is_empty()
        && !obfs.eq_ignore_ascii_case("none")
    {
        let mut obfs_obj = serde_json::Map::new();
        obfs_obj.insert("type".into(), json!(obfs));
        if let Some(obfs_pass) = json_string(value, &["obfs-password", "obfs_password"]) {
            obfs_obj.insert("password".into(), json!(obfs_pass));
        }
        obj.insert("obfs".into(), Value::Object(obfs_obj));
    }

    let mut tls = clash_tls(value, true).unwrap_or_else(|| {
        let mut t = serde_json::Map::new();
        t.insert("enabled".into(), json!(true));
        Value::Object(t)
    });
    if let Some(tls_obj) = tls.as_object_mut()
        && !tls_obj.contains_key("alpn")
    {
        tls_obj.insert("alpn".into(), json!(["h3"]));
    }
    obj.insert("tls".into(), tls);

    Some(Value::Object(obj))
}

fn clash_transport(value: &Value, network: &str) -> Option<Value> {
    let net = network.to_ascii_lowercase();
    match net.as_str() {
        "ws" | "websocket" => {
            let ws_opts = value.get("ws-opts").or_else(|| value.get("ws_opts"));
            let path = ws_opts
                .and_then(|o| json_string(o, &["path"]))
                .or_else(|| json_string(value, &["path"]));
            let (path, early_data_from_path) = split_early_data(path.as_deref());
            let max_early_data = ws_opts
                .and_then(|o| o.get("max-early-data").or_else(|| o.get("max_early_data")))
                .and_then(Value::as_u64)
                .or(early_data_from_path);
            let early_data_header = ws_opts
                .and_then(|o| json_string(o, &["early-data-header-name", "early_data_header_name"]))
                .unwrap_or_else(|| "Sec-WebSocket-Protocol".to_string());

            let mut obj = serde_json::Map::new();
            obj.insert("type".into(), json!("ws"));
            if let Some(p) = path {
                obj.insert("path".into(), json!(normalize_path(&p)));
            }
            if let Some(headers) = ws_opts
                .and_then(|o| o.get("headers"))
                .and_then(Value::as_object)
            {
                let mut h = headers.clone();
                if let Some(host_val) = h.get("host").cloned() {
                    h.entry("Host".to_string()).or_insert(host_val);
                }
                obj.insert("headers".into(), Value::Object(h));
            } else if let Some(host) = ws_opts.and_then(|o| json_string(o, &["host", "Host"])) {
                obj.insert("headers".into(), json!({ "Host": host }));
            }
            if let Some(ed) = max_early_data {
                obj.insert("max_early_data".into(), json!(ed));
                obj.insert("early_data_header_name".into(), json!(early_data_header));
            }
            Some(Value::Object(obj))
        }
        "grpc" => {
            let grpc_opts = value.get("grpc-opts").or_else(|| value.get("grpc_opts"));
            let service_name = grpc_opts
                .and_then(|o| {
                    json_string(
                        o,
                        &[
                            "grpc-service-name",
                            "grpc_service_name",
                            "service_name",
                            "serviceName",
                        ],
                    )
                })
                .or_else(|| json_string(value, &["serviceName", "service_name"]));
            let mut obj = serde_json::Map::new();
            obj.insert("type".into(), json!("grpc"));
            if let Some(name) = service_name {
                obj.insert("service_name".into(), json!(name));
            }
            Some(Value::Object(obj))
        }
        "http" | "h2" => {
            let h_opts = value
                .get("http-opts")
                .or_else(|| value.get("http_opts"))
                .or_else(|| value.get("h2-opts"))
                .or_else(|| value.get("h2_opts"));
            let path = h_opts.and_then(|o| o.get("path"));
            let mut obj = serde_json::Map::new();
            obj.insert("type".into(), json!("http"));
            if let Some(p) = path {
                if let Some(s) = p.as_str() {
                    obj.insert("path".into(), json!(normalize_path(s)));
                } else if let Some(arr) = p.as_array()
                    && let Some(first) = arr.first().and_then(Value::as_str)
                {
                    obj.insert("path".into(), json!(normalize_path(first)));
                }
            }
            if let Some(headers) = h_opts
                .and_then(|o| o.get("headers"))
                .and_then(Value::as_object)
                && let Some(host) = headers.get("Host").or_else(|| headers.get("host"))
            {
                if let Some(h) = host.as_str() {
                    obj.insert("host".into(), json!(split_csv(h)));
                } else if let Some(arr) = host.as_array() {
                    obj.insert("host".into(), Value::Array(arr.clone()));
                }
            }
            Some(Value::Object(obj))
        }
        "httpupgrade" => {
            let opts = value
                .get("httpupgrade-opts")
                .or_else(|| value.get("httpupgrade_opts"));
            let path = opts
                .and_then(|o| json_string(o, &["path"]))
                .or_else(|| json_string(value, &["path"]));
            let host = opts.and_then(|o| json_string(o, &["host"]));
            let mut obj = serde_json::Map::new();
            obj.insert("type".into(), json!("httpupgrade"));
            if let Some(p) = path {
                obj.insert("path".into(), json!(normalize_path(&p)));
            }
            if let Some(h) = host {
                obj.insert("host".into(), json!(h));
            }
            Some(Value::Object(obj))
        }
        _ => None,
    }
}

fn clash_tls(value: &Value, default_on: bool) -> Option<Value> {
    let reality_opts = value
        .get("reality-opts")
        .or_else(|| value.get("reality_opts"));
    let public_key = reality_opts
        .and_then(|o| json_string(o, &["public-key", "public_key", "publicKey", "pbk"]));
    let short_id =
        reality_opts.and_then(|o| json_string(o, &["short-id", "short_id", "shortId", "sid"]));
    let is_reality = public_key.is_some();

    let tls_val = json_bool(value, &["tls"]);
    let enabled = tls_val.unwrap_or(default_on || is_reality);
    if !enabled {
        return None;
    }

    let sni = json_string(value, &["servername", "server_name", "server-name", "sni"]);
    let insecure =
        json_bool(value, &["skip-cert-verify", "skip_cert_verify", "insecure"]).unwrap_or(false);
    let fingerprint = json_string(
        value,
        &[
            "client-fingerprint",
            "client_fingerprint",
            "fingerprint",
            "fp",
        ],
    );

    let alpn = if let Some(alpn_val) = value.get("alpn") {
        match alpn_val {
            Value::Array(arr) => {
                let list: Vec<String> = arr
                    .iter()
                    .filter_map(Value::as_str)
                    .map(ToString::to_string)
                    .collect();
                if list.is_empty() { None } else { Some(list) }
            }
            Value::String(s) => Some(split_csv(s)),
            _ => None,
        }
    } else {
        None
    };

    Some(build_tls(
        sni,
        alpn,
        insecure,
        fingerprint,
        public_key,
        short_id,
    ))
}

fn clash_multiplex(value: &Value) -> Option<Value> {
    let smux = value.get("smux").and_then(Value::as_object)?;
    let enabled = smux.get("enabled").and_then(Value::as_bool).unwrap_or(true);
    if !enabled {
        return None;
    }
    let protocol = smux
        .get("protocol")
        .and_then(Value::as_str)
        .unwrap_or("smux");
    let max_streams = smux
        .get("max-streams")
        .or_else(|| smux.get("max_streams"))
        .and_then(Value::as_u64);
    let max_connections = smux
        .get("max-connections")
        .or_else(|| smux.get("max_connections"))
        .and_then(Value::as_u64);
    let min_streams = smux
        .get("min-streams")
        .or_else(|| smux.get("min_streams"))
        .and_then(Value::as_u64);
    let padding = smux
        .get("padding")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let mut obj = serde_json::Map::new();
    obj.insert("enabled".into(), json!(true));
    obj.insert("protocol".into(), json!(protocol));
    if let Some(ms) = max_streams {
        obj.insert("max_streams".into(), json!(ms));
    } else {
        if let Some(mc) = max_connections {
            obj.insert("max_connections".into(), json!(mc));
        }
        if let Some(ms) = min_streams {
            obj.insert("min_streams".into(), json!(ms));
        }
    }
    if padding {
        obj.insert("padding".into(), json!(true));
    }
    Some(Value::Object(obj))
}

fn json_bool(value: &Value, keys: &[&str]) -> Option<bool> {
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

fn try_parse_json(input: &str) -> Option<Vec<Value>> {
    let value: Value = serde_json::from_str(input.trim()).ok()?;
    let nodes = extract_outbounds(&value);
    if nodes.is_empty() { None } else { Some(nodes) }
}

fn extract_outbounds(value: &Value) -> Vec<Value> {
    match value {
        Value::Array(items) => {
            let mut nodes = Vec::new();
            for item in items {
                match item {
                    Value::String(uri) => {
                        if let Ok(node) = parse_uri(uri) {
                            nodes.push(node);
                        }
                    }
                    other => {
                        let nested = extract_outbounds(other);
                        if nested.is_empty() {
                            if let Some(node) = as_proxy_outbound(other) {
                                nodes.push(node);
                            }
                        } else {
                            nodes.extend(nested);
                        }
                    }
                }
            }
            nodes
        }
        Value::Object(map) => {
            if let Some(outbounds) = map.get("outbounds") {
                return extract_outbounds(outbounds);
            }
            as_proxy_outbound(value).into_iter().collect()
        }
        _ => Vec::new(),
    }
}

fn as_proxy_outbound(value: &Value) -> Option<Value> {
    let obj = value.as_object()?;
    let typ = obj.get("type")?.as_str()?;
    if SKIP_OUTBOUND_TYPES.contains(&typ) {
        return None;
    }
    Some(value.clone())
}

fn parse_uri_list(input: &str) -> Result<Vec<Value>, String> {
    let mut nodes = Vec::new();
    let mut failed = 0usize;
    let mut seen = 0usize;

    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
            continue;
        }
        seen += 1;
        match parse_uri(line) {
            Ok(node) => nodes.push(node),
            Err(_) => failed += 1,
        }
    }

    if nodes.is_empty() && seen <= 1 && !input.contains('\n') {
        return parse_uri(input.trim()).map(|node| vec![node]);
    }

    if nodes.is_empty() {
        return Err(format!(
            "no valid proxy nodes found ({failed} line(s) failed to parse)"
        ));
    }
    Ok(nodes)
}

fn parse_ss(rest: &str) -> Result<Value, String> {
    let (without_frag, fragment) = split_fragment(rest);
    let (without_query, query) = split_query(&without_frag);
    let without_query = without_query.trim_end_matches('/');

    let (method, password, host, port) = if without_query.contains('@') {
        let (userinfo, hostport) = split_userinfo_host(without_query);
        let userinfo = userinfo.ok_or_else(|| "ss missing userinfo".to_string())?;
        let (host, port) = split_host_port(&hostport)?;
        let userinfo = percent_decode(&userinfo);
        let (method, password) = split_ss_method_password(&userinfo)?;
        (method, password, host, port)
    } else {
        let decoded = decode_base64_to_string(without_query)
            .ok_or_else(|| "invalid shadowsocks legacy encoding".to_string())?;
        let (userinfo, hostport) = split_userinfo_host(decoded.trim());
        let userinfo = userinfo.ok_or_else(|| "invalid shadowsocks legacy payload".to_string())?;
        let (host, port) = split_host_port(&hostport)?;
        let (method, password) = userinfo
            .split_once(':')
            .map(|(m, p)| (m.to_string(), p.to_string()))
            .ok_or_else(|| "invalid shadowsocks method:password".to_string())?;
        (method, password, host, port)
    };

    let tag = fragment
        .filter(|s| !s.is_empty())
        .or_else(|| qget(&query, &["remarks"]).map(ToString::to_string))
        .unwrap_or_else(|| format!("{host}:{port}"));
    let mut obj = outbound_base("shadowsocks", &tag, &host, port);
    obj.insert("method".into(), json!(normalize_ss_method(&method)));
    obj.insert("password".into(), json!(password));

    if let Some(plugin) = qget(&query, &["plugin"]) {
        let mut parts = plugin.split(';');
        if let Some(name) = parts.next().filter(|s| !s.is_empty()) {
            let name = if name == "simple-obfs" {
                "obfs-local"
            } else {
                name
            };
            obj.insert("plugin".into(), json!(name));
            let opts: Vec<&str> = parts.filter(|s| !s.is_empty()).collect();
            if !opts.is_empty() {
                obj.insert("plugin_opts".into(), json!(opts.join(";")));
            }
        }
    }
    if qget(&query, &["uot", "udp-over-tcp", "udp_over_tcp"]).is_some_and(truthy) {
        obj.insert(
            "udp_over_tcp".into(),
            json!({ "enabled": true, "version": 2 }),
        );
    }
    if let Some(multiplex) = multiplex_from_query(&query) {
        obj.insert("multiplex".into(), multiplex);
    }

    Ok(Value::Object(obj))
}

fn split_ss_method_password(userinfo: &str) -> Result<(String, String), String> {
    if let Some((method, password)) = userinfo.split_once(':')
        && !method.is_empty()
        && !password.is_empty()
    {
        return Ok((method.to_string(), password.to_string()));
    }
    let decoded = decode_base64_to_string(userinfo)
        .ok_or_else(|| "invalid shadowsocks userinfo".to_string())?;
    decoded
        .split_once(':')
        .filter(|(m, p)| !m.is_empty() && !p.is_empty())
        .map(|(m, p)| (m.to_string(), p.to_string()))
        .ok_or_else(|| "invalid shadowsocks method:password".to_string())
}

fn parse_vmess(rest: &str) -> Result<Value, String> {
    let (body, fragment) = split_fragment(rest);
    if let Some(decoded) = decode_base64_to_string(&body)
        && let Ok(value) = serde_json::from_str::<Value>(decoded.trim())
    {
        return vmess_from_json(&value, fragment);
    }
    parse_vmess_link(&body, fragment)
}

fn vmess_from_json(value: &Value, fragment: Option<String>) -> Result<Value, String> {
    let server = json_string(value, &["add", "server"]).ok_or("vmess missing server")?;
    let port = json_u16(value, &["port", "server_port"]).unwrap_or(443);
    let uuid = json_string(value, &["id", "uuid"]).ok_or("vmess missing uuid")?;
    let tag = json_string(value, &["ps", "tag", "name"])
        .or(fragment)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| format!("{server}:{port}"));
    let security = json_string(value, &["scy", "security"])
        .filter(|s| {
            !matches!(
                s.to_ascii_lowercase().as_str(),
                "http" | "gun" | "none" | ""
            )
        })
        .unwrap_or_else(|| "auto".to_string());

    let mut obj = outbound_base("vmess", &tag, &server, port);
    obj.insert("uuid".into(), json!(uuid));
    obj.insert("security".into(), json!(security));
    obj.insert("packet_encoding".into(), json!("xudp"));
    if let Some(alter_id) = json_u16(value, &["aid", "alterId", "alter_id"])
        && alter_id > 0
    {
        obj.insert("alter_id".into(), json!(alter_id));
    }

    let net = json_string(value, &["net", "network"]).unwrap_or_else(|| "tcp".to_string());
    let header_type = json_string(value, &["type"]).unwrap_or_else(|| "none".to_string());
    let path = json_string(value, &["path"]);
    let host = json_string(value, &["host"]);
    if let Some(transport) = build_transport(
        &net,
        &header_type,
        path.as_deref(),
        host.as_deref(),
        json_string(value, &["serviceName", "service_name"]).as_deref(),
    ) {
        obj.insert("transport".into(), transport);
    }

    let tls_field = json_string(value, &["tls"]).unwrap_or_default();
    let sni = json_string(value, &["sni", "serverName", "server_name"]);
    let fingerprint = json_string(value, &["fp", "fingerprint"]);
    let alpn = json_string(value, &["alpn"]).map(split_csv);
    let tls_on = tls_field.eq_ignore_ascii_case("tls")
        || tls_field.eq_ignore_ascii_case("reality")
        || sni.is_some();
    if tls_on {
        obj.insert(
            "tls".into(),
            build_tls(sni.or(host.clone()), alpn, false, fingerprint, None, None),
        );
    }
    if let Some(multiplex) = multiplex_from_value(value) {
        obj.insert("multiplex".into(), multiplex);
    }
    Ok(Value::Object(obj))
}

fn parse_vmess_link(body: &str, fragment: Option<String>) -> Result<Value, String> {
    let link = parse_common_link(body, fragment)?;
    let uuid = link
        .userinfo
        .clone()
        .ok_or_else(|| "vmess missing uuid".to_string())?;
    let mut obj = outbound_base("vmess", &tag_of(&link), &link.host, link.port);
    obj.insert("uuid".into(), json!(percent_decode(&uuid)));
    let security = qget(&link.query, &["encryption", "security", "scy"]).unwrap_or("auto");
    obj.insert("security".into(), json!(security));
    obj.insert("packet_encoding".into(), json!("xudp"));
    if let Some(transport) = transport_from_query(&link.query) {
        obj.insert("transport".into(), transport);
    }
    if let Some(tls) = tls_from_query(&link.query, false) {
        obj.insert("tls".into(), tls);
    }
    if let Some(multiplex) = multiplex_from_query(&link.query) {
        obj.insert("multiplex".into(), multiplex);
    }
    Ok(Value::Object(obj))
}

fn parse_vless(rest: &str) -> Result<Value, String> {
    let link = parse_common_link_full(rest)?;
    let uuid = link
        .userinfo
        .clone()
        .ok_or_else(|| "vless missing uuid".to_string())?;
    let mut obj = outbound_base("vless", &tag_of(&link), &link.host, link.port);
    obj.insert("uuid".into(), json!(percent_decode(&uuid)));
    if let Some(flow) = qget(&link.query, &["flow"])
        && !flow.is_empty()
        && flow != "none"
    {
        obj.insert("flow".into(), json!(flow));
    }
    let packet_encoding =
        qget(&link.query, &["packetEncoding", "packet_encoding"]).unwrap_or("xudp");
    obj.insert("packet_encoding".into(), json!(packet_encoding));
    if let Some(transport) = transport_from_query(&link.query) {
        obj.insert("transport".into(), transport);
    }
    if let Some(tls) = tls_from_query(&link.query, false) {
        obj.insert("tls".into(), tls);
    }
    fill_tls_server_name_from_ws_host(&mut obj);
    if let Some(multiplex) = multiplex_from_query(&link.query) {
        obj.insert("multiplex".into(), multiplex);
    }
    Ok(Value::Object(obj))
}

fn parse_trojan(rest: &str) -> Result<Value, String> {
    let link = parse_common_link_full(rest)?;
    let password = link
        .userinfo
        .clone()
        .ok_or_else(|| "trojan missing password".to_string())?;
    let mut obj = outbound_base("trojan", &tag_of(&link), &link.host, link.port);
    obj.insert("password".into(), json!(percent_decode(&password)));
    if let Some(transport) = transport_from_query(&link.query) {
        obj.insert("transport".into(), transport);
    }
    if let Some(tls) = tls_from_query(&link.query, true) {
        obj.insert("tls".into(), tls);
    }
    if let Some(multiplex) = multiplex_from_query(&link.query) {
        obj.insert("multiplex".into(), multiplex);
    }
    Ok(Value::Object(obj))
}

fn parse_hysteria2(rest: &str) -> Result<Value, String> {
    let (body, fragment) = split_fragment(rest);
    let (without_query, query) = split_query(&body);
    let (userinfo, hostport) = split_userinfo_host(without_query.trim_end_matches('/'));
    let (host, port) = split_host_port(&hostport)?;
    if host.is_empty() {
        return Err("missing server host".to_string());
    }
    let link = CommonLink {
        userinfo,
        host,
        port,
        query,
        fragment,
    };
    let password = link
        .userinfo
        .as_deref()
        .map(percent_decode)
        .filter(|s| !s.is_empty())
        .or_else(|| qget(&link.query, &["password", "auth", "auth_str"]).map(|s| s.to_string()))
        .ok_or_else(|| "hysteria2 missing password".to_string())?;

    let mut obj = outbound_base("hysteria2", &tag_of(&link), &link.host, link.port);
    obj.insert("password".into(), json!(password));
    if let Some(server_ports) = hy2_server_ports(&hostport, &link.query) {
        obj.insert("server_ports".into(), json!(server_ports));
    }
    if let Some(up) = qget(&link.query, &["upmbps", "up"]).and_then(parse_leading_int) {
        obj.insert("up_mbps".into(), json!(up));
    }
    if let Some(down) = qget(&link.query, &["downmbps", "down"]).and_then(parse_leading_int) {
        obj.insert("down_mbps".into(), json!(down));
    }

    if let Some(obfs) = qget(&link.query, &["obfs"])
        && !obfs.is_empty()
        && !obfs.eq_ignore_ascii_case("none")
    {
        let mut obfs_obj = serde_json::Map::new();
        obfs_obj.insert("type".into(), json!(obfs));
        if let Some(obfs_password) = qget(
            &link.query,
            &["obfs-password", "obfs_password", "obfsPassword"],
        ) {
            obfs_obj.insert("password".into(), json!(obfs_password));
        }
        obj.insert("obfs".into(), Value::Object(obfs_obj));
    }

    if let Some(mut tls) = tls_from_query(&link.query, true) {
        if let Some(tls_obj) = tls.as_object_mut()
            && !tls_obj.contains_key("alpn")
        {
            tls_obj.insert("alpn".into(), json!(["h3"]));
        }
        obj.insert("tls".into(), tls);
    }
    Ok(Value::Object(obj))
}

struct CommonLink {
    userinfo: Option<String>,
    host: String,
    port: u16,
    query: HashMap<String, String>,
    fragment: Option<String>,
}

fn parse_common_link_full(rest: &str) -> Result<CommonLink, String> {
    let (body, fragment) = split_fragment(rest);
    parse_common_link(&body, fragment)
}

fn parse_common_link(body: &str, fragment: Option<String>) -> Result<CommonLink, String> {
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

fn tag_of(link: &CommonLink) -> String {
    link.fragment
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| format!("{}:{}", link.host, link.port))
}

fn transport_from_query(query: &HashMap<String, String>) -> Option<Value> {
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

fn tls_from_query(query: &HashMap<String, String>, default_on: bool) -> Option<Value> {
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

fn build_transport(
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

fn build_tls(
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

fn outbound_base(typ: &str, tag: &str, host: &str, port: u16) -> serde_json::Map<String, Value> {
    let mut obj = serde_json::Map::new();
    obj.insert("type".into(), json!(typ));
    obj.insert("tag".into(), json!(tag));
    obj.insert("server".into(), json!(host));
    obj.insert("server_port".into(), json!(port));
    obj
}

fn uniquify_tags(mut nodes: Vec<Value>) -> Vec<Value> {
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

fn split_scheme(raw: &str) -> Result<(String, String), String> {
    let idx = raw
        .find("://")
        .ok_or_else(|| "missing URI scheme".to_string())?;
    let scheme = raw[..idx].to_ascii_lowercase();
    if scheme.is_empty() {
        return Err("missing URI scheme".to_string());
    }
    Ok((scheme, raw[idx + 3..].to_string()))
}

fn split_fragment(rest: &str) -> (String, Option<String>) {
    match rest.rfind('#') {
        Some(idx) => (
            rest[..idx].to_string(),
            Some(percent_decode(&rest[idx + 1..])),
        ),
        None => (rest.to_string(), None),
    }
}

fn split_query(rest: &str) -> (&str, HashMap<String, String>) {
    match rest.find('?') {
        Some(idx) => (&rest[..idx], parse_query(&rest[idx + 1..])),
        None => (rest, HashMap::new()),
    }
}

fn parse_query(query: &str) -> HashMap<String, String> {
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

fn split_userinfo_host(body: &str) -> (Option<String>, String) {
    match body.rfind('@') {
        Some(idx) => (Some(body[..idx].to_string()), body[idx + 1..].to_string()),
        None => (None, body.to_string()),
    }
}

fn split_host_port(hostport: &str) -> Result<(String, u16), String> {
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

fn parse_port(raw: &str) -> Result<u16, String> {
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

fn qget<'a>(query: &'a HashMap<String, String>, keys: &[&str]) -> Option<&'a str> {
    for key in keys {
        for (candidate, value) in query {
            if candidate.eq_ignore_ascii_case(key) && !value.is_empty() {
                return Some(value.as_str());
            }
        }
    }
    None
}

fn truthy(value: &str) -> bool {
    matches!(
        value.to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

fn split_csv(value: impl AsRef<str>) -> Vec<String> {
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

fn normalize_ss_method(method: &str) -> String {
    match method {
        "chacha20-poly1305" => "chacha20-ietf-poly1305".to_string(),
        "xchacha20-poly1305" => "xchacha20-ietf-poly1305".to_string(),
        other => other.to_string(),
    }
}

fn split_early_data(path: Option<&str>) -> (Option<String>, Option<u64>) {
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

fn parse_leading_int(value: &str) -> Option<u64> {
    let digits: String = value.chars().take_while(|ch| ch.is_ascii_digit()).collect();
    if digits.is_empty() {
        None
    } else {
        digits.parse().ok()
    }
}

fn hy2_server_ports(hostport: &str, query: &HashMap<String, String>) -> Option<Vec<String>> {
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

fn normalize_port_ranges(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(|part| part.replace('-', ":"))
        .collect()
}

fn multiplex_from_query(query: &HashMap<String, String>) -> Option<Value> {
    multiplex_from_fields(
        qget(query, &["protocol"]),
        qget(query, &["max-streams", "max_streams"]),
        qget(query, &["max-connections", "max_connections"]),
        qget(query, &["min-streams", "min_streams"]),
        qget(query, &["padding"]),
    )
}

fn multiplex_from_value(value: &Value) -> Option<Value> {
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

fn fill_tls_server_name_from_ws_host(obj: &mut serde_json::Map<String, Value>) {
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

fn normalize_path(path: &str) -> String {
    if path.is_empty() {
        "/".to_string()
    } else if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    }
}

fn percent_decode(input: &str) -> String {
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

fn decode_base64_to_string(input: &str) -> Option<String> {
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

fn json_string(value: &Value, keys: &[&str]) -> Option<String> {
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

fn json_u16(value: &Value, keys: &[&str]) -> Option<u16> {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn b64(input: &str) -> String {
        STANDARD.encode(input.as_bytes())
    }

    #[test]
    fn test_parses_ss_sip002() {
        let userinfo = b64("aes-256-gcm:password");
        let uri = format!("ss://{userinfo}@192.168.100.1:8388#%E9%A6%99%E6%B8%AF-SS");
        let node = parse_uri(&uri).unwrap();
        assert_eq!(node["type"], "shadowsocks");
        assert_eq!(node["tag"], "香港-SS");
        assert_eq!(node["server"], "192.168.100.1");
        assert_eq!(node["server_port"], 8388);
        assert_eq!(node["method"], "aes-256-gcm");
        assert_eq!(node["password"], "password");
    }

    #[test]
    fn test_parses_ss_legacy() {
        let payload = b64("chacha20-ietf-poly1305:secret@203.0.113.8:443");
        let uri = format!("ss://{payload}#legacy-ss");
        let node = parse_uri(&uri).unwrap();
        assert_eq!(node["server"], "203.0.113.8");
        assert_eq!(node["method"], "chacha20-ietf-poly1305");
        assert_eq!(node["password"], "secret");
        assert_eq!(node["tag"], "legacy-ss");
    }

    #[test]
    fn test_parses_vmess_json() {
        let payload = r#"{
            "v":"2",
            "ps":"HK-VMess",
            "add":"hk.example.com",
            "port":"443",
            "id":"11111111-1111-1111-1111-111111111111",
            "aid":"0",
            "net":"ws",
            "type":"none",
            "host":"hk.example.com",
            "path":"/ws",
            "tls":"tls",
            "sni":"hk.example.com"
        }"#;
        let uri = format!("vmess://{}", b64(payload));
        let node = parse_uri(&uri).unwrap();
        assert_eq!(node["type"], "vmess");
        assert_eq!(node["tag"], "HK-VMess");
        assert_eq!(node["server"], "hk.example.com");
        assert_eq!(node["server_port"], 443);
        assert_eq!(node["uuid"], "11111111-1111-1111-1111-111111111111");
        assert_eq!(node["transport"]["type"], "ws");
        assert_eq!(node["transport"]["path"], "/ws");
        assert_eq!(node["tls"]["enabled"], true);
        assert_eq!(node["tls"]["server_name"], "hk.example.com");
    }

    #[test]
    fn test_parses_vless_tls_ws() {
        let uri = "vless://11111111-1111-1111-1111-111111111111@hk.example.com:443?type=ws&security=tls&sni=hk.example.com&path=/ws&host=hk.example.com#HK-VLESS";
        let node = parse_uri(uri).unwrap();
        assert_eq!(node["type"], "vless");
        assert_eq!(node["tag"], "HK-VLESS");
        assert_eq!(node["transport"]["type"], "ws");
        assert_eq!(node["tls"]["server_name"], "hk.example.com");
    }

    #[test]
    fn test_parses_vless_reality() {
        let uri = "vless://22222222-2222-2222-2222-222222222222@jp.example.com:443?type=tcp&security=reality&sni=www.microsoft.com&fp=chrome&pbk=PublicKeyReality&sid=ab&flow=xtls-rprx-vision#JP-Reality";
        let node = parse_uri(uri).unwrap();
        assert_eq!(node["flow"], "xtls-rprx-vision");
        assert_eq!(node["tls"]["reality"]["enabled"], true);
        assert_eq!(node["tls"]["reality"]["public_key"], "PublicKeyReality");
        assert_eq!(node["tls"]["utls"]["fingerprint"], "chrome");
        assert!(node.get("transport").is_none());
    }

    #[test]
    fn test_parses_trojan() {
        let uri = "trojan://secret%40pass@tw.example.com:443?security=tls&sni=tw.example.com&type=ws&path=/trojan#TW-Trojan";
        let node = parse_uri(uri).unwrap();
        assert_eq!(node["type"], "trojan");
        assert_eq!(node["password"], "secret@pass");
        assert_eq!(node["transport"]["type"], "ws");
        assert_eq!(node["tls"]["enabled"], true);
    }

    #[test]
    fn test_parses_hysteria2_and_hy2_alias() {
        let uri = "hysteria2://hy2pass@jp.example.com:8443?sni=jp.example.com&insecure=1&obfs=salamander&obfs-password=obfs-secret#JP-HY2";
        let node = parse_uri(uri).unwrap();
        assert_eq!(node["type"], "hysteria2");
        assert_eq!(node["password"], "hy2pass");
        assert_eq!(node["obfs"]["type"], "salamander");
        assert_eq!(node["obfs"]["password"], "obfs-secret");
        assert_eq!(node["tls"]["insecure"], true);

        let aliased = "hy2://querypass@198.51.100.9:443?password=ignored#hy2-query";
        let node = parse_uri(aliased).unwrap();
        assert_eq!(node["type"], "hysteria2");
        assert_eq!(node["password"], "querypass");
        assert_eq!(node["server"], "198.51.100.9");
    }

    #[test]
    fn test_parses_hysteria2_password_query() {
        let uri =
            "hysteria2://jp.example.com:8443?password=only-query&sni=jp.example.com#no-userinfo";
        let node = parse_uri(uri).unwrap();
        assert_eq!(node["password"], "only-query");
    }

    #[test]
    fn test_parses_base64_mixed_list() {
        let list = [
            "ss://YWVzLTI1Ni1nY206cGFzc3dvcmQ@192.168.100.1:8388#SS-1",
            "vless://11111111-1111-1111-1111-111111111111@hk.example.com:443?type=tcp&security=tls&sni=hk.example.com#VLESS-1",
            "trojan://secret@tw.example.com:443?security=tls&sni=tw.example.com#Trojan-1",
            "hysteria2://hy2pass@jp.example.com:8443?sni=jp.example.com#HY2-1",
            "not-a-uri",
        ]
        .join("\n");
        let encoded = b64(&list);
        let nodes = parse_subscription(&encoded).unwrap();
        assert_eq!(nodes.len(), 4);
        let types: Vec<&str> = nodes.iter().map(|n| n["type"].as_str().unwrap()).collect();
        assert_eq!(types, ["shadowsocks", "vless", "trojan", "hysteria2"]);
    }

    #[test]
    fn test_parses_singbox_outbound_array() {
        let payload = r#"[
            {"type":"vless","tag":"native-vless","server":"a.example","server_port":443,"uuid":"u"},
            {"type":"selector","tag":"proxy","outbounds":["native-vless"]}
        ]"#;
        let nodes = parse_subscription(payload).unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0]["tag"], "native-vless");
    }

    #[test]
    fn test_parses_full_singbox_json_extracts_proxies() {
        let payload = r#"{
            "log": {"level": "info"},
            "outbounds": [
                {"type":"direct","tag":"direct"},
                {"type":"urltest","tag":"auto","outbounds":["hk"]},
                {"type":"trojan","tag":"hk","server":"hk.example","server_port":443,"password":"p"}
            ]
        }"#;
        let nodes = parse_subscription(payload).unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0]["type"], "trojan");
        assert_eq!(nodes[0]["tag"], "hk");
    }

    #[test]
    fn test_uniquifies_duplicate_tags() {
        let list = "vless://u@a.example:443#dup\nvless://u@b.example:443#dup\n";
        let nodes = parse_subscription(list).unwrap();
        assert_eq!(nodes[0]["tag"], "dup");
        assert_eq!(nodes[1]["tag"], "dup-2");
    }

    #[test]
    fn test_parses_ipv6_host() {
        let uri = "vless://uuid@[2001:db8::1]:8443?security=tls&sni=example.com#v6";
        let node = parse_uri(uri).unwrap();
        assert_eq!(node["server"], "2001:db8::1");
        assert_eq!(node["server_port"], 8443);
    }

    #[test]
    fn test_parses_ss_plugin_cipher_alias_and_uot() {
        let userinfo = b64("chacha20-poly1305:secret");
        let uri = format!(
            "ss://{userinfo}@203.0.113.8:8388?plugin=simple-obfs;obfs=http;obfs-host=download.windowsupdate.com&uot=1&remarks=ss-plugin"
        );
        let node = parse_uri(&uri).unwrap();
        assert_eq!(node["method"], "chacha20-ietf-poly1305");
        assert_eq!(node["plugin"], "obfs-local");
        assert_eq!(
            node["plugin_opts"],
            "obfs=http;obfs-host=download.windowsupdate.com"
        );
        assert_eq!(node["udp_over_tcp"]["enabled"], true);
        assert_eq!(node["tag"], "ss-plugin");
    }

    #[test]
    fn test_parses_vmess_ws_early_data_and_grpc() {
        let ws = r#"{
            "ps":"VM-WS","add":"hk.example.com","port":443,
            "id":"11111111-1111-1111-1111-111111111111",
            "net":"ws","host":"hk.example.com","path":"/ws?ed=2048","tls":"tls","sni":"hk.example.com"
        }"#;
        let node = parse_uri(&format!("vmess://{}", b64(ws))).unwrap();
        assert_eq!(node["packet_encoding"], "xudp");
        assert_eq!(node["transport"]["path"], "/ws");
        assert_eq!(node["transport"]["max_early_data"], 2048);
        assert_eq!(
            node["transport"]["early_data_header_name"],
            "Sec-WebSocket-Protocol"
        );
        assert_eq!(node["transport"]["headers"]["Host"], "hk.example.com");

        let grpc = r#"{
            "ps":"VM-GRPC","add":"hk.example.com","port":443,
            "id":"11111111-1111-1111-1111-111111111111",
            "net":"grpc","path":"GunService","tls":"tls","sni":"hk.example.com","scy":"gun"
        }"#;
        let node = parse_uri(&format!("vmess://{}", b64(grpc))).unwrap();
        assert_eq!(node["security"], "auto");
        assert_eq!(node["transport"]["type"], "grpc");
        assert_eq!(node["transport"]["service_name"], "GunService");
    }

    #[test]
    fn test_parses_vless_grpc_and_reality_utls_without_fp() {
        let grpc = "vless://11111111-1111-1111-1111-111111111111@hk.example.com:443?type=grpc&serviceName=GunService&security=tls&sni=hk.example.com#g";
        let node = parse_uri(grpc).unwrap();
        assert_eq!(node["packet_encoding"], "xudp");
        assert_eq!(node["transport"]["type"], "grpc");
        assert_eq!(node["transport"]["service_name"], "GunService");

        let reality = "vless://22222222-2222-2222-2222-222222222222@jp.example.com:443?security=reality&pbk=PublicKeyReality&sid=ab#r";
        let node = parse_uri(reality).unwrap();
        assert_eq!(node["tls"]["reality"]["enabled"], true);
        assert_eq!(node["tls"]["utls"]["enabled"], true);
        assert!(node["tls"]["utls"].get("fingerprint").is_none());
    }

    #[test]
    fn test_parses_trojan_h2_and_braced_alpn() {
        let uri = "trojan://secret@tw.example.com:443?security=tls&sni=tw.example.com&type=h2&host=tw.example.com&path=/h2&alpn={h2,http/1.1}#h2";
        let node = parse_uri(uri).unwrap();
        assert_eq!(node["transport"]["type"], "http");
        assert_eq!(node["transport"]["path"], "/h2");
        assert_eq!(node["tls"]["alpn"], json!(["h2", "http/1.1"]));
    }

    #[test]
    fn test_parses_hysteria2_port_hopping_and_default_alpn() {
        let uri = "hysteria2://hy2pass@jp.example.com:443,10000-20000?sni=jp.example.com&mport=30000-40000#hop";
        let node = parse_uri(uri).unwrap();
        assert_eq!(node["server"], "jp.example.com");
        assert_eq!(node["server_port"], 443);
        assert_eq!(node["server_ports"], json!(["10000:20000", "30000:40000"]));
        assert_eq!(node["tls"]["alpn"], json!(["h3"]));
    }
    #[test]
    fn test_parses_clash_yaml_mixed_proxies() {
        let yaml = r#"
port: 7890
socks-port: 7891
proxies:
  - name: "SS-Node"
    type: ss
    server: 192.168.100.1
    port: 8388
    cipher: aes-256-gcm
    password: password
  - name: "VMess-Node"
    type: vmess
    server: hk.example.com
    port: 443
    uuid: 11111111-1111-1111-1111-111111111111
    alterId: 0
    cipher: auto
    network: ws
    tls: true
    servername: hk.example.com
    ws-opts:
      path: /ws?ed=2048
      headers:
        Host: hk.example.com
  - name: "VLESS-Reality"
    type: vless
    server: jp.example.com
    port: 443
    uuid: 22222222-2222-2222-2222-222222222222
    flow: xtls-rprx-vision
    network: tcp
    servername: www.microsoft.com
    client-fingerprint: chrome
    reality-opts:
      public-key: PublicKeyReality
      short-id: ab
  - name: "Trojan-Node"
    type: trojan
    server: tw.example.com
    port: 443
    password: trojansecret
    network: ws
    sni: tw.example.com
    ws-opts:
      path: /trojan
  - name: "HY2-Node"
    type: hysteria2
    server: jp.example.com
    port: 8443
    password: hy2pass
    ports: 10000-20000
    obfs: salamander
    obfs-password: obfs-secret
    sni: jp.example.com
    skip-cert-verify: true
  - name: "Unsupported-Snell"
    type: snell
    server: 198.51.100.1
    port: 1324
    psk: secret
"#;
        let nodes = parse_subscription(yaml).unwrap();
        assert_eq!(nodes.len(), 5);
        let types: Vec<&str> = nodes.iter().map(|n| n["type"].as_str().unwrap()).collect();
        assert_eq!(
            types,
            ["shadowsocks", "vmess", "vless", "trojan", "hysteria2"]
        );

        // SS
        assert_eq!(nodes[0]["tag"], "SS-Node");
        assert_eq!(nodes[0]["server"], "192.168.100.1");
        assert_eq!(nodes[0]["server_port"], 8388);
        assert_eq!(nodes[0]["method"], "aes-256-gcm");
        assert_eq!(nodes[0]["password"], "password");

        // VMess
        assert_eq!(nodes[1]["tag"], "VMess-Node");
        assert_eq!(nodes[1]["server"], "hk.example.com");
        assert_eq!(nodes[1]["server_port"], 443);
        assert_eq!(nodes[1]["uuid"], "11111111-1111-1111-1111-111111111111");
        assert_eq!(nodes[1]["transport"]["type"], "ws");
        assert_eq!(nodes[1]["transport"]["path"], "/ws");
        assert_eq!(nodes[1]["transport"]["max_early_data"], 2048);
        assert_eq!(nodes[1]["tls"]["enabled"], true);
        assert_eq!(nodes[1]["tls"]["server_name"], "hk.example.com");

        // VLESS
        assert_eq!(nodes[2]["tag"], "VLESS-Reality");
        assert_eq!(nodes[2]["flow"], "xtls-rprx-vision");
        assert_eq!(nodes[2]["tls"]["reality"]["enabled"], true);
        assert_eq!(nodes[2]["tls"]["reality"]["public_key"], "PublicKeyReality");
        assert_eq!(nodes[2]["tls"]["reality"]["short_id"], "ab");
        assert_eq!(nodes[2]["tls"]["utls"]["fingerprint"], "chrome");

        // Trojan
        assert_eq!(nodes[3]["tag"], "Trojan-Node");
        assert_eq!(nodes[3]["password"], "trojansecret");
        assert_eq!(nodes[3]["transport"]["type"], "ws");
        assert_eq!(nodes[3]["tls"]["enabled"], true);
        assert_eq!(nodes[3]["tls"]["server_name"], "tw.example.com");

        // Hysteria2
        assert_eq!(nodes[4]["tag"], "HY2-Node");
        assert_eq!(nodes[4]["password"], "hy2pass");
        assert_eq!(nodes[4]["server_ports"], json!(["10000:20000"]));
        assert_eq!(nodes[4]["obfs"]["type"], "salamander");
        assert_eq!(nodes[4]["obfs"]["password"], "obfs-secret");
        assert_eq!(nodes[4]["tls"]["insecure"], true);
        assert_eq!(nodes[4]["tls"]["alpn"], json!(["h3"]));
    }

    #[test]
    fn test_parses_clash_yaml_ss_plugin_uot_and_multiplex() {
        let yaml = r#"
proxies:
  - name: "SS-Plugin"
    type: ss
    server: 203.0.113.8
    port: 8388
    cipher: chacha20-poly1305
    password: secret
    plugin: obfs
    plugin-opts:
      mode: http
      host: download.windowsupdate.com
    uot: true
    smux:
      enabled: true
      protocol: smux
      max-streams: 16
"#;
        let nodes = parse_subscription(yaml).unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0]["method"], "chacha20-ietf-poly1305");
        assert_eq!(nodes[0]["plugin"], "obfs-local");
        assert_eq!(
            nodes[0]["plugin_opts"],
            "obfs=http;obfs-host=download.windowsupdate.com"
        );
        assert_eq!(nodes[0]["udp_over_tcp"]["enabled"], true);
        assert_eq!(nodes[0]["multiplex"]["enabled"], true);
        assert_eq!(nodes[0]["multiplex"]["protocol"], "smux");
        assert_eq!(nodes[0]["multiplex"]["max_streams"], 16);
    }

    #[test]
    fn test_parses_clash_yaml_grpc_and_hy2_alias() {
        let yaml = r#"
proxies:
  - name: "VLESS-GRPC"
    type: vless
    server: hk.example.com
    port: 443
    uuid: 11111111-1111-1111-1111-111111111111
    network: grpc
    grpc-opts:
      grpc-service-name: GunService
    tls: true
    servername: hk.example.com
  - name: "HY2-Alias"
    type: hy2
    server: jp.example.com
    port: 443
    auth: secret
    mport: 30000-40000
    up: 50
    down: 200
"#;
        let nodes = parse_subscription(yaml).unwrap();
        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[0]["type"], "vless");
        assert_eq!(nodes[0]["transport"]["type"], "grpc");
        assert_eq!(nodes[0]["transport"]["service_name"], "GunService");

        assert_eq!(nodes[1]["type"], "hysteria2");
        assert_eq!(nodes[1]["password"], "secret");
        assert_eq!(nodes[1]["server_ports"], json!(["30000:40000"]));
        assert_eq!(nodes[1]["up_mbps"], 50);
        assert_eq!(nodes[1]["down_mbps"], 200);
    }

    #[test]
    fn test_clash_yaml_empty_or_only_unknown_returns_err() {
        let empty_yaml = "proxies: []";
        assert!(parse_subscription(empty_yaml).is_err());

        let unknown_yaml = r#"
proxies:
  - name: "WireGuard"
    type: wireguard
    server: 1.1.1.1
    port: 51820
"#;
        assert!(parse_subscription(unknown_yaml).is_err());
    }
}
