# geometry_pages.md — 几何页逻辑/磁盘/内存 ABI（G8.3 M01 + M04）

> **地位**：几何流送页格式语义事实源之一（RFC-0020 §4.9；G8_ACCEPTANCE_MAP §2 M01/M04）。
> 本文件冻结 **逻辑页（未压缩 builder artifact）** ABI（M01）与 **磁盘/内存双 ABI +
> RXPZ-LZ1**（M04）。G8.4 只消费本文件冻结字面，变更走新 major。
>
> **档位**：Full RFC / RFC-0020（字面值与逐字段偏移由本文件一次性冻结并同 PR 落
> byte golden；评审 F7）。
>
> **编号**：RXS-0328 ~ RXS-0331（M01）；RXS-0338 ~ RXS-0342（M04；ledger 实测
> next_free 曾为 335，0335~0337 让并行门，本批自 **0338** 起）。

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

## 5. M04 条款（RXS-0338 ~ RXS-0342）— 磁盘/内存双 ABI

### RXS-0338 内存页 RXPM ABI 布局与 section 表

**Legality**

内存页是 **device decoder 消费面**（magic `"RXPM"`）。全部多字节整型与 `f32` **小端**；
编码手写 LE，禁止 host struct memcpy。

**固定 header（48 字节）**：

| 偏移 | 宽度 | 类型 | 字段 | 字面/语义 |
|---:|---:|---|---|---|
| 0 | 4 | `[u8;4]` | `magic` | `"RXPM"` |
| 4 | 4 | `u32` LE | `format_id` | `2`（≠ RXPL=`1`、≠ RXPD=`3`） |
| 8 | 2 | `u16` LE | `major` | `1` |
| 10 | 2 | `u16` LE | `minor` | `0` |
| 12 | 1 | `u8` | `endian` | `1` |
| 13 | 1 | `u8` | `flags` | bit0=`ROOT`；其余位必须 0 |
| 14 | 2 | `u16` LE | `header_size` | `48`（固定头；不含 section 目录） |
| 16 | 8 | `u64` LE | `logical_page_id` | 对应逻辑页 `page_id` |
| 24 | 4 | `u32` LE | `section_count` | section 目录项数（v1 恒 =4） |
| 28 | 4 | `u32` LE | `reserved` | `0` |
| 32 | 16 | `[u8;16]` | `schema_digest_prefix` | `schema_digest` 前 16 字节 |

其后紧接 `section_count × 16B` **section 目录**（按 `kind` **升序**），再接段体。
目录项：

| 偏移 | 宽度 | 字段 |
|---:|---:|---|
| 0 | 4 | `kind:u32` |
| 4 | 4 | `byte_offset:u32`（自文件起点；必须 ≥ header+目录 且对齐 `align`） |
| 8 | 4 | `byte_size:u32` |
| 12 | 4 | `align:u32` |

**section kind 闭集（v1）**：

| kind | 名 | 元素 | 段对齐 | 说明 |
|---:|---|---|---:|---|
| 1 | `POS_Q16` | 8B：`qx:u16\|qy:u16\|qz:u16\|pad:u16(=0)` | 16 | 每簇一条量化中心 |
| 2 | `INDICES_U8` | `u8` 局部索引；`byte_size` 填至 4B 倍数（尾填 0） | 4 | |
| 3 | `CLUSTER_META` | 32B/簇：`cluster_id,vertex_offset,triangle_offset,vertex_count,triangle_count,level,group,reserved(=0)` 全 `u32` | 16 | |
| 4 | `QUANT_PARAMS` | 32B：`bounds` 六 `f32` 按位 + `pad u32×2(=0)` | 16 | 零浮点运算消费 |

空洞字节恒 0；section 不得重叠；`byte_offset+byte_size` 不得越出文件；未知 `kind`/`major≠1` 拒录。

**schema preimage**（SHA-256 全 32B；header 仅存前 16B 前缀，解码时复核全 digest）：

```
b"RXPM-SCHEMA-V1\0" || major:u16 || minor:u16 || format_id:u32(=2)
|| header_size:u16(=48) || section_count:u32(=4)
```

**Implementation Requirements**：`rurix-geom-pages::memory::{encode,decode}`；
`tests/geom_pages/golden/*.rxpd` 往返后 records 与 expand digest 锚定。

---

### RXS-0339 磁盘页 RXPD envelope 与 codec 注册

**Legality**

磁盘页 magic `"RXPD"`；payload = **RXPZ-LZ1 压缩后的完整 RXPM image**。

**定长 envelope（`header_size = 148`）**：

| 偏移 | 宽度 | 字段 | 字面 |
|---:|---:|---|---|
| 0 | 4 | `magic` | `"RXPD"` |
| 4 | 4 | `format_id:u32` | `3` |
| 8 | 2 | `major:u16` | `1` |
| 10 | 2 | `minor:u16` | `0` |
| 12 | 1 | `endian:u8` | `1` |
| 13 | 1 | `reserved0:u8` | `0` |
| 14 | 2 | `header_size:u16` | `148` |
| 16 | 4 | `section_dir_count:u32` | `0`（v1：段目录仅在解压后 RXPM 内） |
| 20 | 4 | `codec_id:u32` | `1` = RXPZ-LZ1 |
| 24 | 4 | `codec_version:u32` | `1` |
| 28 | 8 | `uncompressed_size:u64` | 解压后 RXPM 字节数 |
| 36 | 8 | `compressed_size:u64` | payload 字节数 |
| 44 | 8 | `logical_page_id:u64` | |
| 52 | 32 | `schema_digest` | RXPD schema SHA-256 |
| 84 | 32 | `payload_checksum` | SHA-256(compressed payload) |
| 116 | 32 | `dependency_digest` | SHA-256(升序 `u64` 依赖页 id 串；无依赖则空输入 digest) |

payload 自偏移 148 起，长度 = `compressed_size`；禁止尾随字节。

**RXPD schema preimage**：

```
b"RXPD-SCHEMA-V1\0" || major:u16 || minor:u16 || format_id:u32(=3)
|| header_size:u16(=148) || codec_id:u32(=1) || codec_version:u32(=1)
```

**codec 注册表**：`codec_id=1` → RXPZ-LZ1（见 RXS-0340）；未知 `codec_id` fail-closed。

**Implementation Requirements**：`rurix-geom-pages::disk::{encode,decode}`。

---

### RXS-0340 RXPZ-LZ1 字节向 LZ77 流格式

**Legality**

RXPZ-LZ1（`codec_id=1`，`codec_version=1`）为 **手写确定性**字节向 LZ77，语义对齐
LZ4-block 序列（非 zstd；本仓禁引入 zstd/flate2）。

**流布局**：连续 sequences，直至输入耗尽。每 sequence：

1. `token:u8`：高 4 位 = 字面长度基值；低 4 位 = 匹配长度基值（匹配侧 +`MINMATCH=4`）。
2. 若字面基值 =15：附加长度字节串（每字节 0..255 累加；255 表示继续）。
3. 字面字节（可为 0）。
4. 若非最后 sequence：`offset:u16` LE（∈`[1,65535]`；`0` 非法）+ 匹配附加长度
   （低 4 位=15 时同字面附加规则）；自 `当前写位 - offset` 起拷贝 `match_len` 字节
   （允许重叠拷贝）。
5. **最后 sequence**：仅字面、无 match；编码器在尾部至少保留 `LASTLITERALS=5` 字节作纯字面
  （输入更短则整块纯字面）。

**编码器确定性**：窗口 64KiB；贪心最长匹配；hash 链桶数 4096、链深上限 64；
同输入两次压缩流 **逐字节相等**。解码全程边界检查，零分配越界路径。

**Implementation Requirements**：`rurix-geom-pages::codec::{compress,decompress}`。

---

### RXS-0341 disk↔memory 映射表与四类拒录

**Legality**

**版本映射表（G8.3 冻结）**：仅允许

```
(RXPD, major=1) → (RXPM, major=1)
```

未列组合（含未知 major、format_id 不符）→ 拒。解码流程严格序：

1. 校验 RXPD envelope（magic/format_id/major/header_size/endian/schema）；
2. 校验 `compressed_size` 与缓冲余量（**截断在任何大分配前拒录**）；
3. 校验 `payload_checksum`；
4. 按 `codec_id` 解压，断言输出长度 = `uncompressed_size`；
5. 按映射表校验并 `decode` RXPM（含 section overlap/OOB）。

**拒录轴（各至少一件 RED 语料）**：截断 payload、checksum 位翻转、未知 codec、
未知 major、section overlap、section OOB。

**Implementation Requirements**：`disk::decode_to_memory`；
`conformance/geom_pages/reject/{truncated_payload,checksum_flip,unknown_codec,unknown_major}.rxpd`、
`{section_overlap,section_oob}.rxpm`。

---

### RXS-0342 整数域展开流与 CPU/device digest 逐位等

**Legality**

对已校验 RXPM，CPU 与 device compute kernel 必须产出 **同一展开流**（全 `u32` 整数域；
kernel 内零浮点运算；`f32` 量化参数按位搬运）。

**展开流字段序**（LE `u32` 序列）：

```
cluster_count
for c in 0..cluster_count:
  cluster_id
  qx, qy, qz          # u16 零扩展
  triangle_count
  for t in 0..triangle_count:
    i0, i1, i2        # 自 INDICES_U8，按 meta.triangle_offset+3*t 取 u8 零扩展
  vertex_offset, triangle_offset, vertex_count, triangle_count, level, group
bound0_bits .. bound5_bits   # QUANT_PARAMS 前 6×f32 的 to_bits
```

`expanded_digest = SHA-256(展开流原始字节)`。device harness 回读展开流后由 smoke 侧
算 SHA-256，必须与 CPU digest **逐位相等**。device 腿 `RURIX_REQUIRE_REAL=1`；
validation ERROR 数必须为 0；缺设备 → `SKIP=dev-env`（不得充绿）。

**Implementation Requirements**：`rurix-geom-pages::expand::{expand_memory_page,expanded_digest}`；
`rxcook decode-page --emit-expanded-digest`；`.rx` kernel `geom_page_decode.rx`（经
rurixc→SPIR-V，禁手写 SPIR-V 替身）；`vk_geom_page_decode` harness。

---

## 7. M91 条款（G9.2，RFC-0022 §4.5）— RXPL major=2 段布局与 v1/v2 共存律

### RXS-0344 RXPL major=2 段布局、新 schema preimage 与 v1/v2 共存律

> 🔒 演进面（RFC-0022 §4.5，Agent Approved 2026-08-09）：RXPL 自 major=1（RXS-0328
> 冻结面）向 major=2 的演进是**新 major**，不是 v1 条款的修订——v1 既有条款
> （RXS-0328~0342）**字面 0-byte 不动**，v2 语义全部落本章。

**Legality**

1. **v2 语义**（RFC-0022 §4.5 逐字）：RXPL `major=2`；schema_digest preimage 为
   **新** preimage（域分离字串 `b"RXPL-SCHEMA-V2\0"` 起首，`major:u16=2`；沿用
   RXS-0328 preimage 逐字段拼接律，`record_size` / 段序 / 新增段标识按 v2 布局随实现
   PR 冻结并进 digest）；v2 页在 v1 簇记录之外新增**簇误差/包围球/骨骼元数据/CLAS
   输入段**（蒙皮元数据与 CLAS 输入字段面见 spec/virtual_geometry.md RXS-0345）；
   v2 编解码**往返无损**（编码→解码→再编码逐字节相等）。
2. **v1 共存律**（RFC-0022 §4.5 逐字）：M01/M04 v1 页格式 ABI（RXS-0328~0342
   冻结面）**0-byte 保持**；v1/v2 页可在同一流送系统共存，G8.4 streamer 只消费
   冻结 ABI、迟到页降级语义不重定；**禁止在实现波次中途重定 v1 ABI**。
3. **未知版本 fail-closed**（RFC-0022 §4.5 逐字）：loader 对未知 major/篡改
   schema_digest/section_digest 的页必须**确定性拒绝**（typed `Err`，沿 RXS-0331
   `UnsupportedVersion` 族同码族先例），**不得按猜测布局解析**。
4. **双构建确定性**（RFC-0022 §4.5 逐字）：沿 M79 判据——同一资产 + 同一 builder
   版本双构建字节一致；物理段偏移/padding 差异进 environment/evidence，**不进**
   语义哈希。

**Implementation Requirements**

- 实现锚定 `src/rurix-geom-pages` v2 编解码臂（v1 `logical::{encode,decode}` 既有面
  0-byte）+ `conformance/geom_pages/reject/` v2 RED fixture（篡改 digest / 未知
  major 各至少一件）随实现 PR 落。
- 本 spec PR 先行落最小 RED 锚定占位语料
  `conformance/geom_pages/reject/rxpl_v2_unknown_major.rx`（条款锚定占位，inert 锚定
  口径与转正路径见该文件头注释）；锚点目标文件（实现 PR 转正）=
  `src/rurix-geom-pages/src/logical.rs` v2 臂单测 + `conformance/geom_pages/reject/`
  v2 fixture。

## 6. 修订记录

| 版本 | 日期 | 说明 | 档位 |
|---|---|---|---|
| v1.0 | 2026-08-06 | 初版：RXS-0328~0331 逻辑页 ABI + 装箱/依赖/converter；M01 host 门 | Full RFC（RFC-0020） |
| v1.1 | 2026-08-06 | M04：RXS-0338~0342 RXPM/RXPD/RXPZ-LZ1/映射拒录/展开 digest；device 腿 | Full RFC（RFC-0020） |
| v1.2 | 2026-08-09 | G9.2 spec-first（M91）：追加 §7 新章 RXS-0344（RXPL major=2 段布局/新 schema preimage/v1-v2 共存律/未知版本 fail-closed/双构建确定性，RFC-0022 §4.5 四行冻结句逐字落实）；v1 既有条款 RXS-0328~0342 字面 0-byte；条款号自 ledger 实测 RXS.next_free=344 顺位领取。依据 [RFC-0022](../rfcs/0022-virtual-geometry-gi-semantics.md)（Agent Approved 2026-08-09）§4.5/§5 + G9_ACCEPTANCE_MAP M91 行；本行同步把本表升格为「版本/日期/说明/档位」四列同构体例（既有两行仅补档位列标记，说明列字面不动） | **Full RFC**（RFC-0022） |
