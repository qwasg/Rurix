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
