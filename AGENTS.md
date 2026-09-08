# Repository Guidelines

## Project Structure & Module Organization

canto 是基于 Rust (Edition 2024) 构建的 sing-box 透明代理与配置编排工具。核心代码组织在 `src/` 目录下：

- `src/main.rs` & `src/lib.rs`：CLI 入口、追踪日志初始化及公共接口导出。
- `src/cli/`：基于 `clap` 的命令行参数与子命令定义（`args.rs`）。
- `src/config/`：应用配置（`settings.rs`，对应 `canto.toml`）与运行时覆盖（`overlay.rs`）。
- `src/network/`：透明代理网络编排，包括 nftables 规则生成（`nftables.rs`）、策略路由（`route.rs`）及 RAII 网络安全回滚守卫（`guard.rs`）。
- `src/supervisor/`：sing-box 子进程管理、异步流式日志分流与配置语法预检（`process.rs`）。
- `src/error.rs`：基于 `thiserror` 的统一强类型错误枚举 `CantoError` 与 `Result<T>`。
- `docs/`：架构文档。[DESIGN.md](docs/DESIGN.md) 是 v1 行为；[INBOUND.md](docs/INBOUND.md) 是 TUN / tproxy / auto_redirect / bypass 选型。改劫持路径、`network.mode` 或大陆 IP 绕过时先读 INBOUND。

## Build, Test, and Development Commands

- **语法与编译检查**：`cargo check`
- **本地构建**：`cargo build`（开发构建）或 `cargo build --release`（生产优化构建）。
- **运行单元测试**：`cargo test`
- **静态检查与格式化**：`cargo clippy` 及 `cargo fmt -- --check`
- **本地常用开发命令**：
  - 初始化配置：`cargo run -- config init`
  - 启动服务（跳过网络规则）：`cargo run -- run --no-network`
  - 检查配置与环境：`cargo run -- status`
  - 校验 sing-box 配置：`cargo run -- config check`
  - 紧急清理网络规则残留：`cargo run -- clean-network`
- **嵌入式/路由器静态目标交叉编译**：`cargo build --release --target x86_64-unknown-linux-musl`（或使用 `cross` 针对 `aarch64-unknown-linux-musl` 构建）。

## Coding Style & Naming Conventions

- **代码格式**：遵循 Rust 官方规范，使用 4 空格缩进，提交前须通过 `cargo fmt`。
- **命名规范**：
  - 结构体、枚举、Trait：大驼峰（`PascalCase`，如 `NetworkGuard`、`ProcessSupervisor`）。
  - 函数、方法、变量、模块及文件名：蛇形小写（`snake_case`，如 `apply_transformations`、`nftables.rs`）。
  - 常量与固定路由标记：大写蛇形（`SCREAMING_SNAKE_CASE`）。
- **错误处理**：统一扩充 `src/error.rs` 中的 `CantoError`，通过 `?` 向上传播；非测试代码严禁直接使用 `.unwrap()` 或 `.expect()`。
- **异常安全与 RAII**：修改系统网络状态（nftables / 策略路由）必须依托 `NetworkGuard` 实现 Drop 特征回滚，保证进程崩溃或接收中断信号时自动撤销规则。

## Testing Guidelines

- **测试组织**：单元测试统一置于对应文件底部的 `#[cfg(test)] mod tests { ... }` 中。
- **测试命名**：测试函数采用 `test_<被测行为或功能>` 命名模式（例如 `test_replaces_tun_inbound_with_tproxy_set`、`test_inserts_hijack_dns_at_head_when_missing`）。
- **测试要求**：核心逻辑（模板合并、锚点注入、配置转换）必须编写单元测试；网络规则相关逻辑应避免污染宿主网络，尽量依托 mock 或参数生成校验。
- **执行指令**：提交前确保 `cargo test` 全部通过。

## Commit & Pull Request Guidelines

- **分支规范**：功能分支采用 `dev/<feature-description>` 格式（例如 `dev/initial-scaffold-and-design`）。
- **Commit 规范**：遵循 Conventional Commits 格式，提交信息为 `<type>: <description>`：
  - 常用类型：`feat:`、`fix:`、`refactor:`、`docs:`、`test:`、`chore:`。
  - 描述需准确明了，动词开头。
- **Pull Request 要求**：
  - 附带清晰的变更背景、改动点与自测验证说明。
  - 若涉及网络防火墙或进程监督改动，须提供异常退出场景下的网络状态恢复验证。
  - CI 检查项（`cargo check`、`cargo test`、`cargo clippy`）必须全绿。

## Agent skills

### Issue tracker

GitHub Issues (`thunderfury-org/canto`) via `gh` CLI. See `docs/agents/issue-tracker.md`.

### Triage labels

Canonical five-role vocabulary (`needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`). See `docs/agents/triage-labels.md`.

### Domain docs

Single-context layout (`CONTEXT.md` and `docs/adr/` at the repo root). See `docs/agents/domain.md`.
