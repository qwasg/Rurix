# G38 T4 — TODO #96 属性保持简化消费面(bake 双变体 + 运行时 gather,tritex −1 强制回退退役)

日期:2026-08-30 | agent:T4 实施 agent | 状态:CPU 面全绿,GPU 验收留主 agent 批次 1

## 0. 一句话总结

RXCS/RXCP/RXHL 三格式各升 v2(UV 加性承载,v1 路径全部逐位不变自证),bake 双变体
(`--attrs on`)接通 rurix-geom-build #96 crate 面;运行时新增 `gather_tri_uv_attrs` /
`geo_patch_proxy_tritex_v2`(+heap 同律),g34 两车道切新函数——v2 资产的代理三角带真
corner UV 走与 Src 三角同一图集采样路径,仅无 UV 资产(v1)维持 −1 常量回退。

## 1. 改动清单(文件 + 行号,全部为当前工作树行号)

### rurix-geom-build(加性,RXGB 冻结面 0 动)
| 文件 | 位置 | 内容 |
| --- | --- | --- |
| src/rurix-geom-build/src/dag.rs | 834(`DagAttrsError`)/ 858(`build_asset_dag_attrs_params`) | 属性资产级包装:panic→typed Err 转译 + base 面单调性核验,与 `build_asset_dag_params` 同律;错误域 = Attr(AttrMeshError) ∪ Dag(DagError) 新枚举(不动既有 DagError 变体集) |
| src/rurix-geom-build/src/dag.rs | 2044(测试 `asset_attrs_wrapper_matches_direct_and_rejects_bad`) | 包装 ≡ 直调(canonical_bytes + UV bits + leaf_source_tris)+ 退化输入 Attr 域 typed Err |
| src/rurix-geom-build/src/lib.rs | 44-51 | 导出 `DagAttrsError` / `build_asset_dag_attrs_params` |

### rurix-asset(RXHL v2 + 两 bake 工具属性臂)
| 文件 | 位置 | 内容 |
| --- | --- | --- |
| src/rurix-asset/src/hlod.rs | 29 | `HLOD_ASSET_VERSION_ATTRS: u16 = 2`(v1 常量不动) |
| src/rurix-asset/src/hlod.rs | 36 / 55 | `ComponentGeometry` / `HlodComponentProxy` 扩 `uv: Option<Vec<[f32;6]>>`(与 triangles 平行,corner UV) |
| src/rurix-asset/src/hlod.rs | 108 | `validate_input`:UV 齐次性(全 Some/全 None)+ 行数平行 + 有限性 fail-closed |
| src/rurix-asset/src/hlod.rs | 186 | `bake_hlod`(stride 抽面):不承载属性臂,产物 uv 恒 None ⇒ 编码恒 v1(M111 golden 锚 0-byte) |
| src/rurix-asset/src/hlod.rs | 322 | `bake_hlod_merged` 属性臂:canonical 排序键扩 UV bits(经索引置换稳定排序,无 UV 键尾常量 ⇒ 旧序逐位)→ 焊接键 (位置,UV) bits → 逐层 `qem::simplify_free_mesh_attrs` → 代理三角带 corner UV;无 UV 臂 = 既有代码逐字分支 |
| src/rurix-asset/src/hlod.rs | 237 | `encode_hlod_asset`:全 proxy 带 UV ⇒ v2(每三角 9×f32 后追加 6×f32),全无 ⇒ v1 字节不变;混合 = assert 拒(构造不变量) |
| src/rurix-asset/src/hlod.rs | 674(测试 `merged_bake_uv_arm_invariants`) | 双构建/声明序免疫/UV 扰动分叉/v2 版本字节/位置面与无属性臂逐位一致(UV=f(位置) 无接缝 fixture,crate 契约锚)/L0 UV 逐位透传/混合与不齐输入拒 |
| src/rurix-asset/src/bin/g31_cluster_lod_bake.rs | 97(reader v1\|v2)/ 186(`partition_blocks` 属性焊接)/ 397(`write_pack` v2)/ 517(`bake` 双臂)/ 698(`--attrs`) | 见 §2 RXCP v2 |
| src/rurix-asset/src/bin/g31_wp_hlod_bake.rs | 96(reader v1\|v2)/ 224(`bake` 双臂,comp_map 平行 UV)/ 452(`--attrs`) | 见 §2 RXHL v2(RXWH 容器维持 v1) |

### rurix-render(lane_body 加性 + 两 g34 车道切换 + dump 接线)
| 文件 | 位置 | 内容 |
| --- | --- | --- |
| g14_3_lane/g14_3_lane_body.rs | 2568 `dump_scene_rxcs` | 就地升级(独占接线面):新参 `tri_uv: Option<&[f32]>`;Some ⇒ v2 尾接 UV 段,None ⇒ v1 字节逐位不变(自证见 §3) |
| 同上 | 2661 `ClusterPack` + 2674 `blocks_vertex_uv: Option<Vec<Vec<[f32;2]>>>` | 逐块顶点 UV 表挂 **pack 级**(与 blocks 平行)——`ClusterPackBlock` 字段集 0 动,g31_frame_cut_arm.rs:1582 的字面构造夹具(并行 agent 在飞文件)零触碰 |
| 同上 | 2711 `read_cluster_pack` | 收 v1\|v2;v2 在 vertices 段后读 UV 平行表;v1 行为逐位不变 |
| 同上 | 3348 `WpHlodLevels` + `levels_uv: Option<Vec<Vec<[f32;6]>>>`;3393 `decode_rxhl` | 收 v1\|v2;v2 每三角 9×f32 后读 6×f32;v1 行为逐位不变 |
| 同上 | 5060 `gather_tri_uv_attrs` | 新函数:Src 位保真同旧;ClusterCoarse 按簇局部索引取 UV 三元组(与 geo_rebuild 顶点取数同式);WpProxy 按层内三角号取 RXHL v2 行;段内序 = prov 相邻同源计数器(geo_rebuild 尾接连续段不变量,越界 fail-closed);无 UV 源回落 [0;6](旧语义逐位等价) |
| 同上 | 5134 `geo_patch_proxy_tritex_v2` / 5178 `geo_patch_proxy_tritex_heap_v2` | 仅无 UV 数据的代理三角置 −1;带 UV 者保留 tri_mat 派生槽号(退役面);tritex_bytes/tex_tris 重建 + 空接线 fail-closed 同旧律;heap 步幅 2 同律 |
| 同上 | 4896/4922/4952 既有 `gather_tri_uv` / `geo_patch_proxy_tritex`(_heap) | **函数体 0 改动**编译保留(#[allow(dead_code)] 原有) |
| g14_3_pipeline_perf.rs | 184-252(--dump-scene 臂) | UV sink 装配接线:默认 `--uv on` ⇒ RXCS v2;`--uv off` = 无 UV 资产逃生口(装配面对缺 TEXCOORD_0 fail-closed);打印追加 `rxcs_v=` 字段(门正则不消费) |
| g34_full_lane.rs | 1791(gather)/ 1930(patch) | 切 `gather_tri_uv_attrs` / `geo_patch_proxy_tritex_v2`(UV 源 = GeoApplied.cluster 包 + wp ctx.pack);host 金标准克隆消费补丁后 `assets.tritex`/`texuv_bytes`(G34HostGold::build 890-931 行既有克隆面)⇒ 两臂自动一致,无需另改 |
| g34_full_lane/g34_2_hzb.rs | 1803(gather)/ 1935(patch) | 同 main ③.4/③.6 同律切换 |

**未触碰(纪律面)**:g31_window_present.rs(0-byte);lane_body lamp/聚类区域(L2274 附近)、
render_exec/frame_cut(并行 agent 在飞);RXGB serialize.rs/canonical_bytes;wp_hlod/cluster/g36
门脚本本体;既有 schema 文件;`bake OK`/`dump-scene OK` 等门正则消费行的既有字段序。

## 2. 格式 v2 布局表(三份)

### RXCS v2(场景 dump,bin-local 非冻结;writer lane_body:2568,readers = 两 bake bin 镜像)
```
magic "RXCS" (4B)
version u32 = 2                      ← v1 唯一头部差异
n u32(三角数) g u32(组数)
gltf_sha256 (64B ASCII)
组段表 g × { tri_offset u32, tri_count u32, is_light_tail u32 }
tris     n × 9 f32(位保真)
albedo   n × 3 f32
emission n × 3 f32
tri_mat  n × u32
[v2 新增尾段] uv n × 6 f32           ← 装配 TEXCOORD_0 sink,与 tris 同序;quad 灯尾恒 0
```
v1 = 无尾段,version=1,其余逐字节同。尾冗余校验两端同步(段长按版本精确)。
bistro 实测:v1 66,997,288 B;v2 92,115,904 B = v1 + 1,046,609×24(精确)。

### RXCP v2(簇包;writer g31_cluster_lod_bake.rs:397,reader lane_body:2711)
```
magic "RXCP" | version u32 = 2 | gltf_sha256 64B | src_tri_count u32
passthrough:  count u32 + count × u32
block_count u32,逐块:
  rec_n/child_n/vert_n/tri_idx_n/leaf_tri_n (5 × u32)
  records   rec_n × 64B ClusterRecord(冻结契约字段逐位)
  nodes     rec_n × { first_child, child_count, level, group } (4 × u32)
  children  child_n × u32
  vertices  vert_n × 3 f32
  [v2 新增] vertex_uv vert_n × 2 f32   ← 与 vertices 等长平行;簇局部切片 =
                                          records[id].vertex_offset..+vertex_count
  triangle_indices tri_idx_n × u8 + pad(4 对齐)
  leaf_source_tris leaf_tri_n × u32
  簇属性 rec_n × { albedo 3f32, emission 3f32, mat u32, pad u32,
                   self_lod 4f32, parent_lod 4f32 }
```
v1 = 无 vertex_uv 段,version=1,其余逐字节同。内存形态:UV 表挂 `ClusterPack.
blocks_vertex_uv: Option<Vec<Vec<[f32;2]>>>`(pack 级平行,`ClusterPackBlock` 0 动)。
属性臂焊接键 = (位置,UV) bits(接缝顶点独立 id,build_dag_attrs 接缝保守锁定)。

### RXHL v2(HLOD 资产;writer rurix-asset hlod.rs:237,reader lane_body:3393)
```
magic "RXHL" | version u16 = 2 | cell 名 (u16 len + bytes)
n_levels u32,逐层:
  level u32 | n_proxies u32,逐 proxy:
    名 (u16 len + bytes) | source_triangles u32 | tri_n u32
    逐三角: 9 f32 位置 + [v2 新增] 6 f32 corner UV   ← 15 f32/tri 交错
```
v1 = 每三角仅 9 f32,version=1。RXWH 容器**维持 v1 不动**(cell digest =
sha256(RXHL 字节) 语义同源,v2 资产字节自然进 digest)。版本判据 = 资产内
全 proxy 齐次带 UV(混合 = bake 构造不变量 assert 拒;输入面 validate_input
已 fail-closed)。

## 3. v1 字节相等自证(全部逐字节 fc /b 或 sha256)

| 判据 | 结果 |
| --- | --- |
| dump `--uv off` 重产 vs 既有 `.tmp/g36_gates/wave1_geo_composition/bistro.rxcs` | **FC: no differences**(66,997,288 B) |
| cluster 缺省 bake(v1 dump)vs HEAD 纯净态同 bake | **FC: no differences**,sha256 同 = `c8621a0a2bbe…`(改动前后字节中性铁证) |
| cluster 缺省 bake 吃 **v2** dump | sha256 = `c8621a0a2bbe…` 同上(缺省路径 v1/v2 输入都吃且位级同) |
| wp 缺省 bake(v1 dump)vs 既有 `bistro.rxwh` | **FC: no differences**(71,888,387 B) |
| wp 缺省 bake 吃 **v2** dump | sha256 = `b1ee7b2fd645…` 与 v1 dump 臂同 |
| cluster 属性臂 `--attrs on --double-build` | **double-build 字节相等 OK**,degraded=0(v2 产物 104,106,092 B,sha `f1af965c…`) |
| wp 属性臂 `--attrs on --double-build` | **double-build 字节相等 OK**(v2 产物 128,028,167 B,sha `b382e097…`);逐层代理三角 [1002585, 501281, 286497, 273180] 递减律维持 |

**如实登记 ①(既有 RXCP 2 字节漂移,非本窗引入)**:cluster 缺省重 bake 与
`.tmp/g36_gates/wave1_geo_composition/bistro.rxcp`(今晨 03:53 产)有 2 字节差
(0xD83B24/0xD83B64,FF→FE,同尺寸同簇数)。三方对拍裁决:HEAD 纯净态重 bake 与
本窗版本**逐字节一致**,故漂移在 HEAD 提交(0e605c34,18:19,含 #96 crate 面
QEM/DAG 属性线程化重构)与该产物 bake 时点之间已发生——m90 golden 只锚默认
ShortestEdge 面,QEM quality 档无 golden,该 LSB 级漂移不在任何冻结锚内。wp 面
(同用 QEM 的 simplify_free_mesh 路径)重 bake 逐字节一致,与该归因自洽。
工件留 `.tmp/t4_attr96/`(rebake/headpure/fc 输出三件)供复核。

**如实登记 ②(接缝锁定质量代价,measured 非门)**:属性臂 UV 接缝顶点保守锁定
⇒ 压缩自由度损失:cluster 属性臂 root_tris 291,398(v1 臂 17,208)、
qem_stuck_groups 151,205(v1 臂 50,117)、levels_max 11(v1 臂 13);wp 属性臂
L3 代理 273,180(v1 臂 125,310)。crate 面已声明为已知行为(最优属性求解留
后续质量档),此处如实登记。

## 4. 测试与检查结果

| 项 | 结果 |
| --- | --- |
| `cargo check -p rurix-geom-build -p rurix-asset` | 绿(仅既有 2 warning 于未触碰的 g10_5_scene_render.rs) |
| `cargo check -p rurix-render --bins --features vendor-upscale` | 绿(经 HEAD worktree + 本窗 9 文件覆盖验证,见登记 ③);warning 集与 HEAD 基线逐 bin 相同(g34_full_lane 8 / g31_window_present 4 / g14_3_pipeline_perf 0)——**0 新增** |
| `cargo test -p rurix-geom-build` | **46/46 绿**(45 既有维持 + 1 新增包装测试) |
| `cargo test -p rurix-asset --lib` | **56/56 绿**(含新增 merged_bake_uv_arm_invariants;既有 hlod 五测维持) |
| `cargo run -p rurix-asset --bin g9_m90_probe` | ok=true,canonical_dag_sha256=`68def89991d49a93…` 与 manifest 冻结值一致,golden_manifest_match=true(RXGB 冻结面零漂移) |
| bake 工具 CPU 真跑 | §3 全表(dump ×3 / cluster ×5 / wp ×4 次真跑) |

**如实登记 ③(并行 agent 在飞编译错,验证代偿路径)**:当前工作树
`src/rurix-rt/src/render_exec.rs:9296`(vendor-upscale 臂,`query_slots` 未定义)
编译错——该文件为并行 agent 在飞面(交接单明示不碰),挡住主树上全部
vendor-upscale bin 的检查与构建。代偿:`git worktree`(HEAD 0e605c34)+ 覆盖本窗
9 文件 → 全 bin check 绿 + warning 基线对拍 + 构建 g14_3_pipeline_perf 产 v2 dump;
worktree 已清理。**主 agent 在 render_exec 修复后请顺手复跑一次
`cargo check -p rurix-render --bins --features vendor-upscale` 收尾自证。**

**如实登记 ④(留 GPU 批次的运行时面)**:read_cluster_pack v2 / decode_rxhl v2 /
gather_tri_uv_attrs / geo_patch_proxy_tritex_v2 的**真跑消费**只在 device 车道
(g34 装配链),CPU 面完成的是编译 + writer/reader 字段镜像互核 + writer 侧双构建;
逐位/视觉判据见 §5 GPU 批次。

## 5. GPU 批次验收命令清单(留主 agent 批次 1;禁 GPU 纪律下本窗未跑)

工件就位(全部已产,直接消费):
```
.tmp/t4_attr96/bistro_v2.rxcs         RXCS v2(dump 默认档)
.tmp/t4_attr96/bistro_v2_attrs.rxcp   RXCP v2(--attrs on,double-build 已证)
.tmp/t4_attr96/bistro_v2_attrs.rxwh   RXWH(内 RXHL v2;--attrs on,double-build 已证)
.tmp/g36_gates/wave1_geo_composition/bistro.{rxcp,rxwh}   v1 对照(既有)
```
以下 `$SLAB = milestones/g31/g31_slab_side_table_bistro_interior.json`;g34 exe 均需
`--features vendor-upscale` 构建(render_exec 修复后)。

**锚 ① 代理臂侧表 UV 位保真(与 bake 产物逐位)**——v1 资产回归 + v2 资产消费两臂:
```
# v1 资产臂(行为与旧路径逐位一致回归):g36 门 ⑦ 同参,digest_seq 必须 == --full 基线
g34_full_lane --frames 12 --warmup 2 --tier 100 --full --slab-table $SLAB --auto-move orbit --hidden \
  --evidence .tmp/t4_attr96/g34_base.json
g34_full_lane --frames 12 --warmup 2 --tier 100 --full --slab-table $SLAB --auto-move orbit --hidden \
  --cluster-lod leaf --cluster-pack .tmp/g36_gates/wave1_geo_composition/bistro.rxcp \
  --wp-hlod full --wp-pack .tmp/g36_gates/wave1_geo_composition/bistro.rxwh \
  --evidence .tmp/t4_attr96/g34_leafxfull_v1.json
# 判据:两 evidence digest_seq 逐帧相等(恒等排列锚 bin 内嵌 fail-closed 亦兜底);
#       stderr 无「geo 代理 tritex 补丁」行或 patched 行为与 g36 门在案一致。
# v2 资产臂(同命令换 v2 包):
g34_full_lane --frames 12 --warmup 2 --tier 100 --full --slab-table $SLAB --auto-move orbit --hidden \
  --cluster-lod leaf --cluster-pack .tmp/t4_attr96/bistro_v2_attrs.rxcp \
  --wp-hlod full --wp-pack .tmp/t4_attr96/bistro_v2_attrs.rxwh \
  --evidence .tmp/t4_attr96/g34_leafxfull_v2.json
# 判据:leaf×full 极限 = 全 Src 出帧 ⇒ digest_seq 仍 == --full 基线(v2 包不改极限臂);
#       UV 位保真锚 = bin 内嵌恒等排列 gather 对拍(位级漂移即 fail)。
```

**锚 ② full/leaf 极限臂与 off 逐位不变(代理不出帧时 0 语义)**:
```
# g14_3 bench 三臂对拍(g36 门 ③ 同构,吃 v2 包):
g14_3_pipeline_perf --bench --scene bistro-interior --tier 100 --backend tsr_device --frames 8 --warmup 2 \
  --out-root .tmp/t4_attr96/bench_off
g14_3_pipeline_perf --bench ...(同上)... --cluster-lod leaf --cluster-pack .tmp/t4_attr96/bistro_v2_attrs.rxcp \
  --out-root .tmp/t4_attr96/bench_leaf_v2
g14_3_pipeline_perf --bench ...(同上)... --wp-hlod full --wp-pack .tmp/t4_attr96/bistro_v2_attrs.rxwh \
  --out-root .tmp/t4_attr96/bench_wpfull_v2
# 判据:三臂 EXR/digest 与 off 位级一致(g36 门 single_open_zero_drift 同锚
#       sha256:f39e9808…;v2 包在极限臂零代理出帧 ⇒ 位级同 off)。
# 同时全量回归:ci/g31_wp_hlod_smoke.py 与 ci/g36_geo_composition_smoke.py 全门重跑须绿
#(门内自产 v2 dump + v1 bake,本窗已证位级兼容)。
```

**锚 ③ 代理出帧臂纹理采样视觉指标(bbox 色块口径)**:
```
# 混合臂(g36 门 ⑧ 同参)× v1/v2 两包对照:
g34_full_lane --frames 12 --warmup 2 --tier 100 --full --slab-table $SLAB --auto-move orbit --hidden \
  --cluster-lod on --cluster-error-px 4.0 --cluster-pack <PACK> \
  --wp-hlod on --wp-threshold-l0 0.25 --wp-pack <WPPACK> \
  --evidence .tmp/t4_attr96/g34_mixed_<v1|v2>.json
# 判据(三点):
#  a) v2 臂 stderr「geo 代理 tritex 补丁 patched=0」(全部代理带 UV,退役面兑现;
#     v1 臂 patched>0 维持旧语义);
#  b) host 金标准对拍 p100 ≤ 冻结容差(g34_budget 程序读)——host 克隆消费同一
#     补丁后 tritex/texuv,v2 采样数学两臂同源,超界即 gather/patch 缺陷;
#  c) 视觉指标:v2 臂远景代理区(粗簇/cell 代理 bbox 内)scene HDR 色块 vs v1 臂
#     常量均值色块——对代理覆盖 bbox 取逐通道方差/结构对比(v2 应现纹理细节,
#     方差显著 > v1 常量面;bbox 口径 = regroup_nodes 重导出代理组段 AABB 投影),
#     数字如实登记不设通过线(#96 质量登记面)。
# HZB 车道同构复核(可选):g34_full_lane --hzb on + 同 v2 参数,金字塔位级见证行维持。
```

## 6. 复现命令(CPU 面,本窗已跑)

```
# dump(须 vendor-upscale 构建的 g14_3_pipeline_perf):
g14_3_pipeline_perf --dump-scene --scene bistro-interior --uv off --out bistro_v1.rxcs   # v1 逐位
g14_3_pipeline_perf --dump-scene --scene bistro-interior --out bistro_v2.rxcs            # v2(默认)
# bake(缺省 = v1 字节不变;--attrs on = v2):
g31_cluster_lod_bake --scene-dump bistro_v2.rxcs --out out.rxcp [--attrs on] [--double-build]
g31_wp_hlod_bake --scene-dump bistro_v2.rxcs --out out.rxwh --cell-size 4.0 --levels 4 [--attrs on] [--double-build]
# 测试/golden:
cargo test -p rurix-geom-build ; cargo test -p rurix-asset --lib
cargo run -p rurix-asset --bin g9_m90_probe
```
