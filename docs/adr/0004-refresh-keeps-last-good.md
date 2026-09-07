# 刷新失败保留当前网关，成功则重启 sing-box

URL 源地址在运行中定时刷新。启动时拉不到且没有可用缓存，则拒绝接管网络。运行中拉取或 check 失败，则保持当前 sing-box 与 nft，不撤规则。刷新成功则再次覆盖、check，然后重启 sing-box；nft 不拆，因为劫持端口来自 canto 配置而非源配置。这轮不做 sing-box 热重载。
