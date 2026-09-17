use axum::http::StatusCode;
use canto::config::WebSettings;
use canto::config::{
    HttpFetcher, NetworkSettings, RefreshOutcome, SourceLocator, apply_runtime_overlay,
    obtain_source, refresh_source, source_cache_path, write_runtime_config, write_source_cache,
};
use canto::supervisor::ProcessSupervisor;
use canto::web::WebServer;
use serde_json::{Value, json};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::oneshot;

const ADMIN_TOKEN: &str = "test_secret_token";

struct StudioServer {
    addr: SocketAddr,
    work_dir: PathBuf,
    shutdown: Option<oneshot::Sender<()>>,
    task: Option<tokio::task::JoinHandle<canto::error::Result<()>>>,
    http: reqwest::Client,
}

impl StudioServer {
    async fn start() -> Self {
        let work_dir = temp_work_dir();
        let settings = WebSettings {
            enabled: true,
            listen: "127.0.0.1:0".to_string(),
            admin_token: ADMIN_TOKEN.to_string(),
            public_url: None,
        };
        let server = WebServer::new(settings, work_dir.clone());
        let bound = server.bind().await.expect("bind web studio");
        let addr = bound.local_addr();
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let task = tokio::spawn(async move {
            bound
                .run_with_signal(async {
                    let _ = shutdown_rx.await;
                })
                .await
        });

        let http = reqwest::Client::new();
        wait_for_http(&http, &format!("http://{addr}/")).await;

        Self {
            addr,
            work_dir,
            shutdown: Some(shutdown_tx),
            task: Some(task),
            http,
        }
    }

    fn base(&self) -> String {
        format!("http://{}", self.addr)
    }

    async fn json(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<Value>,
    ) -> (StatusCode, Value) {
        let mut req = self
            .http
            .request(method, format!("{}{path}", self.base()))
            .header("Authorization", format!("Bearer {ADMIN_TOKEN}"));
        if let Some(body) = body {
            req = req
                .header("Content-Type", "application/json")
                .body(body.to_string());
        }
        let response = req.send().await.expect("studio request");
        let status = StatusCode::from_u16(response.status().as_u16()).unwrap();
        let bytes = response.bytes().await.unwrap();
        let value = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap_or(Value::Null)
        };
        (status, value)
    }

    async fn seed_profile(&self) -> (String, String) {
        let template_body = json!({
            "name": "网关模板",
            "content": {
                "log": { "level": "warn" },
                "inbounds": [{ "type": "tun", "tag": "tun-in" }],
                "outbounds": [
                    { "type": "selector", "tag": "默认策略", "outbounds": ["香港节点", "直连"] },
                    { "type": "urltest", "tag": "香港节点", "outbounds": ["{(?i)(港|hk)}"] },
                    { "type": "direct", "tag": "直连" }
                ]
            }
        });
        let (status, template) = self
            .json(reqwest::Method::POST, "/api/templates", Some(template_body))
            .await;
        assert_eq!(status, StatusCode::CREATED, "{template}");
        let template_id = template["id"].as_str().unwrap().to_string();

        let source_body = json!({
            "name": "自建节点",
            "type": "manual",
            "nodes": [
                { "type": "vless", "tag": "HK-01", "server": "hk.example.com", "server_port": 443 }
            ]
        });
        let (status, source) = self
            .json(reqwest::Method::POST, "/api/sources", Some(source_body))
            .await;
        assert_eq!(status, StatusCode::CREATED, "{source}");
        let source_id = source["id"].as_str().unwrap().to_string();

        let profile_body = json!({
            "name": "家庭网关",
            "templateId": template_id,
            "sourceIds": [source_id]
        });
        let (status, profile) = self
            .json(reqwest::Method::POST, "/api/profiles", Some(profile_body))
            .await;
        assert_eq!(status, StatusCode::CREATED, "{profile}");
        let token = profile["token"].as_str().unwrap().to_string();
        (source_id, token)
    }
}

impl Drop for StudioServer {
    fn drop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        if let Some(task) = self.task.take() {
            task.abort();
        }
        let _ = std::fs::remove_dir_all(&self.work_dir);
    }
}

async fn wait_for_http(client: &reqwest::Client, url: &str) {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    loop {
        if client.get(url).send().await.is_ok() {
            return;
        }
        if tokio::time::Instant::now() >= deadline {
            panic!("web studio did not become ready at {url}");
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

fn temp_work_dir() -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "canto_loop_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&path).unwrap();
    path
}

fn gateway_network() -> NetworkSettings {
    NetworkSettings {
        bypass_cn: false,
        ..NetworkSettings::default()
    }
}

#[tokio::test]
async fn test_gateway_fetches_studio_source_and_overlays_tproxy() {
    let studio = StudioServer::start().await;
    let (_source_id, token) = studio.seed_profile().await;
    let url = format!("{}/sub/{token}", studio.base());
    let locator = SourceLocator::parse(&url).unwrap();
    let cache = source_cache_path(&studio.work_dir);

    let raw = obtain_source(&locator, &cache, &HttpFetcher::default())
        .await
        .expect("fetch studio source");
    assert_eq!(raw["inbounds"][0]["tag"], "tun-in");
    assert_eq!(raw["outbounds"][3]["tag"], "HK-01");
    assert_eq!(raw["outbounds"][1]["outbounds"][0], "HK-01");

    let overlayed = apply_runtime_overlay(raw, &gateway_network()).unwrap();
    assert_eq!(overlayed["inbounds"][0]["tag"], "mixed-in");
    assert_eq!(overlayed["inbounds"][1]["tag"], "tproxy-in");
    assert_eq!(overlayed["inbounds"][2]["tag"], "dns-in");
    assert_eq!(overlayed["outbounds"][3]["tag"], "HK-01");
    assert_eq!(overlayed["route"]["auto_detect_interface"], false);

    let runtime_path = studio.work_dir.join("config.json");
    write_runtime_config(&overlayed, &runtime_path).unwrap();
    let supervisor = ProcessSupervisor::new(
        PathBuf::from("sing-box"),
        runtime_path.clone(),
        studio.work_dir.clone(),
    );
    if supervisor.verify_binary().is_ok() {
        supervisor
            .check_config(Some(&runtime_path))
            .expect("studio overlay should pass sing-box check");
    }
}

#[tokio::test]
async fn test_gateway_refresh_applies_studio_updates_and_skips_unchanged() {
    let studio = StudioServer::start().await;
    let (source_id, token) = studio.seed_profile().await;
    let url = format!("{}/sub/{token}", studio.base());
    let locator = SourceLocator::parse(&url).unwrap();
    let fetcher = HttpFetcher::default();
    let network = gateway_network();

    let first = refresh_source(&locator, &fetcher, &network, None, |_| Ok(())).await;
    let RefreshOutcome::Apply { overlayed, .. } = first else {
        panic!("expected Apply on first studio fetch");
    };
    assert_eq!(overlayed["outbounds"][3]["tag"], "HK-01");
    assert_eq!(overlayed["inbounds"][0]["tag"], "mixed-in");

    let unchanged =
        refresh_source(&locator, &fetcher, &network, Some(&overlayed), |_| Ok(())).await;
    assert!(
        matches!(unchanged, RefreshOutcome::KeepCurrent),
        "identical studio source should not restart"
    );

    let update = json!({
        "nodes": [
            { "type": "vless", "tag": "HK-01", "server": "hk.example.com", "server_port": 443 },
            { "type": "trojan", "tag": "HK-02", "server": "hk2.example.com", "server_port": 443 }
        ]
    });
    let (status, updated) = studio
        .json(
            reqwest::Method::PUT,
            &format!("/api/sources/{source_id}"),
            Some(update),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{updated}");
    assert_eq!(updated["nodeCount"], 2);

    let applied = refresh_source(&locator, &fetcher, &network, Some(&overlayed), |_| Ok(())).await;
    let RefreshOutcome::Apply {
        overlayed: next, ..
    } = applied
    else {
        panic!("expected Apply after studio published new nodes");
    };
    let tags: Vec<&str> = next["outbounds"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["tag"].as_str().unwrap())
        .collect();
    assert!(tags.contains(&"HK-01"), "{tags:?}");
    assert!(tags.contains(&"HK-02"), "{tags:?}");
}

#[tokio::test]
async fn test_gateway_refresh_keeps_current_for_invalid_profile_token() {
    let studio = StudioServer::start().await;
    let (_source_id, token) = studio.seed_profile().await;
    let good_url = format!("{}/sub/{token}", studio.base());
    let bad_url = format!("{}/sub/tok_does_not_exist", studio.base());
    let fetcher = HttpFetcher::default();
    let cache = studio.work_dir.join("source-cache.json");

    let raw = obtain_source(&SourceLocator::parse(&good_url).unwrap(), &cache, &fetcher)
        .await
        .unwrap();
    write_source_cache(&cache, &raw).unwrap();

    let overlayed = apply_runtime_overlay(raw, &gateway_network()).unwrap();
    let outcome = refresh_source(
        &SourceLocator::parse(&bad_url).unwrap(),
        &fetcher,
        &gateway_network(),
        Some(&overlayed),
        |_| panic!("check should not run for invalid token"),
    )
    .await;
    assert!(matches!(outcome, RefreshOutcome::KeepCurrent));

    let fallback = obtain_source(&SourceLocator::parse(&bad_url).unwrap(), &cache, &fetcher)
        .await
        .unwrap();
    assert_eq!(fallback["outbounds"][3]["tag"], "HK-01");
}
