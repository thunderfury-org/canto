# 规则集独立解耦与多源海量规则集管理

扩展 [ADR 0012](0012-web-studio-producer-coexistence.md) 与 [ADR 0013](0013-split-node-groups-from-policy-groups.md)：在 Web Studio 配置模板（Template）中将规则集（Rule Sets）从路由分流规则（Route Rules）中彻底解耦为独立资源，支持多规则源并存、Tag 命名空间隔离防冲突，以及海量规则下的轻量交互体系。

## 背景

此前 Web Studio 模板中，规则集（`rule_set`）直接嵌入在 `content.route.rule_set` 结构内。这带来以下架构与交互挑战：
1. **语义割裂与跨模块复用阻碍**：规则集作为域名与 IP 特征集合，不仅被路由分流规则（`route.rules`）使用，也深度服务于 DNS 分流规则（`dns.rules`）。
2. **多规则源同名冲突**：社区主流维护方式通常将域名（GeoSite）与 IP（GeoIP）拆分在不同仓库或目录中（如 SagerNet GeoSite 与 SagerNet GeoIP，或 MetaCubeX 的 `geo/geosite` 与 `geo/geoip`）。二者均存在同名规则（如 `cn`、`apple`、`private` 等），直接引入会引发 sing-box 的 Tag 重名冲突或匹配歧义。
3. **海量规则（1000+）性能与人机工效问题**：SagerNet 与 MetaCubeX 等全量规则库包含上千至数千个规则文件，若在前端平铺数千个复选框会导致严惩卡顿，且用户实际仅需要配置 10~30 个主流服务规则。

## 决定

1. **模板顶层结构解耦与纯净多源**：
   - 模板 `content` 提升顶层 `rule_sets` 字段，由一组结构化的规则源聚合对象构成，彻底移除单条自定义规则集。
   - 规则集格式全面对齐 sing-box 1.14+ 聚合规范：支持多 tag 数组 `tag: ["geosite-cn", "geosite-ai", ...]`、`format: "binary"`（默认优先使用 `.srs` 二进制格式）、`url: ".../{tag}.srs"` 及下载代理 `download_detour`。
   - 各规则源独立配置 Tag 前缀（如 `geosite-`、`geoip-`），通过前缀命名空间彻底杜绝跨源重名冲突。

2. **精选规则源预设与全形态探测引擎**：
   - 移除失效的 `Loyalsoldier/sing-box-rules@release`。
   - 内置 5 大主流精选预设：
     - `DustinWin 规则集` (Release: `DustinWin/ruleset_geodata@sing-box-ruleset`)
     - `SagerNet 官方 GeoSite 规则集` (Branch: `SagerNet/sing-geosite#rule-set`, 默认前缀: `geosite-`)
     - `SagerNet 官方 GeoIP 规则集` (Branch: `SagerNet/sing-geoip#rule-set`, 默认前缀: `geoip-`)
     - `MetaCubeX GeoSite (域名规则)` (Branch: `MetaCubeX/meta-rules-dat#sing/geo/geosite`, 默认前缀: `geosite-`)
     - `MetaCubeX GeoIP (IP 规则)` (Branch: `MetaCubeX/meta-rules-dat#sing/geo/geoip`, 默认前缀: `geoip-`)
   - 探测端点 `POST /api/rulesets/inspect-release` 统一支持 Release 资产与 Branch/Tree 目录资产解析。

3. **海量规则轻量化交互工作流**：
   - **已选规则池 (Selected Pool)**：主界面常态仅渲染当前已启用的 10~30 个规则药丸，DOM 节点数量降低 95% 以上，页面极致轻快。
   - **智能即时搜索 (Search & Add)**：输入即时下拉呈现匹配项，支持回车一键加入已选。
   - **全库查阅弹窗 (Browse All Modal)**：提供独立弹窗查阅上千条全量规则，支持 A-Z 索引、模糊搜索与分页增量加载，关闭后即刻从 DOM 卸载，消除冗余静态推荐，保持界面极简纯粹。

4. **编译展开与引用完整性校验**：
   - `expand_profile` 将各规则源装配进 sing-box 的 `route.rule_set` 数组中，清理内部辅助字段。
   - 模型校验保证全模板所有规则集的最终 Tag 全局唯一，阻止重名冲突，并校验路由与 DNS 规则引用的有效性。
