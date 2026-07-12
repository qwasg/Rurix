# 路线 B 实施计划：RD 原生 compute pipeline —— 从容器生成到 TAA/tonemap 同帧真替代

状态: spike 产物（规划文档）。前置事实见 `container_format.md`；容器生成器与离线自检
已在本目录落地并对 tonemap fixture 全绿（49/49 结构断言）。**本计划不含任何性能宣称。**

## 0. 路线定位

现行 shim 路径（`src/rurix-godot/shim/rxgd_luminance_record.cpp`）：自管
CreateRootSignature/CreateComputePipelineState/描述符堆，吃裸 ID3D12Resource 句柄，
运行在 RD 帧图之外 → 无同帧语义（TAA history、tonemap 输入依赖帧内产物）。

路线 B：把同一套 Rurix 产物（.dxil + .rts0.bin + descriptor_layout.json）打包成
`RenderingShaderContainerD3D12` 容器字节，走
`RenderingDevice::shader_create_from_bytecode` → `RDD::shader_create_from_container`
→ `compute_pipeline_create` → `compute_list_*`，成为 RD 帧图内的一等公民 pass。
驱动直接吃容器内的 DXIL 字节与 RTS0 字节（rendering_device_driver_d3d12.cpp:3319-3333,
:3342），不重编译不重建 → Rurix GPU 侧字节零改动。

## 1. 分段（S1–S6，预估 6 段；每段独立可验收、fail-closed）

### S1 — 生成器产线化（host-only，无 GPU）
- 把 `generate_rd_container.py` 从 tonemap fixture 泛化为按 pass 清单批量产出：
  tonemap / taa_resolve / ssao_blur / particles_copy / segment4h(luminance) 等
  （输入 = 各 `spike/godot-rurix/passes/<pass>/artifacts/` 三件套）。
- raw-buffer 型 pass（GRX-009 tracked-3a：ByteAddressBuffer 视图）走
  `UNIFORM_TYPE_STORAGE_BUFFER` 映射（驱动侧 R32_TYPELESS RAW 视图
  :3544-3565 与 Rurix raw 视图同构）——生成器已含映射表，需逐 pass 核
  descriptor_layout.json 的 binding_kind 覆盖度。
- `verify_container.py` 进 CI 冒烟（镜像 grx009–013 的 ci/ 脚本模式），
  纯结构级、不跑 GPU。
- 验收：全部既有 pass 容器产出 + 自检绿；对不支持的布局（sampler、静态采样器、
  root descriptor、多表）明确 fail-closed 而非静默。

### S2 — 无 patch 引擎内加载/执行探针（首个真机证据门）
关键发现：**存在无需 patch 的加载路径** —— `shader_create_from_bytecode` 绑定在
ClassDB（rendering_device.cpp:8371），GDScript 可达。最小实测方案（本 spike 不执行，
机器有重签在跑）：
```gdscript
# 以 --rendering-driver d3d12 运行；headless 亦可
var rd := RenderingServer.create_local_rendering_device()
var bytes := FileAccess.get_file_as_bytes("res://tonemap.rd_container.bin")
var shader := rd.shader_create_from_bytecode(bytes)      # -> shader_create_from_container
var pipeline := rd.compute_pipeline_create(shader)       # -> CreateRootSignature + PSO
var src := rd.texture_create(fmt_sampling, view, [seed_bytes])   # SAMPLING|CAN_COPY_TO
var dst := rd.texture_create(fmt_storage, view)                  # STORAGE|CAN_COPY_FROM
var u0 := RDUniform.new(); u0.uniform_type = RenderingDevice.UNIFORM_TYPE_TEXTURE; u0.binding = 0; u0.add_id(src)
var u1 := RDUniform.new(); u1.uniform_type = RenderingDevice.UNIFORM_TYPE_IMAGE;   u1.binding = 1; u1.add_id(dst)
var us := rd.uniform_set_create([u0, u1], shader, 0)
var cl := rd.compute_list_begin()
rd.compute_list_bind_compute_pipeline(cl, pipeline)
rd.compute_list_bind_uniform_set(cl, us, 0)
rd.compute_list_set_push_constant(cl, pc, 28)  # i64 维度高 dword 写 0
rd.compute_list_dispatch(cl, ceil(w/8.0), ceil(h/8.0), 1)
rd.compute_list_end()
rd.submit(); rd.sync()
var out := rd.texture_get_data(dst, 0)
```
- 判定：与既有 `tonemap_real_pass_reference.rgb8` 像素对照（复用 grx010 冒烟的
  比对逻辑），加载失败/管线失败/像素不符皆 RED。
- 该段一次性验证核对表 §4 的 1/3/4/5/6/7/10/11/12/14/15/16/17 全部真机语义。
- 风险回退：若 `create_local_rendering_device` 在 D3D12 下不可用（见开放问题 Q1），
  改用主 RD + `RenderingServer.call_on_render_thread`，仍无需 patch。

### S3 — 同帧注入点 patch（进入 patches/00xx 序列）
- tonemap：调用点在 renderer_rd 的 tonemap 后处理链
  （`servers/rendering/renderer_rd/effects/tone_mapper.*` 及其 `_render_buffers_post_process`
  调用方）。patch 形态：在既有 tonemap dispatch 之前/替代处，用 Rurix pipeline RID
  绑定同一批 render-buffer 纹理 RID 走 `compute_list`；开关取 env/项目设置。
- TAA：`effects/taa.*`，同帧输入 = scene color + depth + velocity + history（RD 内
  皆已有 RID），输出回写 internal texture。
- patch 只做「调用点 + RID 转发 + 开关」，不携带任何格式知识（格式知识全部留在
  离线生成器）——保持 patch 最小可审计。
- 验收：patched 引擎构建 + S2 探针脚本改为帧内截取（现有 visual diff 工装复用）。

### S4 — 资源 RID 化转换面（Rurix ⇄ RD 边界重构）
- 现行 shim 边界：Godot 侧抽 ID3D12Resource 裸句柄 → 传给 Rurix shim。
- 路线 B 边界**反转**：注入点已持有 RID（RenderSceneBuffersRD::get_internal_texture 等），
  直接喂 `uniform_set_create`；Rurix 侧不再见任何 native 句柄。
- 需要的转换面清单：
  1. push constants 打包器（28B 结构，i64 低/高 dword 规约）—— CPU 侧纯字节，
     从 shim 现有代码平移；
  2. uniform set 缓存：RID 组合不变则复用 `uniform_set_create` 结果（TAA history
     双缓冲 → 两个缓存槽轮换）；
  3. usage bits 前置校验：src 需 SAMPLING_BIT、dst 需 STORAGE_BIT
     （rendering_device.cpp:4112/:4163），Godot 内部纹理多数已满足，TAA history
     需确认创建 flags。
- 验收：转换面单元冒烟（host 侧构造 + 断言），无 GPU。

### S5 — 共存与切换策略
- 三态开关：`disabled`（默认，现状）/ `shim`（现行 fallback 路径）/ `rd_native`（路线 B）。
- fail-closed 链：容器加载失败或 pipeline 创建失败 → 记录诊断（双语，沿用 RX 码
  规约）→ 回落 `shim` 或 `disabled`，绝不半启用。
- 生成器进 bench 工装（`spike/godot-rurix/bench/`）：runner 启动前产容器文件，
  项目内以 res:// 资源分发。
- shim 不删除：luminance record 等离线诊断路径仍走 shim；路线 B 只接管同帧 pass。
- 验收：三态切换冒烟 ×（tonemap, taa）红绿对。

### S6 — TAA/tonemap 同帧真替代收口
- TAA：history 持久纹理生命周期（首帧 bootstrap、resize 重建）、velocity/depth
  绑定核对、与 Godot 自带 TAA 的互斥开关。
- tonemap：与 Godot tonemapper 参数面（exposure/white）对齐取值来源
  （Environment → push constants）。
- 证据规格沿用 7c04ff0 反降级门思路：真机多帧、非拷贝语义、像素对照 + 帧序
  一致性（TAA 需要 N>1 帧收敛对照），`real_gpu_pass=true` 翻转须双腿证据。
- 验收：两 pass 的 enablement 冒烟（镜像 ci/grx010/grx012 模式）真机红绿。

## 2. 风险登记

| # | 风险 | 等级 | 缓解 |
|---|------|------|------|
| R1 | 驱动硬编码 push constants 于 root param 0（:4208）——Rurix RTS0 emitter 若未来调整参数顺序即静默错绑 | 高 | 生成器/verifier 双侧断言 param[0] 恒为 32-bit constants；写进 Rurix 侧 emitter 契约 |
| R2 | godot-master 快照漂移：CONTAINER_VERSION/FORMAT_VERSION/枚举序变更 | 中 | verifier 常量与 external 行号锚定；bench 工装升级 Godot 时先跑 S1 冒烟 |
| R3 | `create_local_rendering_device` D3D12 支持度未实证（Q1） | 中 | S2 有主 RD + call_on_render_thread 备选；两者都无需 patch |
| R4 | 含 sampler 的未来 pass（当前全部 sampler-free）需要 sampler 表或静态采样器 + 反射扩展 | 中 | 生成器现阶段对 sampler fail-closed；届时按 §2.4/§2.6 sampler 字段扩展并补 verifier 断言 |
| R5 | 旧式 barrier 模式（enhanced barriers 不可用的机器）依赖 res_class/dxil_stages 推导资源状态（:3672+）——字段撒谎会产出错误 barrier | 中 | 生成器只写真值；S2 探针在两种 barrier 模式下各跑一次（`--rendering-device-*` 开关） |
| R6 | RD 层 uniform set 兼容性哈希用 root_signature_crc（:3354-3357）——同 RTS0 不同 DXIL 的 pass 共享 layout hash 属预期行为，但 pipeline/set 混绑校验依赖它，需避免 crc 碰撞焦虑 → zlib crc32 与 Godot 同源同算法 | 低 | 已同源；verifier 断言 |
| R7 | 多 set 布局（未来 pass 若拆 set）需要 per-set 表序与 root param 序配平 | 低 | 生成器现为单 set fail-closed；扩展时镜像 d3d12.cpp:719-731 的表→param 追加顺序 |
| R8 | DXIL 内嵌 RTS0 part 造成 root sig 二义 | 低 | 生成器已断言 dxil 无 RTS0 part（fixture 实测 7 parts 无 RTS0） |

## 3. 开放问题（带验证路径）

- **Q1**: `RenderingServer.create_local_rendering_device()` 在 d3d12 driver 下返回
  有效实例吗？（Vulkan 下常用；D3D12 分支需实证。）→ S2 第一步；失败即切主 RD 方案。
- **Q2**: 主 RD 帧内注入时，`compute_list_begin` 与渲染帧图的 compute list 复用/嵌套
  约束（draw graph 重排序对注入 pass 的调度影响）→ S3 需读
  `rendering_device_graph.*` 后定注入 API 面（`compute_list_*` vs 直接 draw graph 节点）。
- **Q3**: TAA history 纹理的 usage bits 是否含 STORAGE_BIT（决定能否作为 u# 输出直写，
  否则需要中转 + copy）→ S4 核对 renderer_rd 创建处。
- **Q4**: `D3D12CreateRootSignatureDeserializer` 对 v1.0 blob 为文档保证路径（我方情形）；
  Godot 自产 v1.1 blob 能过说明实现更宽容——无行动项，仅记录不依赖该宽容性。
- **Q5**: 28B push constants 与 Godot 侧 `MAX_PUSH_CONSTANT_SIZE=128` 及 == 校验已核；
  但若未来 pass 超 128B 需改走 CBV（layout 变更 + 生成器映射 `cbuffer` 已预留）。

## 4. 分段汇总

| 段 | 内容 | GPU | patch | 预估证据物 |
|----|------|-----|-------|-----------|
| S1 | 生成器产线化 + CI 结构冒烟 | 无 | 无 | 全 pass 容器 + verifier 绿 |
| S2 | 无 patch 引擎内探针（加载→dispatch→readback） | 有 | 无 | 像素对照红绿对 |
| S3 | 同帧注入点 patch（tonemap 先行） | 有 | 有 | 帧内 visual diff |
| S4 | RID 化转换面 + uniform set 缓存 | 无 | 有 | host 冒烟 |
| S5 | 三态开关 + fail-closed 回落 + bench 工装 | 有 | 有 | 三态切换红绿 |
| S6 | TAA/tonemap 真替代收口 | 有 | 有 | 多帧 enablement 冒烟 |
