# RenderingShaderContainer(D3D12) 序列化格式逆向 — 路线 B spike

状态: spike 文档（只读逆向自 `external/godot-master`，未做任何引擎内实测）。
来源基线: 本仓库 vendored godot-master 快照（下列所有行号以该快照为准）。

逆向对象:

- `external/godot-master/servers/rendering/rendering_shader_container.h` / `.cpp` — 基类容器与序列化骨架。
- `external/godot-master/drivers/d3d12/rendering_shader_container_d3d12.h` / `.cpp` — D3D12 子类 extra-data 与 footer。
- `external/godot-master/drivers/d3d12/rendering_device_driver_d3d12.cpp` — 消费端（`shader_create_from_container` :3267-3352 等）。
- `external/godot-master/servers/rendering/rendering_device.cpp` — RD 层入口 `shader_create_from_bytecode(_with_samplers)` :3773-3845。

一切数值均为 **little-endian**；对齐粒度 = 4 字节（`aligned_to`, rendering_shader_container.cpp:38-44）。
写出端与读入端为同一套镜像逻辑（`to_bytes` :851-947 / `from_bytes` :751-849），且 `from_bytes`
在 :847 要求 **消耗字节数 == 输入总长**，即容器不允许任何尾随垃圾字节。

---

## 1. 容器整体布局（D3D12 特化后）

```
offset  size  section
------  ----  -------------------------------------------------------------
0       20    ContainerHeader                     （基类, .h:50-56）
20      0     header extra                        （D3D12 未覆写 → 0 字节）
20      64    ReflectionData                      （基类, .h:58-71; sizeof=64 含 4B 尾部 padding）
84      12    ReflectionDataD3D12                 （d3d12.h:110-114, extra data 前半）
96      16*S  ReflectionBindingSetDataD3D12 × set_count（d3d12.h:89-94, extra data 后半）
...     L     shader_name 原始 UTF-8 字节（无自身长度前缀，长度在 ReflectionData.shader_name_len）
...     pad   将绝对 offset 对齐到 4（from_bytes:782 / to_bytes:902）
per set:
  ...   4     uint32 uniforms_count               （from_bytes:793）
  per uniform:
    ... 20    ReflectionBindingData               （基类, .h:73-83）
    ... 24    ReflectionBindingDataD3D12          （d3d12.h:96-103, 逐-uniform extra）
per specialization constant:
  ...   16    ReflectionSpecializationData        （基类, .h:85-90）
  ...   24    ReflectionSpecializationDataD3D12   （d3d12.h:105-107, uint64[3]）
...     4*N   reflection_shader_stages: stage_count × uint32（enum ShaderStage, 4B each）
per shader (shader_count 个):
  ...   16    ShaderHeader                        （基类, .h:92-97）
  ...   C     code_compressed_bytes（原始或 zstd）
  ...   pad   将绝对 offset 对齐到 4（from_bytes:841 / to_bytes:939）
  ...   0     shader extra                        （D3D12 未覆写 → 0 字节）
footer:
  ...   8     ContainerFooterD3D12                （d3d12.h:116-119）
  ...   R     root_signature_bytes（序列化 root signature = DXBC(RTS0) blob 原样）
EOF（必须恰好结束, from_bytes:847）
```

## 2. 逐字段

### 2.1 ContainerHeader（20 字节, .h:50-56）

| off | type | field          | 取值/校验（from_bytes:762-765） |
|-----|------|----------------|--------------------------------|
| 0   | u32  | magic_number   | `0x43535247`（ASCII "GRSC"）必须相等 |
| 4   | u32  | version        | 写 2（`CONTAINER_VERSION`, .h:45）；校验 `<= 2` |
| 8   | u32  | format         | `0x43443344`（ASCII "D3DC"，d3d12.cpp:250-252）必须相等 |
| 12  | u32  | format_version | 写 1（`FORMAT_VERSION`, d3d12.h:80）；校验 `<= 1` |
| 16  | u32  | shader_count   | compute 单 kernel = 1 |

### 2.2 ReflectionData（sizeof=64, .h:58-71）

结构含 `uint64_t` 首成员 → align 8 → 8 + 13×4 = 60 → **sizeof = 64，尾部 4 字节 padding**。
写出端整体 `memcpy` 该 struct（to_bytes:896），padding 因 `resize_initialized` 恒为 0。

| off | type | field                        | Rurix compute 容器取值 |
|-----|------|------------------------------|------------------------|
| 0   | u64  | vertex_input_mask            | 0 |
| 8   | u32  | fragment_output_mask         | 0 |
| 12  | u32  | specialization_constants_count | 0（Rurix DXIL 无 SC；必须为 0，见 §4.6） |
| 16  | u32  | pipeline_type                | 1 = `PIPELINE_TYPE_COMPUTE`（commons.h:694-698；RD 层 compute_pipeline_create 强校验, rendering_device.cpp:4626） |
| 20  | u32  | has_multiview                | 0 |
| 24  | u32  | has_dynamic_buffers          | 0 |
| 28  | u32×3| compute_local_size[3]        | 从 DXIL PSV0 提取（tonemap = 8,8,1）；`compute_list_dispatch_threads` 用它换算组数 |
| 40  | u32  | set_count                    | 1 |
| 44  | u32  | push_constant_size           | 28（root constants 字节数；RD 层 set_push_constant 按 == 校验, rendering_device.cpp:6105；≤128, :6101；无 16 倍数要求） |
| 48  | u32  | push_constant_stages_mask    | 0x10 = `SHADER_STAGE_COMPUTE_BIT` |
| 52  | u32  | stage_count                  | 1 |
| 56  | u32  | shader_name_len              | strlen(name)（UTF-8 字节数） |
| 60  | u32  | (padding)                    | 0 |

### 2.3 ReflectionDataD3D12（12 字节, d3d12.h:110-114）

| off | field | Rurix 取值 | 消费点 |
|-----|-------|-----------|--------|
| 0 | spirv_specialization_constants_ids_mask | 0 | :3278；pipeline 创建时 SC patch 过滤（:3242） |
| 4 | dxil_push_constant_stages | 0x10 | :3274-3276 → 非 0 才启用 `dxil_push_constant_size`；:4204 判 0 直接 return |
| 8 | nir_runtime_data_root_param_idx | 0xFFFFFFFF | :3279；Godot 自产 compute 容器同样为 UINT32_MAX（d3d12.cpp:903 初始化，仅 vertex 路径改写） |

### 2.4 ReflectionBindingSetDataD3D12 × set_count（每个 16 字节, d3d12.h:89-94）

| off | field | Rurix tonemap set0 | 消费点 |
|-----|-------|--------------------|--------|
| 0 | resource_root_param_idx | 1（RTS0 中 CBV_SRV_UAV 表的 root param 序号） | :3287 → `SetComputeRootDescriptorTable(idx, …)` :5420-5421；UINT_MAX = 无表跳过 |
| 4 | resource_descriptor_count | 2 | :3288 → uniform_set_create 按此数在 resource heap 里分配连续区间（:3386-3396） |
| 8 | sampler_root_param_idx | 0xFFFFFFFF（无 sampler） | :3289；:5423 判 UINT_MAX 跳过 |
| 12 | sampler_descriptor_count | 0 | :3290；>0 才走 sampler heap 分配（:3400） |

### 2.5 shader_name

原始字节紧跟 extra data 之后（to_bytes:900-903），随后 **绝对 offset** 对齐到 4。
name 参与 `RenderingDevice` 侧显示/调试，无功能约束；建议 ASCII（如 `rurix_tonemap`）。

### 2.6 每 set：u32 uniforms_count + 逐 uniform 交错对

**注意交错**：基类 20 字节与 D3D12 24 字节 **逐-uniform 相邻**（to_bytes:910-915 每写完一个基类
struct 立刻调 `_to_bytes_reflection_binding_uniform_extra_data`），不是两个平行数组。

ReflectionBindingData（20 字节, .h:73-83）：

| off | field | tonemap b0=t0 | tonemap b1=u0 | 说明 |
|-----|-------|----------------|----------------|------|
| 0 | type    | 2 = UNIFORM_TYPE_TEXTURE | 3 = UNIFORM_TYPE_IMAGE | enum 序 commons.h:650-665；决定 uniform_set_create 里写哪种 view（SRV :3483 / UAV :3493）以及 RD 层对 RID usage 的校验（SAMPLING_BIT :4112 / STORAGE_BIT :4163） |
| 4 | binding | 0 | 1 | RD 脚本侧 RDUniform.binding 对应值；**set 内必须严格升序**（写出端 sort, d3d12.cpp:888-897；读入端直接信任） |
| 8 | stages  | 0x10 | 0x10 | SPIR-V 侧 stage mask（基类语义）；uniform 哈希/校验用 |
| 12 | length  | 1 | 1 | 纹理=数组元素数（非数组=1）；UBO=字节数；storage buffer=block 字节数（可 0=不设下限）；RD 层用它校验 RDUniform 提供的 id 个数 |
| 16 | writable| 0 | 1 | STORAGE_BUFFER 时决定 SRV/UAV 选择（:3545）；IMAGE 恒写 UAV |

ReflectionBindingDataD3D12（24 字节, d3d12.h:96-103）：

| off | field | t0 | u0 | 消费点 |
|-----|-------|----|----|--------|
| 0 | resource_class | 2 = RES_CLASS_SRV | 3 = RES_CLASS_UAV | :3298 → 旧式 barrier 状态推导（:3672+）；enum d3d12.h:55-60 |
| 4 | has_sampler | 0 | 0 | :3297 间接（sampler 描述符写入路径） |
| 8 | dxil_stages | 0x10 | 0x10 | :3297 → binding.stages；旧式 barrier 可见性推导。0 会被视作「未使用」 |
| 12 | resource_descriptor_offset | 0 | 1 | :3302 → uniform_set_create 写描述符时在 set 分配区间内的槽位（:3483/:3493 的 `+ offset`）；**必须与 RTS0 表内该 register 的 OffsetInDescriptorsFromTableStart（APPEND 展开后）一致** |
| 16 | sampler_descriptor_offset | 0xFFFFFFFF | 0xFFFFFFFF | :3303 |
| 20 | root_param_idx | 0xFFFFFFFF | 0xFFFFFFFF | :3304；仅 dynamic buffer 的 root descriptor 使用（d3d12.cpp:688） |

### 2.7 specialization constants（Rurix = 0 个 → 0 字节）

若存在：基类 16 字节（type,constant_id,int_value,stage_flags）+ D3D12 24 字节
（`stages_bit_offsets: u64[3]`，DXIL bitcode 内的 VBR bit 偏移，供 pipeline 创建时打补丁
:3249 + 重签名 :3260）。**Rurix DXIL 不能声明 SC**：驱动的 patch 机制假定 NIR 产出的
sentinel 布局（d3d12.cpp:230），对 dxc 产物打补丁必然破坏签名。恒写 0 个。

### 2.8 stages 数组

`stage_count × u32`，枚举值 `SHADER_STAGE_COMPUTE = 4`（commons.h:589-600）。

### 2.9 shader 条目

ShaderHeader（16 字节, .h:92-97）+ code：

| off | field | Rurix 取值 |
|-----|-------|-----------|
| 0 | shader_stage | 4（COMPUTE） |
| 4 | code_compressed_size | len(dxil) |
| 8 | code_compression_flags | 0（不压缩；消费端 flags=0 时 memcpy 解出, rendering_shader_container.cpp:983-990） |
| 12 | code_decompressed_size | len(dxil)（>0 触发解压分支 :3321-3326，flags=0 等价拷贝；镜像 Godot 自产行为） |

code = **完整 DXBC 容器字节**（dxc 已验证签名，HASH 在容器 offset 4..20）。
SC 数为 0 时 `_shader_apply_specialization_constants` 仅 COW 拷贝不改字节（:3238），
PSO 直接吃原始 blob（:5464-5467），签名保持有效。之后对齐 4。

### 2.10 footer：ContainerFooterD3D12 + root signature

| off | field | Rurix 取值 |
|-----|-------|-----------|
| 0 | root_signature_length | len(rts0.bin)（tonemap = 152） |
| 4 | root_signature_crc | zlib crc32(rts0 bytes)（d3d12.cpp:761-762 同源）；消费端作为 `shader_get_layout_hash`（:3354-3357），RD 用于 pipeline/uniform-set 兼容性哈希 |
| 8 | root_signature_bytes | `tonemap.rts0.bin` **原样**（本身即 DXBC 容器包 RTS0 part，即 `D3D12SerializeRootSignature` 的 blob 形态） |

---

## 3. 关键定论：RD **不重建** root signature

消费端 :3335-3346：

1. `D3D12CreateRootSignatureDeserializer(bytes)` → 仅为拿 `D3D12_ROOT_SIGNATURE_DESC*`
   （:3345 存入 `root_signature_desc`；**全驱动仅此一处赋值，无其他读取点** — 结构性依赖为零，
   但 deserializer 调用必须成功否则 ERR_FAIL）。
2. `device->CreateRootSignature(0, bytes, size)` → **容器里的字节原样生效**（:3342）。

因此不存在「RD 按自己规则重建 root sig」的配平问题；约束收敛为**三方自洽**：

- **DXIL ⇄ RTS0**：register/space 一致 — Rurix 产物天然满足（t0/u0/b0, space0）。
  Godot 自产 DXIL 用 `set*100000000 + binding*100000` 的 register 卷积
  （d3d12.cpp:665, d3d12_godot_nir_bridge.h:44-46），但驱动从不重算 register，
  Rurix 用 t0/u0 与之不同**无碍**。
- **RTS0 ⇄ 反射**：root param 序号、表内槽位偏移、描述符总数三者对得上（§4）。
- **反射 ⇄ RD 调用方**：binding 号、UniformType、writable 与脚本/注入端提供的
  RDUniform 一致。

## 4. 消费端逐字段核对表（shader_create_from_container :3267-3352 起）

| # | 消费端行为（行号） | 容器字段 | Rurix tonemap 必须满足 |
|---|--------------------|----------|------------------------|
| 1 | cast_to\<RenderingShaderContainerD3D12\>（:3270） | header.format/format_version | 0x43443344 / 1（由 RD 入口 :3784 按驱动创建同型容器 + from_bytes 校验保证） |
| 2 | dxil_push_constant_stages != 0 → dxil_push_constant_size = push_constant_size（:3274-3276） | §2.2/§2.3 | 0x10 / 28 |
| 3 | `SetComputeRoot32BitConstants(0, …)` **硬编码 root param 0**（:4208） | RTS0 param[0] | 必须是 32-bit constants（type=1），Num32BitValues == push_constant_size/4 == 7 ✓（实测 fixture param0 = b0/space0/7 dwords） |
| 4 | set.resource_root_param_idx → `SetComputeRootDescriptorTable`（:3287/:5421） | §2.4 | =1，且 RTS0 param[1] 为 descriptor table ✓ |
| 5 | resource_descriptor_count → heap 区间分配（:3288/:3386） | §2.4 | =2 == RTS0 表内 NumDescriptors 之和 ✓ |
| 6 | binding.resource_descriptor_offset → 描述符写入槽位（:3302/:3483/:3493） | §2.6 | t0→0, u0→1 == RTS0 range 偏移（fixture 为 APPEND=0xFFFFFFFF，按序展开 0,1）✓ |
| 7 | binding.type/writable → SRV vs UAV 视图选择（:3480-3499/:3536-3565） | §2.6 | t0: TEXTURE(2)/w=0 → CreateShaderResourceView；u0: IMAGE(3)/w=1 → CreateUnorderedAccessView |
| 8 | binding.res_class/dxil_stages → 旧式 barrier 状态（:3298/:3297, :3672+） | §2.6 | SRV/0x10、UAV/0x10（enhanced barriers 下提前 return :3663-3665，仍应写真值） |
| 9 | SC 数组逐条拷贝（:3308-3316） | §2.7 | 空 |
| 10 | 逐 stage 解压 code（:3318-3333） | §2.9 | flags=0 → memcpy；blob == 原 dxil 字节 |
| 11 | RootSignatureDeserializer（:3338） | footer bytes | v1.0 blob（fixture 实测 version=1）为该 API 的标准输入；Godot 自产为 v1.1 亦可用，1.0 更保守 |
| 12 | CreateRootSignature（:3342） | footer bytes | 与 shim 现行 `CreateRootSignature(0, rts0, len)`（rxgd_luminance_record.cpp:579）同一字节同一 API，已有真机通过先例 |
| 13 | root_signature_crc → layout hash（:3346/:3354） | footer.crc | zlib crc32，供 RD 校验 pipeline 与 uniform set 的 shader 兼容性 |
| 14 | RD 层 compute_pipeline_create：pipeline_type == COMPUTE（rendering_device.cpp:4626） | §2.2 | =1 |
| 15 | RD 层 dispatch 前 push constant == 28 字节严格相等（rendering_device.cpp:6105） | §2.2 | 注入端每帧提供 28B（i64 维度高 dword 必须写 0，见 layout json `i64_dims_note`） |
| 16 | RD 层 uniform_set_create：binding 0 需 RID 带 SAMPLING_BIT（:4112），binding 1 需 STORAGE_BIT（:4163） | — | 注入端资源必须是 RD 原生 RID 且 usage 正确（路线 B 的「RID 化」面） |
| 17 | PSO：`CS = stages_bytecode[COMPUTE]` + `pRootSignature`（:5458-5467） | §2.9/§2.10 | DXBC 已签名（dxc validator hash 非零，fixture 实测 72-66-D9-5B…）；DXIL 内**无**内嵌 RTS0 part（fixture 7 parts: SFI0/ISG1/OSG1/PSV0/STAT/HASH/DXIL），root sig 完全来自容器 footer，无二义 |

## 5. Rurix fixture 实测基线（本 spike 解析结果）

`spike/godot-rurix/passes/tonemap/artifacts/`：

- `tonemap.rts0.bin`（152B）= DXBC{RTS0(108B)}：version 1.0, flags 0, params 2：
  - param0 = RootConstants(b0, space0, 7 dwords), visibility ALL；
  - param1 = DescriptorTable[ SRV t0 ×1 (APPEND), UAV u0 ×1 (APPEND) ], visibility ALL。
- `tonemap.dxil`（4436B）= DXBC 7 parts（无 RTS0 内嵌）；PSV0: RuntimeInfoSize=52
  (Info3)，ShaderKind(struct+24)=5(Compute)，NumThreads(struct+36..48)=(8,8,1)。
- `tonemap_descriptor_layout.json`：root_constant_dwords=7 / bytes=28；resources =
  t0 Texture2D\<float4\>（texture2d）、u0 RWTexture2D\<float4\>（rwtexture2d）。

三者与 §4 核对表全部自洽 → 生成器只做「翻译 + 配平校验」，不改任何 GPU 侧字节。

## 6. PSV0 提取规则（generate_rd_container.py 依据）

DXBC part `PSV0`：`u32 RuntimeInfoSize` + struct。实测（dxc, cs_6_0）RuntimeInfoSize=52
= PSVRuntimeInfo3；布局：Info0 = 16B union + Min/MaxExpectedWaveLaneCount(8B) = 24B；
Info1 = +ShaderStage(u8)@24 +UsesViewID +sig 计数等 = 36B；Info2 = +NumThreadsX/Y/Z@36/40/44
= 48B；Info3 = +EntryFunctionName offset = 52B。生成器要求 RuntimeInfoSize ≥ 48 且
ShaderStage==5，否则 fail-closed（不猜 numthreads）。

## 7. 已知差异与非问题清单

| 差异 | 定性 |
|------|------|
| Rurix RTS0 v1.0 vs Godot 自产 v1.1（DATA_VOLATILE range flags） | 非问题：CreateRootSignature 双版本均收；v1.0 语义默认即 volatile 族；deserializer 对 1.0 是文档保证路径 |
| register 卷积（t0/u0 vs set*1e8+binding*1e5） | 非问题：驱动不重算 register（§3） |
| RTS0 range 偏移 APPEND vs Godot 显式偏移 | 非问题：APPEND 展开序 == 反射 offset 序即可（生成器负责展开并校验） |
| DXIL 由 dxc 签名 vs Godot 内部 `RenderingDXIL::sign_bytecode` | 非问题：仅 SC patch 后需重签；SC=0 恒不触碰 |
| visibility ALL vs 分 stage | 非问题：compute 管线 visibility 无效化，Godot 对 compute 也给 ALL（d3d12.cpp:138-147 default 分支） |
| Rurix root constants 寄存器 b0 vs Godot 的 b(1e8*17) | 非问题：同 §3；但 **param 序号 0 是硬约束**（:4208），Rurix RTS0 emitter 必须永远把 root constants 放 param[0]，descriptor table 随后 |
