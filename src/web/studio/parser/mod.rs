use serde_json::Value;

mod clash;
mod protocols;
mod utils;

use clash::try_parse_clash_yaml;
use protocols::{parse_hysteria2, parse_ss, parse_trojan, parse_vless, parse_vmess};
use utils::{decode_base64_to_string, split_scheme, uniquify_tags};

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

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;
    use serde_json::json;

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
