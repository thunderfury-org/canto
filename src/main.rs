use clap::Parser;
use std::fs;
use std::time::Duration;
use tracing::{Level, error, info, warn};
use tracing_subscriber::FmtSubscriber;

use canto::cli::{Cli, Commands, ConfigCommands, RunArgs};
use canto::config::{HttpFetcher, PortsFilter, RuntimeConfigEngine, Settings};
use canto::error::{CantoError, Result};
use canto::network::{
    NetworkGuard, NftablesManager, cn_ip_path, load_cn_ip, read_cn_ip_file, resolve_lan_cidrs,
};
use canto::supervisor::ProcessSupervisor;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    init_tracing(cli.verbose);

    if let Err(e) = run_app(cli).await {
        error!("{e}");
        std::process::exit(1);
    }
}

fn init_tracing(verbose: u8) {
    let level = match verbose {
        0 => Level::INFO,
        1 => Level::DEBUG,
        _ => Level::TRACE,
    };

    let subscriber = FmtSubscriber::builder()
        .with_max_level(level)
        .with_target(false)
        .with_thread_ids(false)
        .with_writer(std::io::stderr)
        .finish();

    let _ = tracing::subscriber::set_global_default(subscriber);
}

async fn run_app(cli: Cli) -> Result<()> {
    let settings = Settings::load(cli.config.as_deref())?;

    match cli.command {
        Commands::Run(args) => handle_run(args, settings).await,
        Commands::Status => handle_status(settings).await,
        Commands::Config(cmd) => handle_config(cmd, settings).await,
        Commands::CleanNetwork => handle_clean_network(settings),
    }
}

async fn handle_run(args: RunArgs, settings: Settings) -> Result<()> {
    info!("Starting canto orchestrator");

    fs::create_dir_all(&settings.canto.work_dir)?;

    // Server-only mode: network capture disabled and Web Studio enabled.
    // Operates purely as a configuration generator and web distribution service,
    // requiring no root privileges or sing-box process supervisor.
    if !settings.network.enabled && settings.web.enabled {
        info!("canto running in server-only mode (Web Studio)");
        let web_server =
            canto::web::WebServer::new(settings.web.clone(), settings.canto.work_dir.clone());
        web_server.run().await?;
        info!("canto shutdown complete");
        return Ok(());
    }

    let apply_network = settings.network.should_apply_capture(args.no_network)?;
    let fetcher = HttpFetcher::default();

    let supervisor = ProcessSupervisor::new(
        settings.singbox.binary.clone(),
        settings.singbox.config_path.clone(),
        settings.canto.work_dir.clone(),
    );
    supervisor.verify_binary()?;

    let config_engine = RuntimeConfigEngine::from_settings(&settings)?;
    config_engine.prepare_initial().await?;

    let cnip = if apply_network && settings.network.bypass_cn {
        load_cn_ip(&settings.canto.work_dir, &fetcher).await?
    } else {
        Vec::new()
    };

    let _network_guard = if apply_network {
        Some(NetworkGuard::setup(settings.network.clone(), &cnip)?)
    } else {
        info!("Network rules disabled (pure proxy mode)");
        None
    };

    let (web_shutdown_tx, web_shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let web_task = if settings.web.enabled {
        let web_server =
            canto::web::WebServer::new(settings.web.clone(), settings.canto.work_dir.clone());
        let bound_server = web_server.bind().await?;
        Some(tokio::spawn(async move {
            bound_server
                .run_with_signal(async {
                    let _ = web_shutdown_rx.await;
                })
                .await
        }))
    } else {
        None
    };

    let supervisor_fut = async {
        if config_engine.is_url_source() && settings.singbox.refresh_interval_secs > 0 {
            let interval = Duration::from_secs(settings.singbox.refresh_interval_secs);
            info!(
                "Refreshing URL source every {} seconds",
                settings.singbox.refresh_interval_secs
            );
            supervisor
                .run_supervised_with_refresh(interval, || async {
                    match config_engine.refresh().await {
                        Ok(outcome) if outcome.is_applied() => true,
                        Ok(_) => false,
                        Err(e) => {
                            warn!("Refresh failed: {e}");
                            false
                        }
                    }
                })
                .await
        } else {
            supervisor.run_supervised().await
        }
    };

    let run_res = if let Some(mut task) = web_task {
        tokio::select! {
            res = supervisor_fut => {
                let _ = web_shutdown_tx.send(());
                match (&mut task).await {
                    Ok(Err(e)) => error!("Web Studio server error on exit: {e}"),
                    Err(e) => warn!("Web Studio task join error: {e}"),
                    Ok(Ok(())) => {}
                }
                res
            }
            web_join_res = &mut task => {
                match web_join_res {
                    Ok(Err(e)) => Err(e),
                    Err(e) => Err(CantoError::Web(format!("Web Studio task panicked: {e}"))),
                    Ok(Ok(())) => Err(CantoError::Web("Web Studio server exited unexpectedly".to_string())),
                }
            }
        }
    } else {
        supervisor_fut.await
    };

    run_res?;
    info!("canto shutdown complete");
    Ok(())
}

async fn handle_status(settings: Settings) -> Result<()> {
    info!("Checking canto environment status...");

    match RuntimeConfigEngine::from_settings(&settings) {
        Ok(engine) => match engine.obtain_raw_source().await {
            Ok(_) => info!("sing-box source: Available ({})", engine.locator()),
            Err(e) => error!("sing-box source: Unavailable ({}: {e})", engine.locator()),
        },
        Err(e) => error!("sing-box source: {e}"),
    }

    let supervisor = ProcessSupervisor::new(
        settings.singbox.binary.clone(),
        settings.singbox.config_path.clone(),
        settings.canto.work_dir.clone(),
    );

    match supervisor.verify_binary() {
        Ok(_) => info!(
            "sing-box binary: Available ({})",
            settings.singbox.binary.display()
        ),
        Err(e) => error!("sing-box binary: Missing or unusable ({e})"),
    }

    if settings.web.enabled {
        info!(
            "web studio: Enabled (http://{}, token={})",
            settings.web.listen,
            if settings.web.admin_token.is_empty() {
                "none"
            } else {
                "configured"
            }
        );
        info!(
            "web studio public url: {}",
            settings.web.resolve_public_url()
        );
    } else {
        info!("web studio: Disabled");
    }

    if settings.singbox.config_path.exists() {
        info!(
            "runtime config: Found ({})",
            settings.singbox.config_path.display()
        );
        let _ = supervisor.check_config(None);
    } else {
        info!(
            "runtime config: Not generated yet ({})",
            settings.singbox.config_path.display()
        );
    }

    info!("capture: tproxy");
    info!("bypass_cn: {}", settings.network.bypass_cn);
    info!("tproxy fwmark: {:#x}", settings.network.fwmark);
    info!(
        "sing-box routing mark: {:#x}",
        settings.network.routing_mark
    );
    info!(
        "proxy scope: lan={}, local={}, docker={}",
        settings.network.lan, settings.network.local, settings.network.docker
    );
    let ports_str = match &settings.network.ports {
        PortsFilter::Common => "common".to_string(),
        PortsFilter::All => "all".to_string(),
        PortsFilter::Custom(ports) => format!("{ports:?}"),
    };
    info!(
        "proxy traffic: tcp={}, udp={}, ports={}",
        settings.network.tcp, settings.network.udp, ports_str
    );
    match resolve_lan_cidrs(&settings.network.lan_cidrs) {
        Ok(cidrs) => info!("LAN CIDRs: {}", cidrs.join(", ")),
        Err(e) => error!("LAN CIDRs: {e}"),
    }
    if settings.network.bypass_cn {
        let path = cn_ip_path(&settings.canto.work_dir);
        match read_cn_ip_file(&settings.canto.work_dir) {
            Ok(Some(cidrs)) => info!("cn_ip.txt: {} IPv4 CIDRs ({})", cidrs.len(), path.display()),
            Ok(None) => info!(
                "cn_ip.txt: missing ({}); will download on run",
                path.display()
            ),
            Err(e) => error!("cn_ip.txt: {e}"),
        }
    }
    Ok(())
}

async fn handle_config(cmd: ConfigCommands, settings: Settings) -> Result<()> {
    match cmd {
        ConfigCommands::Generate { output } => {
            let engine = RuntimeConfigEngine::from_settings(&settings)?;
            let path = engine.export_overlay(output.as_deref()).await?;
            info!("Generated runtime configuration at: {}", path.display());
            Ok(())
        }
        ConfigCommands::Check { config } => {
            let supervisor = ProcessSupervisor::new(
                settings.singbox.binary.clone(),
                settings.singbox.config_path.clone(),
                settings.canto.work_dir.clone(),
            );
            if let Some(target) = config.as_deref() {
                supervisor.check_config(Some(target))
            } else {
                let engine = RuntimeConfigEngine::from_settings(&settings)?;
                engine.export_overlay(None).await?;
                supervisor.check_config(None)
            }
        }
        ConfigCommands::Init { path } => {
            let toml_str = settings.to_toml_string()?;
            fs::write(&path, toml_str)?;
            info!(
                "Generated default configuration file at: {}",
                path.display()
            );
            Ok(())
        }
        ConfigCommands::DumpNft => {
            let mut network = settings.network;
            network.lan_cidrs = resolve_lan_cidrs(&network.lan_cidrs)?;
            info!("LAN CIDRs: {}", network.lan_cidrs.join(", "));
            let cnip = match read_cn_ip_file(&settings.canto.work_dir) {
                Ok(Some(cidrs)) => {
                    info!(
                        "CN CIDRs: {} prefixes from {}",
                        cidrs.len(),
                        cn_ip_path(&settings.canto.work_dir).display()
                    );
                    cidrs
                }
                Ok(None) => {
                    if network.bypass_cn {
                        warn!(
                            "bypass_cn is true but {} is missing; dump omits set cnip",
                            cn_ip_path(&settings.canto.work_dir).display()
                        );
                    }
                    Vec::new()
                }
                Err(e) => return Err(e),
            };
            print!("{}", NftablesManager::new(&network).with_cnip(&cnip).dump());
            Ok(())
        }
    }
}

fn handle_clean_network(settings: Settings) -> Result<()> {
    NetworkGuard::teardown_manual(&settings.network)
}
