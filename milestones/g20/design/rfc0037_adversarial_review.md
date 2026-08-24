<!-- Assisted-by: Cursor Agent（G20.1 治理波） -->
# RFC-0037 对抗评审（D-4xx）

| Finding | Severity | Disposition |
|---|---|---|
| F1 host-only HZB 是否构成 M61 重判条件兑现 | high | accepted — 重判条件字面为「触发条件齐备」非「device 兑现」；HZB host 面 + cluster 差距闭集构成程序输入，M-c 裁决 maintain-no-go/go 均合法不预设 |
| F2 保守性判据可被弱化（只测遮挡命中不测假阳性） | high | accepted — 硬不变量收窄为「判 Occluded ⇒ 精确真值同判」全称断言 + 剔除率非零下界；确定性 rect 夹具 ≥400 例 |
| F3 mip 选级越界/非 2 幂边错误归约风险 | medium | accepted — ceil 减半 + clamp 复采边纹素保守方向不变；金字塔顶层 = 全图最远深度单测锚 |
| F4 cluster P4 差距闭集主观裁剪风险 | medium | accepted — 差距行闭集落 JSON 机核（M-b facts 计数）+ P2 穷举表逐行 disposition |
| F5 既有剔除链溅射 | high | accepted — hzb.rs 加性独立模块零接线；cull/visbuffer 0-byte 由 M-a facts git-diff 机核 |
