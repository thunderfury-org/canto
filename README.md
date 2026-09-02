# canto

canto is a lightweight transparent proxy and configuration orchestrator designed specifically for sing-box.

Built in Rust as a single static binary, canto coordinates network routing policies, nftables transparent proxy rules, JSON configuration templates, and sing-box process lifecycles without external scripting dependencies.

For the in-depth architectural breakdown and design principles, see [docs/DESIGN.md](docs/DESIGN.md).

## Key Features

- **Process Supervision**: Manages sing-box lifecycle with asynchronous streaming logs, configuration pre-flight validation, and graceful termination handling.
- **Atomic Network Orchestration**: Employs `nftables` tables and Linux policy routing (`ip rule` / `ip route`) for transparent redirection.
- **Fail-safe Network Guard**: Implements RAII-based cleanup ensuring firewall rules and routing policies are automatically rolled back upon program exit or crash, preventing network outages.
- **JSON Template Engine**: Deep-merges modular sing-box template fragments (log, dns, inbounds, outbounds, route) and injects direct domain anchors.
- **Zero Shell Dependencies**: Standalone Rust application without dependencies on bash, awk, sed, or busybox idiosyncrasies.

## Architecture

```
src/
├── main.rs            # Entry point, CLI dispatcher, and logging setup
├── lib.rs             # Public crate exports
├── cli/               # Command-line definitions (clap derive)
│   ├── mod.rs
│   └── args.rs
├── config/            # Configuration management
│   ├── mod.rs
│   ├── settings.rs    # canto settings (canto.toml)
│   └── template.rs    # Deep merge engine for sing-box JSON templates
├── network/           # Transparent proxy network orchestration
│   ├── mod.rs
│   ├── nftables.rs    # nftables ruleset generator and manager
│   ├── route.rs       # Policy routing manager (ip rule / ip route)
│   └── guard.rs       # RAII lifecycle guard for automatic rollback
├── supervisor/        # Child process supervision
│   ├── mod.rs
│   └── process.rs     # sing-box process runner, validator, and log stream
└── error.rs           # Unified error handling (thiserror)
```

## Quick Start

### 1. Build

```bash
cargo build --release
```

### 2. Initialize Configuration

Generate a default `canto.toml`:

```bash
cargo run -- config init
```

### 3. Check Status

Verify your local sing-box environment and configuration:

```bash
cargo run -- status
```

### 4. Run

Launch canto in foreground with managed transparent proxying:

```bash
cargo run -- run
```

Or run in pure proxy mode without applying nftables rules:

```bash
cargo run -- run --no-network
```

## Subcommands

- `canto run`: Run in foreground, managing transparent proxy rules and sing-box process.
- `canto status`: Inspect sing-box binary availability, config validity, and proxy parameters.
- `canto config generate`: Deep-merge template fragments into a unified `config.json`.
- `canto config check`: Validate sing-box configuration syntax using `sing-box check`.
- `canto config init`: Initialize a default `canto.toml`.
- `canto clean-network`: Emergency manual teardown of canto nftables table and policy routes.

## Documentation

Detailed technical documentation is available under `docs/`:
- [System Architecture & Design Document (docs/DESIGN.md)](docs/DESIGN.md)

## License

MIT
