# G3.4 bindless B 链 probe——unbounded array + NonUniform 动态索引实测(绿)

| 项 | 值 |
|---|---|
| 类型 | B 链 probe 取证(RFC-0013 §4.C3 / §4.0-8 条件分支条款;probe 绿 → DXIL 腿激活,无 RD-034+ 尾门) |
| 承接 | G3.4 bindless 面(RXS-0234,验收门 G-G3-4) |
| 探测对象 | `OpTypeRuntimeArray`(unbounded `Texture2D[]`)+ `RuntimeDescriptorArray` + `NonUniform` 动态索引经 spirv-cross→dxc 的 B 链可译性;确认 **SM6.0 即可(不需 SM6.6 dynamic resources)** |
| 工具链 | glslang/spirv-val/spirv-cross = Vulkan SDK 1.3.296.0;dxc 同 SDK |
| 语料 | GL_EXT_nonuniform_qualifier bindless fragment(`tbl[nonuniformEXT(idx)]`,set4 无界表 + set3 sampler,与 §4.C2 独占 set4 分配律同构),glslang -V --target-env vulkan1.2 |
| 纪律 | measured-first;工件驻 `build/spike-sampling-probe/` 不入库(再生命令见下) |
| Provenance | `Assisted-by: claude-code:claude-fable-5` |

## 结果

| 阶段 | 结果 |
|---|---|
| spirv-val --target-env vulkan1.2 | **OK**(RuntimeDescriptorArray+ShaderNonUniform capability 合规) |
| spirv-cross --hlsl --shader-model 60 | **OK**——产 `Texture2D<float4> tbl[] : register(t0, space4)` + `tbl[NonUniformResourceIndex(idx)].Sample(smp, uv)`(恰为 RXS-0234 预期 HLSL 形态;无界数组自 space4 独占,与 SPIR-V set4 装饰一致) |
| dxc -T ps_6_0 | **OK**(3968 B DXIL;**SM6.0 unbounded array + 动态索引即可,证 §4.C3 「不需 SM6.6 dynamic resources」的文献推断成立**) |

## 判定

RXS-0234 DXIL 腿(B 链)**可译性实证成立**,条件分支条款按「probe 绿」激活,无需登 RD-034+ 尾门。`NonUniformResourceIndex` HLSL intrinsic 正确承载 SPIR-V `NonUniform` 装饰;unbounded `register(t0, spaceN)` 与 RTS0 unbounded range(NumDescriptors=0xFFFFFFFF)经 RXS-0166 同构一致性门可交叉核验。SM6.0 目标确认(非 SM6.6),与 D-131 图形=B 链现役工具链兼容。

## 复现

```
cd build/spike-sampling-probe   # bl.frag(GL_EXT_nonuniform_qualifier bindless fs)
glslang -V --target-env vulkan1.2 -S frag bl.frag -o bl.spv && spirv-val --target-env vulkan1.2 bl.spv
spirv-cross --hlsl --shader-model 60 bl.spv --output bl.hlsl && dxc -T ps_6_0 -E main bl.hlsl -Fo bl.dxil
```
