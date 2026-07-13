# 路线 B —— raw-buffer 运行时等价探针实测结论（S1 报告 buffer 等价段补遗）

状态: **实测 PASS（真 GPU，零 patch，两 pass）**。本补遗补齐 `s1_pipeline_report.md` §2
「诚实边界」如实标注的缺口——raw-buffer 型容器（`StructuredBuffer<T>` →
`UNIFORM_TYPE_STORAGE_BUFFER`，驱动建 **RAW** 视图）的**运行时等价性**在 S2 中**未被
GPU 证明**（S2 只证纹理路径）。本探针在本机 RTX 4070 Ti 上以 local RD 真机 dispatch +
回读，逐 u32 word **零容差**对照 CPU reference，证明 RAW 视图与 offline 结构化视图运行时
等价——覆盖**两个不同 stride 敏感度的绑定形态**（cluster_store 与 instance_compaction/
scatter）。本段不含任何性能宣称，也不宣称 real_gpu_pass=true。

复现:

```
py -3 ci/grx_rd_buffer_probe_smoke.py
```

产线: `probe_project/rd_buffer_probe.gd`（新增，`res://buffer_probe.tscn` 场景，走
positional-scene override，与 S2 纹理探针 `rd_native_probe.gd` 并存互不影响；manifest 对
绑定形态泛型——`inputs[]` + 单 `output`，同一脚本覆盖 3 绑定与 5 绑定 pass）+
`ci/grx_rd_buffer_probe_smoke.py`（驱动 + CPU reference + fail-closed 对照 + evidence）。
CPU reference 直接 import 各 pass 的 `generate_math_parity_evidence.py`，与 offline
dispatch smoke（`ci/grx014_cluster_store_d3d12_dispatch_smoke.py` /
`ci/grx016_instance_compaction_d3d12_dispatch_smoke.py`）**同一口径、同一 fixture**（scatter
5 case 的 dst sha256 与 tracked `math_parity_evidence.json` 逐 case 核对一致）。

---

## 1. 被测的语义分歧（要证的疑问）

| 侧 | 视图创建 | Format | StructureByteStride | Flags |
|---|---|---|---|---|
| **offline harness**（结构化） | `Create{Shader,Unordered}...View` 逐槽 | `DXGI_FORMAT_UNKNOWN` | 真实元素 stride（4 / 80 / 48） | `_FLAG_NONE`（structured） |
| **Godot 驱动**（RAW） | `uniform_set_create` 按反射 `UNIFORM_TYPE_STORAGE_BUFFER` | `DXGI_FORMAT_R32_TYPELESS` | **0** | `D3D12_BUFFER_{SRV,UAV}_FLAG_RAW` |

驱动侧证据: `external/godot-master/drivers/d3d12/rendering_device_driver_d3d12.cpp:3547-3563`
——`UNIFORM_TYPE_STORAGE_BUFFER` 对 writable 反射建 UAV、否则建 SRV，两者恒为
`R32_TYPELESS` + `StructureByteStride=0` + `FLAG_RAW`，**不看** HLSL 侧声明的 stride。

疑问核心: DXC 编译的 `StructuredBuffer<T>` DXIL 访问 `buf[index].field` 若依赖
**描述符提供的 stride**（`byteAddr = index*descriptorStride + fieldOffset`），则 RAW 视图
stride=0 会令一切 `index` 退化到元素 0 → 全错；若 DXC 把 stride **烘进 DXIL**
（rawBufferLoad 式 `index*stride` 在着色器内算好），则 RAW 视图运行时等价。**这只能实测。**
关键: cluster_store 的 `render_elements`（`StructuredBuffer<RenderElementData>`，stride **80**）
与 scatter 的 `src/dst_transforms`（stride **48**）都是**非 4 字节** stride——若语义依赖描述符
stride，这两个绑定必错；stride-4 的 uint word 缓冲即使依赖描述符也会「偶然」对（index*4 恒等），
故本探针刻意以 stride-80 / stride-48 缓冲证伪该退化路径。

## 2. 逐 case 实测结果（真 GPU，零容差）

真机: local RD（`create_local_rendering_device`）on **NVIDIA GeForce RTX 4070 Ti**，
`--rendering-driver d3d12 --rendering-method forward_plus`，容器经
`shader_create_from_bytecode` → `compute_pipeline_create` → 逐绑定
`storage_buffer_create` → `uniform_set_create`（全 uniform `UNIFORM_TYPE_STORAGE_BUFFER`，
UAV/SRV 由容器 per-binding writable 反射自动裁定）→ dispatch → `buffer_get_data`。
输出 dst 显式零上传（对齐 native buffer_clear）。

### 2a. cluster_store（2 StructuredBuffer SRV + 1 RWStructuredBuffer UAV；stride 敏感槽=render_elements 80B）

| case | cluster 网格 | render_element_count | dispatch | dst words | 非零参考 words | 结果 |
|---|---|---|---|---|---|---|
| store_pack_grid_4x3_e64 | 4×3 | 40 | 1×1×1 | 1632 | 1086 | ✅ 0 mismatch |
| store_pack_grid_2x2_minmax_merge | 2×2 | 16 | 1×1×1 | 528 | 84 | ✅ 0 mismatch |
| store_pack_grid_3x1_touch_overrides | 3×1 | 12 | 1×1×1 | 396 | 132 | ✅ 0 mismatch |

### 2b. instance_compaction/scatter（4 StructuredBuffer SRV + 1 RWStructuredBuffer UAV；stride 敏感槽=transforms 48B）

scatter 是 3-kernel 前缀和链的 D3 段；本探针**单独跑 scatter**，把 CPU 算好的中间量
（`local_prefix` t2、`group_offsets` t3）作为输入喂入，故只测 scatter kernel 自身的缓冲绑定
等价。scatter 是**保位比特搬运**（无算术），dst float 为源的逐字节拷贝，按 u32 word 比对即
比特精确。

| case | total_instances | survivor | dispatch | dst words | 非零参考 words | 结果 |
|---|---|---|---|---|---|---|
| sparse_survival_multi_group | 600 | 150 | 3×1×1 | 7200 | 1800 | ✅ 0 mismatch |
| all_survive | 513 | 513 | 3×1×1 | 6156 | 6155 | ✅ 0 mismatch |
| zero_survive | 384 | 0 | 2×1×1 | 4608 | 0 | ✅ 0 mismatch（dst 全零，证 RAW UAV 无杂散写） |
| mask_tail_garbage_bits_ignored | 70 | 14 | 1×1×1 | 840 | 168 | ✅ 0 mismatch |
| single_survivor_last_instance_empty_leading_group | 300 | 1 | 2×1×1 | 3600 | 12 | ✅ 0 mismatch |

两 pass 合计 **8 case 全部 0 mismatch**。回读 sha256 逐 case 记于 `rd_buffer_probe_evidence.json`。

## 3. RAW vs structured 等价结论

**证明等价（proven_equivalent）**。传递链（两 pass 各自成立）:

- 本探针（**RAW** 视图，Godot 驱动）: GPU 回读 == CPU reference，零容差，8 case 全绿。
- offline dispatch smoke（**结构化** 视图，手搭 D3D12 harness）: `cpu_reference_match=true`、
  `real_d3d12_dispatch_recorded=true`（cluster_store=`passes/cluster_store/real_d3d12_dispatch_smoke.json`；
  scatter=`passes/instance_compaction/real_d3d12_dispatch_smoke.json`），**同一 fixture、同一 CPU
  reference**，零容差。

∴ RAW 视图输出 == 结构化视图输出 == CPU reference。DXC 的 `StructuredBuffer` DXIL **把 stride
烘进了着色器**（不依赖描述符 stride）——由 stride-80（render_elements）与 stride-48（transforms）
两个非-4-字节 stride 绑定坐实，非 stride-4 缓冲的偶然对齐。这也解释了为何 Godot 自有 SSBO 着色器
（NIR→DXIL，字节寻址）与 DXC `StructuredBuffer` 在同一 RAW 绑定下都正确。

## 4. 对 buffer 型 pass 走路线 B 的判定

| pass / kernel | 绑定形态 | 判定 | 依据 |
|---|---|---|---|
| **cluster_store** | 2 StructuredBuffer SRV + 1 RWStructuredBuffer UAV（stride 80/4） | ✅ **路线 B 可行（已真机证）** | 本探针 §2a 零容差三 case |
| **instance_compaction/scatter** | 4 StructuredBuffer SRV + 1 RWStructuredBuffer UAV（stride 48/4） | ✅ **路线 B 可行（已真机证）** | 本探针 §2b 零容差五 case |
| instance_compaction/scan_local, scan_groups | 3 绑定（1 SRV + 2 UAV，stride 4） | ✅ 路线 B 可行（机理已证 + stride-4 无风险） | 与 scatter 同 pass 同机理；纯 u32 word（stride 4，RAW 与结构化恒等）；容器结构有效（S1 §1 绿） |
| **particles_copy** | raw-buffer 型（b0=128B==上限） | ✅ 路线 B 可行（机理已证；建议补 case 探针坐实） | 同为 `StructuredBuffer`/`RWStructuredBuffer`→STORAGE_BUFFER RAW 绑定；容器结构有效（S1 §1 绿）。stride 烘进为 DXC 通性（本探针在 stride-80/48 上证伪退化），非本 kernel 偶然 |

**核心结论**: raw-buffer 型 pass 与纹理型 pass 一样，可经**零 patch** 的 local-RD 路线走路线 B。
RAW-vs-structured 语义分歧在本机真 GPU 上被证明**无运行时差异**——且已在两个非-4-字节 stride 的
绑定形态（cluster_store 80B struct、scatter 48B struct）上坐实，排除「stride-4 偶然对齐」。这是决定
buffer 型 pass 能否走路线 B 的关键证据，判定为**可行**。cluster / compaction 两族已**逐 pass 真机
证明**（cluster_store + scatter，且 scan_local/scan_groups 与 scatter 同机理、stride-4 无风险）；
particles_copy 的等价机理与之同构，本探针已把「机理通性」升格为跨两族两 stride 的实测，建议后续为
particles_copy 补一 case 探针，与 S2 纹理路径的逐 pass 证据完全对齐。

## 5. 不蕴含

本探针**不**宣称: same-frame injection（S3+，须 patch）、default pass enablement、
real_gpu_pass=true、Godot runtime pass 完成、visual diff、GPU timestamp/性能。
runtime 仍 fallback_only / 默认 disabled。fail-closed: 容器/输入加载失败、size 不符、
uniform_set 拒绝、任一 word 不匹配 → 如实 `status=fail` 附首个 mismatch word 定位（gpu vs cpu
hex），已对 `compare_words` 做注入负测坐实（干净 match / 篡改 word 定位 / 短回读 size 拦截 三向）。
