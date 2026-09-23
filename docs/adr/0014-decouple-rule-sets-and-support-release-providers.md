# 规则集独立解耦与 GitHub Release 资产探测装配

扩展 [ADR 0012](0012-web-studio-producer-coexistence.md) 与 [ADR 0013](0013-split-node-groups-from-policy-groups.md)：在 Web Studio 配置模板（Template）中将规则集（Rule Sets）从路由分流规则（Route Rules）中彻底解耦为独立资源，并支持面向 GitHub Release 规则源的资产探测与按需勾选。

## 背景

此前 Web Studio 模板中，规则集（`rule_set`）直接嵌入在 `content.route.rule_set` 结构内，并在模板编辑器的「路由分流规则」子面板底部直接渲染。这带来三个主要缺陷：
1. **语义割裂与跨模块复用阻碍**：规则集是域名与 IP 特征集合的公共资产，不仅被路由分流规则（`route.rules`）使用，也深度服务于 DNS 分流规则（`dns.rules`）。将其挂载在路由分流之下会误导用户，且在配置 DNS 分流时无法直观管理规则集。
2. **sing-box 1.14+ 新规范未利用**：以往每个规则集都需要配置完整的 URL（例如 9 个规则就要配 9 次前缀相同的 GitHub URL）。sing-box 1.14+ 正式支持在 `tag` 中传入数组并配合 `{tag}.srs` 或 `{tag}.json` 占位符进行多 tag 聚合定义，大幅精简配置。
3. **缺乏规则资产探测与批量勾选**：用户使用 GitHub Release 分发的规则集（如 `DustinWin/ruleset_geodata@sing-box-ruleset`）时，需手动逐个查阅 release 文件名并复制到输入框，体验繁琐且容易拼写错误。

## 决定

1. **模板顶层结构解耦**：
   - 模板 `content` 提升顶层 `rule_sets` 字段，与 `node_groups`、`policy_groups` 平级。
   - 规则集格式全面对齐 sing-box 1.14+ 聚合规范：支持多 tag 数组 `tag: ["cn", "ai", ...]`、`format: "binary"`（默认优先使用 `.srs` 二进制格式）、`url: ".../{tag}.srs"` 及下载代理 `download_detour`。
   - 存量模板向后兼容：当旧模板仅含有 `route.rule_set` 时，校验与编译引擎自动向上兼容。

2. **GitHub Release 资产探测 API**：
   - Web Studio 后端提供 `POST /api/rulesets/inspect-release` 接口与 `GET /api/rulesets/presets` 预设接口。
   - 支持解析 GitHub Release 页面链接、下载链接或 `owner/repo@tag` 格式，自动请求 GitHub API 获取 Release 资产列表并解析出可用规则集 Tag 及可用格式（`.srs` / `.json`）。
   - 内置高质量规则源预设（默认推荐 `DustinWin/ruleset_geodata@sing-box-ruleset`，亦包含 `Loyalsoldier/sing-box-rules@release` 等）。

3. **配置编译与引用完整性校验**：
   - 编译（Compilation / Expansion）阶段：将模板顶层 `rule_sets` 装配回输出 sing-box 配置的 `route.rule_set` 中，并从输出顶层剔除 `rule_sets` 字段，确保生成的配置完全符合 sing-box 原生规范。
   - 模板校验阶段：`validate_template_content` 检查 `route.rules` 与 `dns.rules` 中引用的 `rule_set` tags 是否在已声明的规则集集合中存在，阻止悬空引用。

4. **Web Studio 交互优化**：
   - 模板编辑器顶部新增独立的「规则集」子 Tab。路由分流规则子面板聚焦于规则列表决策。
   - 规则集面板提供「GitHub Release 批量规则源」和「自定义规则集」两个卡片。用户可通过选择预设或输入 Release 链接，一键拉取规则并通过复选框勾选需要的规则。
   - 在路由分流规则与 DNS 规则编辑时，针对 `rule_set` 匹配类型提供已定义 tags 的下拉候选与快捷标签。
