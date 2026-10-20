use serde_json::{Value, json};

use super::utils::{
    CommonLink, build_tls, build_transport, decode_base64_to_string,
    fill_tls_server_name_from_ws_host, hy2_server_ports, json_string, json_u16,
    multiplex_from_query, multiplex_from_value, normalize_ss_method, outbound_base,
    parse_common_link, parse_common_link_full, parse_leading_int, percent_decode, qget, split_csv,
    split_fragment, split_host_port, split_query, split_userinfo_host, tag_of, tls_from_query,
    transport_from_query, truthy,
};

pub(super) fn parse_ss(rest: &str) -> Result<Value, String> {
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

pub(super) fn parse_vmess(rest: &str) -> Result<Value, String> {
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

pub(super) fn parse_vless(rest: &str) -> Result<Value, String> {
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

pub(super) fn parse_trojan(rest: &str) -> Result<Value, String> {
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

pub(super) fn parse_hysteria2(rest: &str) -> Result<Value, String> {
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
