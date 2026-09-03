# canto 系统架构与设计文档

本文档详细描述 canto 的系统设计目标、模块划分、透明代理网络编排机制、配置流水线以及故障容错设计。

---

## 1. 背景与设计目标

### 1.1 现状痛点
在 Linux 路由器与嵌入式网关设备上，传统透明代理解决方案（如基于大量 Shell 脚本构建的工具）通常面临以下挑战：
* **执行环境碎片化**：不同固件与发行版中的 Shell 环境（BusyBox ash, Bash, Zsh）以及工具链（GNU sed/awk 与 BusyBox sed/awk）行为不一致，导致边缘工况下容易出现脚本异常。
* **网络状态残留与断网风险**：脚本异常崩溃或被非正常终止时，残留的 iptables 规则与策略路由往往未被清理，导致设备网络彻底瘫痪，必须重启路由器才能恢复。
* **缺乏强类型配置校验**：使用纯文本替换或拼接 JSON/YAML 极易生成非法配置，缺少前置自动化语法检查。
* **进程管理与状态观测困难**：通过 PID 文件或轮询 `pidof` 进行进程监督的方式脆弱，难以捕获结构化日志流与即时崩溃信号。

### 1.2 canto 设计目标
canto 采用 Rust 开发，定位为专注于 **sing-box（1.12 / 1.13+）** 的无依赖单二进制透明网关编排工具：
* **单静态二进制交付**：全静态链接（musl libc），无任何 glibc 或系统脚本解释器依赖，极小内存占用，适配低配嵌入式设备。
* **数据面与控制面彻底分离**：数据面复用成熟的 sing-box 内核；canto 专注作为控制面，负责网络编排、配置流水线和进程监督。
* **原子化网络编排与 RAII 安全兜底**：全面拥抱 Linux 原生 `nftables` 与策略路由；基于 Rust 的 RAII 机制（Drop Guard）确保进程退出时无论正常还是异常均能原子撤销规则，实现不留残留的断网保护。
* **源配置加载与启动覆盖**：启动必须提供完整 sing-box 配置（本地文件或 HTTP(S) URL）；canto 在写出运行时配置时整段替换 `inbounds`，并写入 `route.default_mark`，保证入站与 nftables 端口/防环标记对齐。

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
│   └── settings.rs      # canto.toml 序列化与反序列化
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
1. **启动阶段**：CLI 读取 `canto.toml`，确定运行模式（Tproxy / Tun / None）。
2. **配置准备**：读取必填的 `[singbox].source`（本地文件或 HTTP(S) URL），整段替换 `inbounds` 并写入 `route.default_mark`，再写出 `config_path`，最后调用 `sing-box check` 校验。
3. **网络接管**：`NetworkGuard` 依次配置策略路由表项和 `table inet canto` 防火墙规则。
4. **进程托管**：`ProcessSupervisor` 异步拉起 sing-box 进程，异步管道消费 stdout/stderr 输出并分级接入统一日志。
5. **退出与恢复**：当收到退出信号（SIGINT/SIGTERM）或子进程异常退出时，`NetworkGuard` 触发 Drop 析构函数，原子删除 `inet canto` 表和策略路由，瞬间恢复网络至默认直连状态。

---

## 3. 透明代理与网络编排设计

### 3.1 核心数据链路
透明代理的核心在于静默劫持流入网关的数据报文，重定向至本地 sing-box 监听端口，并杜绝内核自身外联流量陷入回环。

```
客户端流量 (LAN)
      │
      ▼
PREROUTING 链 (inet canto)
      │
      ├─► 匹配保留私网 IP / 本机地址 ──► [直接放行 RETURN]
      │
      ├─► 匹配 sing-box 路由标记 (0x67890) ──► [防环放行 RETURN]
      │
      ├─► 目的端口 == 53 (DNS 流量) ──► [重定向至 local dns_port]
      │
      └─► 其它 TCP/UDP 流量 ──► [Tproxy 到 :tproxy_port 并打标 0x67890]
                                          │
                                          ▼
                                    策略路由 (table 100)
                                          │
                                          ▼
                                    sing-box 内核接管
```

### 3.2 nftables 专属表设计
canto 弃用繁杂且易碎片化的 iptables 链，完全依托原生 `nftables`：
* **专属命名空间**：所有规则集中定义在 `table inet canto` 内。
* **原子提交与清理**：生效时通过 `nft -f -` 单次事务提交完整规则；清理时直接执行 `nft delete table inet canto`，可在微秒级完成全量清除，绝不污染系统预设规则。
* **集合优化（Sets）**：内网保留地址（IPv4 0.0.0.0/8, 10.0.0.0/8, 127.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16 等，以及 IPv6 fc00::/7, fe80::/10 等）统一收敛在 `set reserved_ipv4` 和 `set reserved_ipv6` 中，利用区间树进行快速判定。

### 3.3 策略路由与防回环机制
* **策略路由（Policy Routing）**：
  * 执行 `ip rule add fwmark 0x67890 table 100`，将携带路由标记的数据包引导至专属路由表 `100`。
  * Tproxy 模式下配置 `ip route add local default dev lo table 100`，使内核识别为本机投递并触发套接字分发。
  * Tun 模式下配置 `ip route add default dev tun0 table 100`。
* **防死锁回环（Loop Prevention）**：
  * sing-box 在向境外节点发起连接时，其数据报文同样经过本地网络栈。
  * 在 nftables 的链首放置规则 `meta mark 0x67890 return`，只要报文由 sing-box 内部打上该标记，立即跳过代理重定向规则，保证外联握手正常发往真实 WAN 口。

### 3.4 RAII 异常安全保证
在 Rust 中，`NetworkGuard` 持有 `NetworkSettings` 并实现了标准库的 `Drop` 特征：
```rust
impl Drop for NetworkGuard {
    fn drop(&mut self) {
        if self.active {
            let nft = NftablesManager::new(&self.settings);
            let route = RouteManager::new(&self.settings);
            let _ = nft.flush();
            let _ = route.teardown();
        }
    }
}
```
无论是正常的 `Ctrl+C` 信号、运行时抛出异常、还是未捕获的 Panic 发生，只要栈展开（Stack Unwinding）发生，Rust 的析构函数都会保证清理逻辑严格执行，避免因进程非正常终止而留下断网烂摊子。

---

## 4. 配置流水线与启动覆盖

用户必须提供一份完整的 sing-box 配置；canto 只负责加载、覆盖入站/防环标记，再交给 `sing-box check` 与进程监督。

### 4.1 源配置

`[singbox].source` 为必填项，取值只能是：
* 本地文件路径：指向一份完整的 sing-box JSON。
* `http://` 或 `https://` URL：响应体必须是 sing-box JSON，不是订阅链接、Clash YAML 或其它转换格式。

`config_path` 仍是运行时输出路径。每次 `canto run` 与 `canto config generate` 都重新加载 `source`，完成覆盖后再写入 `config_path`。

加载失败直接拒绝启动，包括：
* 未配置 `source`
* 本地文件不存在或不可读
* URL 拉取失败
* 响应/文件内容不是合法 JSON

用户提供的源文件只读。即使 `source` 与 `config_path` 指向同一路径，也必须先完整读入内存，覆盖后再写回，避免半写入损坏源配置。URL 每次启动现拉，失败不回退本地缓存。

用户继续提供 `outbounds`、`dns`、`route` 规则及其它非 inbound 字段。`experimental.clash_api` 本阶段不覆盖。

### 4.2 启动覆盖规则

覆盖发生在内存中的 JSON 对象上，顺序固定：

1. 整段替换 `inbounds`（用户原 inbound 全部丢弃）。
2. 写入 `route.default_mark = network.routing_mark`；源配置没有 `route` 对象时先创建，已有字段全部保留。
3. 仅在 `network.mode = "tproxy"` 时，若 `route.rules` 中还没有针对 `dns-in` 的 `hijack-dns` 规则，则插入到规则数组头部；没有 `rules` 数组时先创建。

Inbound tag 固定为 `mixed-in`、`tproxy-in`、`dns-in`、`tun-in`。用户 route 规则不得再引用源配置里的旧 inbound tag。

按 `network.mode` 生成 inbound：

**tproxy**
* `mixed`：`tag = mixed-in`，`listen = 0.0.0.0`，`listen_port = mixed_port`（供 LAN 显式代理）。
* `tproxy`：`tag = tproxy-in`，`listen = ::`，`listen_port = tproxy_port`；不设置 `network`，同时接收 TCP/UDP。
* `direct`：`tag = dns-in`，`listen = 0.0.0.0`，`listen_port = dns_port`，承接 nftables 重定向的 53 端口流量。

**tun**
* `mixed-in`：同上。
* `tun`：`tag = tun-in`，`interface_name = tun_interface`，`auto_route = false`，`stack = system`，地址为 `172.19.0.1/30` 与 `fdfe:dcba:9876::1/126`。

**none**
* 仅 `mixed-in`。

tproxy 模式下补齐的 DNS 劫持规则：

```json
{
  "inbound": ["dns-in"],
  "action": "hijack-dns"
}
```

判定“已存在”的条件：某条 `route.rules` 同时满足 `action == "hijack-dns"`，且 `inbound` 包含 `"dns-in"`。已存在则不重复插入。

`canto config generate` 的语义为：加载 `source` → 覆盖 inbound / `default_mark`（及必要时的 `hijack-dns`）→ 写出 `config_path`。

### 4.3 tproxy 覆盖示例

源配置只保留出站与分流；运行时配置的入站由 canto 生成，并与第 3 节 nftables 使用同一组端口与标记（默认 `tproxy_port = 7893`、`dns_port = 1053`、`routing_mark = 424080` / `0x67890`）：

```json
{
  "inbounds": [
    {
      "type": "mixed",
      "tag": "mixed-in",
      "listen": "0.0.0.0",
      "listen_port": 7890
    },
    {
      "type": "tproxy",
      "tag": "tproxy-in",
      "listen": "::",
      "listen_port": 7893
    },
    {
      "type": "direct",
      "tag": "dns-in",
      "listen": "0.0.0.0",
      "listen_port": 1053
    }
  ],
  "route": {
    "default_mark": 424080,
    "rules": [
      {
        "inbound": ["dns-in"],
        "action": "hijack-dns"
      }
    ]
  }
}
```

### 4.4 语法预检（Pre-flight Check）
在拉起 sing-box 之前，`ProcessSupervisor::check_config()` 会显式调用：
```bash
sing-box check -c <config_path>
```
只有当命令返回 exit code 0 时才允许继续启动。如果校验失败，进程拒绝拉起并直接在终端格式化输出详细的语法错误日志，防止线上盲目重启引发服务中断。

---

## 5. 进程监督与生命周期管理

### 5.1 异步管道分流
canto 利用 `tokio::process::Command` 启动 sing-box，将子进程的 `stdout` 和 `stderr` 重定向为管道形式：
* 标准输出由后台异步协程逐行读取，通过 `tracing::info!(target: "sing_box", ...)` 注入 canto 统一日志流。
* 标准错误由后台异步协程逐行读取，通过 `tracing::warn!(target: "sing_box", ...)` 标记为告警。

### 5.2 信号处理与优雅停机
主运行循环通过 `tokio::select!` 同时等待两个分支：
1. 子进程的异步等待句柄 `child.wait()`。
2. 操作系统终端中断信号 `tokio::signal::ctrl_c()`。

一旦接收到 SIGINT 或 SIGTERM，supervisor 立即向子进程下发终止信号，等待子进程退出并释放所有本地资源，最后触发 `NetworkGuard` 的 Drop 流程。

---

## 6. 配置规范（canto.toml）

canto 提供平铺且语义清晰的配置定义：

```toml
[canto]
work_dir = "./run"               # 运行时工作目录（日志与运行时生成文件）
log_level = "info"               # trace / debug / info / warn / error

[singbox]
binary = "sing-box"              # sing-box 执行文件路径（默认在 PATH 中查找）
source = "./upstream.json"       # 必填：完整 sing-box JSON 的本地路径或 http(s) URL
config_path = "./run/config.json"# 覆盖 inbound / default_mark 后的运行时输出路径
api_listen = "127.0.0.1:9090"    # sing-box 内部 Clash API 监听地址（本阶段不覆盖进运行时配置）

[network]
enabled = true                   # 是否接管网络防火墙与策略路由
mode = "tproxy"                  # 透明代理模式: "tproxy" | "tun" | "none"
tproxy_port = 7893               # sing-box 的 tproxy 入站端口
dns_port = 1053                  # sing-box 的本地 DNS 劫持接收端口
mixed_port = 7890                # 本地混合代理端口 (HTTP/SOCKS5)
routing_mark = 424080            # 路由标记 (十六进制 0x67890)
table_id = 100                   # 专属策略路由表 ID
tun_interface = "tun0"           # Tun 模式下的虚拟网卡名
bypass_cn_ips = true             # 是否绕过大陆 IP
bypass_reserved_ips = true       # 是否绕过保留局域网私网网段
```

---

## 7. 部署与交叉编译规范

### 7.1 静态编译构建
为确保构建产物能无缝运行于各主流路由器固件（OpenWrt、Armbian、Alpine 等），推荐采用基于 musl 的纯静态目标：

* **x86_64 架构**：
  ```bash
  cargo build --release --target x86_64-unknown-linux-musl
  ```
* **aarch64（ARM64 软路由）**：
  ```bash
  cross build --release --target aarch64-unknown-linux-musl
  ```
* **armv7（如传统四核硬路由）**：
  ```bash
  cross build --release --target armv7-unknown-linux-musleabihf
  ```
* **mipsle（如部分老旧 MIPS 架构设备）**：
  ```bash
  cross build --release --target mipsel-unknown-linux-musl
  ```

### 7.2 Systemd 服务单元示范
在标准的 Linux 发行版中，可直接以 systemd 单元托管运行：

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

---

## 8. 未来演进路线（Roadmap）

* **Phase 1（当前）**：
  * 完成 single-crate 分包骨架搭建。
  * 实现基于 nftables + 策略路由的透明代理编排与 RAII 安全释放。
  * 从本地文件或 HTTP(S) URL 加载完整 sing-box 配置，启动时整段替换 `inbounds` 并写入 `route.default_mark`。
  * 实现 sing-box 子进程异步托管与语法预检机制。
* **Phase 2**：
  * 集成 Clash REST API 客户端模块，支持从终端或本地 IPC 查询代理节点延迟、实时带宽与流量统计。
  * 按需覆盖 `experimental.clash_api`。
* **Phase 3**：
  * 引入 `ratatui` + `crossterm` 构建现代终端控制台（TUI），提供可视化出站切换与连接观察仪表盘。
* **Phase 4**：
  * 引入订阅拉取流水线，支持远程订阅更新与 sing-box 规则集定时刷新机制。
