# G36 计划 — 全特性合流：互斥项修复与生产组合渲染

> 版本：v1.0（2026-08-27）。事实源 = G36_CONTRACT.md + 门裁决件;本表为
> 波次镜像。

## 波次表（W0-W5）

| 波 | 内容 | 交付载体 | 状态 |
|---|---|---|---|
| W0 | G35-4 收尾批落盘（前置卫生面） | commit 21e00d61（G35-4 九件）+ 895e0fad（并行会话 g31 证据件单独落盘） | 完成 |
| W1 | 逐三角 provenance 地基：TriProvenance + geo_rebuild + 侧表 gather（UV 位保真/代理 tritex −1）+ regroup_nodes 节点段重导出 + 恒等排列锚;选层机核抽取（cluster_lod_select/wp_hlod_select,单开 0-语义漂移） | g14_3_lane_body.rs 加性段 | 完成（门 facts ③ 承载零漂移证明） |
| W2 | cluster×wp 统一几何重建：apply_geo_combined 四态分派（WP 选层先行 → Full 域簇 cut → 组共享多父 DAG 粗簇集合化判定 → 跨界叶级回退差集化）+ 覆盖机核 fail-closed + leaf×full == off 位级锚 | 同上 + g14_3_pipeline_perf 三条互斥撤除 + 双 leg 组合分派 | 完成（facts ④⑤⑥） |
| W3 | g34 全特性组合接线：统一主车道五特性（cluster×wp×纹理×slab×动态,UV gather + tritex 补丁 + host 金标准两臂一致）+ HZB 区段六特性（regroup 节点段进 BLAS 分解/分类/inst_base）+ evidence G36 schema 切换 | g34_full_lane.rs + g34_2_hzb.rs | 完成（facts ⑦⑧⑨） |
| W4 | 粒子×OIT×geo 组合（g35 车道加 geo 旗标,见证臂语义互斥维持）;FIF×动态/FG 组合评估 → 留窗如实登记（RFC-0030 修订面 / G34 out-of-scope 字面） | g35_particle_lane.rs + 契约 §2.2 | 完成（fact ⑩ + windowed_items） |
| W5 | 门体系：schema + smoke（--selftest/--gate）+ check_schemas 三处纯追加 + 四件套 + TODO v1.1.7 登记 + 回归锚复跑 | milestones/g36/ + ci/g36_geo_composition_smoke.py | 本批 |

## 组合语义要点（实现纪律）

1. **provenance 单一事实源**：重建函数输出逐三角出处;一切侧表/节点段/基址
   派生自 provenance,禁按装配序假设旁路。
2. **组共享多父 DAG 语义**（Nanite 同族）：粗簇源覆盖集经多父路径可跨簇
   部分重叠——粗簇级"源恰一次"非 DAG 承诺面（面恰一次由冻结 cut 机制承载,
   与单开 on 模式同信任基）;组合面新增双绘由覆盖机核拒绝（identity 恰一次 +
   identity×粗簇域零交叠 + ≡ F 恰等）。
3. **代理三角侧表回退**：UV=0 + tritex 强制 −1（防 UV=0 采样错色）→ 常量面
   （面积加权均值,slab 预调制一致施加）;host 金标准克隆同一补丁后数组 ⇒
   两臂结构性一致。
4. **锚纪律**：每一步组合都有位级锚（单开三臂 == 在案锚 / leaf×full == off /
   五特性 digest_seq == 基线）+ 双跑确定性 + host parity 冻结容差。

## 帧率/画质 measured（门产出,如实登记不进硬门）

见门裁决件 combined.mixed / g34_lane.host_parity / particles 字段与
.tmp/g36_gates/ 工作区件;g34 五特性混合组合 1080p real_render ≈ 7ms
（12+2 帧短窗,RTX 4070 Ti,含强制回读如实登记）。
