# G36 契约 — 全特性合流：互斥项修复与生产组合渲染（W1-W4 交付面）

> 版本：v1.0（2026-08-27）
> status: active
> implementation_status: unlocked
> 立项状态（诚实登记）：本合同为 **交付事实登记面**——W1-W4 已实现并经门
> `g36.wave1.geo_composition` 机器裁决;正式立项程序（RFC + 
> `milestones/g30/g30_campaign_handover_registry.json` 法定输入面消费,
> RFC-0047 §5.5）**留 owner 走治理程序**,本合同不冒充已批期契约。
> 事实源纪律：冲突时以门裁决件（evidence/g36_geo_composition_gate_*.json）
> 与源码 fail-closed 机核为准。

## 1. 范围（互斥项修复 = "组合面归后续波"的兑现波）

**根因**（G31+ TODO v1.1.5/v1.1.6 已知限制字面）：`--cluster-lod` /
`--wp-hlod` 重建三角汤（升序源三角 + 尾接代理/粗簇）后,按"装配序三角位置"
绑定的三类假设破坏——①B4 逐三角 UV/tritex 侧表同序;②B1 SceneNodeGroup
节点连续段;③A4/B5 dyn/skin 尾接段基址——此前以闭集互斥 fail-closed 拒组合。

**解除机制**（W1 单一事实源）：逐三角 provenance
（`TriProvenance::{Src, ClusterCoarse, WpProxy}`,
`src/rurix-render/src/bin/g14_3_lane/g14_3_lane_body.rs`）——
- 侧表经 `gather_tri_uv` 位保真重排 + `geo_patch_proxy_tritex` 代理三角强制
  −1 常量面回退（cluster/cell 面积加权均值;#96 属性保持简化留窗）;
- 节点段经 `regroup_nodes` 重导出（升序源序保持节点连续性,AABB 自重建几何
  精确重算,"三角 ⊆ AABB 精确包含"不变量维持）;
- dyn/skin 尾接段基址 = 重建后 `scene.indices.len()`（计算点已在重建后,
  组合面零改动成立）;
- 恒等排列锚：leaf/full 极限下 gather/regroup 产物与装配面逐位一致
  fail-closed。

**W2 组合语义**（`apply_geo_combined` 四态分派）：WP cell 互斥选层先行
（Full/Hlod/Culled,生产机核直调链）→ Full 域内簇 cut → 组共享多父 DAG 语义
下粗簇集合化判定（S_c ⊆ F 出帧 / 跨界叶级回退差集化防"粗簇面+回退叶"双绘 /
全外域归 cell 代理）→ 统一重建 + provenance。覆盖机核 fail-closed：identity
恰一次 + identity×粗簇域零交叠 + identity ∪ 粗簇域 ≡ WP Full 域恰等。
选层机核抽取共用（`cluster_lod_select` / `wp_hlod_select` 自 `apply_*` 逐字
抽取）,单开路径 0-语义漂移。

## 2. 互斥项处置矩阵（全量盘点,如实登记）

### 2.1 本波解除（技术性/登记面互斥 → 组合面成立,门 ① 承载）

| 原互斥 | 位置 | 解除机制 | 机核 |
|---|---|---|---|
| `--wp-hlod` × `--cluster-lod` | g14_3_pipeline_perf | W2 组合管线 | leaf×full == off 位级锚 + 覆盖机核 + 双跑位级 |
| `--cluster-lod` × `--dyn-demo`/`--skin-demo` | g14_3_pipeline_perf | 尾接段基址后移（重建后计算） | 位置核验/蒙皮逐顶点位级/MV 三类硬门维持 |
| `--wp-hlod` × `--dyn-demo`/`--skin-demo` | g14_3_pipeline_perf | 同上 | 同上 |
| geo × 纹理×slab×动态（五特性) | g34_full_lane 统一主车道 | UV gather + tritex 代理补丁（host 金标准同一补丁后数组,两臂一致） | 五特性 leaf×full digest_seq == --full 基线 + host parity ≤ 冻结容差 |
| geo × HZB（六特性） | g34_full_lane HZB 区段 | regroup_nodes 重导出节点段进逐节点 BLAS 分解/初剔分类/inst_base 前缀表 | 金字塔位级全等 + 判定逐字节全等 + 零假阳性 + 真实剔除 measured |
| geo × 粒子 × OIT（wboit/sorted） | g35_particle_lane | 粒子为生成几何,splat/OIT 在场景色之上与场景重组正交 | 双跑 presented digest 位级 |

### 2.2 留窗（如实登记不冒充;windowed_items 门字段承载）

1. **FIF×动态**（`--inflight ≥2` × `tlas_update`/`blas_refit`,TODO #90）：
   RFC-0030 §4.3 L2 定义 TLAS instance buffer / BLAS 顶点缓冲为共享 host 写
   面（在飞帧 ray query 读取中不可改写）。真修复 = 每槽实例/TLAS 副本 + 每槽
   AS 描述符集 + provenance 逐槽 AS 代追踪——触冻结确定性协议面,**须 RFC
   修订行**,本波不预支。
2. **FG 组合**（`--fg` × geo/纹理/slab/HZB）：G34 契约 out-of-scope「FG/MFG
   合流归后续波不预支」字面管辖（active 契约越权即违规）。
3. **HZB×蒙皮同车道**：G34-2/G34-3 并行分区面,合并需新 kernel（masked 双
   TLAS × hit 通道/蒙皮分派合体）+ host 金标准扩面;g14_3 MegaSkin×geo 组合
   已验证（门 ⑥）。
4. **#96 代理属性保持简化**：代理三角贴图采样需 QEM 属性保持简化
   （UV/法线）,现走 tritex=−1 常量面回退。
5. **逐帧 device cut→AS 更新**（#77/#89 合流窗）：出帧几何冻结于装配期
   契约相机 cut/选层。

### 2.3 语义互斥维持（测量工装口径纪律,修掉即破坏门体系）

RED 臂 / mv/遮挡/mesh/oit 见证臂（标定夹具构型,几何重组改变夹具判读域）/
`--window-storm`×`--storm-soak` / fault-probe×特性开 / `--static-camera`
锚格模式×特性开 / g35 `--red-arm`。`g31_window_present` 冻结 bin（五门回归
锚）互斥字面不回写——组合面由 g34/g14_3/g35 车道承载。

## 3. 验收门

**G-G36-1（g36.wave1.geo_composition）**：facts 十项闭集（与
`ci/g36_geo_composition_smoke.py` FACT_IDS 逐字同序;判据字面 = smoke
docstring）——builds_green / packs_deterministic / single_open_zero_drift /
combined_leafxfull_bitexact / combined_mixed_deterministic /
dyn_skin_composition / g34_five_feature_leafxfull / g34_mixed_host_parity /
hzb_six_feature_culling / particles_oit_geo_deterministic。
evidence schema = `rurix.g36.geo_composition_gate_evidence.v1`
（milestones/g36/g36_geo_composition_gate_evidence_schema.json,
check_schemas 三处纯追加注册）。

**零降级回归锚**（与门并行复核,不进 facts——既有门体系承载）：Stage A
缺省面 digest 锚（off 路径字面 0-byte——`if mode == Off { return }` 短路,
选层抽取仅触 on 路径）;g31 五门/g34 四门/g35 九门维持（各自 smoke 复跑）。

## 4. out-of-scope（不承诺面）

CPU 参考臂生产化 / 跨硬件位级承诺 / §2.2 留窗五项（各自锚触发再开窗）/
g31_window_present 改写（冻结）/ G34/G35 既有 evidence 改写（0-byte）。

## 5. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-27 | 初版：W1-W4 交付事实登记（provenance 地基 + cluster×wp 组合 + g34 五/六特性接线 + 粒子×OIT×geo）+ 互斥处置矩阵三分类 + 门 facts 十项闭集 + 留窗六项如实登记;立项程序留 owner |
