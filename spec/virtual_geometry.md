# virtual_geometry.md — 虚拟化几何 DAG 深化语义面（G9.2 M90）

> **地位**：虚拟化几何 cluster DAG 深化（误差度量 / 簇对锁定 / 蒙皮元数据 / CLAS
> 烘焙输入）语义事实源之一（RFC-0022 §4.1，Agent Approved 2026-08-09；
> G9_ACCEPTANCE_MAP §2 M90 行）。G8 已冻结的静态 meshlet DAG 底座
> （spec/geometry_pages.md RXS-0328~0342 / spec/asset_pipeline.md RXS-0332~0337）
> **字面 0-byte 不动**；本文件只承载 G9.2 深化新增语义。
>
> **档位**：Full RFC / RFC-0022。
>
> **编号**：RXS-0345（G9.2 spec-first，自合入时 `registry/number_ledger.json` 实测
> `RXS.next_free = 344` 顺位领取之本批第二号；编号永不复用，10 §9.5）。
>
> **新建裁决留痕（G9.2 spec PR）**：RFC-0022 §5 条款映射表把「DAG 误差度量
> monotonic 不变式与簇对锁定语义」的目标 spec 冻结为「新建 `spec/virtual_geometry.md`
> （候选）」；本 PR 裁定**新建本文件**（render_graph.md / rendering_platform.md
> 新建先例，spec/README.md v1.65/v1.70 行）——DAG 深化语义与页格式 ABI
> （geometry_pages.md）、资产管线 schema（asset_pipeline.md）均不同轴，独立成文。

---

## 1. 范围与体例

- 体例 = FLS 风格（spec/README.md §2）；本文件**严禁 UB 节**——builder 侧所有
  失败均为 typed `Err`（fail-closed），不设未定义行为。
- 实现锚定：`src/rurix-asset::geom_build`（`dag.rs` builder 深化面，host 纯 Rust，
  `forbid(unsafe_code)` 纪律维持，RFC-0022 §4.1 逐字）+ `src/rurix-geom-pages`
  v2 编解码消费面（RXS-0344）。
- 每条款 ≥1 `//@ spec: RXS-####` 测试锚定（traceability 矩阵全锚定，10 §4）。

## 2. 术语

- **cluster DAG**：G8 `ClusterDag`（`src/rurix-geom-build`）记录的簇层级有向无环图；
  节点 = 簇（stable id 全局唯一且构建期稳定），边 = parent→child LOD 细化关系。
- **parent_error / cluster_error**：每簇记录的几何近似误差标量（`f32`），配合
  包围球供运行时求屏幕空间误差 cut。
- **簇对锁定（cluster pair locking）**：边塌缩时锁定相邻簇边界、避免 LOD 裂缝的
  builder 约束（术语引自 D1 草案转述，RFC-0022 §4.1 / R4 U4 口径）。
- **CLAS**：cluster-level acceleration structure（`VK_NV_cluster_acceleration_structure`
  语义集）；本文件只冻结其**离线烘焙输入字段 schema**，运行时拼装语义归后续波次。

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

## 4. 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| v1.0 | 2026-08-09 | 新建（G9.2 spec-first，M90）：RXS-0345 DAG 误差 monotonic 不变式（`parent_error ≥ cluster_error` 逐边；破坏者 builder typed `Err` fail-closed）+ 簇对锁定语义（独立自研，不 vendoring nv 代码，RFC-0022 §4.1 D-10）+ 蒙皮元数据字段（最大影响骨数/骨骼索引集/包围体膨胀系数）与 CLAS 离线烘焙输入字段（三角形簇+簇级 AABB）schema。依据 [RFC-0022](../rfcs/0022-virtual-geometry-gi-semantics.md)（Agent Approved 2026-08-09）§4.1/§5 + G9_ACCEPTANCE_MAP M90 行。G8 底座条款 RXS-0328~0342 字面 0-byte | **Full RFC**（RFC-0022） |
