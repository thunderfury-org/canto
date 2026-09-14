# 意图优先，模式下沉的网络配置模型

canto 的网络配置以用户意图为中心组织：劫持谁（lan / local / docker）、劫持什么（tcp / udp / ports）、绕过什么（bypass_cn / reserved_ipv4）。

底层捕获机制（redirect / tproxy / tun）是实现技术管道而非用户意图。网关上系统按意图选择：仅 TCP 走 redirect，打开 UDP 走 tproxy（ADR 0010）。`mode` 仅作为高级可选参数存在，用来强制某条管道；不要为了绕过大陆 IP 或劫持本机去开 `mode`。

这种设计避免了将内核实现复杂度泄漏给用户，规避了不同模式下的配置互斥与隐性陷阱，并与后续可视化配置界面直接映射。
