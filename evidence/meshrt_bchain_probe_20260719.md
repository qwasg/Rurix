# G3.6 mesh/RT B 链 probe——mesh 绿 / RT 红(RFC-0013 §4.E 条件分支实证)

| 项 | 值 |
|---|---|
| 类型 | B 链 probe 取证(RFC-0013 §4.E9 probe-first / §4.0-8 条件分支条款) |
| 承接 | G3.6 mesh-task-RT 面(RXS-0248/0249,验收门 G-G3-6) |
| 工具链 | glslang/spirv-val/spirv-cross = Vulkan SDK 1.3.296.0;dxc 同 SDK |
| 纪律 | measured-first;工件驻 `build/spike-sampling-probe/` 不入库(再生命令见下) |
| Provenance | `Assisted-by: claude-code:claude-fable-5` |

## 结果矩阵

| 腿 | 语料 | spirv-val vulkan1.2 | spirv-cross --hlsl | dxc | 判定 |
|---|---|---|---|---|---|
| **mesh** | GL_EXT_mesh_shader(triangles/max_v 3/max_p 1/SetMeshOutputsEXT/PrimitiveTriangleIndicesEXT) | OK | **OK**——产 `SetMeshOutputCounts` + `SV_Position`/indices 正确 HLSL 形态(--shader-model 65) | **ms_6_5 OK(3172B DXIL)** | **绿 → DXIL mesh 腿可全量落(步骤 68),免 RD-034 尾门** |
| **RT(raygen)** | GL_EXT_ray_tracing(accelerationStructureEXT+rayPayloadEXT+traceRayEXT+gl_LaunchIDEXT) | OK | **FAIL**——`SPIRV-Cross threw an exception: Unsupported builtin in HLSL: 5319`(5319 = LaunchIdKHR;RT builtin 族 HLSL 后端无翻译) | n/a | **红 → DXIL RT 腿上游 blocked,按 RFC §4.E 条件分支登 RD-034 尾门 + 步骤 69 blocked 探针(防静默腐烂:上游能力出现未跟进=红)** |

## 判定

RFC-0013 §4.E「mesh/task probe-first / RT 预判 blocked」的两个条件分支**均获确定性实证**:mesh 分支走「probe 绿」激活全量落地;RT DXIL 分支走「probe 红」——spirv-cross HLSL 后端对 SPV_KHR_ray_tracing 的 builtin/存储类无翻译路径(与 RD-015 A 路上游缺口叠加,DXIL RT 双路均 blocked),以本证据登 RD-034(DXIL RT 腿 blocked-on-upstream),Vulkan RT 主腿不受牵连。

## 复现

```
cd build/spike-sampling-probe
# mesh(绿): mesh.mesh(GL_EXT_mesh_shader 最小三角形)
glslang -V --target-env vulkan1.2 -S mesh mesh.mesh -o mesh.spv && spirv-val --target-env vulkan1.2 mesh.spv
spirv-cross --hlsl --shader-model 65 mesh.spv --output mesh.hlsl && dxc -T ms_6_5 -E main mesh.hlsl -Fo mesh.dxil
# RT(红): rg.rgen(GL_EXT_ray_tracing 最小 raygen)
glslang -V --target-env vulkan1.2 -S rgen rg.rgen -o rg.spv && spirv-val --target-env vulkan1.2 rg.spv
spirv-cross --hlsl --shader-model 63 rg.spv --output rg.hlsl   # → exception: Unsupported builtin in HLSL: 5319
```
