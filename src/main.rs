use clap::Parser;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{Level, error, info};
use tracing_subscriber::FmtSubscriber;

use canto::cli::{Cli, Commands, ConfigCommands, RunArgs};
use canto::config::{Settings, TemplateEngine};
use canto::error::{CantoError, Result};
use canto::network::NetworkGuard;
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
        Commands::Start => handle_start(settings).await,
        Commands::Stop => handle_stop(settings).await,
        Commands::Status => handle_status(settings).await,
        Commands::Config(cmd) => handle_config(cmd, settings).await,
        Commands::CleanNetwork => handle_clean_network(settings),
    }
}

async fn handle_run(args: RunArgs, settings: Settings) -> Result<()> {
    info!("Starting canto orchestrator");

    // Optional config generation from templates
    if args.recompile_config
        || (settings.template.enabled && !settings.singbox.config_path.exists())
    {
        info!("Compiling sing-box configuration from templates...");
        compile_templates(&settings, None, None)?;
    }

    let supervisor = ProcessSupervisor::new(
        settings.singbox.binary.clone(),
        settings.singbox.config_path.clone(),
        settings.canto.work_dir.clone(),
    );

    // Apply network rules (RAII guard will tear them down upon drop)
    let _network_guard = if settings.network.enabled && !args.no_network {
        Some(NetworkGuard::setup(settings.network.clone())?)
    } else {
        info!("Network rules disabled (pure proxy mode)");
        None
    };

    supervisor.run_supervised().await?;
    info!("canto shutdown complete");
    Ok(())
}

async fn handle_start(_settings: Settings) -> Result<()> {
    info!(
        "canto is designed to run in foreground under system supervisors (systemd, OpenRC, Docker)."
    );
    info!("Tip: To run as a background service, register 'canto run' in your systemd service.");
    Ok(())
}

async fn handle_stop(settings: Settings) -> Result<()> {
    info!("Stopping canto service and flushing rules");
    NetworkGuard::teardown_manual(&settings.network)?;
    Ok(())
}

async fn handle_status(settings: Settings) -> Result<()> {
    info!("Checking canto environment status...");

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
            "sing-box config: Found ({})",
            settings.singbox.config_path.display()
        );
        let _ = supervisor.check_config(None);
    } else {
        info!(
            "sing-box config: Not generated yet ({})",
            settings.singbox.config_path.display()
        );
    }

    info!("Proxy mode: {:?}", settings.network.mode);
    info!(
        "Transparent routing mark: {:#x}",
        settings.network.routing_mark
    );
    Ok(())
}

async fn handle_config(cmd: ConfigCommands, settings: Settings) -> Result<()> {
    match cmd {
        ConfigCommands::Generate {
            template_dir,
            output,
        } => compile_templates(&settings, template_dir.as_deref(), output.as_deref()),
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
    }
}

fn handle_clean_network(settings: Settings) -> Result<()> {
    NetworkGuard::teardown_manual(&settings.network)
}

fn compile_templates(
    settings: &Settings,
    override_dir: Option<&Path>,
    override_output: Option<&Path>,
) -> Result<()> {
    let t_dir = override_dir.unwrap_or(&settings.template.template_dir);
    let out_path = override_output.unwrap_or(&settings.singbox.config_path);

    let template_paths: Vec<PathBuf> = settings
        .template
        .files
        .iter()
        .map(|f| t_dir.join(f))
        .collect();

    for p in &template_paths {
        if !p.exists() {
            return Err(CantoError::Config(format!(
                "Required template file does not exist: {}",
                p.display()
            )));
        }
    }

    let engine = TemplateEngine::new(settings.template.direct_domains.clone());
    let merged = engine.merge_files(&template_paths)?;
    TemplateEngine::write_to_file(&merged, out_path)?;
    Ok(())
}
