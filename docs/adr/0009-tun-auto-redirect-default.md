# 默认捕获走 TUN + auto_redirect，canto 不再持有 nft

Linux / OpenWrt 上 canto 默认改写源配置为 TUN inbound，打开 `auto_route` + `auto_redirect`。`strict_route` 在网关 netns 里会掐死 WAN 出站，因此 overlay 关掉它；漏网防护靠 auto_redirect 与接口表，而不是桌面 VPN 那套 unreachable 路由。TCP/UDP 热路径是 sing-box 的 nft 重定向进套接字，不是纯 TUN 协议栈；官方写明这比手搓 tproxy 少 mark、少拷包。内核 `action: bypass` 只挂在这条路上，大陆 IP、非常用端口和关掉的 UDP 都用 sniff 之前的 bypass 回内核。

canto 默认路径不持有 `table inet canto`、fwmark 或策略路由 table 167。`NetworkGuard` 只开 `ip_forward`，停机时清残留的 canto 表、旧 ip rule、`inet sing-box` 和名为 `canto` 的 tun。MASQUERADE 交给系统（OpenWrt 上是 fw4）；netns 检查自己加。

`network.mode = "tproxy"` 是逃生口，行为与 #8 合入后的主干一致。`bypass_cn` 只在 TUN 路径有效。`local = false` 只在 tproxy 有效：`auto_redirect` 依赖 `auto_route`，关掉本机劫持不能靠再写一层 nft。
