# 替换 ShellCrash 是接管，不是行为对等

测试 Linux 上「替换 ShellCrash」的完成标准是：ShellCrash 的进程和规则已卸掉，canto 用现行 tproxy 劫持局域网 + 本机、全部 TCP/UDP。不绕过大陆 IP、不劫持 IPv6、不按常用端口过滤、不单独处理 docker。这些偏差允许存在；上 OpenWrt 之前再补。本轮不把 canto 做成 ShellCrash 克隆。
