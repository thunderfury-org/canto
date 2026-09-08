# canto 不卸载 ShellCrash

两套 tproxy/nft 不能叠跑，但卸载是一次性人工切换。canto 只创建和销毁自己的 `table inet canto` 与策略路由表 167。启动前由操作者停掉 ShellCrash 并清掉它的规则；文档写步骤。不检测 Clash 残留，也不做自动迁移。
