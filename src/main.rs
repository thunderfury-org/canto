use clap::Parser;
use std::fs;
use std::path::Path;
use tracing::{Level, error, info};
use tracing_subscriber::FmtSubscriber;

use canto::cli::{Cli, Commands, ConfigCommands, RunArgs};
use canto::config::{Settings, prepare_runtime_config, write_runtime_config};
use canto::error::Result;
use canto::network::{NetworkGuard, NftablesManager, resolve_lan_cidrs};
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

    let apply_network = settings.network.should_apply_capture(args.no_network)?;
    let runtime_path = write_overlay(&settings, None)?;

    let supervisor = ProcessSupervisor::new(
        settings.singbox.binary.clone(),
        runtime_path,
        settings.canto.work_dir.clone(),
    );

    supervisor.verify_binary()?;
    supervisor.check_config(None)?;

    let _network_guard = if apply_network {
        Some(NetworkGuard::setup(settings.network.clone())?)
    } else {
        info!("Network rules disabled (pure proxy mode)");
        None
    };

    supervisor.run_supervised().await?;
    info!("canto shutdown complete");
    Ok(())
}

async fn handle_status(settings: Settings) -> Result<()> {
    info!("Checking canto environment status...");

    info!("sing-box source: {}", settings.singbox.source.display());
    if settings.singbox.source.exists() {
        info!("sing-box source: Found");
    } else {
        error!(
            "sing-box source: Missing ({})",
            settings.singbox.source.display()
        );
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

    info!("tproxy fwmark: {:#x}", settings.network.fwmark);
    info!(
        "sing-box routing mark: {:#x}",
        settings.network.routing_mark
    );
    match resolve_lan_cidrs(&settings.network.lan_cidrs) {
        Ok(cidrs) => info!("LAN CIDRs: {}", cidrs.join(", ")),
        Err(e) => error!("LAN CIDRs: {e}"),
    }
    Ok(())
}

async fn handle_config(cmd: ConfigCommands, settings: Settings) -> Result<()> {
    match cmd {
        ConfigCommands::Generate { output } => {
            write_overlay(&settings, output.as_deref())?;
            Ok(())
        }
        ConfigCommands::Check { config } => {
            let supervisor = ProcessSupervisor::new(
                settings.singbox.binary.clone(),
                settings.singbox.config_path.clone(),
                settings.canto.work_dir.clone(),
            );
            supervisor.check_config(config.as_deref())
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
            print!("{}", NftablesManager::new(&network).generate_ruleset());
            Ok(())
        }
    }
}

fn handle_clean_network(settings: Settings) -> Result<()> {
    NetworkGuard::teardown_manual(&settings.network)
}

fn write_overlay(settings: &Settings, output: Option<&Path>) -> Result<std::path::PathBuf> {
    let out_path = output.unwrap_or(&settings.singbox.config_path);
    info!(
        "Loading source config from {}",
        settings.singbox.source.display()
    );
    let runtime = prepare_runtime_config(settings)?;
    write_runtime_config(&runtime, out_path)?;
    Ok(out_path.to_path_buf())
}
