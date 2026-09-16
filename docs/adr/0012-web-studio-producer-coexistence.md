# 网关消费端与 Web Studio 生产端共存架构

扩展并修正 [ADR 0001](0001-gateway-consumes-external-source.md)：canto 不再仅限于外部源配置消费，而是内建可选的 Web Studio 模块作为配置生产端。

## 背景

在多平台和复杂网络环境下，手动编写与维护大体量 sing-box JSON 极易产生语法错误，且难以协调外部订阅与自建节点池。以往用户必须依赖外部第三方转换工具，或在网关外部单独维护配置分发服务。

## 决定

1. **双模共存架构**：
   - **网关消费端（Gateway Consumer）**：原有核心职责不变。消费一份完整 sing-box JSON，应用入站覆盖、编排 tproxy nftables 规则、监督 sing-box 进程。
   - **配置生产端（Web Studio Producer）**：通过内建的可视化 Web Studio，管理配置模板（Template）、聚合节点源（Node Source）、按正则占位符执行标签展开（Expansion）生成配置档案（Profile），并通过安全 Token 保护的 HTTP 端点（`/sub/:token`）对下游分发。

2. **运行时与权限模型**：
   - Web Studio 由 `canto.toml` 中的 `[web]` 配置节控制（`enabled`、`listen`、`admin_token`、`public_url`）。
   - 当 `[network].enabled = false` 且 `[web].enabled = true` 时，canto 进入纯服务端模式（Server-only mode），不执行任何网络规则注入，不需要 root 权限，可轻量运行于云 VPS 或桌面端。
   - 当两者同时启用时（Hybrid mode），Axum HTTP 服务与网关监督器并发运行，在 SIGINT/SIGTERM 下均能优雅退出并确保网络规则清理。
   - 默认状态下 `[web].enabled = false`，canto 保持 100% 既有行为兼容。

3. **资源嵌入与单二进制交付**：
   - 基于 Svelte 5 构建的单页面管理控制台在编译期通过 `rust-embed` 嵌入二进制中，对外提供零外部资源依赖的单文件交付体验。
   - 敏感管理 API 受 `admin_token`（通过 Authorization 报头或 Cookie）保护。

落地见 [#16](https://github.com/thunderfury-org/canto/issues/16) 与 [#17](https://github.com/thunderfury-org/canto/issues/17)。
