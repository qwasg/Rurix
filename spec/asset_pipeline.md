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

## 修订历史

| 版本 | 日期 | 说明 | 状态 |
|---|---|---|---|
| v1.0 | 2026-08-06 | M83:`### RXS-0334` AP-TEX。并行避撞:为 M81 落 `### RXS-0332`(AP-SCHEMA)+ `### RXS-0333`(AP-GLTF)条款头要点(M81 可加性扩写)。实测号 **0332/0333/0334**。 | Draft |
| v1.1 | 2026-08-06 | M81 加性扩写:`### RXS-0332`/`### RXS-0333` 补测试要求、conformance 语料目录与 fail-closed 必达类;挂钩 `ci/g8_gltf_import_smoke.py`(numeric_step=106)。**未改** `### RXS-0334`。 | Draft |
