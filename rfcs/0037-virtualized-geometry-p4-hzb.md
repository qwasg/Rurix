<!-- Assisted-by: Cursor Agent（G20.1 治理波） -->
# RFC-0037 — 虚拟化几何 P4：HZB 遮挡剔除 host 参考臂 + cluster 流送 P4 评估 + M61/M98-l4 重判程序

| 字段 | 值 |
|---|---|
| RFC 编号 | RFC-0037（落盘前实测 ledger RFC next_free=37 顺位领取） |
| 状态 | Agent Approved（经对抗评审 milestones/g20/design/rfc0037_adversarial_review.md，D-409 对抗性评审要求程序） |
| 判档 | Full RFC（几何剔除新登记面；渲染器库面零新语言语义条款，G5 先例） |
| 承接 | G20.2 M-a/M-b + G20.3 M-c/M-d（M61 重判条件 HZB 半边兑现 + M98-l4 窗 + RD-039 P4 分项） |
| 上游 | RFC-0034（no-go 终态归档）、G19_P2_DECISIONS §1、RD-039 |

## 1. 摘要

1. **HZB host 参考臂（本期实现）**：`rurix_render::geometry::hzb`——层级深度金字塔（每级纹素 = footprint 内**最远**深度：reverse-Z 取 min / standard-Z 取 max，保守遮挡语义唯一合法归约方向；非 2 幂 ceil 减半 + 越界 clamp）+ `test_rect` 保守遮挡测试（rect 跨度 ≤2 纹素选级 + ≤2×2 窗最远归约）。兑现 geometry 模块头注「HZB 两阶段 P3 预留」第一阶段 host 面。
2. **保守性硬不变量（程序产判据禁手写）**：`test_rect` 判 Occluded ⇒ 逐像素精确真值（`exact_rect_occluded`，测试金标准）必同判——**零假阳性**（不得剔可见物；漏剔合法，保守性只损效率不损正确）+ 剔除率非零 + 双跑位级。
3. **cluster 流送 P4 评估（本期处置）**：streaming/（pool/feedback/engine/resource 页式面）现面盘点 vs P4 目标（cluster 页驻留/请求反馈/优先级/异步 IO 链），差距闭集登记 `milestones/g20/g20_cluster_streaming_p4_gap.json`；go/no-go/defer 均合法。
4. **M61 重判程序（M-c）**：重判条件「HZB/cluster P4 触发条件齐备」HZB 半边本期兑现、cluster 半边以差距闭集落档；mesh shader HW 管线性能差 measured 证据面核验（无证据 → maintain-no-go + RFC-0034 只追加重判记录）；VS 光栅唯一 fallback 兜底 0-byte。
5. **M98-l4 重判程序（M-d）**：HLOD 运行时接口面就绪核验（world/hlod.rs + g9_m111 门绿件机核）+ L4 计数可测性评估（device 车道计数器面缺口如实登记）；实现/维持 L1/L2/L3 三级链均合法。
6. **device kernel 车道（本期 out-of-scope）**：HZB device 化（compute 金字塔构建 + 剔除 pass 接线）显式登记 out-of-scope，承接锚 = 后续期 device 波。

## 2. 不变量

- 既有 cull/visbuffer/streaming 面 0-byte 只读消费；`rurix-render` 维持 `#![forbid(unsafe_code)]`。
- 默认臂 Stage A digest 锚红线（本期 g14_3_pipeline_perf 0-byte）。
- 阈值零手写：唯一硬判据 = 零假阳性不变量（逐像素精确真值对拍）。

## 3. 终态程序

M-a/M-b/M-c/M-d 真跑 evidence 落档后本 RFC 终态 = approved-implemented（HZB host 参考臂）+ cluster P4 差距闭集 + M61/M98-l4 重判记录字面；争议时按只追加程序重判。

## 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-24 | G20.1 起草；对抗评审后 Agent Approved。 |
