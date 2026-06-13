# UI snapshot bless 审批记录(只追加)

> 任何 `tests/ui/**/*.stderr` 的新增/修改/删除必须同 PR 在本表追加一行(14 §6;
> M1 CI_GATES §4 第 6 项,`ci/check_guardrails.py` 机器核对:既有行 0-byte)。

| 日期 | 范围 | 理由 | 批准 |
|---|---|---|---|
| 2026-06-11 | tests/ui/parse/ 初始 12 条 snapshot | M1.4 黄金路径 1 首批落地(D-M1-4),通道建立时的初始 bless | pending-human-review |
| 2026-06-11 | tests/ui/parse/ 初始 12 条 snapshot | 人工终审批准首批 snapshot bless;用于 M1 close-out §8.2/§8.3 留痕 | qwasg |
| 2026-06-11 | tests/ui/resolve/ 新增 4 条 snapshot(RX1001~RX1004 各一) | M2.1 名称解析诊断首批(D-M2-1,RXS-0038);UI 通道接入 resolve 阶段,既有 parse snapshot 零变化 | pending-human-review |
| 2026-06-12 | tests/ui/typeck/ 新增 12 条 snapshot(黄金路径 2,RX2001~RX2006 全覆盖) | M2.2 类型检查诊断首批(D-M2-4 / G-M2-3,RXS-0047);UI 通道阶段化接入 typeck(前一阶段有错即停,防级联),既有 16 条 snapshot 零变化 | pending-human-review |
| 2026-06-12 | tests/ui/resolve/ 4 条 snapshot(2026-06-11 行) | 人工终审批准 M2.1 resolve 首批 snapshot bless;用于 M2 close-out §8.2/§8.3 留痕 | qwasg(会话授权,agent 代笔) |
| 2026-06-12 | tests/ui/typeck/ 12 条 snapshot(2026-06-12 行) | 人工终审批准 M2.2 黄金路径 2 首批 snapshot bless;用于 M2 close-out §8.2/§8.3 留痕 | qwasg(会话授权,agent 代笔) |
| 2026-06-12 | tests/ui/typeck/ 新增 2 条 snapshot(non_exhaustive_match_enum / non_exhaustive_match_fallback,RX2007) | M3.1 模式穷尽性诊断首批(M3_PLAN §1 任务 5,RXS-0051;UI 通道阶段化接入 TBIR 窄门模式检查,既有 28 条 snapshot 零变化) | pending-human-review |
| 2026-06-13 | tests/ui/typeck/ 2 条 RX2007 snapshot(2026-06-12 行) | 人工终审批准 M3.1 模式穷尽性首批 snapshot bless;用于 M3 close-out §8 留痕 | qwasg(会话授权,agent 代笔) |
| 2026-06-13 | tests/ui/borrowck/ 新增 5 条 snapshot(RX4001 ×2 / RX4002 ×2 / RX4003 ×1) | M3.2 move/init 数据流诊断首批(M3_PLAN §2 任务 3,RXS-0053/RXS-0054;UI 通道阶段化接入 MIR 后 move/init 检查,既有 30 条 snapshot 零变化;计入黄金路径 3 计数,G-M3-2) | pending-human-review |
| 2026-06-13 | tests/ui/borrowck/ 新增 5 条 snapshot(RX4004 ×2 / RX4005 ×2 / RX4006 ×1) | M3.3 WP5 黄金路径 3 NLL 借用检查诊断(M3_PLAN §3 任务 5,RXS-0057/0058/0060/0061;用例移植自 WP4 已验证的 conformance/borrowck/reject 反例,既有 35 条 snapshot 零变化;黄金路径 3 计数 5→10 达成 G-M3-2 ≥10) | pending-human-review |
