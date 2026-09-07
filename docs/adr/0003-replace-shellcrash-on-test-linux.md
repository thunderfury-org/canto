# 先在测试 Linux / netns 上验收，生产 OpenWrt 另说

验代码的第一台网关不是家里的生产 OpenWrt。CI 的 `scripts/netns-check.sh` 覆盖劫持、回滚、URL 源和刷新。Docker 只用于这笔检查，不当被替换的网关。劫持路径当前仍是手搓 tproxy。

上家里那台 OpenWrt 的条件见 ADR 0007，不是「测试机替换 ShellCrash 之后立刻上路由器」。ADR 0005 的「接管即可」只适用于测试 Linux / netns。
