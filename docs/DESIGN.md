# canto 系统架构与设计文档

本文档描述 canto v1 的系统设计目标、模块划分、透明代理网络编排、配置覆盖流水线以及故障容错设计。

透明入站（redirect / tproxy / TUN / auto_redirect）和内核 `bypass` 的选型见 [INBOUND.md](INBOUND.md)。本文只写 v1 实际落地的 tproxy 行为。

---

## 1. 背景与设计目标

### 1.1 现状痛点
在 Linux 路由器与嵌入式网关设备上，传统透明代理解决方案（如基于大量 Shell 脚本构建的工具）通常面临以下挑战：
* **执行环境碎片化**：不同固件与发行版中的 Shell 环境以及工具链行为不一致，边缘工况下容易出现脚本异常。
* **网络状态残留与断网风险**：脚本异常崩溃或被非正常终止时，残留的防火墙规则与策略路由往往未被清理。
* **缺乏强类型配置校验**：纯文本替换或拼接 JSON 极易生成非法配置，缺少前置语法检查。
* **进程管理与状态观测困难**：通过 PID 文件或轮询 `pidof` 进行进程监督的方式脆弱。

### 1.2 canto 设计目标
canto 采用 Rust 开发，定位为专注于 **sing-box（1.13+）** 的无依赖单二进制透明网关编排工具：
* **单静态二进制交付**：全静态链接（musl libc），无 glibc 或系统脚本解释器依赖。
* **数据面与控制面分离**：数据面复用 sing-box；canto 作为控制面，负责网络编排、运行时覆盖和进程监督。
* **原子化网络编排与 RAII 兜底**：使用 Linux 原生 `nftables` 与策略路由；`NetworkGuard` 在 SIGINT/SIGTERM 或进程退出时撤销规则。SIGKILL / OOM 无法走 Drop，需配合 systemd `ExecStop=canto clean-network`。
* **源配置加载与启动覆盖**：启动必须提供完整 sing-box JSON 本地文件；canto 整段替换 `inbounds`，写入 `route.default_mark`，并关闭 `auto_detect_interface`，保证入站与 nftables 端口/防环标记对齐。

v1 不做订阅转换、模板合并、TUI、设备过滤、大陆 IP 绕过、IPv6 劫持或 HTTP 拉配置。

---

## 2. 总体架构与模块划分

项目采用单 crate 内部分包架构，代码组织于 `src/` 目录下：

```
canto
├── src/main.rs          # 程序主入口、CLI 分发与全局追踪日志配置
├── src/lib.rs           # 库级接口导出
├── src/cli/             # 基于 clap 的命令行参数与子命令模型
│   ├── mod.rs
│   └── args.rs
├── src/config/          # 配置管理与运行时覆盖
│   ├── mod.rs
│   ├── settings.rs      # canto.toml 序列化与反序列化
│   └── overlay.rs       # 源 JSON 加载与 inbound / default_mark 覆盖
├── src/network/         # 透明代理网络编排
│   ├── mod.rs
│   ├── nftables.rs      # nftables ruleset 模板生成与执行
│   ├── route.rs         # Linux 策略路由 (ip rule / ip route)
│   └── guard.rs         # RAII NetworkGuard 异常安全清理器
├── src/supervisor/      # 子进程监督
│   ├── mod.rs
│   └── process.rs       # sing-box 启动、存活监督、日志分流与信号处理
└── src/error.rs         # 基于 thiserror 的统一强类型错误枚举
```

### 2.1 职责边界与协作流程
1. **启动阶段**：CLI 读取 `canto.toml`。`--no-network` / `network.enabled = false` 跳过网络接管；抓包固定为 tproxy。
2. **配置准备**：读取必填的 `[singbox].source` 本地文件，覆盖 inbound / `default_mark` / `hijack-dns` / `auto_detect_interface`，写出 `config_path`，再调用 `sing-box check`。
3. **网络接管**：check 通过后，`NetworkGuard` 配置策略路由和 `table inet canto`。
4. **进程托管**：`ProcessSupervisor` 异步拉起 sing-box，消费 stdout/stderr。
5. **退出与恢复**：SIGINT/SIGTERM 或子进程退出时，先停止 sing-box，再由 `NetworkGuard` Drop 删除 `inet canto` 表和策略路由。

---

## 3. 透明代理与网络编排设计

v1 固定为 **tproxy + 局域网 + 本机**。IPv6 流量在链首 `return`，不劫持。

不是因为 TUN 更慢才不用：Linux / OpenWrt 上官方更快的路径是 TUN + `auto_redirect`。v1 用手搓 tproxy，是为了按 LAN 来源劫持、挡住 WAN 进站，并且让 `NetworkGuard` 回滚 canto 自己的表。桌面源 JSON 里的 `tun-in` + `auto_route` 会抢网关默认路由，必须整段替换。升级方向见 [INBOUND.md](INBOUND.md)。

### 3.1 核心数据链路

局域网劫持和本机回流拆成两条 prerouting 链，避免已打标回流包和 WAN 进站走进局域网规则。

```
LAN 客户端
      │
      ▼
tproxy_prerouting (mangle - 10)
      │
      ├─► IPv6 / 已打标 ──► RETURN
      ├─► 来源不在 lan_ipv4 ──► RETURN      # 不劫持 WAN 进站
      ├─► 目的是保留地址 / DNS 53 ──► RETURN
      └─► 其它 TCP/UDP ──► tproxy :tproxy_port 并打标

本机进程 outbound
      │
      ▼
tproxy_output
      │
      ├─► 已打标 / 保留地址 / DNS ──► RETURN
      └─► 其它 TCP/UDP ──► 仅打标
                              │
                              ▼
                        策略路由 table 167
                        local default dev lo
                              │
                              ▼
                  tproxy_mark_out (mangle)
                  已打标 TCP/UDP ──► tproxy :tproxy_port
```

DNS：
* prerouting NAT：仅 `lan_ipv4` 来源的 53 端口 redirect 到 `dns_port`
* output NAT：本机 53 端口 redirect 到 `dns_port`
* 已打标流量放行，避免劫持 sing-box 自己的上游 DNS

`input_protect` 拒绝非 `lan_ipv4` 来源访问 mixed/tproxy/dns 端口。禁止在 tproxy 链写无条件 `iif "lo" return`，否则本机回流无法 tproxy。

`lan_ipv4` 来自 `[network].lan_cidrs`。留空时 Linux 探测 `ip -4 route show scope link`（跳过 wan/docker/tun 等接口）；探测失败或非 Linux 回退 RFC1918。

### 3.2 nftables 专属表设计
* 所有规则集中在 `table inet canto`。
* 生效时 `nft -f -` 一次提交；清理时 `nft delete table inet canto`。
* 保留地址放在 `set reserved_ipv4`，用于**目的地址**绕过。
* 局域网来源放在 `set lan_ipv4`。
* 启动时若 `ip_forward=0` 则写成 `1`；尝试 `modprobe nft_tproxy`，内建内核允许失败。

### 3.3 策略路由与防回环
* `ip rule add fwmark <fwmark> table 167`
* `ip route add local default dev lo table 167`（表号写死，避开 Clash/ShellCrash 常用的 100，以及内核保留的 253-255）
* nft 给劫持流量盖 `fwmark`；sing-box `route.default_mark` 使用另一套 `routing_mark`。`ip rule` 只认 `fwmark`，代理出站走 main 表从 WAN 出去。

### 3.4 RAII 异常安全保证
`NetworkGuard` 实现 `Drop`：进程退出或栈展开时删除 nftables 表并拆除策略路由。正常停机路径先向 sing-box 发送 SIGTERM，再 Drop。

---

## 4. 配置流水线与启动覆盖

用户必须提供一份完整的官方 sing-box JSON。节点过滤语法（如 `{My-}`）不处理。dns / outbounds / endpoints / experimental 原样保留，源里的 `tun-in` 被整段替换掉。

### 4.1 源配置

`[singbox].source` 为必填本地文件路径。v1 不支持 HTTP(S) URL。

`config_path` 是运行时输出路径。每次 `canto run` 与 `canto config generate` 都重新加载 `source`，覆盖后再写入。

加载失败直接拒绝启动：
* 未配置 `source`
* 本地文件不存在、不可读或不是 JSON object

即使 `source` 与 `config_path` 指向同一路径，也必须先完整读入内存，覆盖后再写回。

### 4.2 启动覆盖规则

覆盖发生在内存中的 JSON 对象上，顺序固定：

1. 整段替换 `inbounds`。
2. 写入 `route.default_mark = network.routing_mark`；没有 `route` 对象时先创建。
3. 强制 `route.auto_detect_interface = false`。
4. 若 `route.rules` 中还没有针对 `dns-in` 的 `hijack-dns` 规则，则插入到规则数组头部。

Inbound tag 固定为 `mixed-in`、`tproxy-in`、`dns-in`。

**tproxy inbound**
* `mixed`：`tag = mixed-in`，`listen = 0.0.0.0`，`listen_port = mixed_port`
* `tproxy`：`tag = tproxy-in`，`listen = ::`，`listen_port = tproxy_port`；不设置 `network`
* `direct`：`tag = dns-in`，`listen = 0.0.0.0`，`listen_port = dns_port`

tproxy 模式下补齐的 DNS 劫持规则：

```json
{
  "inbound": ["dns-in"],
  "action": "hijack-dns"
}
```

判定“已存在”的条件：某条 `route.rules` 同时满足 `action == "hijack-dns"`，且 `inbound` 包含 `"dns-in"`。

`canto config generate`：加载 `source` → 覆盖 → 写出 `config_path`。

### 4.3 语法预检
在接管网络之前调用 `sing-box check -c <config_path>`。失败则拒绝启动，不修改防火墙。

---

## 5. 进程监督与生命周期管理

### 5.1 异步管道分流
canto 用 `tokio::process::Command` 启动 sing-box：
* stdout → `tracing::info!(target: "sing_box", ...)`
* stderr → `tracing::warn!(target: "sing_box", ...)`

### 5.2 信号处理与优雅停机
主循环同时等待子进程退出、SIGINT 与 SIGTERM。收到信号后先向 sing-box 发送 SIGTERM，超时再 SIGKILL，随后 `NetworkGuard` Drop。v1 不实现 daemon，用 systemd 托管 `canto run`。

---

## 6. 配置规范（canto.toml）

```toml
[canto]
work_dir = "./run"

[singbox]
binary = "sing-box"
source = ".data/config-with-tailscale.json"
config_path = "./run/config.json"

[network]
enabled = true
tproxy_port = 7893
dns_port = 1053
mixed_port = 7890
fwmark = 424081          # 0x67891，tproxy / ip rule，避免用 1 这类常见值
routing_mark = 424080    # 0x67890，sing-box default_mark，须与 fwmark 不同
lan_cidrs = []           # 空则自动探测；可写成 ["192.168.1.0/24"]
```

---

## 7. 部署与交叉编译规范

### 7.1 静态编译构建
* **x86_64**：`cargo build --release --target x86_64-unknown-linux-musl`
* **aarch64**：`cross build --release --target aarch64-unknown-linux-musl`
* **armv7**：`cross build --release --target armv7-unknown-linux-musleabihf`
* **mipsle**：`cross build --release --target mipsel-unknown-linux-musl`

### 7.2 Systemd 服务单元示范

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

macOS 开发机只生成并校验规则，不执行 `nft` / `ip`。

---

## 8. 未来演进路线（Roadmap）

Phase 1 已完成：本地完整 JSON、inbound 覆盖、tproxy 劫持局域网+本机、预检后再接管网络、SIGINT/SIGTERM 清规则。下面只列还没做的。

### 8.1 上路由器前

这些不补的话，真实网关和 ShellCrash 仍会有可见偏差。

* **Linux 真机验收**：用 `scripts/netns-check.sh`（OrbStack/Docker 也可）验证 LAN/本机劫持、WAN 端口拒绝和退出回滚。`dump-nft` 只核对文本。`nft_tproxy` 现在只尝试 modprobe，内建失败不阻断，真正缺能力时仍在 `nft -f` 时报错。

### 8.2 网关语义补齐

* **大陆 IP 绕过**：优先用 sing-box `action: bypass`（预匹配、需 `auto_redirect`）或 TUN `route_exclude_address_set`，不要再造一张 cnip nft 表。exclude set 会跳过后续规则和 Clash Global。详见 [INBOUND.md](INBOUND.md)。
* **常用端口**：ShellCrash 默认只劫持 22/80/443/8080/8443。canto 劫持全部 TCP/UDP。
* **IPv6 劫持**：现在链首 return。要做就需要 IPv6 策略路由和来源网段。
* **设备过滤**：MAC/IP 黑白名单。
* **WAN 防护**：`input_protect` 已挡 mixed/tproxy/dns；仍缺 Clash API、自定义放行端口、fw4 协同。

### 8.3 配置与节点

* **HTTP(S) 拉源配置**：v1 只读本地 `source`。失败是否回退缓存要单独定。
* **订阅/provider**：`{My-}` 这类过滤不处理。节点必须预先写进完整 JSON。
* **覆盖 `experimental.clash_api`**：源配置里有就保留；以后做改写时再加监听地址配置。当前不查延迟/流量。
* **tun 模式**：以后做 `network.mode` 时切到 TUN + `auto_redirect`，缩小 canto 自己的 nftables，而不是优化现行 tproxy。源里的桌面 TUN 仍要 overlay，不能原样 `auto_route`。当前固定 tproxy，legacy `mode` / `bypass_cn_ips` 键忽略。详见 [INBOUND.md](INBOUND.md)。

### 8.4 运行与交付

* **安装与自启**：现在只有文档里的 systemd 示例，没有安装脚本、procd/OpenRC、交叉编译发布。
* **内核与面板**：不下载 sing-box，不安装 Dashboard。
* **TUI / 交互菜单**：不替代 `crash` 选单。
* **daemon**：v1 不实现；用 systemd 托管 `canto run`。
