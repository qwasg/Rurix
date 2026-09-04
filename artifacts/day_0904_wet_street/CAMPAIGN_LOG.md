# day_0904_wet_street 战役日志(G42 湿街展示;不 commit)

> 入役 = 四层入库波之后的工作树。本役**不 commit**,入库归 owner。
> 路线已裁决:`--wet on` 换载 fork 内核 `kernels/g42_direct_gi_wet.rx`;`--wet off`
> 仍装冻结 `g14_3_direct_gi.spv` ⇒ presented 位级等于雨夜 C2/C1 锚。

## W0 侦察(2026-09-04)

两路事实源已落 [`recon/R0.md`](recon/R0.md):

1. **车道插入点**:`g35_particle_lane.rs` 解析臂 / 闭集裁决 / `pack_frame_params`
   返回后覆写 reserved 段 / `UnifiedLaneBits::load` 前换载 `--spv-scene`。
   共享体 `g14_3_lane_body.rs` **0-byte**。参数段取 `[49..56)`(母版头注
   `[42..48)` 已被扩面链与 `RURIX_G18_SKY_INTENSITY` 占用,撞位风险真实)。
2. **fork 内核骨架**:复制冻结件 9 参数签名;湿门 = 地面带 Y∈[0.228,0.54] ×
   法线朝上 smoothstep;反照率 ×`wet_dark`;点灯循环内联 GGX+Schlick
   (`g31_realism.rx` L920–934 逐字);XZ hash 积水;1 条顶层 TLAS 反射射线
   (计数门 `while proceed()`,禁 `&&`-in-while,禁辅助 `fn`)。

预落盘(入库波未动这些文件,待 commit 4 落地后再接线):

| 件 | 状态 |
|---|---|
| `src/rurix-render/src/world/wet_ground.rs` | 已落;20 单测;公式面与 kernel 逐字同源 |
| `src/rurix-render/kernels/g42_direct_gi_wet.rx` | 已落;rurixc 现编 + spirv-val 绿 |
| `ci/g42_wet_street_smoke.py` | 已落;门键 `g42.wet.street` 八 facts |
| `src/rurix-render/src/world/mod.rs` | **未动**(commit 2/3 切分面,待入库波收口) |
| `src/rurix-render/src/bin/g35_particle_lane.rs` | **未动湿旗标**(待 commit 4 雨夜面落地) |

## 换机中断(2026-09-04 11:14)

owner 要求停做并推 GitHub。W1 接线未开始(`mod.rs` / `g35_particle_lane.rs` 湿旗标 0-byte)。
接手步骤见 [`HANDOFF.md`](HANDOFF.md)。

## W1 / W2 实施

〔待回填〕

## W3 / W4 验收与出图

〔待回填〕

## W5 收役

〔待回填〕
