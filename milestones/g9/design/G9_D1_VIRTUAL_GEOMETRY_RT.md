# G9_D1 — 虚拟化几何与 RT 合流 设计草案

> **DRAFT 设计提案——G9 未立项，本文档不构成任何契约/验收承诺。**
> 本文是 G9 正式立项前的模块设计输入，全部波次、验收门、编号、evidence schema
> 均为**建议草案**；立项时须经 G9.1 治理波次重新裁决、按当时实测 `next_free`
> 领取编号、并硬化进 G9_CONTRACT / G9_ACCEPTANCE_MAP 后方可视为承诺。
> 本文只追加，不修改任何 G8 及更早文档；G8 已 closed 的契约与判据 0-byte。
>
> **版本**：v0.1（2026-08-06 之后，起草于 G8 close-out 之后、G9 立项之前）
> **G9.0 冻结引用**：2026-08-08 起，本文作为 G9.0 文档集不可变基线附件被 [G9_PLAN.md](../G9_PLAN.md) 冻结引用；正文 0-byte，后续变更只追加修订记录（追加于文末）。
> **上游调研**（设计依据，§5 逐条引用）：
> [调研1] Nanite 核心 = 离线 cluster DAG + 运行时误差驱动 cluster 选择 + 按需流送；
> UE5.5 Nanite Skeletal Mesh 走 CPU 蒙皮喂静态 cluster 的权宜路线（蒙皮击穿 cluster
> 包围体/误差度量，吞吐低、不支持 Morph、有视角 LOD 瑕疵）；Kerbl et al. 2021
> 《Conservative Meshlet Bounds for Skinned Meshes》(CGF) 给出 LBS 蒙皮下保守
> meshlet 包围球/法向锥的严格解。
> [调研2] Remedy Northlight（Alan Wake 2，GDC 2024《Large Scale GPU-Based
> Skinning for Vegetation》）：mesh shader 硬件光栅 + 两级遮挡剔除（单像素精度）；
> 全部植被骨骼绑定、约 30 万骨骼 GPU 蒙皮（bone shaders 向美术暴露可编程 API）、
> 距离分级动画更新率（10m 内全速 / 其后 1/2、1/3、1/4）。
> [调研3] RTX Mega Geometry（NVIDIA CES 2025，非 Epic）：BVH 插入 CLAS
>（Cluster-level AS）层，簇级 AS 可离线随资产烘焙，BLAS = CLAS 列表拼装；
> `VK_NV_cluster_acceleration_structure` 支持 multi-indirect device 构建 +
> Cluster Template 实例化。Alan Wake 2 2025-02 落地：VRAM −300MB，RTX 4060
> +42%、2080 Ti +13%、4090 几乎无帧率收益（解放的是 VRAM/CPU/构建带宽约束
> 路径）。NVIDIA 开源 nv_cluster_builder / nv_lod_cluster_builder（边塌缩 +
> 簇对锁定，与 Nanite DAG 同源）与 4 个 Vulkan 样例。
> [调研4] 跨厂商前景：`VK_EXT_mesh_shader` 已跨厂商（NV/AMD/Intel）；
> `VK_NV_cluster_acceleration_structure` 目前 NV-only，但 DXR Functional Spec
> Part 2 已把 CLAS 写成厂商中立设计（2025-09 仍在更新），Khronos EXT 标准化
> 可期；AMD RDNA4 走 DGF 路线。DMM（`VK_NV_displacement_micromap`）已被
> NVIDIA 官方归档并被 Mega Geometry 取代——**禁止投入**。
> [调研5] 架构原则：光栅剔除选出的可见 cluster 集合 = 当帧 BLAS 拼装 +
> 动画更新率分级的**单源真相**，避免几何/RT 两个世界错配；验收指标须含
> VRAM 与 AS 构建耗时而非只看 FPS。

---

## 1. 定位与承接锚

**D1 = G9 建造期的几何与 RT 合流模块**：把 G8 冻结的虚拟化几何底座（cluster
DAG、页式流送、VisBuffer）升级为动态资产（骨骼/植被）可用的完整 Nanite 类
管线，并把可见 cluster 集合直接合流进 RT 加速结构（CLAS 簇级 BLAS），使
「光栅世界」与「RT 世界」共享同一份当帧几何真相。

法定承接锚（G8 移交，逐字引用）：

| 承接项 | 来源 | 字面 |
|---|---|---|
| M06 骨骼/植被虚拟几何 | `milestones/g8/G8_P2_DECISIONS.md:11` | 决策 `defer-to-G9+`，承接锚 =「G9+ 虚拟几何评估窗」，backfill = RD-039「动态资产面出现时」 |
| M09 Mega Geometry 簇级 BLAS | `milestones/g8/G8_P2_DECISIONS.md:12` | 决策 `defer-to-G9+`，承接锚 =「G9+ RT×Nanite 合流窗」，backfill = RD-039「RT 与虚拟几何合流需求出现时」 |
| M06 矩阵行 | `milestones/g8/G8_CAPABILITY_MATRIX.md:41` | deformer ABI、skin cache、微实例，档位 A/B/C/D，P2 |
| M09 矩阵行 | `milestones/g8/G8_CAPABILITY_MATRIX.md:44` | 虚拟几何直接进 RT AS，档位 B/C，P2 |
| RD-039 承接映射 | `milestones/g8/G8_CAPABILITY_MATRIX.md:201` | RD-039 → M03/M04/M06/M09/M61（各需决策表） |
| G9 定位 | `milestones/g8/G8_PLAN.md:13,15,113` | 「正式建造归 G9+」；G8 是前置能力完成期 |

**触发条件声明（立项时须留痕）**：本设计假定 G9 立项本身即构成 RD-039 两个
分项的触发——「动态资产面出现」与「RT 与虚拟几何合流需求出现」以 G9 建造期
正式立项书为证据；立项时须在 `registry/deferred.json` history **只追加**登记
M06/M09 分项由 open-defer 转入 G9 承接（不得改写 G8.7 决策表原文）。

**为什么 D1 是 G9 第一优先模块**：G8.7 穷举表中 M12（Surface Cache）、M16
（GI 档位）、M43（World Partition/HLOD）、M55（GPU-driven 提交）等多个
defer 项都以动态几何与统一场景表示为前置（`G8_P2_DECISIONS.md:13,16,22,30`）；
几何/RT 双世界一旦错配，后续 GI、阴影、大世界全部要在错误地基上返工
（[调研5]）。

---

## 2. 范围 in/out

### 2.1 In scope（D1 承担）

| # | 子面 | 承接 |
|---|---|---|
| D1-a | 离线 cluster DAG 构建管线深化：从 G8 静态 meshlet DAG 升级为支持误差度量、簇对锁定（nv_lod_cluster_builder 同源算法）、CLAS 离线烘焙的 builder | M06/M09；[调研1][调研3] |
| D1-b | GPU cluster 感知蒙皮与骨骼植被：LBS 蒙皮 device kernel、保守时空包围体（Kerbl et al. 2021）、bone shader 可编程 API、距离分级动画更新率 | M06；[调研1][调研2] |
| D1-c | 误差驱动运行时 LOD/cluster 选择：monotonic 误差、屏幕空间误差阈值、与流送页驻留联动 | M06/M44 消费端；[调研1] |
| D1-d | CLAS 簇级 BLAS：可见 cluster 集合 → 当帧 BLAS 拼装（multi-indirect device 构建、Cluster Template 实例化），`VK_NV_cluster_acceleration_structure` 主腿 + 非 CLAS 回退腿 | M09；[调研3][调研4] |
| D1-e | 与 VisBuffer / VSM 集成：可见性单源真相同时喂光栅 VisBuffer、VSM 页标记与 RT AS；VSM 阴影光线与 RT 一致 | [调研5] |

### 2.2 Out of scope（D1 不承担）

| 项 | 去向 | 依据 |
|---|---|---|
| Morph target 虚拟化 | 非虚拟化旁路（D1-b 内仅做旁路接线，不做 DAG 化） | [调研1]：UE5.5 亦不支持；无保守包围解 |
| DMM / displacement micromap | **永久禁止** | [调研4]：NVIDIA 已归档，被 Mega Geometry 取代 |
| HZB 两阶段遮挡剔除（M03） | 维持 G8 no-go，除非 G9 决策表重新判档 | `G8_PLAN.md:74`；RD-039 独立分项 |
| mesh shader 第三光栅（M61） | G9 决策表独立判档；D1 的 device kernel 设计**预留** mesh shading 接缝但不主线化 | `G8_P2_DECISIONS.md:33`：RD-039 双条件（跨厂商收敛+measured） |
| World Partition / HLOD（M43）、GPU-driven 提交（M55） | 归 G9 其他模块（建议 D2/D3） | `G8_P2_DECISIONS.md:22,30` |
| DXIL/SPIR-V 后端的 RT mesh 腿 | RD-034 blocked 维持；D1 仅 Vulkan 主腿 | `G8_PLAN.md:55` |
| 编辑器 GUI、USD ingest、多 GPU | 超范围 | `G8_PLAN.md:113-116` out-of-scope |

---

## 3. 依赖前置（G8 已交付，D1 只消费不重定）

| 前置 | 状态 | 证据/锚 |
|---|---|---|
| M01 逻辑页 ABI（RXPL header 136B、stable id、root 页、schema/section digest） | 已冻结 | `spec/geometry_pages.md:1-11`，RXS-0328~0331 |
| M04 磁盘/内存页格式 + RXPZ-LZ1 解码 ABI | 已冻结 | `spec/geometry_pages.md`，RXS-0338~0342 |
| M44 几何页 streamer（只消费冻结 ABI，迟到页降级） | 已绿 | `G8_CAPABILITY_MATRIX.md:104` |
| VisBuffer SW/HW 逐像素 diff=0（depth30\|cluster27\|tri7 64 位格式） | 已绿 | `src/rurix-render/src/geometry/visbuffer.rs`；G8.5a 门 |
| VSM 完整页缓存 device 化 | 已绿 | `G8_PLAN.md:217`（M19） |
| M50 RT 增量面：多 hit group / SBT 用户数据 / stack sizing / pipeline library，与 AsManager 单所有者 | 已绿 | `G8_PLAN.md:167-176`；`src/rurix-render/src/rt/as_manager.rs` |
| cluster DAG 离线构建（`rurix-geom-build` / `rurix-asset::geom_build`） | 已有静态版 | `spec/geometry_pages.md:18-19` |
| GPU 两级剔除 + LOD cut host 参照 + device 对拍 | 已有 | `src/rurix-render/src/geometry/cull.rs` |
| RenderGraph / 多队列（单队列默认）/ streaming 池 | 已有 | `src/rurix-render/src/graph`、`streaming` |
| `forbid(unsafe_code)` host 纪律 | 维持 | `src/rurix-render/src/lib.rs:23` |

**反向依赖禁令**（沿 G8 R-G8-4 纪律，`G8_PLAN.md:313`）：D1 新增 cluster 属性
（保守包围体、CLAS 引用、动画分级元数据）若需入页，必须走**新 major 页格式
版本**并经 G9 spec-first 条款冻结；禁止在实现波次中途重定 M04 v1 ABI。

---

## 4. 模块分解

### 4.1 D1-a：cluster DAG 构建管线（离线，host 纯 Rust）

- 在 `rurix-geom-build` 现有 `ClusterDag` 上扩展：
  - **误差度量**：每簇记录 `parent_error` / `cluster_error`（几何近似误差 +
    包围球），保证沿 DAG 单调（monotonic error），运行时选择即求误差 cut
    （[调研1]）。
  - **簇对锁定（cluster pair locking）**：边塌缩时锁定相邻簇边界，避免 LOD
    裂缝；算法与 nv_lod_cluster_builder 同源但独立实现（许可裁决归 G9 治理，
    不直接 vendoring）（[调研3]）。
  - **CLAS 离线烘焙**：builder 输出每簇 CLAS 构建输入（三角形簇 + 簇级
    AABB），随资产页打包；运行时 CLAS 构建退化为 device 侧拼装而非几何
    处理（[调研3]）。
  - **蒙皮元数据**：每簇记录最大影响骨数、骨骼索引集、蒙皮包围体膨胀
    系数（Kerbl 保守界所需输入）（[调研1]）。
- 输出走页格式**新 major**（建议 RXPL major=2 / 新 schema_digest preimage），
  与 M04 v1 共存；双构建确定性 CI 沿 M79 判据。

### 4.2 D1-b：GPU 蒙皮与骨骼植被

- **cluster 感知蒙皮 kernel**（`.rx`，compute 主腿）：按簇读取骨骼 palette，
  输出蒙皮后顶点 + **保守包围球/法向锥**（Kerbl et al. 2021：LBS 权重下
  包围球半径按各骨最大位移保守放大，法向锥按骨旋转角保守放大），写回
  skin cache 供剔除/光栅/RT 三方消费（[调研1]）。
- **拒绝 UE5.5 权宜路线**：不做「CPU 蒙皮喂静态 cluster」——该路线击穿
  cluster 包围体与误差度量、吞吐低、有视角 LOD 瑕疵（[调研1]）。
- **bone shader 可编程 API**：美术可编程骨骼行为（植被风场等）以受限
  `.rx` kernel 形式接入骨骼求值阶段，语义需 RXS 条款（见 §9）
  （[调研2]）。
- **距离分级动画更新率**：按相机距离分档（全速 / 1/2 / 1/3 / 1/4），
  分级状态进场景表；更新率降级时保守包围体按最大未更新帧数放大，
  保证剔除不错杀（[调研2][调研5]）。
- **Morph target 旁路**：Morph 资产走非虚拟化传统 vertex path，与虚拟化
  路径在 GpuScene 中以 instance flag 区分，禁止混入 DAG（[调研1]）。

### 4.3 D1-c：误差驱动 LOD / cluster 选择

- 运行时 selection = 对 DAG 求屏幕空间误差 cut：投影簇误差 ≤ 阈值则选
  该簇，否则下降子簇；cut 上每簇恰好选中一次（无重叠无空洞）（[调研1]）。
- 选择结果与 **M44 页驻留**联动：选中簇若页未驻留 → 用父簇（更粗 LOD）
  兜底渲染 + 登记迟到页（沿 G8.4 迟到页降级语义，不重定）。
- selection 输出 = `VisibleClusterSet`（紧凑数组：cluster stable id + LOD
  level + 蒙皮版本 + 变换 id），**这是 D1-e 单源真相的数据载体**（[调研5]）。
- 静态几何路径保留 G8 现有两级剔除；蒙皮簇在剔除前注入保守包围体。

### 4.4 D1-d：CLAS 簇级 BLAS 与 RT 合流

- **主腿（NV CLAS）**：`VK_NV_cluster_acceleration_structure`：
  - 离线烘焙的 CLAS 输入随页流送；当帧用 multi-indirect device 构建把
    `VisibleClusterSet` 涉及的 CLAS 拼成 BLAS；静态重复几何用 Cluster
    Template 实例化共享底层 AS（[调研3]）。
  - BLAS 生命周期归 `AsManager` 单所有者扩展（新增 `ClasBlasKey` =
    可见簇集合 digest），沿用 G8 的显式策略 + `AsStats` 计数面纪律。
- **回退腿（非 CLAS 厂商）**：同一份 `VisibleClusterSet` 走传统
  triangles BLAS（按对象/按 LOD 段分组），保证 AMD/Intel 可跑、可对拍；
  回退腿是正确性基线，CLAS 腿是性能/VRAM 优化（[调研4]）。
- **验收导向**：CLAS 收益按 [调研3] 落地数据定义为 **VRAM 占用、CPU 构建
  耗时、AS 构建带宽**三维指标，不以 FPS 为唯一判据（4090 级 GPU 上
  Mega Geometry 几乎无帧率收益）。
- **DMM 禁止线**：位移走 WPO/tessellation 既有面（M05 决策另行判档），
  任何 micromap 提案直接拒绝（[调研4]）。

### 4.5 D1-e：VisBuffer / VSM 集成与单源真相

- `VisibleClusterSet` 一份三喂：
  1. **光栅**：VisBuffer 64 位格式（depth30|cluster27|tri7）不变，蒙皮簇
     经 skin cache 顶点进 SW/HW 双路，diff=0 判据维持；
  2. **RT**：D1-d 当帧 BLAS 拼装的输入数组直接由 selection 输出 memcpy/
     device copy 派生，禁止独立再算一遍可见性；
  3. **VSM**：阴影页标记用同一可见集 + 灯光视角 selection，蒙皮簇的阴影
     包围体与相机路径同源。
- 动画更新率分级同样作用于 AS 更新：降级的簇其 CLAS/BLAS 引用不变
  （静态帧零 AS 构建），只有全速更新的簇触发 refit/重拼（[调研2][调研5]）。
- 新增帧级 evidence 计数：`visible_clusters`、`clas_builds`、`blas_refit`、
  `anim_update_tier_histogram`、`vram_as_bytes`、`as_build_ms`——全部进
  evidence schema（§7）。

---

## 5. 关键设计决策表

| # | 决策点 | 选择 | 理由 | 调研引用 |
|---|---|---|---|---|
| D-1 | 蒙皮路线 | GPU cluster 感知蒙皮 + 离线预计算保守时空包围体；**拒绝** CPU 蒙皮喂静态 cluster | UE5.5 权宜路线击穿包围体/误差度量、吞吐低、无 Morph、有 LOD 瑕疵；Kerbl 2021 给出 LBS 保守包围严格解 | [调研1] |
| D-2 | Morph target | 非虚拟化旁路，不进 DAG | 无保守包围解；UE5.5 亦回避 | [调研1] |
| D-3 | 植被动画 | 全部骨骼绑定 + GPU 蒙皮 + bone shader 可编程 API + 距离分级更新率（全速/1/2/1/3/1/4） | Northlight 生产验证：30 万骨骼 GPU 蒙皮可行；分级更新率是性能主杠杆 | [调研2] |
| D-4 | LOD 选择 | monotonic 误差驱动 DAG cut + 页驻留联动（未驻留用父簇兜底） | Nanite 核心机制；与 G8 已冻结页流送 ABI 正交 | [调研1] |
| D-5 | RT 合流结构 | 离线烘焙 CLAS + 当帧 multi-indirect 拼装 BLAS + Cluster Template 实例化 | CLAS 可随资产烘焙；运行时构建带宽与 VRAM 大幅下降（AW2：VRAM −300MB） | [调研3] |
| D-6 | 厂商分层 | NV CLAS 为主腿；非 CLAS 传统 BLAS 回退腿为正确性基线；API 抽象按 DXR Part 2 厂商中立 CLAS 设计预留 | NV-only 现状，但 EXT 标准化可期；回退腿保证跨厂商可验收 | [调研4] |
| D-7 | DMM | 永久禁止 | NVIDIA 已归档 DMM，被 Mega Geometry 取代 | [调研4] |
| D-8 | 几何/RT 一致性 | `VisibleClusterSet` 单源真相：selection 一份输出同时喂光栅/RT/VSM + 动画分级 | 双世界错配是返工根源；分级更新使静态帧零 AS 构建 | [调研5] |
| D-9 | 验收指标 | VRAM 占用 + AS 构建耗时 + CPU 侧构建带宽为硬指标，FPS 仅观察项 | AW2 4090 几乎无帧率收益；收益在约束路径 | [调研3][调研5] |
| D-10 | DAG 构建算法 | 自研边塌缩 + 簇对锁定（与 nv_lod_cluster_builder 同源），不 vendoring NVIDIA 代码 | 许可与确定性双构建纪律；算法公开 | [调研3] |
| D-11 | 页格式演进 | 新增 cluster 属性走 RXPL 新 major，M04 v1 ABI 0-byte | G8 R-G8-4 反向依赖纪律 | `G8_PLAN.md:313` |
| D-12 | mesh shader | 不作为 D1 主线；kernel 组织预留 mesh shading 接缝 | M61 backfill 双条件未成立，G9 决策表独立判档 | `G8_P2_DECISIONS.md:33` |

---

## 6. 波次建议（融入 G9 整体波次的假设）

> **假设声明**：以下假设 G9 沿用 G8 波次范式（文档集 → 治理 → 实施波 →
> soak → close-out，`G8_PLAN.md:126-134`），且 D1 是 G9 首批模块之一。
> 实际波次切分以 G9_PLAN 立项裁决为准。

```text
G9.0 文档集（含本 DRAFT 评审）→ G9.1 治理（契约/RFC-00xx/决策表/验收映射/编号领取）
  → G9.2 D1-a 离线 DAG 构建管线深化（新 major 页格式 ABI 冻结，spec-first）
  → G9.3 D1-b GPU 蒙皮与骨骼植被（保守包围体 + 分级更新率）
  → G9.4 D1-c 误差驱动 LOD 选择（与流送联动）
  → G9.5 D1-d CLAS 簇级 BLAS（主腿 + 回退腿，依赖 G9.4 的 VisibleClusterSet）
  → G9.6 D1-e VisBuffer/VSM 集成 + 代表性动态场景 soak
```

依赖理由：D1-d 的消费方是 D1-c 的 selection 输出，不得抢跑（防 D1-d 用
静态全集拼装冒充合流）；D1-b 的保守包围体是 D1-c 蒙皮簇剔除正确性的
前置；新 major 页格式必须在 D1-a 波冻结，禁止后续波重定（D-11）。

---

## 7. 验收门草案（防假绿纪律：断言 + device 真跑 + golden + 负例 RED 臂）

> 门 key 命名沿 `g8.p{0,1}.m##.<slug>` 先例暂拟 `g9.d1.<子面>.<slug>`；
> evidence schema 名沿 `rurix.<域>.<名>.v1` 先例暂拟，立项时以 G9
> ACCEPTANCE_MAP 为准。每门均要求：device 真跑（RTX 4070 Ti 基线 +
> 非 CLAS 回退腿）+ golden 对拍 + 负例 RED 臂 + evidence 落
> `evidence/g9_d1_<slug>_<timestamp>.json`。

| 门 key（草案） | 断言 | device 真跑 | golden | 负例 RED 臂 | evidence schema（草案） |
|---|---|---|---|---|---|
| `g9.d1.dag_builder.error_monotonic` | DAG 每簇 `parent_error ≥ cluster_error` 逐边成立；双构建字节一致 | builder 对 ≥3 个真实资产（含 ≥1 骨骼资产）跑全量构建 | DAG 记录 + 误差表 byte golden；构建耗时进预算 | 人为破坏单调性的 fixture 必须被 builder 校验 typed Err 拒绝（fail-closed，无 UB） | `rurix.g9d1.dag_build.v1` |
| `g9.d1.pages_v2.abi_golden` | 新 major 页格式编解码往返无损；M04 v1 页 0-byte 兼容 | v2 页经 G8.4 streamer 真机消费（迟到页路径各 ≥1 次） | 页 byte golden + digest 表 | 篡改 schema_digest / section_digest 的页必须被拒 | `rurix.g9d1.pages_v2_codec.v1` |
| `g9.d1.skinning.conservative_bounds` | 任意姿态序列下，蒙皮后顶点 100% 落在保守包围球内；法向锥覆盖真实法向 | GPU 蒙皮 kernel 对骨骼植被资产（≥10 万骨等效规模可分批）逐帧真跑，与 CPU Kerbl 参照逐簇对拍 | 参照实现（host 纯 Rust）逐簇包围球/法向锥 golden | 故意缩小包围系数的 variant 必须产生可检测的剔除错杀断言失败（证明包围体真被剔除消费） | `rurix.g9d1.skin_bounds.v1` |
| `g9.d1.skinning.anim_tiers` | 距离分级更新率按档位生效；降级帧顶点缓冲逐位不变；恢复全速无跳变越界 | 相机场内距离扫描真跑，tier histogram 计数器非零采集 | 分级切换序列表 golden | 降级簇包围体未按未更新帧数放大的 variant 必须被一致性校验拒绝 | `rurix.g9d1.anim_tiers.v1` |
| `g9.d1.lod_select.cut_validity` | selection cut 无重叠无空洞（每叶路径恰好一簇）；未驻留页父簇兜底 | 相机推拉/旋转真跑，页驻留压力人为制造迟到页 | 逐帧选中簇 id 序列 golden（确定性相机路径） | 空洞（父子同选/都不选）注入 fixture 必须触发 selection 校验 RED | `rurix.g9d1.lod_cut.v1` |
| `g9.d1.clas_blas.merge_parity` | CLAS 腿与回退腿对同一场景 ray query 结果逐命中一致（任意 hit 集合 + 最近 hit 距离容差 0） | 4070 Ti 上 CLAS 主腿 + 强制回退腿双跑对拍；VRAM/构建耗时双指标落 evidence | 命中流 digest golden（沿 G7 RayQuery 对拍体例） | 可见集与 BLAS 输入故意错开一簇的 variant 必须被帧末一致性校验 RED（单源真相防假绿核心） | `rurix.g9d1.clas_parity.v1` |
| `g9.d1.clas_blas.budget` | `vram_as_bytes` ≤ 预算；`as_build_ms` 中位/尾帧 ≤ 预算；静态帧零 AS 构建 | 代表性场景 soak（阈值立项时按 G8.8a 量级硬化）真跑采集 | 预算基线 JSON（立项时 measured，禁手写） | 静态帧出现非零 `clas_builds`/`blas_refit` 即 RED | `rurix.g9d1.as_budget.v1` |
| `g9.d1.visbuffer_integration.diff_zero` | 蒙皮簇 VisBuffer SW/HW 逐像素 diff=0 维持；VSM 页标记与可见集同源 | 动态场景全帧 SW/HW 双路真跑 | 像素级 golden（沿 G8.5a 体例） | 旁路单源真相（RT/VSM 独立再算可见性）的 variant 必须被 provenance 校验 RED | `rurix.g9d1.vis_integration.v1` |

**横切纪律**（沿 G8 体例）：
- 任一门的 RED 臂先行（spec-first + RED 先行），条款 PR 先于实现 PR；
- 条件实现刚绿不得当日进 close-out，必经 soak；
- 预算 JSON 非空、零 estimated/skip 方可 close；
- 编号（RXS/CI step/U/RX）立项时按实测 `next_free` 领取，禁止沿用本文任何数字。

---

## 8. 风险与止损

| ID | 风险 | 预警 | 止损 |
|---|---|---|---|
| R-D1-1 | `VK_NV_cluster_acceleration_structure` 长期 NV-only，跨厂商承诺落空 | G9 中期 Khronos EXT 仍无草案 | 回退腿升为主交付；CLAS 腿降级为可选 fast path，验收门 `clas_blas.merge_parity` 以回退腿为准 |
| R-D1-2 | Kerbl 保守包围过松 → 蒙皮簇剔除/LOD 效率崩 | 实测包围体膨胀比 > 阈值导致 cluster 数暴涨 | 按骨骼数分桶收紧（每簇限骨数 builder 约束）；仍不达标则蒙皮簇退化为逐对象 LOD 并诚实登记 |
| R-D1-3 | 页格式 v2 演进拖累 M44 streamer | D1-a 波重定 v1 ABI 的提案出现 | D-11 硬禁：v1 0-byte，v2 新 major；互锁 validator 阻断 |
| R-D1-4 | 双世界错配（RT 与光栅各算可见性）悄然引入 | 帧末一致性校验缺位或旁路 | `clas_blas.merge_parity` 负例 RED 臂为硬门；provenance 校验进 CI |
| R-D1-5 | 动画分级造成视觉跳变/阴影闪烁 | soak 帧间差分超阈 | 分级阈值表进调参预算；跳变超阈档位回退更保守更新率 |
| R-D1-6 | 以 FPS 为唯一指标导致 CLAS 假绿 | 验收叙述只提帧率 | D-9：VRAM/构建耗时/CPU 带宽为硬指标，FPS 仅观察 |
| R-D1-7 | DMM/micromap 提案借「位移需求」复活 | 任何 micromap 字样进 RFC | D-7 永久禁止线，G9 治理评审一票否决 |
| R-D1-8 | 骨骼资产制作管线缺位（无真实骨骼植被资产可验收） | G9.3 前无代表性资产 | G9.1 治理波冻结资产制作/采购计划；程序生成骨骼资产仅作过渡、验收须含真实美术资产 |

---

## 9. spec / RFC 需求（草案）

需 RXS 条款（spec-first，编号立项时领取）：

| 面 | 条款内容 | 建议落点 |
|---|---|---|
| 页格式 v2 | RXPL major=2：簇误差/包围球/骨骼元数据/CLAS 输入段布局、新 schema preimage、与 v1 共存律 | `spec/geometry_pages.md` 追加新章（v1 条款 0-byte） |
| 蒙皮语义 | LBS 蒙皮 kernel 的确定性律（骨骼 palette 读取序、浮点累加序）、skin cache 布局与失效律 | `spec/shader_stages.md` 或新建 `spec/skinned_geometry.md` |
| 保守包围体 | Kerbl 保守界公式冻结（膨胀系数定义、法向锥放大律）、降级帧放大律 | 同上 |
| LOD 选择 | 屏幕空间误差阈值语义、cut 无重叠无空洞不变式、未驻留父簇兜底律 | 新建 `spec/virtual_geometry.md`（候选） |
| bone shader API | 美术可编程骨骼 kernel 的受限语义面（可访问资源闭集、禁副作用面、capability 键） | `spec/shader_stages.md` + capability 闭集扩展 |
| RT 合流 | `VisibleClusterSet` 单源真相契约（生产者/消费者、帧内一致性校验义务）、CLAS 构建输入页内布局、回退腿等价性定义 | `spec/rendering_platform.md` 追加 RP-GEO-RT diff key 章 |

需 RFC（G9.1 治理波立项，沿用伞形 RFC 先例）：

| RFC（暂名） | 内容 |
|---|---|
| G9 RFC 渲染建造期-几何篇 | 页格式 v2 语义、蒙皮/包围体/LOD 语义面、bone shader 可编程面、单源真相契约、多队列若涉 AS 异步构建的 ownership 修订（触 G5 Barrier 冻结面须显式修订行） |
| G9 RFC 资产管线修订 | builder 新产物（CLAS 输入、骨骼元数据）进 SourceAsset/Recipe/Artifact 图式与 DDC key 组成 |
| 决策表登记 | RD-039 M06/M09 分项 open-defer → G9 承接的 history 只追加行；M61 mesh shader、M03 HZB 的 G9 决策表独立判档 |

---

## 10. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v0.1 | G8 close-out 后 | 首版 DRAFT：承接 M06/M09 defer 锚；五子面分解；12 条设计决策；8 门验收草案；8 条风险止损；spec/RFC 需求清单。G9 未立项，零契约效力。 |
