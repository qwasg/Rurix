# G37 W1:g34 三 kernel fx/fy 双线性同源 bug 修复报告

日期:2026-08-29(day_0830_delivery 批次)。
授权:day_0828 HANDOVER §A.1 登记缺陷,当时战役纪律禁改他区域,现获主线授权修复。
缺陷:双线性采样**底行 G/B 通道误用 fy 做水平混合**(同源传播自 kernels/g31_texture_gi.rx
旧代码;g31 自有面已于 day_0828 Phase B 修复,修法 = 逐处 fy→fx,R 通道正确形为准)。

## 一、修改清单与前后表达式对照

修法与 g31 参考(kernels/g31_texture_gi.rx L254-262 已修形)逐字同式。
每处错误行:

```text
修前: let b0g = p01_g * (1.0 - fy) + p11_g * fy;   // 底行 G 误用 fy 做水平混合
修后: let b0g = p01_g * (1.0 - fx) + p11_g * fx;
修前: let b0b = p01_b * (1.0 - fy) + p11_b * fy;   // 底行 B 误用 fy 做水平混合
修后: let b0b = p01_b * (1.0 - fx) + p11_b * fx;
```

垂直混合(`samp_* = t0* × (1−fy) + b0* × fy`)与 R 通道原本正确,未动。

### kernel 侧(3 文件 × 2 处 = 6 处)

| 文件 | 修前行号 | 修后行号(+注释偏移) | 内容 |
|---|---|---|---|
| `src/rurix-render/kernels/g34_unified_gi.rx` | L288 / L291 | L290 / L293 | b0g / b0b fy→fx |
| `src/rurix-render/kernels/g34_unified_gi_skin.rx` | L274 / L277 | L276 / L279 | b0g / b0b fy→fx |
| `src/rurix-render/kernels/g34_unified_shade.rx` | L227 / L230 | L229 / L232 | b0g / b0b fy→fx |

g34_unified_primary.rx / g34_unified_mv.rx **无双线性采样**(全文无 fx/fy 混合表达式),
未改,对应 SPV 未动。

### host 镜像侧(1 文件 × 2 处)

| 文件 | 函数 | 修前行号 | 修后行号 | 内容 |
|---|---|---|---|---|
| `src/rurix-render/src/bin/g14_3_lane/g14_3_lane_body.rs` | `g31_tex_host_sample`(原形态,g34_full_lane 系消费) | L6019 / L6022 | L6022 / L6025 | b0g / b0b fy→fx |

风险登记落实:HANDOVER §A.1 警告"host 镜像同错致对拍恒绿假象"——本次 host+device
成对同步修,修后两侧为同一个正确双线性(位级同式),对拍面语义对齐。
heap/mip 形态 `g31_tex_host_sample_mip`、SVT 采样器、`g31_tex_host_sample_srgb`
腿均已在 day_0828 修过或本就正确,未触碰。全仓 grep 复查:`p01_[gb]*(1−fy)`
同款 bug 形态 0 残留。

修改处注释:每处已加「G37 W1:fx/fy 双线性同源 bug 修复(day_0828 HANDOVER §A.1)」。

## 二、SPV 重编(rurixc)与 sha256 前后

编译形状(与 ci/g34_*_smoke.py 一致):`target/debug/rurixc <src.rx> --target vulkan -o <dst.spv>`。
rurixc 以 `cargo build -p rurixc --features vulkan-backend --bin rurixc`(dev profile)现建。

**基线复编 sanity(位级全中)**:修复前源(备份于 `rx_pre_fix/`)复编产物与
`.tmp/g34_gates/` 现存部署 SPV **sha256 逐一位级一致**(见 `sanity_prefix_recompile/`)
⇒ 编译器确定性成立,下表 sha 变化**完全归因于本修复**。

| SPV 工件 | 修前 sha256 | 修后 sha256 | spirv-val |
|---|---|---|---|
| `.tmp/g34_gates/unified/g34_unified_gi.spv` | `BD72EA21B89FC5000CA6CD3AEB0CD9B75B5F2E264DFC91E98CEF4D7D653672EF` | `27A1FC7E120DF12847E8E756FD5798D11D2B1F90F0E7B710B184DDEE1A74ACB2` | 绿(exit=0) |
| `.tmp/g34_gates/unified/g34_unified_shade.spv` | `112C75FBCE2C74E4008C9E601020D6CA17C9F9428F73851C17C8E3DA7340EEC6` | `5445B9F6C253A8B8E9AF7304CC5FF04DAC50E8BB3093082901269267B6645D20` | 绿(exit=0) |
| `.tmp/g34_gates/hzb/g34_unified_shade.spv` | 同上(同源副本) | 同上(同源副本) | 绿(exit=0) |
| `.tmp/g34_gates/skin/g34_unified_gi_skin.spv` | `260E4C5D332A18E59993026FC53FE1821F0B855638E6FFADBB860386F984A1A6` | `CFB9EA731F7146D363D8CA982129F20996AE350EA65F08F310DB9E30366301EF` | 绿(exit=0) |

旧 SPV 备份:本目录 `spv_backup/*.spv.bak`(4 件,文件名前缀标注所属门目录)。
spirv-val 双重验证:rurixc 内嵌接受 + PATH 独立 `spirv-val`(vulkan-1.3.296.0)exit=0 ×4。
未动 SPV:`hzb/g34_unified_primary.spv`、`skin/g34_unified_mv.spv`(kernel 未改)。

## 三、cargo check

`cargo check -p rurix-render`(默认 dev profile、默认 target 目录):**绿,exit=0**。
编辑文件 0 linter 错误。未跑 release,未碰 target-night,未运行任何 GPU 程序。

## 四、需主 agent GPU 复跑的门清单

| 门脚本 | 消费的受影响面 |
|---|---|
| `ci/g34_unified_lane_smoke.py` | unified/gi + unified/shade SPV;host parity 臂经 `g31_tex_host_sample` |
| `ci/g34_hzb_unified_smoke.py` | hzb/shade SPV(primary 未动) |
| `ci/g34_skin_unified_smoke.py` | skin/gi_skin SPV(mv 未动) |

注:三脚本内部均 rurixc 现编 kernel,复跑即消费修复后源。

**预期影响预判**(供主 agent 复跑时对照):
- 缺省面(全特性关,纹理采样不触发)digest **应不变**——Stage A 锚
  (default_faces_bitexact_anchor)应保持绿。
- 纹理开臂的 render_digest **必然变化**(G/B 通道采样值修正)——若门内有
  纹理开臂金 digest 钉子,需按新正确值重锚。
- host parity(merged_semantics_host_parity)**应保持绿**:host/device 成对
  同步修,对拍两侧仍位级同式(且是正确式)。
- `evidence/g34_*.json` 历史工件内记录的旧 SPV sha 为历史事实,未改动。

## 五、遗留登记(纪律禁改区域,留主线处置)

1. `g14_3_lane_body.rs` L5957-5963:`g31_tex_host_sample` 的 **fn 文档注释与
   `#[allow(dead_code)]` 行注释已过时**——文档称"G/B 底行 fy 系数与(g34 冻结
   kernel)位级同式,不施加 heap 侧 fy→fx 修正",本修复后该表述失效;
   dead_code 行注释称"g31_window_present 独消费面"亦与 F6 后事实
   (g34_full_lane 系消费)不符。纪律限定只许改采样函数体内 fy→fx 表达式,
   故未动,登记待主线更新。
2. 本目录附:`rx_pre_fix/`(修复前 kernel 源 3 件)、`spv_backup/`(旧 SPV 4 件)、
   `sanity_prefix_recompile/`(基线复编 sanity 产物 3 件)。
