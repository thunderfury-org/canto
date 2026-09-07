# 在测试 Linux 上替换 ShellCrash

本轮第一台网关是 Linux 虚拟机或云服务器，不是生产 OpenWrt。canto 只创建和销毁 `table inet canto` 与策略路由表 167，不卸载 ShellCrash。

## 1. 停掉 ShellCrash

在替换前先停掉 ShellCrash（或 Clash 系服务）并清掉它的 nftables / `ip rule`。两套 tproxy 叠在一起会双劫持。具体命令因安装方式而异，常见是停掉其 systemd/init 服务，再按它的文档执行卸载规则。用 `nft list tables` 和 `ip rule` 确认 Clash/ShellCrash 的表和 fwmark 规则已经消失。

## 2. 安装 canto

交叉编译 musl 静态二进制后拷到主机：

```bash
cargo build --release --target x86_64-unknown-linux-musl
sudo cp target/x86_64-unknown-linux-musl/release/canto /usr/local/bin/canto
```

## 3. 配置

`/etc/canto/canto.toml` 示例：

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
tproxy_port = 7893
dns_port = 1053
mixed_port = 7890
fwmark = 424081
routing_mark = 424080
lan_cidrs = ["192.168.1.0/24"]
```

`source` 也可以是本地 JSON 路径。URL 刷新失败时保持当前 sing-box 与 nftables。把 `lan_cidrs` 写成测试机上的内网段，以便劫持局域网 + 本机。

## 4. systemd

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

Docker 只用于 `scripts/netns-check.sh`，不要把容器当成被替换的那台网关。
