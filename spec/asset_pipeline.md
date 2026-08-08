# 资产管线条款(RFC-0020)

> G8.3 资产闭环条款文件。本文件按合入序追加;编号以 `registry/number_ledger.json` 实测为准。
> 并行批实测占用:**M01** RXS-0328~0331(见 `spec/geometry_pages.md` 若已落)、**M81** RXS-0332~0333、**M83** RXS-0334。主 agent 校准 ledger。

## 条款

### RXS-0332 AP-SCHEMA 六对象字段闭集与 logical_uri(G8.3 M81)

**依据**:RFC-0020 §4.1;G8_ACCEPTANCE_MAP §2 M81 行(共享 schema 面)。

**义务**(M81 实现锚;`rurix-asset::schema`):

- 六对象字段闭集:`SourceAsset` / `ImportRecipe` / `CookProfile` / `DerivedArtifact` / `ToolManifest` / `BuildManifest`(字段集合 = RFC-0020 §4.1 表)。
- 未知 major / 未知 required field → fail-closed。
- `logical_uri` 语法:禁绝对路径、反斜杠、`.` / `..` 段、NUL。
- schema 面与 glTF 导入共享失败语义:非法 `logical_uri` / 未知 required field 不得静默忽略。

> 注:本条由 M81 并行批消费;扩写以加性修订行追加,不得改号。

**测试要求**:

- `//@ spec: RXS-0332` 锚定于 `rurix-asset::schema`(≥1)
- `cargo test -p rurix-asset --lib` 覆盖 schema / logical_uri 红绿断言

### RXS-0333 AP-GLTF 严格导入与 canonical 六表(G8.3 M81)

**依据**:RFC-0020;G8_ACCEPTANCE_MAP §2 M81;`g8.p0.m81.gltf_import`。

**义务**(M81 实现锚;`rurix-asset::gltf`):

- 手写严格 JSON parser:拒重复 key、非法 UTF-8/裸控制字符;GLB magic/version/chunk 全校验。
- 七类验证(先验证后产物):accessor/bufferView 边界、index 值域、node 无环、`extensionsRequired ⊆ allowlist`、引用存在、sparse 边界、以及 extras 纪律。
- 扩展 allowlist v1:`KHR_materials_unlit`、`KHR_texture_basisu`、`KHR_mesh_quantization`。
- 产物 = canonical scene/node/mesh/primitive/material/texture 六表 + digest。
- 同输入两次 `import-gltf --emit-digest` 六表 count/digest 逐字节稳定;不得静默丢字段(`coverage_complete=true`)。
- fail-closed 必达类(至少):`extension_not_allowed` / `accessor_oob` / `missing_buffer`;JSON 可解析但语义非法不得仅靠"能 parse"充绿。

**conformance 语料**(目录闭集):

- accept:`conformance/asset/gltf/accept/`(≥3;`.gltf`/`.glb` + 配对 `.golden.json`)
- reject:`conformance/asset/gltf/reject/`(≥3;含越界扩展 / accessor OOB / 缺失 buffer)

> 注:细则与 RED 语料由 M81 smoke/conformance 闭环;本条可加性扩写,不得改号。

**测试要求**:

- `//@ spec: RXS-0333` 锚定于 `rurix-asset::gltf`(≥1)
- `ci/g8_gltf_import_smoke.py --gate g8.p0.m81.gltf_import`(numeric_step=106)
- evidence schema:`milestones/g8/g8_m81_gltf_import_evidence_schema.json`

### RXS-0334 AP-TEX 纹理 cook 语义与确定性 codec(G8.3 M83)

**依据**:RFC-0020 §4.8;G8_ACCEPTANCE_MAP §3 M83;`g8.p1.m83.texture_transcode`。

**输入语义 key(recipe 闭集,全入签名)**:

- `semantics` ∈ {`color`,`normal`,`mask`}
- `color_space` ∈ {`srgb`,`linear`}(首版 cook 记录但不改编码路径)
- `mip_filter` ∈ {`box`,`none`}(首版仅 `none`=单 mip)
- `alpha_coverage_threshold` ∈ u8(默认 128)
- `encoder_profile` ∈ {`win-vulkan-bcn-v1`,`mobile-astc-v1`}
- `quality` ∈ u8(过渡编码器忽略;保留字段位)

**四条产物腿与 `artifact_kind`**:

| 腿 | artifact_kind | 容器 / 载荷 | 过渡态诚实边界 |
|---|---|---|---|
| KTX2 | `texture.ktx2` | KTX2 magic `«KTX 20»…`,supercompressionScheme=**0**(禁 zstd) | 过渡期载荷可为真实 BC7 块(非 UASTC);完整 basis_universal 合入后升 UASTC |
| Basis | `texture.basis` | `.basis` ETC1S | **未实现 → 缺腿 FAIL**(禁占位文件充绿) |
| BCn | `texture.bcn` | `RXBC` 容器 + BC7/BC5/BC4 块 | 过渡期 **BC7 真实非空**必达;`win-vulkan-bcn-v1` 标注 `BC7_UNORM` |
| ASTC | `texture.astc` | `RXAS` 容器 + ASTC 4×4 | 过渡期允许 void-extent 实块(非常量全零占位) |

**确定性钳制**(实现义务,违例 = FAIL):

1. 编码线程恒 **1**;
2. 固定 seed / 固定块遍历序(行主序 4×4);
3. **禁止** zstd / 任意 supercompression;
4. 同 source+profile **两次 cook 各腿逐字节相等**;
5. 只改扩展名 / 全零占位 codec / 缺腿 = FAIL。

**三项 tolerance**(首批 measured 后字面冻结;解码器 = `rurix-asset::bcdec` 独立实现):

| 项 | 冻结值 |
|---|---|
| 颜色误差(每通道 max Δ,8-bit) | ≤ **48** |
| normal length 平均绝对偏差 | ≤ **0.15** |
| alpha coverage 漂移(阈值 128) | ≤ **0.08** |

**vendor / SBOM**:`src/rurix-basis-sys/VENDOR.md` + `NOTICE` + `SBOM.md` 必须存在;FFI 版本串 == VENDOR.md pin。

**运行时红线**:不新建运行时解包通道;`PagedResource::transcode` 恒等留口 0-byte(RD-041)。

**测试要求**:

- `ci/g8_texture_transcode_smoke.py --gate g8.p1.m83.texture_transcode`
- ≥1 `//@ spec: RXS-0334` 锚定(`rurix-asset::texture`)

### RXS-0335 AP-CANON deterministic CBOR 与 RXAP envelope(G8.3 M79)

**依据**:RFC-0020 §4.2;设计案 §3.1;`g8.p0.m79.asset_determinism`。

**义务**(`rurix-asset::canon`):

- 冻结子集:map key = 非负整数 field-ID(严格递增);整数最短编码;禁 indefinite-length;禁浮点;用户字符串限 ASCII 可打印。
- envelope:`magic "RXAP" | schema_id u32 | major u16 | minor u16 | canonicalizer_version u32 | payload_len u64 | schema_digest 32B | payload_digest 32B | payload`(全 LE)。
- `canonicalizer_version` 进 schema digest;`Unicode/NFC` 与浮点面未开放(开放前须条款修订)。

**测试要求**:

- `//@ spec: RXS-0335` 锚定于 `rurix-asset::canon`
- `conformance/asset/canon/{accept,reject}` + `rxcook canon-check`

### RXS-0336 AP-GRAPH 声明式工具 DAG(G8.3 M79)

**依据**:RFC-0020 §4.4;设计案 §3.1。

**义务**(`rurix-asset::graph`):

- 节点五元组:`tool_id + tool_digest + typed_inputs + typed_outputs + canonical_params`。
- 无环;未注册 tool fail-closed;类型系统无 shell/`build.rs`/网络节点。
- 首批注册工具:`rurix.gltf.import.v1` / `rurix.geom.pages.v1` / `rurix.texture.cook.v1`;三者在 M79 DAG 中**均须真实执行**,不得以占位注册(`let _ = TOOL_*`)或跳过节点充数。
- **真实数据流边**:`rurix.geom.pages.v1` 的 `typed_inputs` 须消费 `rurix.gltf.import.v1` 的网格输出(`artifact.gltf_mesh`);禁以程序化几何(`TriMesh::uv_sphere`/`cube` 等)旁路替代导入网格作为 geom 上游。程序化网格仅许用于 M01/M04 自身单测与 golden,不得进入 M79 DAG 的 geom 上游。
- **产物真实性 fail-closed**:签名 artifact 须通过容器识别(如 KTX2 magic)与非常量填充检查;全零/单字节常量填充载荷即 `Err`,不得计入绿。
- 调度序不进签名输出。

**测试要求**:

- `//@ spec: RXS-0336` 锚定于 `rurix-asset::graph`
- `conformance/asset/graph/reject` + graph 单测环/未注册工具

### RXS-0337 AP-GRAPH 双构建与单变量 mutation(G8.3 M79)

**依据**:RFC-0020 §4.6;设计案 §3.1。

**义务**(`rurix-asset::verify` + `rxcook verify --double-build`):

- 两隔离绝对路径根 + 起始空产物目录;同 plan 双构建 → DAG/artifact/manifest digest 逐字节相等。
- 四类单变量 mutation(依赖内容 / recipe / profile / tool version)翻转受影响 key,无关节点 key 稳定。
- 「依赖内容」腿须为**真实源字节 mutation**(两份 glTF 文档 JSON 逐字节相同、仅外部顶点缓冲字节不同),不得以 recipe/profile 类**参数**变更冒充依赖内容变更;该腿同时充当「导入网格真实流入 geom 下游」的守门断言——若 geom 由程序化几何旁路产出,源内容变化不传导,本腿必红。
- 签名字节扫描:零绝对路径/时间戳/PID 字面。

**测试要求**:

- `//@ spec: RXS-0337` 锚定于 `rurix-asset::verify`
- `ci/g8_asset_determinism_smoke.py --gate g8.p0.m79.asset_determinism`(numeric_step 合入时回填)

### RXS-0343 AP-DDC 九段 preimage 与不可变 CAS(G8.3 M80)

**依据**:RFC-0020 §4.3;设计案 §3.2;`g8.p0.m80.ddc_content_address`。

**义务**(`rurix-asset::ddc`):

- key = `SHA-256("rurix-ddc-artifact-v1\0" || (len_u64_le || canonical(seg)) × 9)`；段序:source_set / dependency_keys / import_recipe / cook_profile / tool_chain / schema_set / abi_set / artifact_kind / output_id。
- CAS:`objects/<aa>/<64hex>` + `meta/<aa>/<64hex>.rxap`；tmp 原子 rename。
- get 复核 byte_len + payload SHA-256；不符 → Corruption 拒脏对象。
- put 同 key 异 payload → KeyCollision；evict 后可重建同 key。

**测试要求**:

- `//@ spec: RXS-0343` 锚定于 `rurix-asset::ddc`
- `ci/g8_ddc_content_address_smoke.py --gate g8.p0.m80.ddc_content_address`(numeric_step=110)

## 修订历史

| 版本 | 日期 | 说明 | 状态 |
|---|---|---|---|
| v1.0 | 2026-08-06 | M83:`### RXS-0334` AP-TEX。并行避撞:为 M81 落 `### RXS-0332`(AP-SCHEMA)+ `### RXS-0333`(AP-GLTF)条款头要点(M81 可加性扩写)。实测号 **0332/0333/0334**。 | Draft |
| v1.1 | 2026-08-06 | M81 加性扩写:`### RXS-0332`/`### RXS-0333` 补测试要求、conformance 语料目录与 fail-closed 必达类;挂钩 `ci/g8_gltf_import_smoke.py`(numeric_step=106)。**未改** `### RXS-0334`。 | Draft |
| v1.2 | 2026-08-06 | M79:`### RXS-0335` AP-CANON + `### RXS-0336` AP-GRAPH + `### RXS-0337` 双构建 mutation。 | Draft |
| v1.3 | 2026-08-06 | M80:`### RXS-0343` AP-DDC。 | Draft |
| v1.4 | 2026-08-07 | M79 降级清零(加性收紧,未改既有字面):`### RXS-0336` 增「三工具均须真实执行」+「真实数据流边(禁程序化几何旁路作 geom 上游)」+「产物真实性 fail-closed」;`### RXS-0337` 增「依赖内容腿须为真实源字节 mutation」并明确其兼作导入网格流入守门断言。**未改** `### RXS-0335`/`0343`。 | Draft |

