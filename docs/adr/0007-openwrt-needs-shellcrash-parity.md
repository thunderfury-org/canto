# 上 OpenWrt 前必须对齐现网 ShellCrash 用法

生产路由器上的 ShellCrash 实际是：绕过大陆 IP；劫持局域网、本机、docker；只劫持常用端口的 TCP（redirect）；不劫持 UDP；不代理 IPv6。canto 在这些稳定之前不上这台 OpenWrt。OpenWrt 真机交付因此不是下一步。ADR 0005 的「接管即可」只适用于测试 Linux / netns，不适用于家里的路由器。
