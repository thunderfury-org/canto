# 网关捕获由 canto 持有 nft，不再默认 TUN + auto_redirect

canto 的产品是网关，不是桌面 VPN。劫持意图是局域网 + 本机（以及可选 docker）：LAN 走 prerouting，本机走 output。ADR 0009 把默认捕获交给 sing-box `auto_redirect`，只覆盖转发流量，没有 output；`local = true` 并不能劫持本机。内核 `action: bypass` 换不来这个缺口。OrbStack 测试机和 ShellCrash 源码都证实了这一点：ShellCrash 的 Tun 能同时拦 LAN 和本机，是因为它自己 `auto_route=false`，用 mark + 策略路由，而不是 sing-box `auto_redirect`。

默认路径改回 canto 持有 `table inet canto`。机制按意图选择，不让用户先选管道：仅 TCP 时用 redirect（对齐现网 ShellCrash Redir）；打开 UDP 时用 tproxy。在 redirect inbound 落地之前，省略 `mode` 允许先走已有的 tproxy，但那是过渡，不是终态。TUN + `auto_redirect` 不再是默认，也不再是上 OpenWrt 的前提；需要时才用 `mode = "tun"`，并写明它劫持不了本机。

`bypass_cn` 回到 nft 在 redirect / 打标之前 `return`。CIDR 仍来自源配置的 `cnip`，canto 不另下一份 geoip。不再把 `bypass_cn` 和 tproxy/redirect 互斥。

ADR 0009 作废。ADR 0008 的意图模型保留，终态从 TUN + auto_redirect 改为 redirect / tproxy。落地见 [#13](https://github.com/thunderfury-org/canto/issues/13)。
