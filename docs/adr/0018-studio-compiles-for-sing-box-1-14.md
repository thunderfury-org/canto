# Web Studio 的配置编译只面向 sing-box 1.14

Web Studio 编译出的 sing-box JSON 只保证 1.14 能通过检查。1.13 拉到同一份订阅后失败也可以。已保存的配置模板直接改成新形状，应用不认识旧字段，也不留迁移代码。网关对源配置的覆盖不在这次范围里。

`dns.strategy` 仍是 DNS 的默认域名策略，不改。`independent_cache` 删掉。拒绝是路由分流规则的结果，不是出站；`type: block` 从出站策略组消失。规则源仍只指定下载所走的出站，保存为 `http_client.detour`；顶层 `http_clients` 与 `route.default_http_client` 由配置编译生成，不写进配置模板。
