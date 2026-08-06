# geometry_pages.md — 几何页逻辑 ABI（G8.3 M01）

> **地位**：几何流送页格式语义事实源之一（RFC-0020 §4.9；G8_ACCEPTANCE_MAP §2 M01）。
> 本文件首批冻结 **逻辑页（未压缩 builder artifact）** ABI；磁盘/内存双 ABI（M04）
> 条款另批追加。
>
> **档位**：Full RFC / RFC-0020（字面值与逐字段偏移由本文件一次性冻结并同 PR 落
> byte golden；评审 F7）。
>
> **编号**：RXS-0328 ~ RXS-0331（主 agent 预留区间；合入时 ledger 校准）。

---

## 1. 范围与体例

- 体例 = FLS 风格（spec/README.md §2）；本文件**严禁 UB 节**——失败均为 typed `Err`。
- 实现锚定：`src/rurix-geom-pages`（逻辑页 encode/decode）、`src/rurix-asset::geom_build`
  （`ClusterDag → Vec<LogicalPage>` 确定性装箱 + `rxgb_to_pages`）。
- 每条款 ≥1 `//@ spec: RXS-####` 测试锚定。
- 流送页大小上限单源引用：`rurix_render::graph::types::STREAM_PAGE_SIZE = 128×1024`；
  builder 侧字面复述同一数值，改值 = 升版。

## 2. 术语

- **逻辑页（RXPL）**：未压缩的 builder 产物页；magic `"RXPL"`。
- **stable id**：`ClusterDag::records` 下标（`u32`），全局唯一且构建期稳定。
- **root 页**：含顶层 LOD cut（`ClusterDag::top_level_ids`）任一簇的页；`flags` bit0=1。
- **schema_digest**：装箱/编码参数闭集的 SHA-256；改装箱策略或字段布局 = 新 digest / 升版。
- **section_digest**：header 之后全部段字节的 SHA-256。

---

## 3. 条款（RXS-0328 ~ RXS-0331）

### RXS-0328 逻辑页 RXPL header 布局与 byte golden

**Legality**

逻辑页文件/缓冲以 **136 字节定长 header**（`header_size = 136`）起首，其后紧接段体。
全部多字节整型与 `f32` **小端（LE）**；`endian` 字段字面 `1`。禁止依赖 Rust/C++ 原生
struct padding（手写 LE 编解码）。

**Header 逐字段（偏移/宽度/字面）**：

| 偏移 | 宽度 | 类型 | 字段 | 字面/语义 |
|---:|---:|---|---|---|
| 0 | 4 | `[u8;4]` | `magic` | `"RXPL"`（`0x52 0x58 0x50 0x4C`） |
| 4 | 4 | `u32` LE | `format_id` | `1`（逻辑页 ABI id） |
| 8 | 2 | `u16` LE | `major` | `1` |
| 10 | 2 | `u16` LE | `minor` | `0` |
| 12 | 1 | `u8` | `endian` | `1` = little-endian |
| 13 | 1 | `u8` | `flags` | bit0 = `ROOT`；其余位必须为 0 |
| 14 | 2 | `u16` LE | `header_size` | `136` |
| 16 | 8 | `u64` LE | `page_id` | 页稳定序号（从 0 递增） |
| 24 | 2 | `u16` LE | `lod_level_min` | 本页簇 level 下限（含） |
| 26 | 2 | `u16` LE | `lod_level_max` | 本页簇 level 上限（含） |
| 28 | 4 | `u32` LE | `cluster_count` | 本页簇记录数 |
| 32 | 4 | `u32` LE | `vertex_count` | 本页顶点元素数 |
| 36 | 4 | `u32` LE | `index_count` | 本页索引字节数（`u8` 元素） |
| 40 | 24 | `6×f32` LE | `bounds` | `xmin,ymin,zmin,xmax,ymax,zmax`（按位） |
| 64 | 4 | `u32` LE | `dependency_page_count` | 依赖页 id 表长度 |
| 68 | 4 | `u32` LE | `dag_link_count` | DAG 边表长度（parent→child 对数） |
| 72 | 32 | `[u8;32]` | `schema_digest` | SHA-256(schema preimage) |
| 104 | 32 | `[u8;32]` | `section_digest` | SHA-256(段体) |

**schema preimage**（域分离，字节拼接，全 LE）：

```
b"RXPL-SCHEMA-V1\0"
|| major:u16 || minor:u16
|| STREAM_PAGE_SIZE:u32 (=131072)
|| header_size:u16 (=136)
|| record_size:u16 (=96)
|| packing_algo_id:u32 (=1)
|| format_id:u32 (=1)
```

**段体顺序**（header 之后，无额外 directory；尺寸由 header 计数推导）：

1. `cluster_count × 96B` 簇记录（见下）
2. `vertex_count × 12B` 顶点（`f32×3` LE 按位）
3. `index_count × 1B` 三角形局部索引（`u8`）
4. `dependency_page_count × 8B` 依赖页 id（`u64` LE，**升序去重**）
5. `dag_link_count × 8B` DAG 边（`parent_id:u32 || child_id:u32`，按 `(parent,child)` 升序）

**簇记录（96B，非 RXGB 64B）**：

| 偏移 | 宽度 | 字段 |
|---:|---:|---|
| 0 | 4 | `cluster_id:u32`（stable id） |
| 4 | 2 | `qx:u16`（中心相对页 AABB 量化） |
| 6 | 2 | `qy:u16` |
| 8 | 2 | `qz:u16` |
| 10 | 2 | `pad:u16`（=0） |
| 12 | 4 | `center_x_bits:u32`（`f32::to_bits`） |
| 16 | 4 | `center_y_bits:u32` |
| 20 | 4 | `center_z_bits:u32` |
| 24 | 4 | `radius_bits:u32` |
| 28 | 4 | `cone_axis_x_bits:u32` |
| 32 | 4 | `cone_axis_y_bits:u32` |
| 36 | 4 | `cone_axis_z_bits:u32` |
| 40 | 4 | `cone_cutoff_bits:u32` |
| 44 | 4 | `error_bits:u32` |
| 48 | 4 | `parent_error_bits:u32` |
| 52 | 4 | `vertex_offset:u32`（页内顶点元素下标） |
| 56 | 4 | `triangle_offset:u32`（页内索引字节下标） |
| 60 | 4 | `vertex_count:u32` |
| 64 | 4 | `triangle_count:u32` |
| 68 | 4 | `level:u32` |
| 72 | 4 | `group:u32` |
| 76 | 4 | `reserved0:u32`（=0） |
| 80 | 16 | `reserved1..4:u32×4`（=0） |

量化律：`q = round((c - lo) / max(hi-lo, ε) × 65535)` clamp 到 `[0,65535]`；
`ε = 2⁻¹²⁶`。解码以 `center_*_bits` 恢复 CPU 参照（按位）；`q*` 供下游 ABI 消费。

**byte golden 义务**：固定程序化输入 `TriMesh::uv_sphere(1.0,24,24)` 装箱后**首页**
header 的 136 字节必须逐字节等于 `tests/geom_pages/golden/m01_header.bin`；
`schema_digest` 字段必须等于 digest 清单中的冻结值。

**Implementation Requirements**：`rurix-geom-pages::logical::{encode,decode}`；
未知 `magic` / `major≠1` 在任何段体消费前返回 `Err`（RXS-0331）。

---

### RXS-0329 装箱确定性与 root 页标记

**Legality**

`ClusterDag → Vec<LogicalPage>` 装箱必须确定性，同输入两次输出**全页串接逐字节相等**。

**装箱律**（`packing_algo_id = 1`）：

1. 以 stable id **升序**遍历；同 `level` 的簇优先聚于同一页（按 level 升序外层循环，
   level 内 id 升序）。
2. 贪心：将下一簇加入当前页后若编码字节（header+段体）将 **> `STREAM_PAGE_SIZE`
   （131072）**，则封页并开新页再放入。
3. 单簇自身编码若已 > `STREAM_PAGE_SIZE` → 装箱失败（typed `Err`；合法 meshlet 输入下不应触发）。
4. 含 `top_level_ids` 任一簇的页置 `flags.ROOT`。
5. `page_id` 按封页顺序从 0 递增。
6. 装箱参数进入 `schema_digest`；改策略必须升 `major`/`minor` 并更新 digest。

**Implementation Requirements**：`rurix-asset::geom_build::pack_cluster_dag`。

---

### RXS-0330 依赖边语义与解码参照全等

**Legality**

- **依赖页**：若存在 DAG 边 `(parent,child)` 且二者分属不同页，则 child 所在页的依赖表
  必须包含 parent 所在页的 `page_id`（反之不强制）；表内 id **升序去重**。
- **DAG 边段**：本页编码所有「至少一个端点落在本页」的 parent→child 边；按
  `(parent_id, child_id)` 升序。
- **解码全等**：对全部逻辑页解码并合并后，必须与 CPU reference `ClusterDag` 满足：
  - 节点集（`cluster_id` + `level` + `group`）全等；
  - 边集（parent→child）全等（含跨页）；
  - 每簇 `center/radius/cone_*/error/parent_error` 的 `f32` **按位**相等；
  - LOD parent 关系（由边集导出的 parent 映射）全等——同组多父簇共享孩子时为
    `child → {parents}` 多值映射，不得压成单父。

**Implementation Requirements**：decode 合并校验单测 + smoke 腿
`decoded_*_equal_reference`。

---

### RXS-0331 RXGB→pages 显式 converter 与拒录

**Legality**

1. **显式 converter**：`rxgb_to_pages(bytes) -> Result<Vec<LogicalPage>, _>` 必须
   先走既有 RXGB v1 全校验路径（`rurix_geom_build::read_dag`），版本/magic 失败则
   原样传播；成功后再 `pack_cluster_dag`。禁止静默把 RXGB 字节重解释为 RXPL。
2. **旧 RXGB reader 0-byte**：本门不得修改 `rurix-geom-build` 的 RXGB
   `read_dag`/`write_dag` 行为；既有 roundtrip 单测保持绿。
3. **消费前拒录**：`decode` 在读取任何簇/顶点/索引段之前，若 `magic ≠ "RXPL"` 或
   `major ≠ 1`，必须返回 `UnsupportedVersion` / `BadMagic`；截断输入返回 `Truncated`。
4. RXGB 内 `ClusterRecord::page_id` **不回写**；页归属仅存在于 RXPL artifact。

**Implementation Requirements**：`rurix-asset::geom_build::rxgb_to_pages`；
`conformance/geom_pages/reject/{unknown_version,bad_magic,truncated}.rxpl`。

---

## 4. 修订记录

| 版本 | 日期 | 说明 |
|---|---|---|
| v1.0 | 2026-08-06 | 初版：RXS-0328~0331 逻辑页 ABI + 装箱/依赖/converter；M01 host 门 |
