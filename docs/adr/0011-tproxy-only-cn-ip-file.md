# 默认捕获只走 tproxy；大陆 IP 绕过来自 cn_ip.txt

TUN 路径从 canto 删除，不再作为高级选项。`network.mode` 从公开配置移除：旧 toml 里的 `mode` 被忽略，启用网络时一律 tproxy。redirect inbound 仍未落地，仅 TCP 时也由 nft 只劫持 TCP，入站仍是 `tproxy-in`。

`bypass_cn` 对齐 ShellCrash：读 `work_dir/cn_ip.txt`（一行一个 IPv4 CIDR）。文件能解析出至少一条则用，不重下；否则下载 `https://testingcf.jsdelivr.net/gh/juewuy/ShellCrash@master/bin/geodata/china_ip_list.txt`，成功落盘，失败且无可用文件则拒绝启动。源配置 tag `cnip` 只留给 sing-box 分流，canto 不解析、不读 `.srs`。

nft 在 prerouting / output 的 reserved 与 53 return 之后、tproxy/打标之前 `ip daddr @cnip return`。DNS 链不绕过 CN。

修正 [ADR 0010](0010-gateway-owns-nft.md)：撤回「`mode = tun` 高级选项」和「CIDR 来自源配置 cnip」。落地见 [#13](https://github.com/thunderfury-org/canto/issues/13) 的 tproxy 切片。
