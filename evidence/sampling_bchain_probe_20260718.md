# G3.3 采样超集 B 链 probe——SampleCmp / Gather 子模式实测(绿)

| 项 | 值 |
|---|---|
| 类型 | B 链 probe 取证(RFC-0013 §4.B6 L5 / §4.0-8 条件分支条款;probe 绿 → 子模式激活,无 RD-034+ 尾门) |
| 承接 | G3.3 采样超集面(RXS-0226,验收门 G-G3-3) |
| 探测对象 | 分离 image/sampler 形态的 `OpImageSampleDrefExplicitLod`(sample_cmp)与 `OpImageGather`(gather)经 spirv-cross→dxc 的 B 链可译性 |
| 工具链 | spirv-as/spirv-val/spirv-cross = Vulkan SDK 1.3.296.0;dxc 同 SDK(`-T ps_6_0`) |
| 语料形态 | 手写最小 SPIR-V asm(spv1.0,分离 OpTypeImage+OpTypeSampler → OpSampledImage,与 rurixc emit 同构形态;cmp 语料 image depth=1) |
| 纪律 | measured-first;工件驻 `build/spike-sampling-probe/` 不入库(再生命令见下) |
| Provenance | `Assisted-by: claude-code:claude-fable-5` |

## 结果矩阵

| 子模式 | spirv-val vulkan1.0 | spirv-cross --hlsl --shader-model 60 | dxc -T ps_6_0 |
|---|---|---|---|
| sample_cmp(`OpImageSampleDrefExplicitLod`+Lod 0) | **OK** | **OK**——产 `SamplerComparisonState _5 : register(s1)` + `_4.SampleCmpLevelZero(_5, uv, 0.5f)`(恰为 RXS-0226 表预期的 HLSL 形态) | **OK**(3688 B DXIL) |
| gather(`OpImageGather` component=0) | **OK** | **OK**——产 `SamplerState` + `_4.GatherRed(_5, uv)`(RXS-0226 表预期) | **OK**(3676 B DXIL) |

分离 image/sampler → `register(t0)`/`register(s1)` 映射正确;比较采样器类型正确降级为 `SamplerComparisonState`。

## 判定

RXS-0226 L5 的两个 probe-gated 子模式(sample_cmp/gather)**B 链可译性实证成立**,条件分支条款按「probe 绿」分支激活,无需登 RD-034+ 尾门。诚实边界:本 probe 语料为与 rurixc emit 同构形态的手写最小 asm(rurixc 自产 `.spv` 的 spirv-val 已由步骤 62 host 段覆盖);rurixc 自产语料的端到端 B 链见证归步骤 62/63 device 轮(同工具链同机)。

## 复现

```
cd build/spike-sampling-probe   # 语料 cmp.spvasm / gather.spvasm(工件区,再生即写)
spirv-as --target-env spv1.0 cmp.spvasm -o cmp.spv && spirv-val --target-env vulkan1.0 cmp.spv
spirv-cross --hlsl --shader-model 60 cmp.spv --output cmp.hlsl && dxc -T ps_6_0 -E main cmp.hlsl -Fo cmp.dxil
# gather 同构;坑:spirv-as 缺 --target-env 默认产 spv1.6,spirv-val vulkan1.0 会拒
```
