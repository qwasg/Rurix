<!-- Assisted-by: Cursor Agent（G19.1 治理波） -->
# RFC-0036 — 帧生成独立层兑现（host 参考臂 + MFG 多档 + vendor disposition 三臂）

| 字段 | 值 |
|---|---|
| RFC 编号 | RFC-0036（落盘前实测 ledger RFC next_free=36 顺位领取） |
| 状态 | Agent Approved（经对抗评审 milestones/g19/design/rfc0036_adversarial_review.md，D-409 对抗性评审要求程序） |
| 判档 | Full RFC（帧生成为独立呈现层新登记面；渲染器库面零新语言语义条款，G5 先例） |
| 承接 | G19.2 M-a / M-b（G13-N7 兑现；RFC-0035 defer 终态重判条件命中） |
| 上游 | RFC-0035（defer 终态归档）、G18_P2_DECISIONS §1 G13-N7、RD-041 FG/MFG 分项 |

## 1. 摘要

G18 M-h 以 defer-to-G19+ 收口（RFC-0035：FSR3-FG vs DLSS-G 双 vendor measured 窗不齐备）。其重判条件「RFC-0035 终态落档后按只追加程序重判」已命中。本 RFC 立项 G19 兑现路径：

1. **host 参考臂（本期实现）**：`rurix_render::temporal::framegen` 模块——mv 双向 warp 帧插值（prev 取 `p − t·mv`、cur 取 `p + (1−t)·mv`）+ 遮挡感知混合（warp 一致性权重 + 兜底最近帧采样）+ MFG 多档（×2/×3/×4，t = i/(N+1)）。纯 f32 host 确定性实现，双跑位级一致。
2. **质量判据（程序产禁手写）**：逐帧对照阈——`SSIM(interp_i, GT_i) > SSIM(frame_hold_i, GT_i)`（frame-hold = 复制最近真渲帧的零成本基线；插帧必须严格优于帧保持才构成兑现）。ground truth = 确定性程序化动画序列的解析式全帧率渲染。
3. **口径纪律（G13-N7 字面 0-byte）**：真实渲染帧率口径不变——FG/MFG 生成帧**禁计入**真实渲染帧率与 upscale ratio；`presented_fps`（真渲 + 生成）为**独立新登记面**，与 `real_render_fps` 并列输出、永不混算。
4. **vendor 三臂 disposition（本期处置）**：FSR3-FG（external/fidelityfx-sdk-2.0.0，C++ 集成面）、DLSS-G（需 D3D12 swapchain 宿主车道，RFC-0032 车道终态约束）、SL-310.6.0 换版窗（external/streamline-2.10.3-ngx310.6.0，G17-MB-F1 兜底重评）——逐臂 integrated / rejected / not-available 三态登记 `milestones/g19/g19_vendor_sdk_registry.json`，均为合法终态。
5. **device kernel 车道（本期 out-of-scope）**：framegen 的 .rx device 化（G13.3 TSR device 化同模式）显式登记 out-of-scope，承接锚 = G25 全量终审窗重判或后续期。

## 2. 不变量

- 默认臂 Stage A digest 锚 18 格零漂移；`g14_3_pipeline_perf` 本期 0-byte 只读消费。
- framegen 为加性独立模块：不接线任何既有渲染车道；`rurix-render` 维持 `#![forbid(unsafe_code)]`。
- 阈值零手写：唯一质量判据为 interp vs frame-hold 程序产对照。

## 3. 终态程序

M-a/M-b 真跑 evidence 落档后本 RFC 终态 = approved-implemented（host 参考臂）+ vendor 三臂逐臂 disposition 字面；争议时按只追加程序重判。

## 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-24 | G19.1 起草；对抗评审后 Agent Approved。 |
