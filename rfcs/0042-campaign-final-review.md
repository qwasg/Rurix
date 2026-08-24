<!-- Assisted-by: Cursor Agent（G25.1 治理波） -->
# RFC-0042 — 战役终审程序：画质终态维持核验 + fps 终判两态 + 全链零降级 + 承接锚归档

| 字段 | 值 |
|---|---|
| RFC 编号 | RFC-0042（落盘前实测 ledger RFC next_free=42 顺位领取） |
| 状态 | Agent Approved（经对抗评审 milestones/g25/design/rfc0042_adversarial_review.md，D-409 对抗性评审要求程序） |
| 判档 | Full RFC（战役终审程序留档；零新实现语义面） |
| 承接 | G25.2 M-a/M-b + G25.3 M-c/M-d（七期战役收官） |
| 上游 | G18 §8.7（画质达标 + 17/18 诚实红终值）、G19~G24 六期 close-out、全量 registry |

## 1. 摘要

1. **画质终态维持核验程序（M-a）**：终审输入 = G18 M-d 商用画质达标绿件（AI 读图 + SSIM/FLIP 程序产阈）+ 战役期画质表面 0-byte 机核闭集（display/post_chain/view_transform、presentation 契约、默认渲染 kernels、g14_3_pipeline_perf——vs g18-closed git-diff 逐文件）+ 加性面零接线核验（framegen/hzb/restir_reservoir/slab 四模块不接线任何生产车道的模块引用扫描）。表面 0-byte ∧ 加性零接线 ⇒ G18 达标终态维持有效（重渲无信息增量——UE 全渲重跑触发条件 = 表面变化证据，未命中显式登记）；任一命中 ⇒ 降级检出如实登记 + 重测程序。
2. **fps 终判程序（M-b）**：G14 M-d 最新 18 格 evidence 定盘（met 计数 + 全格 ratio）+ 性能面 0-byte 机核 + 焦点格新鲜单测（canonical 160 帧 bench 一轮，GPU 独占窗，ratio 对 UE 暖态包络登记面）。终判两态：≥1.00 全格 → 18/18；否则 **17/18 诚实红终判**（G15「物理不可达维持未达标登记」兜底同源——战役合法收官态，不冒充）。
3. **全链零降级（M-c）**：G24 链 verify-latest（递归涵盖 G13~G23）+ budget --strict 全量。
4. **承接锚归档闭集（M-d）**：`g25_campaign_handover_registry.json` = G26+ 唯一法定输入面——七期 P2 表 defer/maintain 行、RD 八条锚、历史清册十一条、SAFE-GPU 归档、RD-045 累计观察复核（G19.3 12/12 + G19~G25 六期 soak 零漂移累计）。
5. **out-of-scope**：新优化/新特性、无表面变化证据的 UE 全渲重跑。

## 2. 不变量

- 终审零冒充：达标/诚实红均以机器事实定盘；表面 0-byte 证明为维持终态的必要条件。
- 阈值零手写：×1.00 口径 0-byte；画质阈 G18 程序产面 0-byte。

## 3. 终态程序

M-a~M-d 真跑 evidence 落档后本 RFC 终态 = 战役终审四面记录字面（tag g25-closed 收官）；争议时按只追加程序重判。

## 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-24 | G25.1 起草；对抗评审后 Agent Approved。 |
