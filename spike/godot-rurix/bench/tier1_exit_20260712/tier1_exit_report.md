# Tier 1 出口 benchmark 报告（2026-07-12，measured_local，诚实结论：无 pass 达默认启用门）

双腿同 exe：`target/grx/godot-scratch-0022`（0001..0022 全栈）；DLL = feature
`d3d12-recording-shim` 构建（**测试 harness 形态——见"测量效度"节**）。baseline =
`baseline_full_workload_v2_1_20260712.json`（3 次 full 中位 run2，geomean 219.58，
本目录 baseline_run1..3.json 为三次原始 summary）。rurix 腿 = 六条 full
（all5 + 单 pass 二分×5，matrix_*.json 为各腿 override 输入）。

## per-pass avg_fps ratio（rurix / baseline，逐场景）

| scene | all5 | luminance | tonemap | ssao_blur | taa_resolve | particles |
|---|---|---|---|---|---|---|
| clustered_lights | 0.322 | 0.962 | 0.323 | 0.908 | 0.962 | 0.979 |
| many_mesh_instances | 0.160 | 0.994 | 0.312 | 1.001 | 0.281 | 1.006 |
| material_variants | 0.257 | 0.982 | 0.260 | 0.988 | 0.983 | 1.007 |
| post_fx_chain | 0.277 | 0.985 | 0.965 | 0.277 | 0.999 | 0.998 |
| volumetric_fog | 0.380 | 0.988 | 0.382 | 0.985 | 1.000 | 1.002 |
| particles | 0.372 | 0.968 | 0.370 | 1.004 | 1.002 | 1.002 |
| mixed_forward_plus | 0.132 | 0.960 | 0.362 | 0.400 | 0.350 | 0.995 |
| **geomean** | **0.2542** | **0.9769** | **0.3866** | **0.7180** | **0.7124** | **0.9982** |

## engagement（RXGD_SUMMARY 解析）

- luminance_reduction：post_fx_chain / mixed_forward_plus 各 recorded=4598, fallback=0
  （仅 auto_exposure 场景触发，符合设计）。
- tonemap：除 post_fx_chain 外六场景 recorded=6900, fallback=0；post_fx_chain
  recorded=0, fallback=1（2.0x supersampling 下源/目标非 1:1，preflight 诚实拒绝，
  子集边界按契约生效）。per-scene 6900 ≈ 3×帧数，计数口径待核（open question）。
- ssao_blur / taa_resolve / particles_copy：**engagement=NONE**（三腿）。

## 测量效度声明（本报告不产生任何性能宣称）

1. **本轮数字不可作为 pass 本征开销的证据**：唯一具备 real-dispatch 能力的 DLL 是
   `d3d12-recording-shim` feature 构建，其 real-pass 路径每次 dispatch 打印
   RXGD_BRIDGE_REC 行（含 readback/checksum 语义），per-frame stdout+readback
   开销主导测量（tonemap 腿 0.39、all5 0.25 的量级即由此而来）。
2. ssao/taa 两腿 engagement=NONE 但 FPS 显著下降（如 taa 腿 many_mesh 0.281）——
   dispatch 实际发生但 RXGD_SUMMARY（session 销毁时打印）未出现在 runner 捕获里，
   属 engagement 上报缺口（session 关闭路径依赖）；这两腿按 engagement 门
   规则 **invalid，不得用于对比结论**。
3. particles_copy 腿 engagement=NONE 且 ratio≈1.0 = 纯噪声：bench 粒子场景
   transform_align=DISABLED（hook 早返回），带 VIEW_DEPTH 的 emitter 又触发
   do_sort=true 被 FILL_INSTANCES 子集排除——场景/子集错配。
4. 第一轮六腿（本报告前）因 override.cfg 反斜杠转义 bug 全部零 engagement，
   已废弃不入档；runner 已修转义并实测 engagement 恢复。

## 结论与决策输入

- **无任何 pass 满足契约默认启用门（per-pass FPS ≥ 0.95x baseline 的有效证据）**：
  五个 real_pass_default_enable_decision 全部维持 keep_default_disabled。
  luminance 0.9769 最接近，但受 instrumentation 污染，须在生产 dispatch 路径
  （无打印/无 readback）落地后重测方可复评。
- Wave 4 前置工作清单（由本轮实测产生）：
  1. **生产 real-dispatch 路径**：shipping 形态的 real-pass（无 per-dispatch
     stdout/readback、engagement 走计数器落文件而非依赖 stdout+session 干净关闭）。
  2. engagement 上报可靠化（ssao/taa 腿 SUMMARY 缺失根因）。
  3. particles bench 场景与子集对齐（非 sort emitter 改 Z_BILLBOARD align）。
  4. tonemap recorded 计数口径核实（6900 vs 2300 帧）。
