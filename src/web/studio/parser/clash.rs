use serde_json::{Value, json};

use super::utils::{
    build_tls, fill_tls_server_name_from_ws_host, json_bool, json_string, json_u16, normalize_path,
    normalize_port_ranges, normalize_ss_method, outbound_base, parse_leading_int, split_csv,
    split_early_data,
};

pub(super) fn try_parse_clash_yaml(input: &str) -> Option<Result<Vec<Value>, String>> {
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

pub(super) fn yaml_to_json_value(val: serde_yaml::Value) -> Value {
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

pub(super) fn parse_clash_proxy(value: &Value) -> Option<Value> {
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
