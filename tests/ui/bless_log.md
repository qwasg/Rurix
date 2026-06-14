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
| 2026-06-13 | tests/ui/consteval/ 新增 4 条 snapshot(RX5001 ×2 / RX5003 ×2) | M3.4 WP2 const eval 诊断首批(M3_PLAN §4 任务 2,RXS-0062/0063/0065;UI 通道阶段化接入 typeck 后 const 求值检查,既有 40 条 snapshot 零变化;5xxx 错误码 snapshot 入 UI 通道,G-M3-4) | pending-human-review |
| 2026-06-13 | tests/ui/consteval/ 4 条 snapshot(2026-06-13 行) | 人工终审批准 M3.4 const eval 首批 snapshot bless;用于 M3 close-out §8 留痕 | qwasg(会话授权,agent 代笔) |
| 2026-06-13 | tests/ui/coloring/ 新增 3 条 + tests/ui/addrspace/ 新增 1 条 snapshot(RX3001 ×2 / RX3003 ×1 / RX3002 ×1) | M4.1 黄金路径 4 的 3xxx 子集首批(M4_PLAN §1 任务 3/4,spec/device.md RXS-0066/0067/0068;UI 通道阶段化接入 typeck 后着色/barrier 骨架 + typeck 内地址空间一致性,既有 44 条 snapshot 零变化;6xxx codegen/ptxas 子集随 M4.2/M4.3 补足 G-M4-3 ≥10) | pending-human-review |
| 2026-06-13 | tests/ui/coloring/ + tests/ui/addrspace/ 4 条 snapshot(2026-06-13 行) | 人工终审批准 M4.1 黄金路径 4 的 3xxx 子集首批 snapshot bless;放行 M4.2(NVPTX codegen)开工;用于 M4 close-out §8 留痕 | qwasg(会话授权,agent 代笔) |
| 2026-06-13 | tests/ui/codegen/ 新增 3 条 snapshot(RX6001 ×1〔2 诊断〕 / RX6003 ×1 / RX6005 ×1) | M4.2 黄金路径 4 的 6xxx 子集首批(M4_PLAN §2 任务 3,spec/device.md RXS-0070/0071/0073;UI 通道阶段化接入 device codegen——`kernel fn` 为根的 NVPTX codegen,既有 48 条 snapshot 零变化)。RX6001 device 数组索引/数组表达式作用面外、RX6003 device codegen 不支持值类型(NVPTX 作用面外)、RX6005 host 地址空间 view 超出约束子集;`m4.counter.ui_golden_path4_snapshots` 现 7 条 < 10,余随 M4.3 补足 G-M4-3 ≥10 | pending-human-review |
| 2026-06-13 | tests/ui/launch/ 新增 4 条 snapshot(RX3004 ×1 / RX3005 ×1 / RX2001 ×1〔launch 参数复用〕 / RX3006 ×1) | M4.3 黄金路径 4 的 launch 子集(M4_PLAN §3 任务 5,spec/device.md RXS-0074/0075;UI 通道阶段化接入 typeck 后 launch 类型契约检查,既有 51 条 snapshot 零变化)。RX3004 launch 非 kernel 着色、RX3005 launch 维度不匹配、RX2001 launch 参数元素类型不符(复用)、RX3006 launch context-brand 不匹配;`m4.counter.ui_golden_path4_snapshots` 计数目录扩为 coloring+addrspace+codegen+launch,现 11 条 ≥10 达成 G-M4-3 | pending-human-review |
