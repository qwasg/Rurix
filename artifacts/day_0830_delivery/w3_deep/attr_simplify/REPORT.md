# G36 留窗 #96「代理属性保持简化(UV/法线)」——geom-build crate 面交付报告

- 日期:2026-08-29(G37 W3 子任务;TODO #96 crate 面)
- 修改域:`src/rurix-geom-build/`(mesh.rs / dag.rs / qem.rs / lib.rs)+ 本交付物目录
- 判档:**crate 面 + 单测 + 消费面设计**(全链 cook→bake→运行时 gather 超出本窗,分界登记见 §6)
- 纪律核验:未跑 GPU;未 `--release`;未碰 target-night / kernels / milestones / registry / ci;
  未改 g31_window_present.rs / g14_3_lane_body.rs;既有函数签名与默认行为 0 改动(全加性入口)

## 1. 现状侦察(锚)

| 项 | 事实 |
|---|---|
| TriMesh | `mesh.rs` 仅 position+index,模块头写明「P0 不引入法线/UV」 |
| QEM(v1.1.5) | `qem.rs::simplify_group_qem`(位置 QEM + fold-over 拒绝 + 锁定端逐位保持);`simplify_free_mesh` 为 HLOD 合并简化(#67/#97)事实源 |
| 默认简化 | `build_dag` 恒 ShortestEdge;质量档经 `DagBuildParams::quality()` 加性(v1.1.5 先例) |
| HLOD 烘焙 | `rurix-asset/src/hlod.rs::bake_hlod_merged`(域外):位置 bits 焊接 → 逐层直调 `qem::simplify_free_mesh`,产物 RXHL 三角为纯位置 9×f32 |
| G36 消费缺口 | v1.1.7:侧表 gather 对代理三角(簇粗代理/cell 代理)强制 tritex=−1 常量回退(纹理均值)——本行为其留窗 |
| m90 golden | `tests/virtual_geometry/golden/m90_dag_digest_manifest.json`,`canonical_dag_sha256 = 68def899…de926`(uv_sphere(1.0,24,24) build_asset_dag static);探针 `cargo run -p rurix-asset --bin g9_m90_probe` |

## 2. 设计

### 2.1 属性扩面结构(mesh.rs)

**伴随结构,`TriMesh` 本体 0-byte**:

- `TriMeshAttrs { uv: Vec<[f32;2]>, normal: Option<Vec<[f32;3]>> }`——与 `positions` 等长平行
  (meshopt `simplifyWithAttributes` 顶点属性口径:UV 接缝由上游按「位置+属性」预拆分顶点承载);
- `AttrTriMesh { mesh, attrs }`——校验后载体,新入口 `TriMesh::with_attrs(self, attrs) -> Result<AttrTriMesh, AttrMeshError>`;
- `AttrMeshError`(typed Err 六变体:EmptyMesh / IndicesNotTriples / IndexOutOfBounds /
  UvLengthMismatch / NormalLengthMismatch / NonFinite)+ 单一校验事实源 `validate_attr_input`
  (三个消费入口共用;字段公开可绕过构造,消费入口再次 fail-closed)。

选型理由:泛型化 TriMesh 或改字段都会侵入既有 API/序列化;伴随结构 + 平行表是对既有 API
侵入最小、且与仓内既有「平行扩展表」惯例(ClusterDagV2.skin/clas、leaf_source_tris)同构的方案。

### 2.2 QEM 属性策略:位置 QEM 主导 + 收缩点线段投影插值

候选二选一(任务书字面):meshopt 风格属性加权四次型 vs 收缩点插值。**选后者**,理由:

1. **实现风险**:属性加权四次型 = (3+m) 维二次型 + (3+m)×(3+m) 求解,数值奇异面与
   属性权重标定(meshopt `attribute_weights` 需逐资产调参)引入新失败模式;收缩点插值零新增
   数值求解,复用既有全部机核。
2. **0-byte 结构性可证**:属性完全不参与选边 / 定新位置 / fold-over 判定——位置/拓扑决策与
   无属性链**逐字同路**(加权四次型会改变选边序,QEM 档产物字节必漂,牵连 g31_cluster_lod_bake
   双构建锚)。
3. **单测可证**:插值有闭式可断言性质(见 §5:仿射 UV 精确、误差受位移上界支配、锁定端逐位)。
4. Nanite 参照即「簇内属性插值」(#96 行字面);meshopt 加权四次型登记为后续质量档对照臂。

**精确规则**(`qem.rs::merge_attrs_at`):坍塌 (keep,drop)→new_pos 后,
`t = clamp(⟨new−p_keep, p_drop−p_keep⟩ / ‖p_drop−p_keep‖², 0, 1)`(f64):

- `t = 0/1`(锁定端收缩、端点候选、投影出端)= 端点属性**逐位拷贝**——锁定顶点/组边界的
  属性裂缝保护与位置面同一律免费获得(new_pos 逐位等于端点 ⇒ 点积精确为 0/len²);
- 内点 = 线性插值(clamp ⇒ 属性恒在两端点凸包内,不外插飞出原面片邻域);
- 法线插值后归一化,近反向抵消退化时 keep 端逐位保持。

**可证误差界**(单次简化内,归纳于误差累计式):`|uv − f(pos)| ≤ |∇f|·max_error`
(f 为位置的仿射场时;单测按 0.5 梯度断言,实测远低于界,见 §5)。

**ShortestEdge 属性面平凡**:端点保持收缩 keep 端位置不动 ⇒ keep 属性逐位保持,drop 属性随
顶点消亡,属性只参与末端压缩重映射——`build_dag_attrs` 因此支持全部 `DagBuildParams` 档位。

### 2.3 UV 接缝:保守锁定(v1 纪律)

同位置 bits 出现于多顶点 id = 接缝顶点(预拆分两侧拷贝)。两侧独立收缩会产生几何裂缝,v1
一律锁定(`attr_seam_flags`,DAG 链逐层重算,自由网格链即锁定集)——接缝两侧位置与 UV
逐位保持,无裂缝、无图集串扰;压缩自由度损失如实计入既有 stuck 统计。meshopt 的位置重映射
协动简化(接缝两侧同步坍塌)登记为后续质量档。

### 2.4 簇 DAG / HLOD 重导出带 UV 的两条腿

- **簇腿** `dag::build_dag_attrs(&AttrTriMesh, &DagBuildParams) -> Result<ClusterDagAttrs, _>`:
  产物 `ClusterDagAttrs { base: ClusterDag, vertex_uv, vertex_normal }`,属性表与 `base.vertices`
  数据段等长平行(切片口径同 `cluster_vertices`,访问器 `cluster_uvs`/`cluster_normals`)——
  粗簇/代理三角自此带真 UV。跨组焊接键扩为「位置+属性 bits」(接缝拷贝不被误并;锁定顶点
  位置与属性双逐位 ⇒ 跨组键一致,无裂缝)。**RXGB v1 序列化与 canonical_bytes 均不含属性表**
  (与 leaf_source_tris 同待遇:内存直构面,m90 golden 0-byte)。
- **HLOD 合并腿** `qem::simplify_free_mesh_attrs(positions, indices, uv, normal, target)
  -> Result<AttrSimplifyOutput, _>`:输出即带 UV 数组的代理三角集(`indices` 每三连一个代理三角,
  corner UV 经 `uv[indices[k]]`)。未来 `bake_hlod_merged` UV 变体把焊接键从位置 bits 扩为
  (位置,UV[,法线]) bits 后直调本函数(域外,分界登记 §6)。

### 2.5 默认路径 0-byte 的结构性论证

全部改动经 `Option` 线程化汇入单一实现(`build_dag_impl` / `simplify_group_impl` /
`simplify_group_qem_impl`),`None` 路径与既有代码逐字同路:属性仅在①收缩执行后插值
②焊接键扩位③接缝锁定三点生效,三点全部条件于 `Some`;无任何浮点运算/迭代序/决策变更。
既有公共与 `pub(crate)` 函数签名全部保持(`simplify_group` 二元包装保留供既有单测锚,
生产路径直调 impl)。字节不变由 m90 golden 探针 + 既有 45 项单测(含
`build_dag == build_dag_kind(ShortestEdge)` 逐位锚、QEM/quality 双构建锚)双证,见 §5。

## 3. 新增入口清单

公共 API(全加性):

| 入口 | 位置 | 说明 |
|---|---|---|
| `TriMeshAttrs` | mesh.rs | 顶点属性平行表(uv 必备 + normal 可选) |
| `AttrTriMesh` | mesh.rs | TriMesh + 属性的校验后载体 |
| `AttrMeshError` | mesh.rs | 退化输入 typed Err(六变体,Display + Error) |
| `TriMesh::with_attrs` | mesh.rs | 属性附着入口(typed Err) |
| `dag::ClusterDagAttrs`(+`cluster_uvs`/`cluster_normals`) | dag.rs | 簇 DAG 属性链产物(平行表) |
| `dag::build_dag_attrs` | dag.rs | 属性保持 DAG 构建(任意 DagBuildParams;typed Err) |
| `qem::AttrSimplifyOutput` | qem.rs | 属性保持自由网格简化产物 |
| `qem::simplify_free_mesh_attrs` | qem.rs | HLOD 合并简化带 UV 腿(typed Err) |

内部(`pub(crate)`/私有,签名不动面之外的加性):`dag::SubMeshAttrs`(构建期属性载体)、
`dag::attr_seam_flags`、`dag::simplify_group_impl`、`qem::simplify_group_qem_impl`、
`qem::merge_attrs_at`;`extract_group` 增返「局部→全局」映射(私有,2 调用点同步)。

lib.rs 根导出追加:`AttrMeshError / AttrTriMesh / TriMeshAttrs / ClusterDagAttrs / build_dag_attrs`
(qem 入口维持模块路径消费惯例,与 `simplify_free_mesh` 同)。

## 4. 单测(新增 7 项,与既有 38 项同锚)

| 测试 | 断言 |
|---|---|
| mesh::with_attrs_valid_and_degenerate_typed_err | 合法两态 + 六臂 typed Err(含绕过 TriMesh::new 的字面构造) |
| qem::attr_simplify_plane_affine_uv_exact | 平面仿射 UV 逐点 <1e-4(平面退化候选恒在坍塌边线段上 ⇒ 插值对仿射场无损) |
| qem::attr_simplify_sphere_bitmatch_plain_and_uv_bounded | **位置/拓扑/误差与 `simplify_free_mesh` 逐位一致**;UV ≤ 0.5×max_error+1e-4 且恒在凸包盒;法线单位性 + 径向对齐 cos>0.9 |
| qem::attr_simplify_uv_seam_locked_bitexact | 双图集接缝五顶点:两侧 (位置,UV) 拷贝恰 2 份全逐位存活,无串扰 |
| qem::attr_simplify_degenerate_typed_err | 六臂 typed Err(空/非3倍/越界/UV 不齐/法线不齐/非有限) |
| qem::attr_simplify_double_run_deterministic | 位置/拓扑/UV/误差全 bit 级双跑一致 |
| dag::attr_dag_zero_position_drift_and_uv_bounded | 默认档+质量档两臂:**`attr.base` canonical_bytes == 无属性链逐位**;属性表与顶点段平行(逐簇切片口径);叶层 UV 逐位等于输入;全层 UV 有界+凸包;双构建(含 UV 位);EmptyMesh typed Err |

## 5. 验证结果(默认 dev profile / 默认 target;无 GPU、无 release)

```
cargo test -p rurix-geom-build
  45 passed; 0 failed(38 既有 + 7 新增)——连跑三轮全绿(stuck 对照臂并行稳定)
cargo run -p rurix-asset --bin g9_m90_probe        ← m90 DAG digest golden
  canonical_dag_sha256 = 68def89991d49a93f7325dbcf95632b2faa4dd9bde9c946a7a849d6f477de926
  golden_manifest_match = true(manifest 逐字命中,不漂);double_build_byte_equal 等 6 checks 全 true
cargo test -p rurix-asset --lib hlod               ← 下游消费面交叉验证
  6 passed(含 merged_bake_invariants_and_quality:QEM 质量烘焙判据不漂)
```

实测数据(测试打印,如实登记):

```
[attr_free_sphere] max_uv_err=0.005429  pos_err_bound=0.155494   ← 0.5×err 支配界成立(0.0777),实测低一个量级
[attr_dag] simplify=ShortestEdge levels=10 max_uv_err=0.000000   ← 端点保持 ⇒ UV 全链逐位精确
[attr_dag] simplify=Qem          levels=10 max_uv_err=0.159246   ← 十层累计(层间误差为 max 语义、UV 漂移加性累计,<0.30 断言)
[quality_vs_legacy] quality stuck=46 < legacy stuck=64           ← 既有对照登记未漂
```

## 6. 运行时消费面设计(不改代码,设计登记)+ 分界登记

**消费链设计**(G36 侧表 gather 的 tritex=−1 回退退役路径):

1. **bake 侧(rurix-asset,下一窗)**:
   - `g31_cluster_lod_bake` 属性臂:RXCS dump 需带 UV(cook 面,见分界)→ `build_dag_attrs`
     → RXGB v1 字节不动 + 属性 sidecar 段(digest 寻址,双构建字节相等判据沿用);
   - `bake_hlod_merged` UV 变体:焊接键位置 bits → (位置,UV) bits,直调
     `simplify_free_mesh_attrs`;RXHL 三角 9×f32 → 追加 6×f32 corner UV(RXHL v2 或平行段,
     v1 字节不动);cell 代理三角自此带 UV。
2. **运行时(g14_3 lane_body / g36 侧表,本窗 0 改动)**:
   - geo_rebuild 统一重建时,`TriProvenance::簇粗代理/cell 代理` 三角从簇属性表 / RXHL v2 取
     corner UV 三元组,侧表按既有 Src 三角同形位布局写入(UV 位保真面复用);
   - gather 侧仅改判定:代理三角 tritex 不再强制 −1,走与 Src 三角同一采样路径(常量均值回退
     仅保留给「无 UV 资产」臂);
   - 验收锚建议:①代理臂侧表 UV 位保真(与 bake 产物逐位)②`--wp-hlod full`/`leaf` 极限臂
     与 off 逐位不变(代理不出帧时 0 语义)③代理出帧臂纹理采样视觉指标(bbox 色块口径沿用
     day_0828 B 期方法)。

**分界登记**(本窗如实不做):

| 面 | 状态 |
|---|---|
| geom-build crate 面(扩面/QEM 属性/两条重导出腿/单测) | ✅ 本窗交付 |
| cook 面(RXCS dump 带 UV;gltf→RXGB 属性直通) | ❌ 未动(rurix-asset 域) |
| bake 面(cluster bake 属性臂 / bake_hlod_merged UV 变体 / RXHL v2·RXGB 属性段格式) | ❌ 未动(设计如上) |
| 运行时 gather(lane_body / g36 侧表 / tritex 判定) | ❌ 未动(任务书禁改;设计如上) |
| 切线(#96 行字面「UV/法线/切线」) | ❌ 未做(UV+法线先行;切线待法线消费面立项) |
| meshopt 属性加权四次型 / 接缝位置重映射协动 | ❌ 后续质量档对照臂登记 |
