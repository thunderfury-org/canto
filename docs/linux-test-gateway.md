# 在测试 Linux 上接管网关

第一台网关是 Linux 虚拟机或云主机，不是家里正在用的 OpenWrt（ADR 0003）。现网路由器继续跑 ShellCrash。验收标准见 [#12](https://github.com/thunderfury-org/canto/issues/12)。

默认捕获按 [ADR 0010](adr/0010-gateway-owns-nft.md)：canto 持有 `table inet canto`，LAN 走 prerouting，本机走 output。仅 TCP 用 redirect，打开 UDP 用 tproxy。不要用 TUN + `auto_redirect` 验收本机劫持。canto 不卸载 ShellCrash（ADR 0006）。MASQUERADE 仍交给系统。

代码若还停在 #6 的 TUN 默认路径，先做 [#13](https://github.com/thunderfury-org/canto/issues/13)，再按本文测。

## 1. 拓扑

把测试机挂在现网 LAN 后面，只让一两台设备走它：

```
互联网
  └── 家里 OpenWrt（ShellCrash，不动）
        └── 现网 LAN 192.168.1.0/24
              ├── 日常设备 → 网关仍是 OpenWrt
              ├── 测试 Linux（canto）
              └── 测试手机 / 电脑 → 把默认网关改成测试 Linux
```

单网卡即可：测试机 WAN 和劫持口是同一块网卡。双网卡更干净，但不是起步条件。

Docker 只用于 `scripts/netns-check.sh`，不要把容器当成这台网关。本机 macOS 上的 OrbStack 虚拟机进不了家里 Wi-Fi 设备的默认网关，只能测这台虚拟机自己的 LAN 网段。

## 2. 测试机准备

1. 给测试机一个稳定的 LAN 地址（DHCP 预留或静态），例如 `192.168.1.100`。
2. 打开转发。canto 启动时会写 `ip_forward=1`，但 NAT 要自己做：

```bash
nft add table inet masq
nft add chain inet masq postrouting '{ type nat hook postrouting priority srcnat; policy accept; }'
nft add rule inet masq postrouting oifname "eth0" masquerade
```

把 `eth0` 换成测试机上连现网的接口。这条 `masq` 表属于系统，canto 不会删它。

3. 确认测试机上没有 ShellCrash / Clash 残留：`nft list tables` 里不应再有它们的表。
4. 安装 **sing-box 1.13+**。canto 不下载它。

## 3. 安装 canto

交叉编译 musl 静态二进制后拷到主机：

```bash
cargo build --release --target x86_64-unknown-linux-musl
sudo cp target/x86_64-unknown-linux-musl/release/canto /usr/local/bin/canto
```

aarch64 用 `cross build --release --target aarch64-unknown-linux-musl`。

## 4. 配置

`/etc/canto/canto.toml`：

```toml
[canto]
work_dir = "/var/lib/canto"

[singbox]
binary = "sing-box"
source = "https://config.example/source.json"
config_path = "/var/lib/canto/config.json"
refresh_interval_secs = 86400

[network]
enabled = true
lan = true
local = true
docker = false
tcp = true
udp = false
ports = "common"
bypass_cn = true
mixed_port = 7890
lan_cidrs = ["192.168.1.0/24"]
```

`source` 也可以是本地 JSON 路径。省略 `mode`：仅 TCP 时走 redirect，打开 UDP 时走 tproxy。redirect inbound 落地前，省略 `mode` 可以先走现有 tproxy。不要设 `mode = "tun"` 来验收 #12。

## 5. systemd

```ini
[Unit]
Description=canto sing-box transparent proxy orchestrator
After=network.target network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart=/usr/local/bin/canto run
ExecStop=/usr/local/bin/canto clean-network
Restart=on-failure
RestartSec=5s
LimitNOFILE=65535
AmbientCapabilities=CAP_NET_ADMIN CAP_NET_BIND_SERVICE

[Install]
WantedBy=multi-user.target
```

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now canto
```

SIGKILL / OOM 走不到 Drop，所以 `ExecStop` 必须是 `canto clean-network`。

## 6. 怎么证明拦到了

不要用测试机本机 `curl 1.1.1.1` 的 RTT 当 LAN 劫持证据。

1. **本机 output**：`nft list table inet canto` 里要有 output 链；本机访问常用端口 TCP 应进 sing-box，大陆 IP / 非常用端口应在 nft 被 `return`。
2. **LAN prerouting**：另找一台把默认网关指到测试机的客户端（或第二台虚拟机）。`nft` 计数和 `ip route get <dst> from <client> iif <lan>` 应显示进了 canto 的 prerouting，而不是只改了 mixed 端口。
3. 未改网关的设备行为不变。

## 7. 出问题怎么退

测试设备改回原网关（OpenWrt）即离开 canto，家里其余设备本来就没动。

停 canto 并清残留：

```bash
sudo systemctl stop canto
sudo canto clean-network
nft list tables
```

正常停机后不应再有 `inet canto`，也不应再留下 canto 的 ip rule / table 167。系统自己的 `inet masq` 还在，这是 NAT，不是劫持。
