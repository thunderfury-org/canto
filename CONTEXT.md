# Canto

canto 是 Linux / OpenWrt 透明代理网关的控制面：消费完整 sing-box JSON，覆盖入站与标记，编排劫持规则并监督 sing-box。

## Language

**源配置**:
canto 消费的一份完整 sing-box JSON。由 canto 之外的服务生产。
_Avoid_: 模板, 订阅, 配置

**源地址**:
取得源配置的位置，本地路径或 HTTP(S) URL。
_Avoid_: 订阅链接

**覆盖**:
canto 对源配置的改写，使入站与路由标记对齐网关的数据包劫持。
_Avoid_: 合并, 补丁, 转换

**刷新**:
网关运行中重新取得源配置并再次覆盖。
_Avoid_: 热更新, 订阅更新, reload

**网关**:
canto 编排透明代理的那台 Linux / OpenWrt 机器。
_Avoid_: 客户端, 节点, 路由器
