# 透明入站选型

对照 sing-box 1.13 / 1.14 官方路径，以及 canto 作为 **Linux / OpenWrt 网关** 的职责。源配置通常来自桌面 TUN JSON（例如 sing-box-config 的 `config-with-tailscale.json`）。

改劫持路径、`network.mode`、nftables 规则或大陆 IP 绕过时先读本文。v1 运行时行为以 [DESIGN.md](DESIGN.md) 为准。

## 结论

1. **v1 继续手搓 tproxy。** 网关要按 LAN 来源劫持、WAN 进站不劫持，并且自己掌握 nftables 的申请/回滚。桌面源配置里的 TUN `auto_route` 不能原样用在路由器上。
2. **Linux / OpenWrt 的升级方向是 TUN + `auto_redirect`，不是把 tproxy 再加厚。** 官方写明 `auto_redirect` 快于 tproxy，OpenWrt 会自动插 fw4 兼容规则，内核 `bypass` 也挂在这条路上。
3. **Apple / 无 root 的 Android 不是 canto 的目标。** 那些平台最好的透明路径仍是 TUN，但没有 nftables，`auto_redirect` 无效。canto 不编排它们。

## 四条路径

性能差在内核把流量交给 sing-box 的方式，官方没有 Gbps 跑分。透明代理里用户态做得越多越慢。

| 路径 | 覆盖 | 内核做什么 | 用户态额外成本 | 平台 |
|---|---|---|---|---|
| `redirect` inbound | 基本只有 TCP | NAT 到本地端口 | 几乎没有 | Linux / macOS |
| `tproxy` inbound | TCP / UDP | mark + 策略路由 + tproxy | 透明套接字 | 仅 Linux |
| 纯 TUN | 任意 IP，含 ICMP | 改路由，包进 tun | L3→L4 协议栈 | 全平台 |
| TUN + `auto_redirect` | TCP / UDP 走套接字；ICMP 等仍走剩余 TUN | nftables 重定向，仍创建 tun | 比 tproxy 少 mark/拷包 | Linux；Android 需 root |

应用能填代理时，`mixed` / `socks` 比这四条都轻，因为没有透明拦截。canto 的 `mixed-in` 只给能填代理的客户端，不是 LAN 透明的主路径。

相关开关不要混：

- `auto_route`：改系统路由表，让流量进 TUN。桌面当默认网关用。
- `strict_route`：堵住绕开 TUN 的漏网流量。Linux / Windows 有效；Apple 图形客户端没实现。
- `auto_redirect`：TUN inbound 上的 Linux 选项，必须同时开 `auto_route`。不是另一种 inbound，也不会把 tun 网卡拆掉。
- `route.rules`：包已经进 sing-box 之后怎么分流。

## 为什么源 JSON 里的 TUN 不能直接启动

桌面配置典型是：

```json
{
  "type": "tun",
  "tag": "tun-in",
  "auto_route": true,
  "strict_route": true
}
```

再配 `route.auto_detect_interface: true`，防止 sing-box 出站再钻回 TUN。

这在本机 VPN 是对的：默认路由指向 tun，本机进程进代理。放到网关上会抢 WAN 默认路由，LAN 转发和回程都会乱。canto v1 因此整段替换 `inbounds` 为 `mixed` / `tproxy` / `dns`，写 `route.default_mark`，并关掉 `auto_detect_interface`，让出站走 main 表从 WAN 出去。

`auto_redirect` 可以接管 **转发流量**（OpenWrt、Android 热点），这是以后做 TUN 模式的前提，不是现在把桌面 JSON 原样 `sing-box run` 的理由。

## 预匹配、bypass、大陆 IP

pre-match 不是新配置块。TUN / WireGuard / Tailscale 这些 L3 入站在连接建立前会先跑同一套 `route.rules`。能用 IP、端口、`wifi_ssid`、`clash_mode` 拍板的规则立刻生效；碰到 `sniff` 或域名就停。

1.14 起 UDP 可用首包在预匹配里 sniff。Linux 开了 `auto_redirect` 时，预匹配走 nfqueue，才能用内核 `bypass`。

两条绕过方式：

```json
{
  "rule_set": ["cnip", "privateip"],
  "action": "bypass",
  "outbound": "直连"
}
```

`bypass` 必须在 `sniff` 之前，且只能用预匹配里已有的字段。没有 `outbound` 时，规则只在 `auto_redirect` 预匹配生效，其它入站会跳过它。`outbound` 是退路：内核放行失败时退化成普通 `直连`。

```json
{
  "type": "tun",
  "auto_route": true,
  "auto_redirect": true,
  "route_exclude_address_set": ["cnip", "privateip"]
}
```

`route_exclude_address_set` 在规则之前就把网段挡在 sing-box 外，Clash Global 也管不到。网关如果还要按设备、端口或模式分流，用逐条 `bypass`，不要用 exclude set。

域名集（`cn`、`domain_suffix`）在 TCP 上通常要等 sniff，不能内核 bypass。那部分继续 `outbound: 直连`。

canto v1 的 `reserved_ipv4` 目的地址绕过是 nft 层的粗粒度排除，对应 exclude set，不是 `action: bypass`。以后做大陆 IP 绕过，优先复用 sing-box 这两套，而不是再造一张 cnip 表。

## 平台

**Linux / OpenWrt**：`TUN + auto_route + auto_redirect + strict_route` 是官方推荐。这是 canto 以后 `network.mode` 该切换到的路径。v1 用手搓 tproxy 换来 LAN/WAN 来源控制和 `NetworkGuard` 回滚。

**Android**：无 root 走 VpnService TUN，按包名分流。`auto_redirect` 要 root 服务或 root shell 才完整，才能抓热点。canto 不编排 Android。

**Apple**：NetworkExtension TUN，没有 nftables。`auto_redirect` 无效，图形客户端也没实现 `strict_route`。系统代理只覆盖 TCP。canto 不编排 Apple。

## 对 canto 代码的约束

- 不要把 `auto_redirect` 写进共用桌面源 JSON；那是网关 overlay 的事。
- v1 不要重新引入 `network.mode` / `bypass_cn_ips` 配置项。legacy TOML 键被忽略是故意的。
- 以后做 TUN 模式时，overlay 应保留或改写为 TUN inbound，打开 `auto_redirect`，缩小 canto 自己的 nftables（LAN 来源、WAN 防护、fw4 协同仍可能要留）。不要再实现一套平行的 tproxy 优化。
- Tailscale 源配置里的 `192.168.5.0/24 → ts-ep` 在 1.14 可走 L3 forwarding，发生在预匹配。overlay 不要丢掉这条路由。
