# 路线 B S1 —— RD 原生容器生成器产线化报告

状态: spike 产物（host-only 结构级，无 GPU）。本报告记录把
`generate_rd_container.py` 从 tonemap fixture 泛化为按 pass 清单批量产 RD 原生
`RenderingShaderContainerD3D12` 容器的结果。所有断言均为字节/结构级；容器的**运行时
消费**（真机 dispatch/readback）由 S2 探针 `ci/grx_rd_native_probe_smoke.py` 单独证明
（tonemap 纹理路径 ~1 ULP，见 `route_b_plan.md` §5 / `rd_native_probe_evidence.json`），
**不在本段范围**。本报告不含任何性能宣称。

复现:

```
py -3 spike/godot-rurix/rd-native-pipeline/generate_rd_container.py --all
py -3 ci/grx_rd_container_smoke.py
```

产线: `generate_rd_container.py`（批量产容器，内建 `PASS_REGISTRY`）+
`verify_container.py`（独立结构自检，暴露 `verify_container_file()` 供批量调用）+
`ci/grx_rd_container_smoke.py`（fail-closed 漂移门，pin 每 kernel 期望结果）。

---

## 1. 逐 pass 容器产出状态表

批量对 10 个 pass（含 2 个多 kernel pass → 共 13 个 kernel 单元）产出。每个可产出
kernel 的容器写到 `rd-native-pipeline/out/<stem>.rd_container.bin` + `.report.json`。

| pass / kernel | 结果 | 容器字节 | b0(推常量) | 描述符数 | local_size | verify | 说明 |
|---|---|---|---|---|---|---|---|
| tonemap | ✅ container | 4836 | 28B | 2 (t0,u0) | 8×8×1 | 49/49 | 纹理型；与既有 fixture **字节完全一致**（回归基线 `ef8668e7…`） |
| taa_resolve | ✅ container | 13512 | 28B | 6 (t0..t4,u0) | 8×8×1 | 69/69 | 纹理型；5 SRV 折叠为 1 range `SRV x5` |
| ssao_blur | ✅ container | 5072 | 28B | 2 (t0,u0) | — | 49/49 | 纹理型 |
| particles_copy | ✅ container | 6564 | **128B** | 2 (t0,u0) | — | 49/49 | raw-buffer 型；b0 恰 128B（==上限，允许） |
| luminance_reduction | ✅ container | 4436 | 28B | 2 (t0,u0) | — | 49/49 | 纹理型；layout 无 `pass_id`（module=lib_texture），按 registry 显式 pass_id 处理 |
| cluster_store | ✅ container | 5728 | 32B | 3 (t0,t1,u0) | — | 54/54 | raw-buffer 型；2 SRV 折叠为 1 range `SRV x2` |
| **gpu_culling** | ⛔ **fail-closed** | — | 144B | (t0,u0,u1) | — | — | **`push_constant_too_large`**：144B > 128B MAX_PUSH_CONSTANT_SIZE，须迁 b0 到 CBV |
| fused_post_chain | ✅ container | 6720 | 64B | 5 (t0..t2,u0,u1) | — | 64/64 | 纹理型；3 SRV + 2 UAV |
| instance_compaction/scan_local | ✅ container | 5104 | 32B | 3 (t0,u0,u1) | — | 54/54 | raw-buffer 型；变体 kernel（resources 取自 `variants[]`） |
| instance_compaction/scan_groups | ✅ container | 4932 | 32B | 3 (t0,u0,u1) | — | 54/54 | 同上，共享 b0 |
| instance_compaction/scatter | ✅ container | 5136 | 32B | 5 (t0..t3,u0) | — | 64/64 | 同上；4 SRV 折叠为 1 range `SRV x4` |
| **indirect_args/write** | ⛔ **fail-closed** | — | 176B | (t0,u0,u1) | — | — | **`push_constant_too_large`**：176B > 128B，须迁 b0 到 CBV |
| **indirect_args/validate** | ⛔ **fail-closed** | — | 176B | (t0,u0,u1) | — | — | 同上（write/validate 共享一份 RTS0 与 b0；两 kernel 均 176B 被拦） |

汇总: **10 容器产出 + 全部结构自检绿（合计 555 断言）**，**3 fail-closed（全部
`push_constant_too_large`）**。CI 冒烟 `status=pass`（13/13 kernel 匹配 pin 期望）。

---

## 2. binding_kind → RD 反射映射核对表

生成器 `BINDING_KIND_MAP` 把 descriptor_layout.json 的每个资源 `binding_kind` 映射到
RD `UNIFORM_TYPE_*` + writable + 期望 RTS0 range 类型 + D3D12 `RES_CLASS_*`。每个
kernel 的 t#/u# 逐槽与 RTS0 展开后的 range 配对（含折叠 range 的 register 展开）。

| binding_kind | UNIFORM_TYPE | writable | RTS0 range | RES_CLASS | 本批出现的 pass |
|---|---|---|---|---|---|
| `texture2d` | TEXTURE (2) | 0 | SRV (0) | SRV (2) | tonemap, taa_resolve, ssao_blur, luminance_reduction, fused_post_chain |
| `rwtexture2d` | IMAGE (3) | 1 | UAV (1) | UAV (3) | tonemap, taa_resolve, ssao_blur, luminance_reduction, fused_post_chain |
| `structured_buffer` | STORAGE_BUFFER (8) | 0 | SRV (0) | SRV (2) | particles_copy, cluster_store, instance_compaction, gpu_culling*, indirect_args* |
| `rwstructured_buffer` | STORAGE_BUFFER (8) | 1 | UAV (1) | UAV (3) | particles_copy, cluster_store, instance_compaction, gpu_culling*, indirect_args* |
| `byteaddressbuffer` | STORAGE_BUFFER (8) | 0 | SRV (0) | SRV (2) | （GRX-009 tracked-3a 型；本批无 pass 直接用，映射保留） |
| `rwbyteaddressbuffer` | STORAGE_BUFFER (8) | 1 | UAV (1) | UAV (3) | 同上 |
| `cbuffer` | UNIFORM_BUFFER (7) | 0 | CBV (2) | CBV (1) | （占位，供未来 >128B b0 迁 CBV；本批无 pass 用） |

`*` = gpu_culling / indirect_args 的资源 binding_kind 本身受支持，但整 pass 在 b0>128B
处 fail-closed，未走到逐槽映射。

**逐槽覆盖度核对**（生成器 `build_reflection` + 独立 `verify_container.py` 双侧）:
每个 RTS0 descriptor table 的 range 按 `NumDescriptors` **逐 register 展开**（emitter 把
连续同类 register 折叠成单 range，如 `SRV x5`=t0..t4、`SRV x2`=t0,t1、`SRV x4`=t0..t3、
`UAV x2`=u0,u1），展开后每个 (class, register, space) 恰配一个 layout 资源，无遗漏/无
多余；binding 序 == 展开槽序 == `resource_descriptor_offset`（APPEND 展开后 0..N-1）。

**诚实边界（raw-buffer 型的 RAW-vs-structured 语义）**: 容器本身**只**编码
`UNIFORM_TYPE_STORAGE_BUFFER` + `res_class`，**不含** StructureByteStride/RAW 标志——视图
类型由驱动在 `uniform_set_create` 时按反射 UNIFORM_TYPE 决定（STORAGE_BUFFER →
R32_TYPELESS RAW 视图，`rendering_device_driver_d3d12.cpp:3544-3565`）。因此 HLSL 侧
`StructuredBuffer<T>` 与 `ByteAddressBuffer` 两种写法在**容器结构层**同构，均映射到
STORAGE_BUFFER。但「RAW 视图 vs structured 视图」的**真机执行等价性**尚未被证明——S2 只
证明了 tonemap 纹理路径。raw-buffer 型 pass（particles_copy/cluster_store/
instance_compaction）的容器**结构上有效**，其 GPU 语义正确性是 S2 等价探针的**待办**，本
段不宣称。

---

## 3. 现可路线 B 的 pass 清单

条件（route_b_plan R4/R7 收敛的本 spike 可表示布局）: **sampler-free + 单 set + 单
descriptor table + b0 ≤ 128B + 无 root descriptor / 无静态采样器 / 无多表 / 无描述符
数组**。满足并已产出结构有效容器 + 全绿自检的 kernel（10 个）:

- **纹理型（S2 路径已真机证明同构）**: `tonemap`、`ssao_blur`、`taa_resolve`、
  `fused_post_chain`、`luminance_reduction`。
- **raw-buffer 型（容器结构有效；RAW-vs-structured 真机等价待 S2 等价探针）**:
  `particles_copy`、`cluster_store`、`instance_compaction`(scan_local / scan_groups /
  scatter 三变体)。

全部 10 个 kernel: RTS0 v1.0、0 静态采样器、param[0]=32-bit root constants @ b0/space0
（**R1 硬断言恒成立**）、param[1]=单 descriptor table、全部 space0、DXIL 内无内嵌 RTS0。

---

## 4. Fail-closed 清单 + 原因（待扩）

| kernel | 类别 | 原因 | 扩展路径 |
|---|---|---|---|
| gpu_culling | `push_constant_too_large` | b0=144B(36 dword) > 128B | 迁 b0 到 CBV：layout 加 `cbuffer` binding + RTS0 emitter 改用 root/table CBV；`cbuffer` 映射已在 `BINDING_KIND_MAP` 预留。届时把 CI `EXPECT['gpu_culling']` 翻为 `("container", None)` |
| indirect_args/write | `push_constant_too_large` | b0=176B(44 dword) > 128B | 同上（write/validate 共享 b0，一次迁移覆盖两 kernel） |
| indirect_args/validate | `push_constant_too_large` | b0=176B(44 dword) > 128B | 同上 |

**fail-closed 契约**: 生成器对不支持布局抛结构化 `FailClosed(category, reason)` 而非静默
丢弃/强塞。除本批命中的 `push_constant_too_large` 外，以下类别已由 spike 负测覆盖并会同样
fail-closed（route_b_plan R1/R4/R7）: `static_samplers`、`root_descriptor`、`multi_table`、
`sampler_range`、`unsupported_binding_kind`、`descriptor_array`、`param0_not_32bit_constants`、
`not_compute`、`missing_input`。CI 冒烟把每 kernel pin 到期望结果（container / fail_closed +
类别），**任一漂移即 FAIL**（route-B pass 停产、不支持 pass 静默产出、类别不符、自检失败、
kernel 缺失/多余、report 混入 CR 字节）——已双向验证（注入漂移 exit 1，干净 exit 0）。

---

## 5. 生成器泛化要点（相对 tonemap 原型的改动）

1. **批量 registry**: `PASS_REGISTRY` 描述 10 pass；普通 pass 单 kernel，
   `instance_compaction` 3 变体（resources 取自 `variants[]`，各自 `_<variant>.dxil/.rts0`），
   `indirect_args` write+validate 两 kernel 共享单 RTS0。`--all` 批量、`generate_all()`
   返回结构化结果供 CI 消费。
2. **折叠 range 展开**: 原型假设一 range 一资源（tonemap 恰好每 range num=1）；泛化后按
   `NumDescriptors` 逐 register 展开，正确处理 `SRV x5`/`SRV x2`/`SRV x4`/`UAV x2` 等折叠
   range。verify 侧同步 slot→range 展开配对。
3. **buffer binding_kind**: 新增 `structured_buffer`/`rwstructured_buffer` → STORAGE_BUFFER
   映射（原型只有 `byteaddressbuffer` 族）。
4. **128B 硬门 + R1 断言**: 新增 `MAX_PUSH_CONSTANT_SIZE=128` fail-closed
   (`push_constant_too_large`)；param[0] 恒 32-bit root constants 断言归类
   `param0_not_32bit_constants`（驱动硬编码 root param 0，`rendering_device_driver_d3d12.cpp:4208`）。
5. **结构化 fail-closed**: `fail()`/`check()` 抛 `FailClosed(reason, category)` 而非
   `sys.exit`，批量驱动逐 kernel 捕获记录、单文件 CLI 仍打印+非零退出（向后兼容）。
6. **tonemap 回归零漂移**: 单文件 CLI 产物与既有 `out/tonemap.rd_container.bin`
   **字节完全一致**（`ef8668e7330a7bfabb59a80b318c132058baee014a279a7e8ffdba8dec5fa8db`）。
