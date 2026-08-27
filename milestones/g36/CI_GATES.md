# G36 CI 门册

> 版本：v1.0（2026-08-27）。门键/facts 闭集事实源 = G36_CONTRACT.md §3 +
> ci/g36_geo_composition_smoke.py docstring（判据字面）。

## 门表

| 门键 | 脚本 | facts | evidence schema | 说明 |
|---|---|---|---|---|
| `g36.wave1.geo_composition` | `python ci/g36_geo_composition_smoke.py --gate g36.wave1.geo_composition` | 10 项闭集（builds_green / packs_deterministic / single_open_zero_drift / combined_leafxfull_bitexact / combined_mixed_deterministic / dyn_skin_composition / g34_five_feature_leafxfull / g34_mixed_host_parity / hzb_six_feature_culling / particles_oit_geo_deterministic） | `rurix.g36.geo_composition_gate_evidence.v1` | W1-W4 互斥项修复组合面唯一门;PASS-only 落 evidence/g36_geo_composition_gate_<ts>.json |

## 运行前置

- GPU（Vulkan;门内 gpu_device_lock 串行）+ bistro 派生资产
  （K:/rurix_g10_cache/.../BistroInterior.gltf）+ slab 侧表资产
  （milestones/g31/g31_slab_side_table_bistro_interior.json）。
- SPV 依赖既有构建产物（.tmp/g14_gates/m_c 五件 + .tmp/g34_gates/unified、
  hzb + .tmp/g35_gates/render、sort_oit）——缺失 = 三态 SKIP,先跑各族门
  脚本编译面（g31_cluster_lod / g34_unified_lane / g34_hzb_unified /
  g35_render / g35_sort_oit）。

## 自检

`python ci/g36_geo_composition_smoke.py --selftest`（无 GPU:6 正则 GREEN +
schema required/facts 闭集互核 + 留窗登记面 ≥5）。

## 回归锚（门外并行复核,既有门体系承载）

- Stage A 缺省面：`ci/g31_wave_a_anchor_check.py`（off 路径短路,字面 0-byte）。
- g31 五门/g34 四门/g35 九门：各自 smoke 复跑（本波改动为加性,缺省路径
  digest 由 facts ③ 三臂零漂移机核背书）。
