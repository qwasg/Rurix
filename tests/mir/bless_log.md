# MIR golden bless 审批记录(只追加)

> 任何 `tests/mir/**/*.mir` 的新增/修改/删除必须同 PR 在本表追加一行(14 §2 常驻集
> MIR 文本 golden;M3 CI_GATES §4 第 2 项,`ci/check_guardrails.py` `check_mir_bless`
> 机器核对:既有行 0-byte)。bless 纪律对齐 UI snapshot(`RURIX_BLESS=1` 重写 +
> 本表追加留痕)。

| 日期 | 范围 | 理由 | 批准 |
|---|---|---|---|
| 2026-06-13 | tests/mir/ 初始 3 条 golden(hello_world / drop_order / conditional_drop_flag) | M3.3 WP6 MIR 文本 golden guardrail 激活(M3_PLAN §3 任务 6;M3 CI_GATES §4 第 2 项)。三类形态代表:无 drop 回归网底线 / Move + drop 消去 + 无条件 Drop / 条件初始化 drop flag(SwitchBool 守卫 + flag set/clear)。基线 = 全管线 `mir::pretty` 文本逐字节 | pending-human-review |
| 2026-06-13 | tests/mir/ 初始 3 条 golden(2026-06-13 行) | 人工终审批准 M3.3 WP6 MIR 文本 golden 首批 bless;基线经审阅(无 drop / drop 顺序 + move 消去 / 条件初始化 drop flag SwitchBool 守卫)与 `rurixc --emit=mir` 逐字节同源,红绿验证通过;用于 M3 close-out §8 留痕 | qwasg(会话授权,agent 代笔) |
