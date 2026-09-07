# 先在测试 Linux 上替换 ShellCrash，再上 OpenWrt

第一台要跑通的网关不是家里的生产 OpenWrt，而是一台测试用 Linux 虚拟机或云服务器：主机网络，systemd 跑 `canto run`。完成标准是它能替换该机器上现有的 ShellCrash，并且劫持局域网 + 本机，以便接近后续 OpenWrt 网关。Docker 只用于现有 netns 检查，不当被替换的那台网关。OpenWrt 是后续阶段。劫持路径这轮仍用手搓 tproxy。
