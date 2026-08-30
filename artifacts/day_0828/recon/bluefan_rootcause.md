# 蓝色吊扇根因报告（day_0828 recon）

**症状**：窗口车道 presented 输出中吊扇（mat 40 `Paris_CeilingFan`，契约暖橙 Le）呈饱和蓝
(R0,G62,B170)；bench 车道 EXR 同像素 = Le×2⁴ 暖橙正确。

**根因（已钉死）**：`kernels/g31_display_encode.rx` 的 ACES 1.3 分段样条（c5 RRT + c9 ODT）
基函数手工展开**转置错误**——写成列向量形 `M·cf`：

```
let b1 = 0.0 - cf0 + cf1 + 0.5 * cf2;   // 错：混入 0.5·cf2
let b2 = 0.5 * cf0;                      // 错：缺 0.5·cf1
```

单源 host 参考（`display/aces13.rs` `vmul(cf, &SPLINE_M)`，CTL `mult_f3_f33` 行向量约定
`out[i]=Σ_j v[j]·m[j][i]`）应为 `b1 = cf1 − cf0`、`b2 = 0.5·(cf0+cf1)`。`b0` 两约定恰好
同值，故结构审读难以察觉。全 12 处样条块同错（c5/c9 × RGB × low/high 段）：
L213-214 / 224-225 / 244-245 / 255-256 / 275-276 / 286-287 / 321-322 / 332-333 /
352-353 / 363-364 / 383-384 / 394-395。

**机理**：错基破坏样条节点连续性 ⇒ 逐通道色调曲线非单调 + 段间跳变。中亮饱和色三通道落
不同段 ⇒ 色相反转（扇叶 AP1 R≈0.31 → 显示 0；B≈0.11 → 0.38，暖橙→饱和蓝）；深暗部同段
⇒ 色相保持但系统性提亮（墙 25 vs 修正后 0）——两车道墙面"色相一致"的既有观测由此成立；
高亮走线性延伸支 ⇒ 灯具正常 clip 白。故全画面只有吊扇"显眼地"错。bug 臂灯罩渐变的同心
环带（见对照图左半）是非单调曲线的另一直接可见证据。

## 证据链（像素对账）

| 层位 | fan(1500,12) 值 | 结论 |
|---|---|---|
| bench EXR（g18 kernel, TSR 输出） | (0.3569, 0.2537, 0.1063) = Le×2⁴ | 正确 |
| 窗口 TSR 输出 f32（`RURIX_G31_DUMP_F32`，本日短跑，digest==锚 5596a730） | (0.35686296, 0.25368458, 0.10629952) | **与 bench 一致 ⇒ encode 上游全对** |
| 窗口 presented（同帧 raw dump，receipt bgra8_unorm） | (R0, G62, B170) | 真蓝，非工具误读 |
| kernel-as-written f32 仿真（转置基） | (0, 62, 170)；wall (25,13,16)；**全帧 99.9918% 位级 == 实测** | 复现 ⇒ 根因即此 |
| 改正基仿真 == host aces13 f64 | (144, 122, 77) 暖米黄；0.18 灰→99（ACES 0.104 设计点） | 最小修复有效 |

排除项：dump/PNG 工具链（receipt 通道序 + PIL BGRA 模式正确）；scene kernel 差异（两 kernel
仅 miss 天光项差、契约 sky=0 恒等；rurixc 现编 sha256 == 盘上 SPV == 双车道 receipt）；场景
数据（mats 装配共享 `g14_3_lane_body.rs`）；TSR（双车道同 SPV + 输出实测一致）。

## 修复建议（未施工：src 只读红线 + 冻结面破锚治理）

1. 上述 24 行两式改写（12× b1 删 `+ 0.5 * cf2`；12× b2 加 `+ 0.5 * cf1`）。
2. 重编 `g31_display_encode.spv`；窗口车道全部 presented digest 锚重定基
   （5596a730 / b02b08b57 / 12d5dc91 / 48353e86 / 2b6efac6 / soak / Stage A 窗口格）；
   render_digest 面（TSR 输出锚 f39e9808 / cde1b255 / c1d28ad7）不受影响。
3. 防复发：补 device-vs-host encode parity 门（对拍 `display::aces13` ±1 LSB，或最低限度
   0.18 灰 → 99±1 显示地标探针）——A3 收口只验了确定性 digest，未验 host parity。

## 遗留

- 全帧仿真残差 169 px（≤26 LSB，样条段界 ULP 放大），不影响结论。
- `presentation_night.png` zlib 流损坏（--export-png 写出面独立缺陷，另立）。
- 夜巡 P3 色带的窗口车道观测可能混入本缺陷贡献（跳变曲线自制色带），修复后建议复测。

证物：`bluefan_probe*.{py,json}`、`bluefan_encode_sim.py`、`bluefan_sim_report.json`、
`bluefan_fullframe_verify.py`、`bluefan/`（raw + f32 dump + `fan_bug_vs_fixed.png` 对照图）、
`spv_check/`。
