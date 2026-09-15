# 透明入站选型

对照 sing-box 1.13 / 1.14 官方路径，以及 canto 作为 **Linux / OpenWrt 网关** 的职责。源配置通常来自桌面 TUN JSON（例如 sing-box-config 的 `config-with-tailscale.json`）。

改劫持路径、nftables 规则或大陆 IP 绕过时先读本文。v1 运行时行为以 [DESIGN.md](DESIGN.md) 为准。

## 结论

1. **网关默认是 canto 持有的 nft：LAN 走 prerouting，本机走 output。** 当前入站是 tproxy；仅 TCP 时 nft 只劫持 TCP。redirect inbound 尚未落地。见 ADR 0010 / 0011。
2. **不要把 TUN + `auto_redirect` 当网关默认。** 桌面源配置里的 `auto_route` 不能原样用在路由器上；`auto_redirect` 只有 prerouting，没有 output。canto 已删除 TUN 路径。
3. **Apple / 无 root 的 Android 不是 canto 的目标。** 那些平台最好的透明路径仍是 TUN，但没有 nftables。canto 不编排它们。

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

这在本机 VPN 是对的：默认路由指向 tun，本机进程进代理。放到网关上会抢 WAN 默认路由，LAN 转发和回程都会乱。canto 因此整段替换 `inbounds` 为 `mixed` / `tproxy` / `dns`，写 `route.default_mark`，并关掉 `auto_detect_interface`，让出站走 main 表从 WAN 出去。

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

canto 的 `reserved_ipv4` 目的地址绕过是 nft 层的粗粒度排除。大陆 IP 绕过同样放在 nft：打标/tproxy 之前 `ip daddr @cnip return`。CIDR 来自 `work_dir/cn_ip.txt`，不解析源配置 `cnip`，也不靠 `auto_redirect` 预匹配里的 `action: bypass`。

## 平台

**Linux / OpenWrt**：网关默认 canto 持有 `table inet canto`，入站是 tproxy。来源用 `lan_ipv4` / docker0。官方的 TUN + `auto_redirect` 是桌面/转发捷径，canto 不编排（ADR 0011）。

**Android**：无 root 走 VpnService TUN，按包名分流。`auto_redirect` 要 root 服务或 root shell 才完整，才能抓热点。canto 不编排 Android。

**Apple**：NetworkExtension TUN，没有 nftables。`auto_redirect` 无效，图形客户端也没实现 `strict_route`。系统代理只覆盖 TCP。canto 不编排 Apple。

## 对 canto 代码的约束

- 默认路径持有 `inet canto`。LAN 必须有 prerouting，本机必须有 output。不要为了少写 nft 把本机劫持丢掉。
- 不要恢复 `network.mode` 或 TUN 路径。不要解析已删除的 `bypass_cn_ips`。大陆 CIDR 只来自 `cn_ip.txt`。
- 不要把 `auto_redirect` 写进共用桌面源 JSON。
- Tailscale 源配置里的 `192.168.5.0/24 → ts-ep` 在 1.14 可走 L3 forwarding，发生在预匹配。overlay 不要丢掉这条路由。
