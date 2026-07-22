# Rurix → UE5 级别渲染器:差距评估报告

> 评估时点:2026-07-19
> 评估对象:Rurix v1.0.1-dist.2(stable channel latest),active 里程碑 EA1 + G3
> 评估方法:对照 UE5(Lumen/Nanite/TSR/VSM/Path Tracer 等)核心渲染组件,逐项核对 Rurix 编译器、运行时、ruridrop demo、RFC、deferred 注册表、deep-research
> 评估目的:回答"用 Rurix 制作 UE5 级别渲染器还差哪些东西"

---

## 0. 一句话结论

**Rurix 当前不是渲染器,而是"渲染器构建工具/语言底座"。** 距离 UE5 级别渲染器还差 **一整个渲染算法层 + 内容工具链层 + 生态平台层**,粗估 5-8 年单人 + AI 集群连续工程。这不是质量差距,是范围差距——Rurix 当前阶段刻意聚焦语言/编译器/render graph 基础设施层,渲染算法与内容工具链既未登记也未调研。

---

## 1. Rurix 当前能力边界(三档清单)

### A. 已落地(端到端 measured,v1.0 stable 冻结 + G3.4/3.5 已合入)

| 域 | 能力 | 证据 |
|---|---|---|
| Shader stage | `compute` (kernel fn) / `vertex` / `fragment` | DXIL `dxil-unknown-shadermodel6.0-compute` / `vs_6_0` / `ps_6_0` + Vulkan `GLCompute` / `Vertex` / `Fragment` |
| 纹理 | `Texture2D<F>` (SRV) / `TextureRw2D<F>` (storage) | RXS-0223,`dxil_spirv.rs:151~166` |
| Sampler | `Sampler` / `SamplerCmp` (shadow 比较) | `src/rurix-rt/src/sampler.rs:47` |
| 采样方法族 | `sample / sample_lod / sample_grad / sample_bias / sample_cmp / gather / load / load_lod / TextureRw2D.store/load` | `dxil_spirv.rs:151~166` |
| 各向异性 | `SamplerDesc { filter, address, max_anisotropy, lod_bias, min_lod, max_lod, compare }`,device 探测 `samplerAnisotropy` | `sampler.rs:47` |
| Bindless | `[Texture2D<F>]` 无界 SRV 纹理表(G3.4, RXS-0231/0233/0234) | `binding_layout.rs:189` `UnboundedTable`;仅限 SRV 纹理,Sampler/CBV/UAV 无界表未支持 |
| Render graph | 自动 barrier 状态机(G3.5, RXS-0236~0241,单 queue) | `src/rurix-rt/src/graph.rs`(968 行,`#![forbid(unsafe_code)]`) |
| Present | D3D12 flip-model swapchain + Vulkan present | `rurix-d3d12/src/lib.rs` / `vk.rs:run_graphics_present` |
| 几何库 | Point3/Vector3/Normal3/Aabb/Ray/Triangle + BVH(median-split,叶=2)+ ray-sphere + ray-plane | `rurix-geometry/src/lib.rs:259-369`;**无三角形-射线相交,仅 AABB slab** |
| 路径追踪 | ruridrop PT:3D-DDA 均匀网格 + Lambert + Schlick Fresnel + NEE(矩形面光源)+ any-hit shadow + Reinhard + gamma 2.0 | `apps/ruridrop/src/render_pt.rx`(540 行) |
| 实时档 | ruridrop rt:1spp 同核退化 + 单 shadow ray + ambient+Lambert+Fresnel 反射 | `render_rt.rx`(252 行) |
| 图像 I/O | PPM P6 only,Rgb/Rgba f32→u8 量化 | `image-io/src/lib.rs`;**无 PNG/JPG/EXR/BCn/ASTC** |

**当前最高 demo**:ruridrop realtime = 1280×720 / 1spp / 131072 粒子 SPH + PT 软光栅化,D3D12 present。**研究级验证档,非生产渲染器**。

### B. 在做(G3.6,RFC-0013 §4.E)

| 能力 | 状态 | 风险 |
|---|---|---|
| Mesh shader / Task shader | SPIR-V `SPV_EXT_mesh_shader`(triangles-only),类型面已就位,codegen STUB | 上游钳制 |
| RT 六阶段 | `raygen / miss / anyhit / closesthit / intersection / callable` + AS + SBT + `trace_ray`(Vulkan `VK_KHR_ray_tracing_pipeline` + `VK_KHR_acceleration_structure`) | 高,DXIL 腿 probe-first |
| DXIL RT 腿 | 受 LLVM #90504/#57928 上游钳制,RD-012/RD-015 deferred | 可能 blocked,落 RD-034+ 尾门 |

**G3.6 完成后**:Rurix 将具备现代图形 API 抽象层的完整管线(compute + 9 shader stage + bindless + render graph + RT + mesh),但**仍无渲染算法实现**。

### C. 未登记(gap,RFC / deferred / deep-research 均未提及)

> 下列关键字在 `H:/rurix/src/**/*.rs` 全仓搜索**无实现命中**;在 `rfcs/` 与 `registry/deferred.json` 中**无登记**;`deep-research/r1~r12` 无一份聚焦渲染引擎特性。

#### C.1 渲染算法层(完全空白)
- **PBR 材质系统**:GGX / Sheen / Coat / 多层材质 / 材质图节点编辑器
- **阴影**:shadow map / VSM(Virtual Shadow Maps)/ RT soft shadow
- **全局光照(GI)**:irradiance volume / DDGI / light probe / SDF GI / RT GI / Lumen-class 多方法混合
- **反射**:SSR / RT reflection / planar reflection
- **环境光遮蔽**:SSAO / GTAO / RTAO / HBAO
- **体积效果**:volumetric fog / volumetric cloud / participating media
- **抗锯齿**:FXAA / TAA / TSR(MSAA 显式 `out_of_scope msaa_blend_stencil_indirect`,零登记不承诺)
- **后处理栈**:bloom / ACES / Hable / Filmic tonemap / color grading LUT / motion blur / DOF / lens flare / chromatic aberration / vignette
- **多 pass 全屏后处理管线**(render graph 首期封闭枚举:ColorAttachment/Depth/UAV/Readback/Copy/Present,无 post-process pass kind)

#### C.2 几何与场景层
- **Nanite-class 虚拟化几何**:cluster / visibility buffer / 自动 LOD / mesh shader pipeline
- **LOD / Impostor / HLOD**
- **大世界**:World Partition / 流式加载 / chunk
- **虚拟纹理(VT)**:feedback + page cache
- **indirect draw / multi-draw-indirect / GPU-driven pipeline**

#### C.3 内容系统层
- **GPU 粒子系统**(Niagara-class)
- **景观 / 植被 / foliage / grass wind**
- **骨骼动画**:skinning / morph target / animation graph / motion matching
- **Decal**:deferred decal / mesh decal

#### C.4 特殊着色
- **Subsurface Scattering(SSS)**
- **毛发**:Marschner / strand-based
- **水体**:FFT 波浪 / Gerstner
- **眼球**:角膜/虹膜

#### C.5 资产与工具链层
- **资产导入**:GLTF / FBX / USD / Alembic(全无)
- **纹理压缩**:ASTC / BCn / ETC2(显式"不登记不承诺")
- **材质图编辑器**:node-based / visual scripting
- **场景编辑器**:viewport / gizmo / outliner

#### C.6 高级管线特性
- **multi-queue / async compute / async transfer**(render graph 显式不做)
- **ray query(inline RT,OpRayQuery)**("零 deferred 登记不静默带入")

#### C.7 生态与平台层
- **跨平台**:当前 Windows + NVIDIA only(Vulkan MB1 仅 AMD/Android preview)
- **资产市场 / 商店**
- **文档与教程生态**
- **社区与插件生态**

---

## 2. UE5 渲染器核心组件清单(对照基准)

| UE5 组件 | Rurix 当前 | 差距 |
|---|---|---|
| **Nanite**(虚拟化几何 / cluster / visibility buffer / mesh shader) | mesh shader 在 G3.6 做;cluster/visibility buffer 未登记 | 大 |
| **Lumen**(GI:card capture / surfel / voxel / RT 混合) | 无任何 GI 子系统 | 巨大 |
| **Virtual Shadow Maps**(VSM) | 无 | 大 |
| **TSR**(时序超分) | 无 AA | 大 |
| **硬件 RT**(reflection / shadow / GI / ao) | RT 管线 G3.6 做;算法无 | 大 |
| **SSGI / SSR / SSAO / GTAO** | 无 | 巨大 |
| **Volumetric Fog** | 无 | 大 |
| **Material Editor**(node-based layered) | 无 | 巨大 |
| **Post Process Stack**(bloom/DOF/MB/tonemap/CG/LUT/lens) | 仅 Reinhard | 巨大 |
| **Path Tracer**(UE5.6 production-grade) | ruridrop PT 研究级(DDA+Lambert+NEE) | 大 |
| **World Partition** | 无 | 大 |
| **HLOD / Impostor** | 无 | 中 |
| **Niagara**(GPU 粒子) | 无 | 大 |
| **Landscape / Foliage** | 无 | 大 |
| **Skeletal Animation** | 无 | 大 |
| **SSS / Hair / Water / Eye** | 无 | 中 |
| **Decals** | 无 | 中 |
| **Virtual Texture** | 无 | 中 |
| **Material Instance / Layered** | 无 | 大 |
| **DLSS / FSR support** | 无 | 中 |

---

## 3. 分层差距分析与时间估算

> 单人 + AI 集群,延续 Rurix 当前节奏(MVP 12-18 个月已兑现,每里程碑严格 measured-first)

### 层 1:基础设施层(语言/编译器/IR/三后端/render graph/bindless)
- **Rurix 当前**:接近现代图形 API 抽象层;G3.6 完成后 mesh/RT 管线就位
- **UE5 等价层**:Hardcoded D3D12/Vulkan/Metal backend + RHI 抽象
- **gap**:小,G3 + EI1 收尾即接近
- **估算**:~3-6 个月(G3 剩余 + EI1 引擎集成期)

### 层 2:渲染算法层(材质/光照/阴影/GI/反射/AO/体积/AA/后处理)
- **Rurix 当前**:几乎完全空白,仅有研究级 PT demo
- **UE5 等价层**:数百人渲染团队多年投入
- **gap**:巨大,每个子系统都是独立工程项目
- **估算**:~2-3 年(若按 Rurix 治理节奏,每个子系统走 RFC + conformance + measured 验证)

### 层 3:内容工具链层(资产导入/材质图/动画/景观/粒子/LOD/编辑器)
- **Rurix 当前**:完全空白
- **UE5 等价层**:Editor 全套 + 资产管线 + DDC
- **gap**:巨大
- **估算**:~2-3 年

### 层 4:生态与平台层(跨平台/市场/文档/社区)
- **Rurix 当前**:Windows + NVIDIA only,Vulkan MB1 preview
- **UE5 等价层**:全平台 + Quixel / Marketplace / 大量教程
- **gap**:取决于采纳,长期
- **估算**:无限期,取决于社区采纳

### 总计
**~5-8 年**(单人 + AI 集群,延续当前节奏,从 G3.6 完成后开始计算)。

---

## 4. 现实路径建议

### 路径 A:做"rurix-native 渲染引擎核心"(语言级抽象)
- **定位**:Rurix 作为编写渲染器的语言,而非替代 UE5 整体
- **目标**:类似 Bevy / rend3 / wgpu-rs 生态位,但 GPU-native + 安全类型系统
- **路径**:G3.6 完成 → 在 rurix-rt 上构建 `rurix-render` crate(材质/光照/后处理子系统) → ruridrop 升级为 demo 引擎
- **时间**:3-5 年可达到"现代渲染器核心"(无 Nanite/Lumen 级别特性)

### 路径 B:嵌入 UE 类引擎做特定子系统(EI1 引擎集成期方向)
- **定位**:Rurix 编译产物通过 `#[export(c)]` 嵌入 C++/D3D12 引擎,承担特定子系统
- **候选**:
  - **离线路径追踪器**(替代 UE5 Path Tracer 的部分场景,语言级安全保证)
  - **特定 GPU 算子库**(替代 Nanite cluster culling / Lumen surfel raster 等)
  - **shader 安全重写**(把引擎现有 HLSL 子集翻译为 .rx 获得类型安全)
- **路径**:G3.6 完成 → EI1 引擎集成 → 选定一个子系统做 reference 实现
- **时间**:1-2 年可见单子系统成果

### 路径 C:定位为"研究/教学级 GPU 安全语言"
- **不追求**生产级渲染器,聚焦"GPU 安全类型系统"学术贡献
- **目标**:论文 + 安全 GPU 编程范式推广
- **路径**:深耕现有 device 安全模型(execution resources + views + context brand)
- **时间**:学术周期

---

## 5. 关键风险

| 风险 | 说明 |
|---|---|
| **bus factor=1**(R-601) | 单人 + AI 集群模式,UE5 级别渲染器需要大量子系统并行工程,单人带宽瓶颈 |
| **AI 语义漂移**(R-602) | 大量渲染算法子系统由 AI 实现可能引入隐蔽 bug |
| **范围蔓延**(R-603) | 渲染算法层诱惑大,易破坏"strict-only + measured-first"治理纪律 |
| **NVIDIA-only**(R-204) | 渲染器需跨平台,但 Rurix 当前 Windows + NVIDIA only |
| **LLVM 上游钳制** | RD-012/RD-015 DXIL RT 腿受 LLVM #90504/#57928 钳制,G3.6 关键路径风险 |

---

## 6. 评估总结

Rurix 在**语言与编译器基础设施层**已达到生产级(v1.0 stable,G3.6 完成后将具备现代图形 API 完整管线抽象),但**渲染算法层、内容工具链层、生态平台层**几乎完全空白。

更准确的定位不是"Rurix 离 UE5 还差多远",而是:
- **Rurix 不是 UE5 的竞品**,而是"GPU 安全编程语言"——它的对标是 CUDA C++ / Mojo / Slang / Descend,不是 UE5
- **ruridrop 是语言验证档**,不是渲染器产品
- 若要用 Rurix 制作 UE5 级别渲染器,需要在 Rurix 之上构建一整个渲染引擎 + 工具链 + 生态,这是远超 Rurix 当前范围的工作

**最现实的高价值路径**:G3 + EI1 完成后,选路径 B(嵌入 UE 类引擎做特定子系统),用 1-2 年做出一个有说服力的 reference 子系统(如离线 PT 或安全 shader 重写),证明 Rurix 的语言级抽象价值,再决定是否扩展为完整渲染器。
