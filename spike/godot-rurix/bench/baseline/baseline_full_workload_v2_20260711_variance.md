# workload v2 baseline 三次运行方差记录（2026-07-11）

伴随 `baseline_full_workload_v2_20260711.json` 的方差留档。**本文档不是 gate 输入**；
canonical baseline 取三次 full 运行中 geomean(avg_fps) 的中位那一次（run2）。
三次运行为同夜连续串行（热身 1 次 iter 吸收 PSO 冷缓存后），机器安静
（无并行构建/agent 任务），RTX 4070 Ti @1080p，warmup 300 / sample 2000 / vsync off。

| scene | run1 avg_fps | run2 avg_fps | run3 avg_fps | spread% | p95 ms (r1/r2/r3) |
|---|---|---|---|---|---|
| clustered_lights | 239.9 | 244.7 | 235.7 | 3.79 | 4.49 / 4.17 / 4.55 |
| many_mesh_instances | 210.0 | 212.1 | 218.3 | 3.93 | 5.41 / 5.36 / 5.21 |
| material_variants | 250.0 | 257.3 | 250.7 | 2.90 | 4.17 / 4.17 / 4.17 |
| post_fx_chain | 191.8 | 191.1 | 184.4 | 3.96 | 5.56 / 5.56 / 5.56 |
| volumetric_fog | 218.4 | 203.0 | 210.1 | 7.58 | 4.83 / 5.00 / 5.00 |
| particles | 219.6 | 211.8 | 211.6 | 3.75 | 4.76 / 5.00 / 5.00 |
| mixed_forward_plus | 239.2 | 229.7 | 230.1 | 4.16 | 4.55 / 4.55 / 4.55 |

- run1 geomean=223.33（run_id 20260711T123236Z_full）
- **run2 geomean=220.33（run_id 20260711T123402Z_full）← canonical（中位）**
- run3 geomean=219.22（run_id 20260711T123528Z_full）

逐场景 avg_fps 离散度 2.9%~7.6%（最大 volumetric_fog 7.58%）。对比结论使用时须注意：
离散度量级意味着 <8% 的单场景差异不足以单独支撑结论；gate 判定以
perf_gate --strict 的固定阈值为准，本表仅供解读参考。无任何性能提升宣称。
