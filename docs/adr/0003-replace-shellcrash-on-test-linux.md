# 先在测试 Linux / netns 上验收，生产 OpenWrt 另说

验代码的第一台网关不是家里的生产 OpenWrt。CI 的 `scripts/netns-check.sh` 覆盖劫持、回滚、URL 源和刷新。Docker 只用于这笔检查，不当被替换的网关。劫持路径按 ADR 0010：canto 持有 nft，LAN 走 prerouting、本机走 output。省略 `mode` 时仅 TCP 走 redirect，打开 UDP 走 tproxy；redirect inbound 落地前允许先用已有 tproxy。TUN + `auto_redirect` 不是测试机默认路径。

上家里那台 OpenWrt 的条件见 ADR 0007，不是「测试机替换 ShellCrash 之后立刻上路由器」。家里只有一台正在用的路由器时，先在现网后面挂测试 Linux（#12）。ADR 0005 的「接管即可」只适用于测试 Linux / netns。
