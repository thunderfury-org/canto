---
status: accepted
---

# 确认弹窗与抽屉草稿放弃守卫

使用统一的暗色控制台确认弹窗（Confirm Modal）与基于 Promise 的单例调用契约，彻底替换 Web Studio 中所有浏览器原生 window.confirm()，并在 Drawer 抽屉中内聚未保存草稿的放弃拦截。

## 背景

Web Studio 在演进过程中散落了 18 处浏览器原生 `window.confirm()`，用于模板、节点源、分组、分流规则的删除防呆与抽屉草稿放弃。
1. **视觉割裂与层级破坏**：原生弹窗依赖宿主系统，无法融入暗色炭灰与铜的控制台表面，且无法按 ADR 0016 规范结构化展示受影响的关联实体清单。
2. **逻辑冗余与分散**：各编辑器抽屉（DNS、入站、路由规则）各自重复编写遮罩与按键监听来做 `if (isDirty && !confirm(...))`。
3. **安全防误触不足**：原生弹窗在部分浏览器上默认焦点在确定按钮，用户按回车极易误执行高危删除。

## 决定

1. **单例 Promise 调用契约**：
   - 在 Store 统一提供 `store.confirm(options): Promise<boolean>`，根组件挂载全局唯一的 `ConfirmModal.svelte`，避免在 7 个视图中重复声明弹窗组件与局部状态。
2. **严格禁止实心铜按钮**：
   - 确认弹窗仅提供二元决策，遵循 ADR 0016/0017，确认按钮按语气使用带边框的 `btn-secondary btn-danger`（删除）或 `btn-secondary btn-warning`（警告/放弃），取消使用 `btn-secondary`。坚决不用 `btn-primary`。
3. **结构化明示关联影响（Impacts）**：
   - 废除多行换行与 emoji 拼接字符串。对于被订阅引用的模板或节点源，通过 `impacts` 属性在等宽炭灰面板中清晰列出关联实体与后效警示。
4. **键盘焦点防呆闭环**：
   - 弹窗打开后初始焦点强制落于 [取消] 按钮防误触；支持 Esc 与遮罩取消；Tab 键严格限制在弹窗两个按钮内轮转；关闭后恢复前序焦点。
5. **Drawer 内聚放弃拦截**：
   - `Drawer.svelte` 接收 `dirty` 属性。在脏状态下，用户点击遮罩、X 按钮或按 Esc 时由 Drawer 自动唤起放弃确认，外部视图无需再监听脏退出。
6. **三态结算边界隔离**：
   - `Navbar.svelte` 针对退出登录/切页导航的“保存/放弃/取消”三态流水线（`store.leaveOpen`）保持专有，不强行并入二元确认弹窗。

## 影响

- 彻底移除全站 18 处原生 `window.confirm()`。
- 抽屉组件代码大幅精简，未保存草稿的放弃行为全端高度一致。
- 新增 `web/test/confirmModal.test.js` 测试，确保状态转换与 Promise 契约零退化。
