# canto

canto is a lightweight transparent proxy and configuration orchestrator designed specifically for sing-box.

Built in Rust as a single static binary, canto loads a complete sing-box JSON, overlays gateway inbounds, overlays gateway TUN inbounds with auto_redirect for LAN and local traffic, and supervises the sing-box process.

For the in-depth architectural breakdown and design principles, see [docs/DESIGN.md](docs/DESIGN.md).

## Key Features

- **Source overlay**: Loads a complete sing-box JSON from a local file or HTTP(S) URL, replaces `inbounds` with mixed/tproxy/dns listeners, writes `route.default_mark`, and disables `auto_detect_interface`. URL sources refresh on an interval; a failed refresh keeps the current process and nftables rules.
- **Process Supervision**: Manages sing-box lifecycle with asynchronous streaming logs, configuration pre-flight validation, and graceful termination handling.
- **Atomic Network Orchestration**: Default capture is TUN + `auto_redirect` (no canto nftables). `mode = "tproxy"` still uses `nftables` and policy routing.
- **Fail-safe Network Guard**: Implements RAII-based cleanup ensuring firewall rules and routing policies are rolled back on SIGINT/SIGTERM or process exit.
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
│   └── overlay.rs     # Runtime inbound / default_mark overlay
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

Point `[singbox].source` at a complete sing-box JSON file or an HTTP(S) URL that serves one.

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

- `canto run`: Overlay the source JSON, validate it, apply tproxy rules, and supervise sing-box.
- `canto status`: Inspect sing-box binary availability, source/runtime config, and proxy parameters.
- `canto config generate`: Load `source`, overlay inbounds / `default_mark`, and write `config_path`.
- `canto config check`: Validate sing-box configuration syntax using `sing-box check`.
- `canto config init`: Initialize a default `canto.toml`.
- `canto config dump-nft`: Print the tproxy nftables ruleset, or a note that the TUN path has none.
- `canto clean-network`: Emergency manual teardown of canto nftables table and policy routes.

## Gateway check without a router

Unit tests and `canto config dump-nft` do not send packets. `scripts/netns-check.sh` builds a LAN/gateway/WAN topology with network namespaces and checks TUN hijack, CN/port bypass, local hijack, WAN port reject, leftover rollback, HTTP(S) source fetch, last-good cache, refresh, and a tproxy escape-hatch regression. CI runs this on every pull request.

On Linux as root:

```bash
sudo ./scripts/netns-check.sh
```

From macOS with OrbStack:

```bash
docker run --rm --privileged -e CARGO_TARGET_DIR=/tmp/canto-target \
  -v "$PWD":/src -w /src rust:bookworm bash scripts/netns-check.sh
```

## Documentation

Detailed technical documentation is available under `docs/`:
- [System Architecture & Design Document (docs/DESIGN.md)](docs/DESIGN.md)
- [Transparent inbound selection (docs/INBOUND.md)](docs/INBOUND.md)
- [Replace ShellCrash on a test Linux gateway (docs/linux-test-gateway.md)](docs/linux-test-gateway.md)

## License

MIT
