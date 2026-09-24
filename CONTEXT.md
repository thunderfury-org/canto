# Canto

canto 是基于 sing-box 的透明代理网关控制面与可视化配置中心：既能在 Linux / OpenWrt 上消费完整 sing-box JSON、覆盖入站并编排劫持规则；也能通过内置 Web Studio 管理模板与节点源、组装 Profile 并向各端分发订阅。

## Language

**源配置**:
canto 网关消费的一份完整 sing-box JSON。可来自外部服务或 canto 自身的 Web Studio。
_Avoid_: 订阅, 配置

**源地址**:
取得源配置的位置，本地路径或 HTTP(S) URL。
_Avoid_: 订阅链接

**覆盖**:
canto 对源配置的改写，使入站与路由标记对齐网关的数据包劫持。
_Avoid_: 合并, 补丁, 转换

**刷新**:
网关运行中重新取得源配置并再次覆盖。
_Avoid_: 热更新, 订阅更新, reload

**运行时配置引擎 (Runtime Config Engine)**:
网关消费端统一管理源配置获取、入站覆盖、语法校验、last-good 缓存与原子落盘刷新的深模块。
_Avoid_: 配置管理器, 配置生成器, 配置同步器

**网关**:
canto 编排透明代理的那台 Linux / OpenWrt 机器。
_Avoid_: 客户端, 节点, 路由器

**劫持意图**:
拦谁（lan / local / docker）、拦什么（tcp / udp / ports）、绕过什么（bypass_cn）。
_Avoid_: 模式, 代理模式

**捕获**:
把数据包送进 sing-box 入站的内核路径。网关默认是 tproxy，由 canto 编排 nft。
_Avoid_: 管道, auto_redirect, TUN

**大陆 IP 绕过**:
nft 在打标/tproxy 之前按 `cn_ip.txt` 对目的地址 return。
_Avoid_: geoip, 源配置 cnip

**配置模板 (Template)**:
Web Studio 中可被多份配置档案共用的 sing-box 配置骨架。配置编译只读取已保存的这份。
_Avoid_: 模板文件, 基础配置, 草稿

**草稿 (Draft)**:
已登录后，用户对一份配置模板做出、但尚未保存的修改。同一次打开里离开编辑器再回来，草稿仍在。保存之前不影响任何配置档案。
_Avoid_: 已同步更新, 自动保存, 临时模板

**保存 (Save)**:
用按下保存时编辑器展示的这份模板，替换已保存的配置模板。引用它的每份配置档案在下次编译时都看到新内容。
_Avoid_: 同步, 自动保存

**放弃 (Discard)**:
丢掉一份配置模板的草稿，编辑器回到服务器上当前已保存的那份。取不回那份时，草稿还在。
_Avoid_: 还原, 撤销, 重置

**节点源 (Node Source)**:
向 Web Studio 提供代理节点的输入源，支持 Clash YAML（`proxies:`）、Base64 协议 URI 列表与 sing-box 原生 JSON。
_Avoid_: 节点池, 订阅源

**配置档案 (Profile)**:
绑定配置模板与若干节点源的分发单元，具备独立访问 Token 与公开 HTTP 订阅端点。
_Avoid_: 订阅配置, 导出档案

**节点分组 (Node Group)**:
配置模板中负责按地区或用途通过正则占位符（如 `{(?i)hk}`）筛选并聚合代理节点的分组单元（通常为 `urltest` 自动测速或 `selector` 手动分组）。
_Avoid_: 策略组, 节点池, 代理列表

**出站策略组 (Policy Group)**:
配置模板中面向路由分流规则与 DNS Detour 的高层出站决策单元，其候选出站严格仅能选择已定义的节点分组与基础直连/拒绝出站。
_Avoid_: 节点分组, 出站组

**标签展开 (Expansion)**:
Web Studio 在编译配置时，将节点分组中的正则占位符匹配并替换为具体节点标签的计算过程。
_Avoid_: 宏展开, 占位符替换

**配置编译 (Compilation)**:
Web Studio 将配置档案（Profile）结合配置模板（Template）与关联节点源（Node Source），经由标签展开与完整性校验组装为可用 sing-box JSON 与分发元数据的过程。
_Avoid_: 动态生成, 导出配置, 打包

**规则集 (Rule Set)**:
定义域名、IP CIDR 等特征集合的可重用分流资产单元，独立于具体的路由分流或 DNS 规则，支持 sing-box 1.14+ 多标签聚合定义。
_Avoid_: 路由集, 规则文件, 分流包

**规则源 (Rule Source)**:
配置模板中的一条规则集来源，拥有自己的 Tag 前缀和已选规则集。
_Avoid_: provider, 规则订阅源

**规则源预设 (Rule Set Release Provider)**:
提供批量规则集发布的 GitHub Release 资产源与预设配置，支持 Web Studio 自动化探测 Release 资产并提供规则勾选装配。
_Avoid_: 规则订阅, 外部规则源, 规则抓取器
