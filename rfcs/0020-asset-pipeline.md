# RFC-0020 — G8 资产管线、确定性派生数据与版本化页 ABI

| 字段 | 值 |
|---|---|
| RFC 编号 | RFC-0020（4 位制，编号永不复用，10 §9.5） |
| 标题 | G8 资产管线、确定性派生数据与版本化页 ABI |
| 档位 | **Full RFC**（新增资产 schema、内容寻址缓存语义、磁盘/设备内存页 ABI 与供应链边界；触及公开格式/FFI 相邻面，依 10 §3 与 AGENTS 硬规则 5/8 向上取严） |
| 状态 | **Agent Approved**（2026-08-02）；§9.1 独立 provenance 对抗性评审完成，17 findings 逐条 disposition（3 blocker + 8 major 正文实改）。**批准不授权 G8.2+ 实现** |
| 承接里程碑 | G8.1 起草/评审；事实互锁解除后由 G8.3 实现 **有 symbolic gate 的行**：M79/M80/M81/M01/M04（P0）+ M83（已 go P1）+ M85 的 G8.3 阶段腿。**M82/M84-BLAS/M88 当前无 symbolic gate 也不在已 go P1 集合 `{M25,M72,M83}` 内**：其实现前必须先补 `G8_CANDIDATE_DECISIONS.md` 行并按 `G8_ACCEPTANCE_MAP.md` §6 修订验收表（评审 F1） |
| 关联条款 | 拟新增 `spec/asset_pipeline.md` 与 `spec/geometry_pages.md`；**本 Draft 不 claim RXS/CI/RD/U/RX 等编号**，条款号与数字 CI 步骤仅在 G8.2 互锁解除后按 ledger 的 actual next-free materialize |
| 依据决策 | D-308（manifest/lock/vendor/checksum）· D-309（无任意构建脚本）· D-311（GPU 元数据进 manifest/lock）· D-313（再分发白名单）· P-01/P-05/P-09/P-11/P-12/P-13/P-14 · [G8_PLAN](../milestones/g8/G8_PLAN.md) v1.2 · [G8_CAPABILITY_MATRIX](../milestones/g8/G8_CAPABILITY_MATRIX.md) v1.2 |
| Provenance | `Assisted-by: Codex:gpt-5 rfc20-drafter-session`（起草 provenance A；只起草，不自批） |
| Agent 批准 | **Agent Approved 2026-08-02**；只表示语义/治理评审完成，实现仍由 G8.2 互锁与 `ci/check_g8_implementation_interlock.py` 决定 |
| 对抗性评审 | **完成**（D-409）：评审 provenance `Assisted-by: Kiro:claude-opus-5 rfc-review-session` ≠ 起草 provenance `Codex:gpt-5 rfc20-drafter-session`；findings 与 disposition 见 §9.1 |

---

## 1. 摘要

本 RFC 冻结 G8 的确定性资产构建闭环：不可变 `SourceAsset` 经版本化 `ImportRecipe` 与 `CookProfile` 进入声明式工具 DAG，产生内容寻址的 `DerivedArtifact`、可删除重建的 DDC 对象、可审计的 `BuildManifest` 与 `PackageChunk`。同一闭环承载 glTF 导入、meshoptimizer 交叉验证、BCn/ASTC/KTX2/Basis 纹理 cook、shader/PSO manifest 入 DDC、几何页/BLAS 派生数据，以及受条件门控制的 VT/OMM baker。

本 RFC 同时把 M01/M04 从“实现细节”提升为明确的版本化 ABI：**磁盘压缩页**与**解码后内存页**使用不同的格式 ID、版本与 schema digest；G8.3 冻结，G8.4 只能消费，不能重定格式。所有构建节点都由固定工具标识、版本、输入与参数决定；shell、任意构建脚本、隐式网络、时间戳、宿主绝对路径和未声明环境变量不得影响签名产物。

```text
SourceAsset + ImportRecipe + CookProfile + ToolManifest
          │             （canonical bytes）
          └───────────────┬─────────────────────┐
                          ▼                     │
                  declarative tool DAG          │
                    │       │       │           │
                    ▼       ▼       ▼           │
                 glTF    texture   geometry     │
                 M81      M83      M01/M04      │
                    └───────┬───────┘           │
                            ▼                   │
                    DerivedArtifact ── key = SHA-256(preimage)
                            │                   │
                     immutable local/remote DDC│
                            │                   │
                            ▼                   ▼
                   PackageChunk / manifest / SBOM
```

## 2. 动机、现状与范围

### 2.1 现状缺口

- `rurix-pkg` 已有 lock/vendor/checksum 与 `package.build = "declarative"` 红线，可作为供应链与无脚本纪律的基座，但不是资产 import/cook/DDC。
- `rurix-geom-build` 已有确定性 DAG 与 RXGB v1 序列化；`ClusterRecord::page_id` 仍是预留面。现有 RXGB v1 是输入事实，不等于 M01/M04 的压缩磁盘页和设备内存页双 ABI 已完成。
- shader/PSO、纹理、BLAS、VT/OMM 等派生物尚无共同的 key preimage、对象完整性、工具版本和 SBOM 事实源。
- 若每类资产各自定义缓存键与序列化，会重复制造隐式输入、路径漂移、供应链漏项与“同输入不同产物”。

### 2.2 为何需要 Full RFC

M01/M04 会成为 G8.4 流送器和 device decoder 的跨组件 ABI；`SourceAsset`/`Recipe`/`Artifact`/`CookProfile` 与 DDC key 会成为工具和包格式的长期互操作面。它们不是纯内部重构。依 10 §3、P-10/P-11 与 AGENTS 硬规则 5/8，本提案按 Full RFC 留档，并在实现前走 spec-first、RED-first 与独立对抗性评审。

### 2.3 in-scope

| 能力 | 本 RFC 冻结面 | G8 行 |
|---|---|---|
| 核心 schema | `SourceAsset`、`ImportRecipe`、`DerivedArtifact`、`CookProfile`、`ToolManifest`、`BuildManifest`、`PackageChunk` | M79/M88 |
| 确定性编码 | canonical serialization、稳定排序、浮点/路径规则、双构建证明 | M79 |
| DDC | key preimage、不可变 CAS、校验、损坏拒录、本地/远端等价语义 | M80 |
| 导入/交叉验证 | 锁定扩展集的 glTF 2.0；meshoptimizer 作可审计工具或参考器 | M81/M82 |
| 纹理 cook | KTX2/Basis、BCn、ASTC、mip/normal/alpha-coverage 语义 | M83 |
| shader/PSO | manifest canonical merge、interface/PSO key 入 DDC preimage | M85 的 **G8.3 阶段腿**（G8.2 腿属 RFC-0019；该门 schema 要求 `phase_g8_2_pass` 与 `phase_g8_3_pass` 各自为真，评审 F12） |
| 页格式 | builder 输出、压缩磁盘页、设备内存页、decoder 兼容拒录 | M01/M04 |
| 派生 baker | BLAS 分腿；VT 与 OMM 分腿按各自候选决策门 | M84（**无 symbolic gate**，见头部与 §4.10） |
| 包与合规 | chunk/streaming manifest、vendor license、SBOM、补丁链 | M88（**无 symbolic gate**，见头部与 §4.12） |

> **无门交付面纪律（评审 F1 blocker）**：上表的 M82（meshoptimizer 交叉验证）、M84 三分腿与 M88 当前既非 P0 也不在 G8.1 已 go P1 集合内，`G8_ACCEPTANCE_MAP.md` 也无对应行。本 RFC **只冻结它们的 schema 与职责**，不构成实现许可；任何实现前必须先走 `G8_ACCEPTANCE_MAP.md` §6 的治理流程（补决策表行 → 修订验收表覆盖集合 → 再开实现），不得静默并入既有 key。

### 2.4 out-of-scope 与事实互锁

- 本 RFC 不实现编辑器、DCC GUI、完整 OpenUSD composition、MaterialX 全栈、runtime residency、I/O 调度或 G8.4 streamer。
- 不引入 package `build.rs`、shell hook、任意插件进程或网络下载脚本；D-309 保持不变。
- 不把 GPU 编码的本地缓存结果自动提升为签名 cook artifact；无法跨驱动逐字节复现时只能作为非签名 local cache。
- 不以本 RFC 批准替代 M40（SVT）或 M53（OMM）的 go/strategic_override；条件未满足时对应 executor 不得注册。
- 不触碰 G7 的 RFC-0018、claim、spec、conformance、source 或 RD-038 状态。
- **G8.1 仅 governance-only**。即便 RFC 完成评审并 Agent Approved，G8.2+ 仍必须同时满足：`G7_CONTRACT.status == closed`，且 RD-038 为 closed，或在 G7 closed 后六行终态接入表与独立 RD-038 override 齐备。互锁为红时，禁止 materialize 本 RFC 的 spec 条款、实现、数字 CI 步骤或新诊断号。

## 3. 指导级解释（工具使用者视角）

用户声明“是什么、如何导入、为哪个目标 cook”，而不是提交一段可执行构建脚本。示意形态如下；具体文本语法由下游 spec 冻结，本例不构成 stable 语法承诺：

```text
source "assets/hero.glb" {
    media_type = "model/gltf-binary"
}

recipe "gltf.scene.v1" {
    importer = "rurix.gltf"
    importer_version = "1.0.0"
    coordinates = "right-handed-y-up"
    units = "meter"
    extensions = ["KHR_materials_unlit", "KHR_texture_basisu"]
}

cook_profile "win-vulkan-sm89-high" {
    target = "windows-x86_64"
    gpu_profile = "vulkan-sm89"
    texture_formats = ["bc7", "bc5", "bc6h"]
    geometry_disk_abi = "geom-page-disk-v1"
    geometry_memory_abi = "geom-page-memory-v1"
}
```

工具先把声明解析为类型化 schema，再生成 canonical bytes。绝对输出目录、当前时间和执行线程数不进入语义输入；因此两个隔离空缓存目录构建同一输入，所有签名 artifact 必须逐字节相等。任一源内容、间接依赖、recipe、tool 版本、profile 或 ABI 变化都会产生新 DDC key，旧对象不会误命中。

## 4. 参考级设计

### 4.1 核心对象与不变量

#### `SourceAsset`

| 字段 | 约束 |
|---|---|
| `schema_version` | 显式版本；未知 major fail-closed |
| `logical_uri` | 仓库/包内稳定 URI；禁止宿主绝对路径、反斜杠、`.`/`..` 段与 NUL |
| `media_type` | 规范化 ASCII 标识；不得由扩展名静默猜测后写回 |
| `content_digest` | 源 blob 的 SHA-256；读取后复核 |
| `byte_len` | 与 blob 实长相等，溢出/不符拒录 |
| `dependency_ids` | 稳定 ID 排序后的直接依赖；传递闭包进入 build manifest |

源 blob 不可变；同一 logical URI 内容改变必须形成新 digest。DDC 对象可删除重建，`SourceAsset` 不能由 DDC 对象反向冒充。

#### `ImportRecipe`

固定字段至少含 importer ID、精确版本/二进制 digest、输入 schema、输出 artifact kind、坐标系、单位、颜色空间、显式扩展 allowlist 与类型化参数 map。未识别参数、越界扩展或依赖运行时默认值必须拒录，禁止“忽略并继续”。

#### `CookProfile`

固定字段至少含目标 OS/arch、GPU API/profile、capability set、质量等级、目标纹理格式、浮点模式、压缩器 profile、磁盘/内存页 ABI ID、字节序与打包策略。capability set 和无序列表按 canonical ID 排序；“latest”“auto”“native default”等漂移值非法。

#### `DerivedArtifact`

固定字段至少含 artifact kind、artifact key、payload digest/length、producer tool identity、recipe/profile/schema digest、直接输入 key、ABI ID/version、license/SBOM component references 与可再生标志。payload 与 metadata 均不可变；metadata 指向的 digest 不符时整个对象拒录。

#### `ToolManifest` / `BuildManifest`

`ToolManifest` 描述单个允许工具的稳定 ID、精确版本、可执行/wasm/内建实现 digest、输入/输出 kind、参数 schema、许可组件与 deterministic capability。`BuildManifest` 汇总整张 DAG 的 source、依赖、tool、recipe、profile、artifact、ABI、SBOM 与 license digest；每次签名 cook 必须生成，且自身也 canonical/hash-addressed。

### 4.2 Canonical serialization v1

核心 schema 先转为类型化值，再用 **deterministic CBOR（RFC 8949 确定性编码规则的冻结子集）**编码。Rurix 子集额外规定：

1. map key 仅允许 schema 分配的非负整数 field ID，并按其最短编码字节序排序；同一 key 重复即非法。
2. 整数使用最短编码；长度必须确定，禁止 indefinite-length item。
3. key-bearing ID 为 ASCII；用户字符串必须是有效 UTF-8，并在 parser 阶段验证为固定 Unicode 版本的 NFC。canonicalizer 版本进入 schema digest。
4. 核心对象禁止浮点。确需浮点的 recipe 参数使用 schema 指定的宽度；`-0` 归一为 `+0`，所有 NaN 归一为唯一 quiet-NaN bit pattern，非有限值默认拒录。
5. 语义有序数组保序；语义无序集合按元素 canonical bytes 排序并拒绝重复。
6. 禁止 timestamp、随机数、进程/线程 ID、当前工作目录、宿主绝对路径、临时文件名和未声明环境变量进入签名对象。
7. unknown required field、unknown major schema、非 canonical 输入均 fail-closed；reader 不接受“解析后再帮忙规范化”的宽容路径。

对象 envelope 至少包含 magic、schema ID、major/minor、canonicalizer version、payload length、schema digest 与 payload digest。canonical bytes 是 DDC key 与 golden 的唯一输入；人类文本不直接参与 hash。

### 4.3 DDC/content-addressed key

artifact key 冻结为：

```text
SHA-256(
  "rurix-ddc-artifact-v1\0" ||
  len(source_set)       || canonical(source_set)       ||
  len(dependency_keys)  || canonical(dependency_keys)  ||
  len(import_recipe)    || canonical(import_recipe)    ||
  len(cook_profile)     || canonical(cook_profile)     ||
  len(tool_chain)       || canonical(tool_chain)       ||
  len(schema_set)       || canonical(schema_set)       ||
  len(abi_set)          || canonical(abi_set)          ||
  len(artifact_kind)    || canonical(artifact_kind)    ||
  len(output_id)        || canonical(output_id)
)
```

**多产物消歧（评审 F5）**：`artifact_kind`（产物类别，如 `geom.page.disk` / `texture.ktx2` / `shader.manifest`）与 `output_id`（同一节点的输出槽稳定 ID）是 preimage 的必需段——否则 M01「一 mesh 产 N 页」、M83「一 source 多格式腿」会得到同 key 不同 payload，被下文「不同 payload 是 hard error」判死。页类产物的 `output_id` 必须含 `logical_page_id`，编码沿同一 `u64` 长度前缀规则。

- 每段长度使用固定 little-endian `u64`，domain separator 固定，避免串接歧义。
- `source_set` 含所有源内容 digest；`dependency_keys` 是直接依赖 artifact key 的稳定排序集合，递归闭包由依赖 key 承载。
- `tool_chain` 含每个实际执行工具的 ID、精确版本、实现 digest、补丁 digest 和声明的 deterministic mode；只记录工具名不够。
- shader/PSO 产物的 interface hash、PSO key 与 manifest digest 进入 M85 对应节点的 recipe/dependency preimage。
- DDC 是 immutable CAS：同 key 的第二次 put 必须逐字节相同；不同 payload 是 hard error。get 后复核长度和 digest；损坏、截断、位翻转均视为 miss+corruption 诊断，不能返回脏对象。
- 落盘采用同卷临时文件、完整校验、原子 rename；本地与远端 store 必须遵循同一对象语义。remote miss 不允许悄悄改变 recipe 或 profile。
- cache eviction 可删除对象但不能改源；签名 cook 可从空 DDC 完整重建。

### 4.4 声明式工具 DAG（禁止任意构建脚本）

工具图节点由 `tool_id + tool_digest + typed_inputs + typed_outputs + canonical_params` 唯一描述。构图与执行遵守：

1. DAG 必须无环；节点 ID 来自节点 canonical bytes，调度顺序不进入输出。
2. 工具必须先在受控 `ToolManifest` 注册；未知工具或参数 schema 拒录。
3. 节点只能读声明输入 CAS 与只读 vendor snapshot，只能写声明的临时输出；无隐式 cwd 扫描、父目录访问或网络。
4. shell、PowerShell、`cmd.exe`、`build.rs`、任意脚本 hook 和用户拼接命令行不是工具节点类型。
5. 环境变量默认清空；确需的白名单值必须类型化、写入 recipe 并进入 key。时区/locale 固定，随机算法必须有 schema 固定 seed；签名输出不得使用当前时间。
6. 并行节点结果按稳定 artifact ID 合并；遍历 map/set、文件目录和 glTF object table 时必须稳定排序。
7. **G8.3 内 GPU encoder 一律不得产出签名 artifact**，只能作为 local acceleration cache；签名 artifact 恒由 CPU 确定性基线生成（评审 F9：`G8_ACCEPTANCE_MAP.md` 的 M79/M80/M83 均为 host 硬门，若在 G8.3 开出 GPU 签名通道，就出现无 gate 覆盖的产物路径）。未来若要开启 `deterministic_signing=true`，前置为：同 profile、跨隔离构建、**给定 N 次重跑与显式驱动集合矩阵**下逐字节稳定，且先按 `G8_ACCEPTANCE_MAP.md` §6 增行并给出该矩阵判据。

这延续 `rurix-pkg` 的 D-309 红线，而不是为资产管线打开第二套可执行构建世界。

### 4.5 M79/M80：双构建与缓存验收

M79 的有效双构建必须使用两个隔离、空 DDC、不同绝对输出根；固定 source/recipe/profile/tool-set 后比较 canonical DAG、每个 artifact payload 和顶层 manifest digest。复用同一目录或 warm cache 不算双构建。source、任一依赖、recipe、CookProfile 与 tool version 分别做单变量变更时，受影响节点 key 必须变化且无关节点保持命中。

M80 必须覆盖 CAS put/get、**preimage 各段各一次单变量 mutation**（§4.3 的九段全覆盖；`G8_ACCEPTANCE_MAP.md` 的「四类」是下限而非上限，`abi_set`/`schema_set`/`import_recipe`/`artifact_kind`/`output_id` 同样必须有独立 mutation 断言，评审 F6）、对象位翻转/截断/错误长度拒录、并发同 key put 与 eviction 后重建。DDC 命中率是观测指标，不是正确性判据；错误 key 命中为 hard failure。

### 4.6 M81：glTF 2.0 严格导入

- 首版只接受 spec 中列出的 glTF 2.0 core 与显式 extension allowlist；allowlist 是 CookProfile/schema 的版本化输入。
- accessor 范围/stride/component type、bufferView 越界、索引、node cycle、required extension、image/texture/sampler 引用与 sparse accessor 必须先完整验证再产生 artifact。
- unknown required extension、非法范围和缺失必需 buffer fail-closed；unknown optional extension 仅在 recipe 明确 `preserve_opaque` 且其不影响输出语义时可保留，否则拒录，禁止静默丢字段。
- import 后形成 canonical scene/node/mesh/primitive/material/texture 表；source 顺序不具语义的表按稳定 ID 排序。两次导入的数量、拓扑与 digest 与 golden 全等。
- OpenUSD/MaterialX 不进入本首版导入硬门；后续须另行决策，不能借 glTF adapter 偷渡完整 composition/runtime。

### 4.7 M82：meshoptimizer 的角色

`rurix-geom-build` 仍是 DAG/页格式事实源；meshoptimizer 不替换为黑盒。其允许角色为：

- 拓扑清理、partition/simplification/codec 的可审计后端；
- 与 Rurix builder 的质量和 bounds/cluster 数交叉验证参考器；
- 固定版本、固定 flags、固定线程/浮点模式的 benchmark/codec 对照。

vendor 时记录精确 revision、源码 digest、MIT license digest、编译器/flags 与补丁 digest。输出若在双构建中不稳定，不得进入签名 artifact；reference 差异必须显式报告，不能择优静默切换实现。

### 4.8 M83：纹理 cook

**RD-041 触发机制逐字承接（评审 F2 blocker）**：RD-041 backfill 该分项字面为「KTX2/BasisU 在真实纹理资产管线出现时经 `PagedResource::transcode` 留口接入(解包确定性单测口径不变)」。本 RFC 不改写该字面，因此额外冻结两条：

1. **运行时接入点唯一**：cook 出的 KTX2/Basis→BCn/ASTC 产物在运行时只经既有 `PagedResource::transcode` 留口消费（`src/rurix-render/src/streaming/resource.rs` 的默认恒等实现是接入基线），不新造第二条纹理解包通道；
2. **既有单测口径 0-byte**：streaming 解包的确定性单测（同输入同输出）判据字面不改、不放宽，只允许追加新语料。

纹理节点把语义作为 key 输入，而不是只看像素 blob：color/normal/mask/HDR、颜色空间、mip filter、alpha coverage、normal renormalization、encoder/profile/quality、目标 format 与容器版本均写入 recipe/profile。

- Windows/Desktop 基线覆盖 BCn；跨平台 profile 覆盖 ASTC；传输/通用容器覆盖 KTX2 + Basis Universal。
- 同一 source/profile 两次 cook 字节相等；container、layer、face、mip 与目标 GPU format 逐项验证。
- 解码质量分别验证颜色误差、normal length 与 alpha coverage；容差只能由真实样本 evidence 冻结，不在 RFC 中预造数字（P-09）。**冻结程序（评审 F10）**：`G8_ACCEPTANCE_MAP.md` 的 M83 判据要求三项「落在冻结 tolerance」，其载体即下游 `AP-TEX` 条款——三项 tolerance 由 G8.3 首批 measured 样本采样后以 `AP-TEX` 条款字面冻结（与 evidence 同 PR），冻结前 `g8.p1.m83.texture_transcode` 只能是 RED/未实现，不得以「tolerance 待定」跳过判据。
- “只改扩展名”、占位 codec、缺少任一声明格式腿或隐式改用另一 encoder 都是 FAIL。
- CPU 确定性实现是签名基线；GPU encoder 只有满足 §4.4 第 7 条才可签名。

### 4.9 M01/M04：磁盘页与内存页双 ABI

#### Builder artifact（M01）

M01 从 canonical mesh/DAG 生成稳定的逻辑 page set。每页至少记录 stable page ID、LOD/DAG links、bounds、material ranges、vertex/index/cluster record counts、依赖页、payload section digest 与 schema digest。builder 遍历、分组和并行合并均按 stable ID；同一输入两次输出逐字节相等。

现有 RXGB v1 可作为转换输入与回归 fixture，但不能被静默重命名为 G8 page ABI。转换器必须显式读取 RXGB v1、验证版本，再生成新格式；旧 reader 行为 0-byte。

#### 压缩磁盘页（M04 disk ABI）

磁盘页 envelope 至少冻结：独立 magic/format ID、major/minor、little-endian 标记、header/section directory size、codec ID+version、uncompressed size、compressed size、logical page ID、schema digest、payload checksum 与 dependency digest。section 目录按 kind 排序、偏移/对齐全校验、空洞字节固定为 0。

> **字面值冻结归属（评审 F7）**：`G8_ACCEPTANCE_MAP.md` 的 M01/M04 判据要求 header「精确等于冻结 golden」、ABI id/version「不同且冻结」。本 RFC 冻结的是**字段集合与拒录规则**；magic/format ID 的具体字面值、初始 major/minor、逐字段偏移/宽度/字节序/对齐/填充与量化规则由下游 `AP-PAGE-DISK`/`AP-PAGE-MEM` 条款在 G8.3 spec PR 中一次性冻结并同 PR 落 byte golden。§3 示意中出现的 `geom-page-disk-v1`/`geom-page-memory-v1` 是**非规范示例**，不构成字面值承诺。条款冻结前 `g8.p0.m01.*` / `g8.p0.m04.*` 不可判 GREEN。

#### 解码后内存页（M04 memory ABI）

内存页使用**不同** magic/ABI ID/version，冻结 device decoder 消费的 section offset、alignment、element width、quantization、addressing 和 bounds。磁盘页版本通过显式 decoder 映射到某一内存 ABI；不得把压缩字节直接解释为 device struct，也不得依赖 Rust/C++ 原生 struct padding。

**disk→memory 版本映射表归属（评审 F8）**：该映射表属 **G8.3 冻结面**，随 `AP-PAGE-MEM` 条款与 M04 golden 一同落盘。它不得留到 G8.4——否则 G8.4 就实质参与定义 M04 的消费语义，触 R-G8-4 与 G8_CONTRACT「G8.4 只消费不重定」。G8.4 streamer 只能引用已冻结映射；新增映射条目须走 RFC 修订 + 新 major。

#### 兼容与拒录

- unknown major、unknown codec、截断、section overlap/out-of-bounds、checksum/schema mismatch 全部在 allocation/upload 前 fail-closed。
- minor 仅允许 reader 明确列出的向后兼容增量；unknown required section 仍拒录。
- golden 包含 encode→decode canonical records、两次压缩字节相等、CPU/device decoder digest 全等，以及 corrupt/unknown-version RED corpus。
- G8.3 冻结 ABI 与 golden 后，G8.4 streamer 只能引用；需要变更时走 RFC 修订与新 major，禁止原地重定格式。

### 4.10 M84：VT、BLAS 与 conditional OMM baker

M84 必须拆成独立 artifact kind、tool node 与验收腿，不能“一项实现三项全绿”。

| 分腿 | G8.3 行为 | artifact 最小内容 | 门控 |
|---|---|---|---|
| BLAS input baker | **可实现** | canonical geometry ranges、build flags、position/index format、transform、compaction policy、source/page digests | 随 M01/M04；运行时消费另门 |
| VT tile baker | **仅冻结 schema/hook**；executor 注册前提见下方 SVT 门槛条 | tile/border/mip-tail/layer format/page directory/fallback/dependency digest | 当前 M40 no-go 时不得以空 baker 充绿；门-VT 维持 `SKIP=not-triggered` |
| OMM baker | **G8.3 不承接**（决策表：G8.7 穷举） | microtriangle subdivision、2/4-state encoding、alpha cutoff/texture dependency、BLAS attachment metadata | 需同时满足：`g8.p0.m50.rt_pipeline_incremental` 先绿 + alpha-tested foliage 资产/收益证据 + M53 判档 go/override（评审 F4） |

**SVT 门槛缺项（评审 F3 blocker）**：`G8_PLAN` §1.2 与 `G8_CANDIDATE_DECISIONS.md` 均记「RD-041 backfill **无独立 SVT 门槛**，G8.1 须补登记『SVT 触发 = 真实大纹理资产管线出现』或 strategic_override」。该登记**尚未完成**，且本 RFC **不构成该登记**。因此：

- VT executor 的注册条件不是「M40 go」四个字，而是「deferred history 逐字追加 SVT 门槛或独立 strategic_override」**在先**，M40 判档 go 在后；
- §4.8 建立的「真实纹理资产管线（M83 cook）」**不等于**门槛谓词中的「真实**大**纹理资产管线」——前者是 cook 通道，后者要求大纹理 residency/feedback 规模证据。本 RFC 明确拒绝以 §4.8 自触发 SVT 门槛。

三分腿的 key 必须包含各自输入、工具/profile/ABI；VT 或 OMM 未触发不阻塞 BLAS，但也绝不能由 BLAS 结果代替其状态。

### 4.11 M85：shader/PSO manifest 入 DDC

G8.2 产生的 shader interface hash、capability profile、permutation key、PSO key 与二进制/toolchain identity 由 canonical manifest 合并：key 集合稳定排序，重复项只在 payload 全等时去重，冲突同 key/different payload 为 hard error。G8.3 把 manifest digest 纳入相应 DDC 节点；任一 interface hash、PSO key、compiler/tool digest 变化必须产生新 artifact key。M85 只有 `phase_g8_2_pass` 与 `phase_g8_3_pass` 同时成立才算绿。

### 4.12 M88：PackageChunk 与 streaming install manifest

`PackageChunk` 至少含 stable chunk ID、artifact key 集合、压缩/对齐策略、依赖 chunk、streaming priority、payload digest/length、CookProfile digest、SBOM subset 与 license notice references。artifact 集与依赖按 stable ID 排序；依赖图无环；同一 artifact 不得由两个冲突 payload 提供。package/stage 只装配 DDC 已校验对象，不在打包时隐式重 cook。

安装 manifest 自身 canonical/hash-addressed，支持按 chunk 校验和原子发布；缺依赖、digest 不符、未知 ABI/profile fail-closed。priority 只影响运行时调度，不影响 chunk payload bytes。

### 4.13 Vendor 许可、SBOM 与补丁链

每个 vendored 或外部工具组件必须记录：SPDX ID（不能确定时用明确自定义 license ref）、upstream URL、精确 tag/revision、source digest、license/notice 文件 digest、transitive components、启用目录、编译 flags、补丁文件及补丁 digest。目录级许可证扫描结果进入 `ToolManifest` 和顶层 SBOM。

- meshoptimizer：MIT，可在精确版本与审计后 vendor。
- Basis Universal、KTX-Software、Arm astc-encoder：Apache-2.0 主线；仍需检查仓内第三方目录和 notice，不能只凭项目首页放行。
- AMD Compressonator：组件许可分散，必须逐目录 allowlist；未完成审计不得整体 vendor。
- NVTT 3：仅可选外部工具，不进入可复现构建基线，也不得让“本机有 SDK”改变默认产物。
- vendor 目录外隐式 DLL/SDK 搜索、运行时联网取 codec、缺 SBOM 的签名 cook 一律失败。

这是沿 D-313（NVIDIA 再分发白名单 CI 审计）的**审计模式加性扩展**到非 NVIDIA vendor 组件，不改写 D-313 字面、不扩大其原决策范围（评审 F16）；目的是不把“可运行”误写为“可再分发”。

## 5. 下游 spec 条款映射（先符号、后实号）

本 RFC 不占用任何条款号。G8.2 事实互锁解除、ledger 校准后，spec PR **按合入当时实测 `next_free` 顺位**将以下符号映射为真实 RXS，并同时落 traceability；实现 PR 不得先行。顺位分配**不保证连续、不预留区间**（评审 F15：RXS 段为 G7 与 RFC-0019/0021 共享在途空间，承诺「连续」等于隐性预占号段）。

| 符号条款 | 拟定标题 | 最小测试锚定计划 |
|---|---|---|
| `AP-SCHEMA` | 核心对象字段与版本拒录 | schema accept + unknown-major/field RED |
| `AP-CANON` | canonical serialization v1 | order/path/float/duplicate-key golden |
| `AP-DDC` | key preimage 与 immutable CAS | 单变量 key、put/get、corruption RED |
| `AP-GRAPH` | 声明式工具 DAG 与无脚本边界 | cycle/unknown-tool/network/shell RED |
| `AP-GLTF` | glTF locked extension import | golden corpus + three reject fixtures |
| `AP-TEX` | 纹理语义与 deterministic codec | KTX2/Basis→BCn/ASTC + quality evidence |
| `AP-PAGE-DISK` | M01/M04 磁盘页 ABI | byte golden + corrupt/version RED |
| `AP-PAGE-MEM` | device 内存页 ABI 与 decoder | CPU/device digest equality |
| `AP-BAKER` | BLAS/VT/OMM 分腿与条件门 | per-leg gate，禁止互相充绿 |
| `AP-MANIFEST` | shader/PSO/DDC 与 package/chunk | merge/conflict/dependency/SBOM corpus |

**错误码策略**：本 RFC 只冻结诊断类别（schema invalid、non-canonical、DDC corruption、tool disallowed、ABI unsupported、license/SBOM incomplete）。不预造 RX 号；实现期按真实可达类别从 actual next-free 追加，en/zh message-key 同步。数字 CI step 同理不在 G8.1 materialize。

**gate key 事实源（评审 F11）**：验收一律引用 `G8_ACCEPTANCE_MAP.md` §2/§3 与 `CI_GATES.md` §4/§4.0 的唯一命名空间 `g8.p{0,1}.m##.<slug>` + `ci/g8_<slug>_smoke.py`（本 RFC 面对应：`g8.p0.m79.asset_determinism`、`g8.p0.m80.ddc_content_address`、`g8.p0.m81.gltf_import`、`g8.p0.m01.meshlet_page_builder`、`g8.p0.m04.page_format_abi`、`g8.p0.m85.shader_manifest_ddc` 的 G8.3 阶段腿、`g8.p1.m83.texture_transcode`）。三份文档的一致性由 `ci/check_g8_acceptance_map.py` 三向比对强制；本 RFC 不自行裁定 key 或脚本名。

## 6. feature gate、tracking 与实现序

### 6.1 治理门与实现门

1. G8.1：本 RFC Draft → 独立对抗性评审 → findings 全 disposition → Agent Approved。此阶段只改治理文档/RFC，不落 spec/实现/数字 workflow。
2. G8.2 entry interlock：必须读取 G7 close ref、RD-038 status/终态接入表与独立 override（若需要），不得读取人工口头“应当完成”。任一红即停止。
3. 互锁绿后校准 ledger actual next-free，落 spec 条款和 RED corpus；随后才可实现。

### 6.2 实现序（G8.3）

1. core schema + canonicalizer + manifest encoder（`AP-SCHEMA/AP-CANON`）。
2. immutable DDC 与 determinism verifier（M79/M80）。
3. glTF importer 与 meshoptimizer reference integration（M81/M82）。
4. texture cook + license/SBOM evidence（M83）。
5. M01 logical pages → M04 disk/memory ABI + CPU/device decoder goldens。
6. shader/PSO manifest 入 DDC（M85）。
7. BLAS baker；VT/OMM 仅在各自门已 go 时单独实现（M84）。
8. PackageChunk/install manifest（M88）；最后跑空缓存双构建与完整合规审计。

实现 feature gate 名称、tracking 载体和真实数字 CI 步骤由 spec PR/实现 PR 在互锁绿后确定；本 Draft 不预留。

## 7. 备选方案与裁决

| 备选 | 裁决 | 理由 |
|---|---|---|
| 各 importer 自行定义 JSON/cache key | 否决 | 无单一事实源，map/float/path/工具版本必然漂移，违 P-11 |
| 以文件路径+mtime 作 DDC key | 否决 | 不可移植、非内容寻址，隔离双构建会误命中/误失效 |
| 直接复用宿主 struct 内存布局作磁盘页 | 否决 | padding/编译器/端序漂移，无法冻结 device ABI |
| RXGB v1 原地扩展为压缩页 | 否决 | 会混淆既有 DAG 容器与 disk/memory 双 ABI；改用显式转换与新 major |
| 允许用户脚本换灵活性 | 否决 | 违反 D-309，引入隐式 I/O/网络/环境与不可审计供应链 |
| GPU codec 默认签名产物 | 否决 | 跨驱动确定性未证；只允许经双构建证明的 profile，CPU 为基线 |
| 所有 M84 分腿一次性实现 | 否决 | VT/OMM 尚受候选决策门，空实现会造成条件项假绿 |
| DDC 当源资产备份 | 否决 | DDC 必须可删可重建，源资产与派生物生命周期不同 |

## 8. 风险、止损与显式不做

| 风险 | 触发信号 | 止损 |
|---|---|---|
| canonicalizer 过宽 | 同语义多编码或 unknown field 被吞 | 收窄 schema；non-canonical 一律 RED，不做宽容读 |
| 资产面爆炸 | G8.3 出现 USD editor/MaterialX runtime/GUI | 保留 glTF+texture+DDC 最小闭环，其余回 G8.7 |
| page ABI 反向漂移 | G8.4 为 streamer 改 G8.3 格式 | 新 major + RFC 修订；旧 golden/decoder 继续保留 |
| vendor 许可误判 | 只按仓库顶层 LICENSE 放行 | 逐目录 scan + SBOM/notice digest；不明即拒绝签名 cook |
| GPU 工具非确定 | 相同输入跨 run/driver bytes 不同 | 降级为 local cache，CPU 产签名 artifact |
| DDC key 漏输入 | 改 tool/profile 仍命中旧对象 | 单变量 mutation corpus 阻断；扩 key schema major |
| 条件 baker 抢跑 | M40/M53 no-go 但 executor 在树 | governance gate 拒绝注册/实现；仅 schema hook 可存在 |

显式不做：runtime streaming/residency、多队列 I/O、asset editor、任意 build script、完整 USD composition、MaterialX 到生产 shader 的全链、remote DDC 部署拓扑、云复制策略、registry 服务与 GPU 主刚体。

## 9. 未决问题 / 关键裁决（Draft）

| 问题 | Draft 倾向 | 批准前需核 |
|---|---|---|
| Q1 canonical 格式 | deterministic CBOR 冻结子集 | 实现复杂度、第三方依赖与 malformed corpus 完整性 |
| Q2 hash | SHA-256 + domain-separated length-prefixed preimage | 与现有 `rurix-pkg` SHA-256 复用边界 |
| Q3 logical URI Unicode | 固定 Unicode 版本 NFC；key-bearing ID 仅 ASCII | Windows 路径与包 URI roundtrip corpus |
| Q4 RXGB v1 迁移 | 保持既有 reader，新增显式 converter | golden 兼容与旧 fixture 生命周期 |
| Q5 VT/OMM | schema hook 可冻结，executor 受 M40/M53 独立门 | governance validator 如何机验“未注册” |
| Q6 external codec | NVTT 等只作为显式非基线 profile | evidence 如何标 `non_signing/local_cache` |

以上只是 Draft 倾向；只有 §9.1 完成且 findings 已 disposition 后才能随 Agent Approved 冻结。

## 9.1 对抗性评审记录（10 §3/§7，D-409）

| 字段 | 值 |
|---|---|
| 评审者 provenance | `Assisted-by: Kiro:claude-opus-5 rfc-review-session`（≠ 起草 `Codex:gpt-5 rfc20-drafter-session`） |
| 评审轮次 | R1（独立会话只读评审，findings 由本会话逐条落改） |
| 日期 | 2026-08-02 |
| 评审镜头 | ① correctness（在树资产/DDC/页格式实况：`rurix-pkg` manifest/lock 红线、`rurix-geom-build` RXGB v1 与 `ClusterRecord::page_id` 恒 0、纹理仅恒等 `transcode` 留口、`spec/imageio.md` RXS-0114~0117 既有面）② redline（编号 claim、RD-039/040/041 backfill 字面、无门交付面、G8.4←M04 反向依赖、Draft 冒充许可）③ implementability（preimage/页 ABI/cook 确定性能否被 MAP 断言精确求值） |

**结论**：3 blocker + 8 major + 6 minor；blocker 与全部 major 已在正文实改，minor 逐条 disposition 后翻 **Agent Approved**。

| # | Finding | 严重度 | Disposition |
|---|---|---|---|
| F1 | 头部与 §2.3/§4.10/§4.12 把 M82/M84-BLAS/M88 写成 G8.3 交付面并称「已放行」，但三者既非 P0 也不在已 go P1 集合 `{M25,M72,M83}`，决策表无对应行，绕过 MAP §6 | **blocker** | **采纳，正文实改**：头部承接里程碑改为只列有 gate 的行，并在 §2.3 增「无门交付面纪律」段：三者只冻结 schema/职责，实现前必须先补决策表行 + 修订验收表 |
| F2 | §4.8 静默改写 RD-041 KTX2/Basis 触发机制字面（未提 `PagedResource::transcode` 留口与既有解包确定性单测口径） | **blocker** | **采纳，正文实改**：§4.8 首段逐字引 RD-041 该句，并冻结「运行时接入点唯一 = 既有 transcode 留口」「既有单测口径 0-byte」两条 |
| F3 | §4.10 把 SVT 门写成「M40 go 即可」，掩盖 G8.1 未完成的 SVT 门槛登记；且 §4.8 的「真实纹理资产管线」与门槛谓词同形，存在自触发风险 | **blocker** | **采纳，正文实改**：§4.10 新增「SVT 门槛缺项」段——本 RFC 不构成该登记；门槛登记在先、M40 判档在后；明记「真实纹理资产管线」≠「真实**大**纹理资产管线」，门-VT 维持 `SKIP=not-triggered` |
| F4 | §4.10 的 OMM 行省略「M50 先绿」与「承接波次 G8.7 穷举」两项前置 | major | **采纳，正文实改**：该行 G8.3 行为改为「不承接（决策表：G8.7 穷举）」，门控补 M50 先绿 + 资产/收益证据 + M53 判档 |
| F5 | DDC preimage 七段缺 `artifact_kind`/输出槽/`logical_page_id`，导致 M01 多页与 M83 多格式腿同 key 不同 payload，被「不同 payload 是 hard error」判死 | major | **采纳，正文实改**：§4.3 preimage 追加 `artifact_kind` 与 `output_id` 两段（页类含 `logical_page_id`），并加多产物消歧说明 |
| F6 | §4.5 只要求四类 preimage 单变量变更，而 §4.3 有七（现九）段；`abi_set`/`schema_set`/`import_recipe` 无 mutation 门 | major | **采纳，正文实改**：§4.5 改为「preimage 各段各一次单变量 mutation」，并注明 MAP 的四类是下限 |
| F7 | 两个页 ABI 的 magic/format ID/version 字面值与逐字段偏移未冻结，唯一出现的 ID 位于明示「不构成 stable 承诺」的示例中，MAP 要求 header 精确等于 golden → 不可机验 | major | **采纳，正文实改**：§4.9 增「字面值冻结归属」段——集合与拒录规则本 RFC 冻结，字面值/偏移/量化由 `AP-PAGE-DISK`/`AP-PAGE-MEM` 在 G8.3 一次冻结并同 PR 落 byte golden；§3 示例明标非规范；冻结前不可判 GREEN |
| F8 | disk→memory 版本映射表归属与冻结时点未定，若留到 G8.4 即触 R-G8-4 反向依赖 | major | **采纳，正文实改**：§4.9 明记该映射表属 G8.3 冻结面，随 `AP-PAGE-MEM` 与 M04 golden 落盘；G8.4 只引用 |
| F9 | §4.4 第 7 条为 GPU 签名开口但无运行次数/驱动矩阵判据，而 M79/M80/M83 均为 host 硬门 → 出现无门覆盖的签名通道 | major | **采纳，正文实改**：第 7 条改为「G8.3 内 GPU encoder 一律不产签名 artifact」，未来开启须先按 MAP §6 增行并给 N 次×驱动矩阵判据 |
| F10 | §4.8 拒绝冻结纹理三项 tolerance，而 MAP M83 要求「落在冻结 tolerance」，全仓无第三载体 | major | **采纳，正文实改**：指定载体与时序——`AP-TEX` 条款以 G8.3 首批 measured 样本冻结三项 tolerance，冻结前该门只能 RED（仍零预造数字） |
| F11 | §5 单方采用 MAP 口径而未指出与 CI_GATES/CONTRACT 的 key/脚本冲突，实现 PR 无法确定脚本路径 | major | **采纳，正文实改**：§5 增「gate key 事实源」段，逐面列出统一后的 canonical key，并交由 `ci/check_g8_acceptance_map.py` 强制；上游三份文档已统一 |
| F12 | 头部把 M85 写成「由 G8.3 实现」，而它是 G8.2+G8.3 双阶段门 | minor | **采纳，正文实改**：§2.3 M85 行标注「G8.3 阶段腿（G8.2 腿属 RFC-0019）」并引 `phase_g8_2_pass`/`phase_g8_3_pass` |
| F13 | §2.1 漏两条在树事实：`PagedResource::transcode` 留口 + io/transcode/upload 三预算；`spec/imageio.md` RXS-0114~0117 既有确定性图像编码面 | minor | **采纳，部分正文实改**：transcode 留口事实已由 §4.8 新段逐字锚定并声明为唯一运行时接入点；`imageio` 边界属 `AP-TEX` 条款起草时的 P-11 核对项，记录于本表，不在 G8.1 扩正文 |
| F14 | 依据决策行零引 RD-039/040/041 与决策表，而 M04/M83/M84 全由这三条 RD 分项门控 | minor | **留痕**：三条 RD 的门控关系已在 §2.4、§4.8、§4.10 逐处显式引用（含逐字 backfill），头部依据行保持决策号（D-3xx/P-xx）体例不混列 RD；实现 PR 的 traceability 以正文引用为准 |
| F15 | §5「映射为连续真实 RXS」= 隐性预占号段 | minor | **采纳，正文实改**：改为「按合入当时实测 next_free 顺位分配，不保证连续、不预留区间」 |
| F16 | §4.13「这延续 D-313」把 MIT/Apache codec 逐目录审计说成延续，越过 D-313（NVIDIA 再分发白名单）原范围 | minor | **采纳，正文实改**：改为「沿 D-313 审计模式加性扩展至非 NVIDIA vendor 组件，不改写字面、不扩大原决策范围」 |
| F17 | §4.2 Unicode 版本未钉（Q3 未决）→ NFC golden 不可复现；「NaN 全部归一」与「非有限值默认拒录」互斥 | minor | **驳回改正文，留 Q3 未决 + 实现期硬前置**：Q3 是本 RFC 明列的未决问题，钉住具体 Unicode 版本属 `AP-CANON` 条款实现期决定；但此处登记硬约束——`AP-CANON` 条款落盘时必须同时钉住 Unicode 版本并把 NaN 归一收窄为 schema 显式 opt-in 字段，否则不得落 golden |

**评审者对跨文档矛盾的移交**：go-P1 集合 vs `G8_PLAN` §2.3 波次表（M82/M88/M84 出现在波次表但无 gate）→ 已由 F1 的「无门交付面纪律」显式化，波次表列出的是**面**而非已授权交付；key/脚本双口径 → 已统一并加机器锁；`G8_PLAN` §3「G8.1 起 spec-first」→ 已勘误为 G8.2 起；OMM 承接波次 → 已统一为 G8.7；M83 tolerance 载体 → 由 `AP-TEX` 承担。

## 10. 稳定化与 provenance

- 本 RFC 批准只冻结治理/设计，不代表任何能力实现绿色。
- 互锁解除后：spec-first → RED corpus → gated implementation → acceptance evidence → 两个里程碑无重大修订 → stabilization report。
- M01/M04 一旦由 G8.3 evidence 冻结，变更需新 ABI major 与迁移测试；不得原地重解释旧 bytes。
- 起草 provenance：`Assisted-by: Codex:gpt-5 rfc20-drafter-session`。
- 评审 provenance：待 §9.1 独立填写；不得与起草 provenance 相同。

## 11. 规范与实现依据

- 仓内治理：[04_DESIGN_PRINCIPLES.md](../04_DESIGN_PRINCIPLES.md) P-01/P-05/P-09/P-11/P-12/P-13/P-14；[10_GOVERNANCE.md](../10_GOVERNANCE.md) D-402/D-406/D-409；[13_DECISION_LOG.md](../13_DECISION_LOG.md) D-308/D-309/D-311/D-313；[14_ENGINEERING_DISCIPLINE.md](../14_ENGINEERING_DISCIPLINE.md) §3/§5/§9。
- G8 事实源：[G8_PLAN](../milestones/g8/G8_PLAN.md) v1.2；[G8_CAPABILITY_MATRIX](../milestones/g8/G8_CAPABILITY_MATRIX.md) v1.2；[G8_ACCEPTANCE_MAP](../milestones/g8/G8_ACCEPTANCE_MAP.md)；[G8-R3](../milestones/g8/research/R3_GPU_API_ASSET_PIPELINE.md) §4。
- 现状锚：[`rurix-pkg`](../src/rurix-pkg/src/lib.rs)（manifest/lock/vendor/checksum、无任意 build script）；[`rurix-geom-build` serialize](../src/rurix-geom-build/src/serialize.rs)（RXGB v1）；[`rurix-geom-build` DAG](../src/rurix-geom-build/src/dag.rs)。
- 外部格式依据：RFC 8949（CBOR deterministic encoding）；Khronos glTF 2.0/KTX2；meshoptimizer；Basis Universal；KTX-Software；Arm astc-encoder。实际 pin、许可与 source digest 必须由实现 PR 的 vendor manifest/SBOM 给出，本 RFC 不凭名称代替审计。

---

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| Draft v0.1 | 2026-08-02 | AI 起草初版；冻结核心 schema、canonical serialization、DDC key、声明式工具 DAG、M79~M85/M88、M01/M04 双 ABI、M84 条件分腿与 vendor/SBOM 边界；§9.1 留空待独立评审。起草 provenance `Codex:gpt-5 rfc20-drafter-session` | Full RFC（Draft） |
| v1.0 | 2026-08-02 | **Agent Approved**：D-409 独立 provenance（`Kiro:claude-opus-5` ≠ 起草 `Codex:gpt-5`）三镜头评审完成，17 findings 全 disposition。正文实改要点：无门交付面纪律（M82/M84-BLAS/M88 不得静默实现）、RD-041 transcode 留口与单测口径逐字承接、SVT 门槛缺项显式化（禁自触发）、OMM 归 G8.7 且以 M50 先绿为前置、DDC preimage 增 `artifact_kind`/`output_id`、preimage 各段 mutation、页 ABI 字面值冻结归属与 disk→memory 映射表归 G8.3、G8.3 内禁 GPU 签名、`AP-TEX` 承担 tolerance 冻结、统一 canonical gate key、RXS 顺位不保证连续、D-313 措辞收回原范围。零 RXS/CI/RD/U/RX 数字 claim；批准不解锁实现。 | Full RFC（Agent Approved） |
