# 配置模板模块化目录拆分与 Canto 命名空间隔离

扩展 [ADR 0012](0012-web-studio-producer-coexistence.md)、[ADR 0013](0013-split-node-groups-from-policy-groups.md) 与 [ADR 0014](0014-decouple-rule-sets-and-support-release-providers.md)：将 Web Studio 配置模板（Template）在磁盘上的持久化形式从单体 JSON 文件重构为模块化目录结构，并通过 `canto.<module>.json` 统一前缀实现 Canto 编排扩展与原生 sing-box 配置的隔离。

## 背景

此前 Web Studio 模板以单体 JSON（`work_dir/studio/templates/<id>.json`）平铺持久化，将 canto 专有的编排 DSL 字段（`node_groups`、`policy_groups`、`rule_sets`）与 sing-box 原生顶层配置（`log`、`dns`、`inbounds`、`outbounds`、`route`、`endpoints`、`experimental`）混合在同一 `content` 根对象下。

这带来了以下风险与人机工效局限：
1. **潜在命名空间冲突**：sing-box 官方配置规范演进频繁（例如社区一直有提议将规则集从 `route.rule_set` 提升为顶层 `rule_sets`）。若未来官方引入同名根字段，canto 编译器将无法区分这是原生官方配置还是待展开的 canto DSL。
2. **大体量配置维护与 Git 协同劣势**：单体 JSON 包含各模块的所有配置，查看、微调或通过 Git 协同管理时 diff 噪音大，难以局部聚焦。
3. **与界面心智未完全映射**：Web 控制台编辑器早已按「节点分组」、「出站策略组」、「规则集」、「DNS」、「路由」等子 Tab 拆分管理，但存储层仍是单体巨石。

## 决定

1. **模板持久化升级为模块化目录**：
   - 每个模板独立为一个目录：`work_dir/studio/templates/<id>/`。
   - 彻底废除单体 `<id>.json` 文件存储。

2. **模块划分与文件映射契约**：
   - **元数据**：`meta.json`，仅包含 `id`、`name`、`description` 与 `updatedAt`。
   - **Canto 专属编排模块**（采用 `canto.*.json` 命名，文件级彻底隔离）：
     - `canto.node_groups.json`：节点筛选正则占位符与测速分组（数组）。
     - `canto.policy_groups.json`：面向分流的业务策略组（数组）。
     - `canto.rule_sets.json`：多源规则集聚合声明（含 `preset_id`、`tag_prefix` 等扩展属性）。
   - **原生 sing-box 模块**（100% 对齐官方同名字段，直接 1:1 映射）：
     - `dns.json`、`route.json`、`inbounds.json`、`outbounds.json`、`log.json`、`experimental.json`、`endpoints.json` 等。
     - 任意其它合法的顶层配置键同样独立映射为 `<key>.json`。

3. **原子持久化保障**：
   - `persist_template` 写入时先落盘至同级临时目录 `.<id>.tmp/`，各模块文件写入并 `sync_all` 完成后，通过文件系统原子操作替换目标目录，杜绝写入中断导致的不完整状态。
   - 模板删除时同步清理目标目录。

4. **对外 API 兼容性保证**：
   - 对外 REST API（`/api/templates`）保持现有契约不变，输入输出仍然将模块聚合为统一的 `content` 树形结构，前端 Svelte 控制台与下游配置编译器（`ProfileCompiler`）无感零破坏。
