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
Web Studio 中的基础 sing-box 配置骨架，包含各核心模块（DNS、Inbounds、策略组等）的可视化定义与正则占位符。
_Avoid_: 模板文件, 基础配置

**节点源 (Node Source)**:
向 Web Studio 提供代理节点的输入源，支持 Base64 订阅链接与 sing-box 原生 JSON。
_Avoid_: 节点池, 订阅源

**配置档案 (Profile)**:
绑定配置模板与若干节点源的分发单元，具备独立访问 Token 与公开 HTTP 订阅端点。
_Avoid_: 订阅配置, 导出档案

**标签展开 (Expansion)**:
Web Studio 在编译配置时，将策略组中的正则占位符匹配并替换为具体节点标签的计算过程。
_Avoid_: 宏展开, 占位符替换
