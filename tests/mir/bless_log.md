# MIR golden bless 审批记录(只追加)

> 任何 `tests/mir/**/*.mir` 的新增/修改/删除必须同 PR 在本表追加一行(14 §2 常驻集
> MIR 文本 golden;M3 CI_GATES §4 第 2 项,`ci/check_guardrails.py` `check_mir_bless`
> 机器核对:既有行 0-byte)。bless 纪律对齐 UI snapshot(`RURIX_BLESS=1` 重写 +
> 本表追加留痕)。

| 日期 | 范围 | 理由 | 批准 |
|---|---|---|---|
| 2026-06-13 | tests/mir/ 初始 3 条 golden(hello_world / drop_order / conditional_drop_flag) | M3.3 WP6 MIR 文本 golden guardrail 激活(M3_PLAN §3 任务 6;M3 CI_GATES §4 第 2 项)。三类形态代表:无 drop 回归网底线 / Move + drop 消去 + 无条件 Drop / 条件初始化 drop flag(SwitchBool 守卫 + flag set/clear)。基线 = 全管线 `mir::pretty` 文本逐字节 | pending-human-review |
