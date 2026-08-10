# virtual_geometry.md — 虚拟化几何 DAG 深化与 RT 合流语义面（G9.2 M90 / G9.3 M93~M95）

> **地位**：虚拟化几何 cluster DAG 深化（误差度量 / 簇对锁定 / 蒙皮元数据 / CLAS
> 烘焙输入）**与几何×RT 合流（VisibleClusterSet 运行时 selection cut / CLAS×RT
> 合流 / 单源真相 provenance）** 语义事实源之一（RFC-0022 §4.1/§4.2/§4.3/§4.4，
> Agent Approved 2026-08-09；G9_ACCEPTANCE_MAP §2 M90/M93/M94/M95 行）。G8 已冻结的
> 静态 meshlet DAG 底座（spec/geometry_pages.md RXS-0328~0342 /
> spec/asset_pipeline.md RXS-0332~0337）**字面 0-byte 不动**；本文件只承载 G9.2
> 深化新增语义与 G9.3 合流新增语义。
>
> **档位**：Full RFC / RFC-0022。
>
> **编号**：RXS-0345（G9.2 spec-first，自合入时 `registry/number_ledger.json` 实测
> `RXS.next_free = 344` 顺位领取之本批第二号）+ RXS-0350~0352（G9.3 spec-first，
> 自合入时实测 `RXS.next_free = 350` 顺位领取，0350~0352 连续不跳号；
> 编号永不复用，10 §9.5）。
>
> **新建裁决留痕（G9.2 spec PR）**：RFC-0022 §5 条款映射表把「DAG 误差度量
> monotonic 不变式与簇对锁定语义」的目标 spec 冻结为「新建 `spec/virtual_geometry.md`
> （候选）」；本 PR 裁定**新建本文件**（render_graph.md / rendering_platform.md
> 新建先例，spec/README.md v1.65/v1.70 行）——DAG 深化语义与页格式 ABI
> （geometry_pages.md）、资产管线 schema（asset_pipeline.md）均不同轴，独立成文。
>
> **目标 spec 合并裁决留痕（G9.3 spec PR）**：RFC-0022 §5 条款映射表中
> 「误差驱动 cut 无重叠无空洞不变式、未驻留父簇兜底律」的候选目标 spec 为
> 「同上（= 本文件）」，「`VisibleClusterSet` 单源真相契约」与「CLAS 构建输入页内
> 布局、当帧拼装语义、回退腿等价性定义」的候选目标 spec 为
> `spec/rendering_platform.md`；G9.3 波裁定**三条全部合并落本文件**——三条均为
> 虚拟化几何运行时合流语义，与 RXS-0345 同轴同卷，rendering_platform.md 只经
> spec/README.md §4 行登记、文件本体 0-byte（候选目标 spec 的合并裁决沿 G9.2
> 新建裁决先例，在本头注留痕）。**CLAS 页内字段布局不重定**：CLAS 离线烘焙输入
> 字段 schema 已由 RXS-0345 L4（三角形簇 + 簇级 AABB，随 RXPL major=2 新段打包，
> RXS-0344 消费）冻结，RXS-0351 只冻结**运行时当帧拼装语义与双腿等价性**，页内
> 布局面 0-byte 复用不重复定义。**`rt.clas` 不入本波**：capability ID（拟
> `rt.clas`，符号名非 vendor extension 名）触 `spec/shader_stages.md` RXS-0311
> capability ID 闭集加性修订行义务（F-1 先例：入集必经该条款加性修订行，禁静默
> 扩），RXS-0349 修订行（G9.2 波）**未含 `rt.clas`**；本波先冻结双腿运行时合流
> 语义与装配期换腿禁止线（RXS-0351 L7 装配期不变量），**语言层 capability 门控
> 面随实现波经 RXS-0311 加性修订行落 spec/shader_stages.md**，本波条款不重复
> 冻结、不冲突。

---

## 1. 范围与体例

- 体例 = FLS 风格（spec/README.md §2）；本文件**严禁 UB 节**——builder 侧与
  运行时 selection/provenance 校验侧所有失败均为 typed `Err` / 确定性拒绝
  （fail-closed），不设未定义行为。
- 实现锚定（G9.2 面）：`src/rurix-asset::geom_build`（`dag.rs` builder 深化面，
  host 纯 Rust，`forbid(unsafe_code)` 纪律维持，RFC-0022 §4.1 逐字）+
  `src/rurix-geom-pages` v2 编解码消费面（RXS-0344）。
- 实现锚定（G9.3 面）：`src/rurix-render::geometry::cull`（VisibleClusterSet
  生产者，G8 两级剔除 + LOD cut host 参照底座）+ `src/rurix-render::rt::as_manager`
  （AS 单所有者扩展：`ClasBlasKey` / 计数面）+ `src/rurix-render::streaming`
  （页驻留联动）+ host 侧纯 safe 帧末 provenance 校验模块（实现期命名）。
- 每条款 ≥1 `//@ spec: RXS-####` 测试锚定（traceability 矩阵全锚定，10 §4）。

## 2. 术语

- **cluster DAG**：G8 `ClusterDag`（`src/rurix-geom-build`）记录的簇层级有向无环图；
  节点 = 簇（stable id 全局唯一且构建期稳定），边 = parent→child LOD 细化关系。
- **parent_error / cluster_error**：每簇记录的几何近似误差标量（`f32`），配合
  包围球供运行时求屏幕空间误差 cut。
- **簇对锁定（cluster pair locking）**：边塌缩时锁定相邻簇边界、避免 LOD 裂缝的
  builder 约束（术语引自 D1 草案转述，RFC-0022 §4.1 / R4 U4 口径）。
- **CLAS**：cluster-level acceleration structure（`VK_NV_cluster_acceleration_structure`
  语义集）；离线烘焙**输入字段 schema** 见 RXS-0345 L4；运行时当帧拼装语义与
  双腿等价性见 RXS-0351。
- **VisibleClusterSet**：当帧几何可见性的**唯一事实源**——由 DAG 误差 cut 产生的
  紧凑数组，元素 = cluster stable id + LOD level + 蒙皮版本 + 变换 id
  （RFC-0022 §4.4 逐字）；光栅/RT/VSM 三消费者不各自重算可见性
  （RFC-0022 §4.0-1 / D-8）。
- **selection cut**：DAG 上按屏幕空间误差阈值选中的簇集合；**无重叠无空洞** =
  每条根→叶路径恰好被一个选中簇覆盖（父子同选 = 重叠，父子同不选且路径截断
  = 空洞）。
- **双腿（two legs）**：CLAS 主腿（NV cluster acceleration structure 路径）与
  传统 BLAS 回退腿（非 CLAS 厂商路径）；**回退腿是正确性基线，CLAS 腿是
  性能/VRAM 优化腿**（RFC-0022 §4.3 / D-6 逐字）。
- **Cluster Template 实例化**：静态重复几何以模板共享底层 AS，实例仅携带变换
  （D-5，[调研3] 转述）。
- **provenance 链**：消费者输入 digest ↔ `VisibleClusterSet` digest 的帧末精确
  一致校验链（M95 硬门负例轴，RFC-0022 §4.4「帧末 provenance 校验」逐字）。

---

## 3. 条款（RXS-0345）

### RXS-0345 DAG 误差 monotonic 不变式、簇对锁定语义与蒙皮/CLAS 烘焙字段 schema

**Legality**

1. **monotonic 不变式**（RFC-0022 §4.1 逐字）：每簇记录 `parent_error` /
   `cluster_error`；DAG **每一边** `parent_error ≥ cluster_error` **逐边**成立。
   builder 校验发现破坏单调性的输入/中间态必须 **typed `Err` 拒绝**（fail-closed，
   无 UB）——不得静默继续、不得 clamp 修复后照常出页。
2. **簇对锁定语义**（RFC-0022 §4.1 / D-10 逐字）：边塌缩时锁定相邻簇边界，避免
   LOD 裂缝。算法与 nv_lod_cluster_builder 同源但**独立自研实现，不 vendoring
   NVIDIA 代码**（许可与确定性双构建纪律）。
3. **蒙皮元数据字段 schema**（RFC-0022 §4.1/§4.2 逐字，每簇，随页烘焙）：
   **最大影响骨数**、**骨骼索引集**、**蒙皮包围体膨胀系数**（Kerbl 保守界所需
   输入）。三字段为 builder 输出的强制面；缺任一字段的骨骼资产 = builder typed
   `Err` 拒录（fail-closed）。
4. **CLAS 离线烘焙输入字段 schema**（RFC-0022 §4.1 逐字）：builder 输出每簇 CLAS
   构建输入 = **三角形簇 + 簇级 AABB**，随资产页打包（经 RXPL major=2 新段，
   RXS-0344）；运行时 CLAS 构建退化为 device 侧拼装而非几何处理。
5. **确定性与非 stable 边界**（RFC-0022 §4.0-3/§4.0-5）：同一资产 + 同一 builder
   版本**双构建字节一致**（沿 M79 判据）；CLAS/BLAS 物理字节、device address、
   driver 构建耗时数值为实现确定、**非 stable**；误差语义、字段 schema、
   monotonic 校验规则由本条款冻结。

**Implementation Requirements**

- 实现锚定 `src/rurix-asset::geom_build::dag`（单调性校验 typed `Err` 拒录臂 +
  蒙皮元数据/CLAS 输入字段产出）与 `src/rurix-geom-pages` v2 段编解码（RXS-0344
  消费）；纯 safe，零新 unsafe。
- RED 锚定计划（实现 PR 落）：单调性破坏 fixture → builder typed `Err` RED；
  双构建 byte golden。
- 本 spec PR 先行落最小 RED 锚定占位语料
  `conformance/virtual_geometry/reject/dag_error_nonmonotonic.rx`（条款锚定占位，
  inert 锚定口径与转正路径见该文件头注释）；锚点目标文件（实现 PR 转正）=
  `src/rurix-asset/src/geom_build/dag.rs` 校验拒录臂单测。

---

## 4. 条款（RXS-0350，G9.3 M93 VisibleClusterSet）

### RXS-0350 VisibleClusterSet 载荷、屏幕空间误差 selection cut 覆盖性与未驻留页父簇兜底律

**Legality**

1. **载荷冻结**（RFC-0022 §4.4 逐字）：`VisibleClusterSet` = 紧凑数组，元素 =
   **cluster stable id + LOD level + 蒙皮版本 + 变换 id**；由 RXS-0345 DAG 的
   误差 cut 产生。
2. **selection cut 覆盖性不变式**（RFC-0022 §4.0-2/§4.4；判据逐字引
   G9_ACCEPTANCE_MAP §2 M93 行）：固定多视图场景 `VisibleClusterSet` 的**屏幕
   空间误差 selection cut 逐帧无重叠无空洞**（每条根→叶路径恰好一个选中簇，
   覆盖性机器核验）；**输出 digest 与 golden 全等**。覆盖性破坏 =
   fail-closed 确定性拒绝，不得静默继续、不得 clamp 修复（§4.0-2 strict-only）。
3. **未驻留页父簇兜底律**（RFC-0022 §4.4 逐字；判据逐字引 G9_ACCEPTANCE_MAP §2
   M93 行）：**强制未驻留页时命中父簇兜底、页到达后转为正确内容**——沿 G8.4
   迟到页降级语义（spec/geometry_pages.md RXS-0328~0342 底座面）**不重定**；
   兜底与转正过程必须有 evidence 可机核。
4. **空洞注入负例（RED 臂）**（判据逐字引 G9_ACCEPTANCE_MAP §2 M93 行）：
   **空洞注入负例 RED 臂独立有效**——向选中簇集合注入空洞（或父子同选重叠）
   的 variant 必须被 selection 覆盖性校验判 RED，且该负例臂**独立于正例臂**
   成立（负例臂失效即本条款整体 FAIL，沿 G7 对拍门负例先例）。
5. **静态 LOD cut 不充数**（判据逐字引 G9_ACCEPTANCE_MAP §2 M93 行）：**静态
   LOD cut 无运行时误差驱动的旧输出不能充绿**——selection 必须当帧由屏幕空间
   误差驱动产生；以构建期固定 LOD cut（无运行时误差驱动路径）的输出冒充本条款
   判据产物 = FAIL。

**Implementation Requirements**

- 实现锚定 `src/rurix-render::geometry::cull`（selection cut 生产者，G8 两级
  剔除 + LOD cut host 参照底座扩写）+ `src/rurix-render::streaming`（页驻留
  联动：未驻留判定、父簇兜底登记、迟到页转正）；host 侧纯 safe，零新 unsafe
  方向（unsafe 确需时按当时 `U.next_free` 实测顺位登记 unsafe-audit）。
- RED 锚定计划（实现 PR 落）：空洞注入 variant → selection 校验 RED；逐帧选中
  簇 id 序列 golden；输出 digest golden 全等。
- 本 spec PR 先行落最小 RED 锚定占位语料
  `conformance/virtual_geometry/reject/selection_cut_hole_injected.rx`（条款锚定
  占位，inert 锚定口径与转正路径见该文件头注释）；锚点目标（实现 PR 转正）=
  selection cut 覆盖性校验空洞注入负例（`ci/g9_visible_cluster_set_smoke.py` 门，
  symbolic key `g9.p0.m93.visible_cluster_set`，G9.1 冻结字面 0-byte 不动）。

---

## 5. 条款（RXS-0351，G9.3 M94 CLAS×RT 合流）

### RXS-0351 CLAS 当帧拼装语义、Cluster Template 实例化与双腿 ray query 逐命中一致等价性

**Legality**

1. **CLAS 主腿当帧拼装**（RFC-0022 §4.3 逐字）：离线烘焙的 CLAS 输入
   （RXS-0345 L4：三角形簇 + 簇级 AABB）随页流送；**当帧用 multi-indirect
   device 构建把 `VisibleClusterSet` 涉及的 CLAS 拼成 BLAS**；静态重复几何用
   **Cluster Template 实例化**共享底层 AS（D-5）。运行时 CLAS 构建 = device
   侧**拼装**，不做几何处理（RXS-0345 L4 既冻结面复用）。
2. **回退腿与逐命中一致等价性**（RFC-0022 §4.3 / D-6 逐字；判据逐字引
   G9_ACCEPTANCE_MAP §2 M94 行）：同一份 `VisibleClusterSet` 走传统 triangles
   BLAS（按对象/按 LOD 段分组）为**正确性基线**，CLAS 腿为性能/VRAM 优化腿；
   **同一场景 CLAS 主腿与传统 BLAS 回退腿 ray query 逐命中一致**——任意 hit
   集合 + 最近 hit 距离**容差 0**，命中流 digest golden 沿 G7 RayQuery 对拍
   体例；**回退腿为正确性基线，两条腿各自独立 evidence**。
3. **错开一簇负例（RED 臂）**（判据逐字引 G9_ACCEPTANCE_MAP §2 M94 行）：
   **可见集与 BLAS 内容错开一簇的注入必须判 RED**（与 RXS-0352 L3 provenance
   校验同一负例轴）。
4. **静态帧零 AS 构建**（RFC-0022 §4.3「验收指标语义」/ §4.4 逐字；判据逐字引
   G9_ACCEPTANCE_MAP §2 M94 行）：**静态帧零 AS 构建（构建计数非零即 RED）**——
   降级簇 CLAS/BLAS 引用不变，只有全速更新簇触发 refit/重拼。
5. **拼装 digest golden 与 validation 纪律**（判据逐字引 G9_ACCEPTANCE_MAP §2
   M94 行）：**Cluster Template 实例化与当帧 multi-indirect 拼装 digest 等于
   golden；validation error=0**。
6. **BLAS 生命周期单所有者**（RFC-0022 §4.3 逐字）：归 `AsManager` 单所有者
   扩展——新增 `ClasBlasKey` = 可见簇集合 digest，沿用 G8 显式策略 + `AsStats`
   计数面纪律。
7. **构建/装配期换腿纪律（不变量冻结，capability 门控留实现波）**：两条腿都是
   manifest 显式 variant，选择发生在**构建/装配期**（RFC-0022 §4.3 逐字）；
   运行期发现 device 不满足所选腿 → **装载 fail-closed**，**禁止运行时发现不
   支持后静默换腿**（RFC-0022 §4.3 逐字）。语言层 capability ID（拟 `rt.clas`）
   门控面**随实现波经 RXS-0311 加性修订行**落 spec/shader_stages.md（文件头注
   裁决留痕），本条款不重复冻结。
8. **DMM 禁止线**（RFC-0022 §4.3 / D-7 逐字）：位移走 WPO/tessellation 既有面；
   **任何 micromap 提案直接拒绝**（NVIDIA 官方归档原文，R4 §5.4）。
9. **非 stable 边界**（RFC-0022 §4.0-5）：CLAS/BLAS 物理字节、device address、
   driver 构建耗时数值为实现确定、**非 stable**（RXS-0345 L5 同口径延伸至
   运行时拼装产物）。

**Implementation Requirements**

- 实现锚定 `src/rurix-render::rt::as_manager`（`ClasBlasKey` 单所有者扩展 +
  构建计数面）+ `src/rurix-rt::vk`（CLAS/multi-indirect/Cluster Template FFI
  面，U 号按当时实测 `U.next_free` 顺位登记 unsafe-audit，沿 U54 先例）+
  回退腿传统 BLAS 分组路径（G8 AS 管理底座扩写）。
- RED 锚定计划（实现 PR 落）：可见集/BLAS 错开一簇注入 → RED；静态帧非零
  构建计数 → RED；双腿命中流 digest golden（距离容差 0）。
- 本 spec PR 先行落最小 RED 锚定占位语料
  `conformance/virtual_geometry/reject/clas_blas_cluster_mismatch.rx`（条款锚定
  占位，inert 锚定口径与转正路径见该文件头注释）；锚点目标（实现 PR 转正）=
  可见集/BLAS 错开一簇负例（`ci/g9_clas_rt_convergence_smoke.py` 门，symbolic
  key `g9.p0.m94.clas_rt_convergence`，G9.1 冻结字面 0-byte 不动）。

---

## 6. 条款（RXS-0352，G9.3 M95 单源真相）

### RXS-0352 VisibleClusterSet 单源真相：一份三喂 provenance 链、双世界否决与蒙皮簇 VisBuffer diff=0 维持

**Legality**

1. **一份三喂**（RFC-0022 §4.0-1/§4.4 / D-8 逐字；判据逐字引
   G9_ACCEPTANCE_MAP §2 M95 行）：`VisibleClusterSet` 是当帧几何可见性的**唯一
   事实源**，**一份三喂**——
   - **光栅**：VisBuffer 64 位格式（`depth30|cluster27|tri7`）**不变**，蒙皮簇经
     skin cache 顶点进 SW/HW 双路；
   - **RT**：当帧 BLAS 拼装的输入数组**直接由 selection 输出 memcpy/device copy
     派生**，**禁止独立再算一遍可见性**；
   - **VSM**：阴影页标记用同一可见集 + 灯光视角 selection，蒙皮簇阴影包围体与
     相机路径同源。
2. **蒙皮簇 VisBuffer diff=0 维持**（判据逐字引 G9_ACCEPTANCE_MAP §2 M95 行）：
   **蒙皮簇 VisBuffer SW/HW diff=0 维持**——G7.5b RXS-0303 对拍判据（整数域
   零容差）在蒙皮簇路径上**不减损、不重定**。
3. **帧末 provenance 校验与旁路负例（硬门 RED 臂）**（RFC-0022 §4.4 逐字；判据
   逐字引 G9_ACCEPTANCE_MAP §2 M95 行）：**`VisibleClusterSet` 一份三喂光栅/RT/
   VSM 的 provenance 链完整可机核**——帧末校验三方消费者输入 digest 与
   `VisibleClusterSet` digest **精确一致**；**旁路单源真相的 variant 必须被
   provenance 校验判 RED（负例臂为硬门，R-G9-8）**。
4. **双世界结构否决**（RFC-0022 §4.0-1 / D-8；判据逐字引 G9_ACCEPTANCE_MAP §2
   M95 行）：光栅/RT **各自独立计算可见性的双世界结构即使出图相似也判
   FAIL**——单源真相是**结构**判据不是出图相似判据，不得以出图相似充绿。
5. **动画分级作用于 AS 更新**（RFC-0022 §4.4 逐字；判据逐字引
   G9_ACCEPTANCE_MAP §2 M95 行）：**动画分级作用于 AS 更新、静态帧零 AS
   构建**——降级簇 AS 引用不变，全速簇触发 refit/重拼（与 RXS-0351 L4 同口径）。
6. **帧末一致性断言与 validation 纪律**（判据逐字引 G9_ACCEPTANCE_MAP §2 M95
   行）：**帧末一致性校验断言全真、validation error=0**。

**Implementation Requirements**

- 实现锚定 `src/rurix-render::geometry::cull`（生产者单源）+
  `src/rurix-render::rt::as_manager`（RT 消费者由 selection 输出派生输入）+
  host 侧纯 safe 帧末 provenance 校验模块（三喂 digest 一致性断言；实现期命名，
  沿 `render_exec` 纯 safe 先例）。
- RED 锚定计划（实现 PR 落）：旁路单源真相 variant → provenance 校验 RED
  （负例臂为硬门）；三喂 digest 精确一致 golden；帧末一致性断言全真。
- 本 spec PR 先行落最小 RED 锚定占位语料
  `conformance/virtual_geometry/reject/bypass_single_source_variant.rx`（条款锚定
  占位，inert 锚定口径与转正路径见该文件头注释）；锚点目标（实现 PR 转正）=
  旁路单源真相 variant provenance 负例（`ci/g9_single_source_truth_smoke.py` 门，
  symbolic key `g9.p0.m95.single_source_truth`，G9.1 冻结字面 0-byte 不动）。

---

## 7. 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| v1.0 | 2026-08-09 | 新建（G9.2 spec-first，M90）：RXS-0345 DAG 误差 monotonic 不变式（`parent_error ≥ cluster_error` 逐边；破坏者 builder typed `Err` fail-closed）+ 簇对锁定语义（独立自研，不 vendoring nv 代码，RFC-0022 §4.1 D-10）+ 蒙皮元数据字段（最大影响骨数/骨骼索引集/包围体膨胀系数）与 CLAS 离线烘焙输入字段（三角形簇+簇级 AABB）schema。依据 [RFC-0022](../rfcs/0022-virtual-geometry-gi-semantics.md)（Agent Approved 2026-08-09）§4.1/§5 + G9_ACCEPTANCE_MAP M90 行。G8 底座条款 RXS-0328~0342 字面 0-byte | **Full RFC**（RFC-0022） |
| v1.1 | 2026-08-10 | 加性扩写（G9.3 spec-first，几何×RT 合流波 M93/M94/M95，硬规则 7 条款先行）：RXS-0350（VisibleClusterSet 载荷 + 屏幕空间误差 selection cut 逐帧无重叠无空洞覆盖性 + 未驻留页父簇兜底〔沿 G8.4 迟到页降级语义不重定〕+ 空洞注入 RED 臂独立有效 + 静态 LOD cut 不充数）/ RXS-0351（CLAS 当帧 multi-indirect 拼装 + Cluster Template 实例化 + BLAS 回退腿逐命中一致〔容差 0〕+ 错开一簇 RED + 静态帧零 AS 构建 + `ClasBlasKey` 单所有者 + 构建/装配期换腿纪律 + DMM 禁止线）/ RXS-0352（一份三喂光栅/RT/VSM provenance 链 + 旁路 variant provenance RED 硬门〔R-G9-8〕+ 双世界结构即使出图相似也判 FAIL + 蒙皮簇 VisBuffer SW/HW diff=0 维持 + 动画分级作用于 AS 更新）。**目标 spec 合并裁决**：RFC-0022 §5 映射表「单源真相契约」与「CLAS 当帧拼装语义/回退腿等价性」候选目标 spec=rendering_platform.md，本波裁定合并落本文件（与 RXS-0345 同轴同卷），rendering_platform.md 本体 0-byte；**CLAS 页内字段布局不重定**（RXS-0345 L4 已冻结，RXS-0351 只冻结运行时拼装语义与双腿等价性）；**`rt.clas` 语言层 capability 门控随实现波经 RXS-0311 加性修订行落 shader_stages.md**（RXS-0349 修订行未含 `rt.clas`，本波不重复冻结）。条款号自 ledger 实测 `RXS.next_free=350` 顺位领取（0350~0352 连续不跳号）。conformance 最小 RED 锚定占位语料三件（inert + `//@ spec` 锚定 + 预期诊断注释 + 转正路径旁注）同 PR 落；symbolic key `g9.p0.m93/m94/m95` 与 ci 脚本名 G9.1 冻结字面 0-byte 不动；零 src/ 改动、零 workflow 步骤、零新 RX 码、零新 U/RD/SG。依据 [RFC-0022](../rfcs/0022-virtual-geometry-gi-semantics.md)（Agent Approved 2026-08-09）§4.0/§4.2/§4.3/§4.4 + G9_ACCEPTANCE_MAP §2 M93/M94/M95 行（判据逐字）+ G9_CONTRACT §4.2 M93/M94/M95 行。既有条款 RXS-0345 字面 0-byte | **Full RFC**（RFC-0022） |
