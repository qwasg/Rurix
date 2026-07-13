# GRX-012 TAA Resolve — Real-Pass Default-Enable Decision（close-out）

> Evidence 伴生文件：`real_pass_default_enable_decision.json`
> 前置事实：taa_resolve real-pass enablement strict measured success（`real_pass_enablement_success_evidence.json`，`status=success`、`strict_success=true`、`real_gpu_pass=true`、`real_d3d12_dispatch_recorded=true`；opt-in real dispatch 在 0001..0022 scratch Godot（Windows D3D12 Forward+，NVIDIA GeForce RTX 4070 Ti）上完成，三腿 pass enable matrix 全绿——candidate 腿 `real_pass_marker_observed=true` + `writeback_marker_observed=true`（`RXGD_GODOT_RUNTIME_TAA_RESOLVE_REAL_PASS recorded=1`），forced 腿实测 `first_missing_prerequisite=runtime_binding_preflight_failed`/`fallback_reason=unsupported_device`（`RXGD_TAA_REAL_PASS_BLOCKED`）。**temporal 硬约束（GRX_PLAN DoD）**：real TAA resolve 是时序累积，本 gate 捕获连续 8 帧序列（非单帧截图），逐帧对 reference 腿 diff（candidate/forced 全 8 帧 `max_abs=0`/`mean_abs=0`，即逐帧 bit-exact），并记录 reference 序列的帧间稳定性证明序列携带真实运动（`nonzero_delta_pairs=7/7`，`max_interframe_abs_diff` 非零——若静止 TAA 场景 velocity 全零则时序证据无意义）；telemetry `measured_local` 通过 GRX-008 校验，`0001..0022` patch-stack/溯源/日志审计全绿）。standalone dispatch smoke（`real_d3d12_dispatch_smoke.json`）`real_d3d12_dispatch_recorded=true`、`cpu_reference_match=true`。

## Owner Decision

- decision: **`keep_default_disabled`**
- approved_by: `owner`
- approved_at_utc: `2026-07-12T00:00:00Z`
- machine_role: `local_test_machine`
- 适用对象：`rendering/rurix_accel/passes/taa_resolve/*` 全部 per-pass 设置保持默认 `false`；`pass_manifest.json` 的 `default_enable_state` 保持 `disabled`。

即使 taa_resolve real-pass enablement 已取得 strict measured success（opt-in real dispatch 真正执行且完成、逐帧时序视觉门保持绿、红腿实测、溯源/日志审计全绿），**默认启用状态仍保持 disabled**。

## Rationale

1. **无 per-pass FPS 证据**：GRX 契约要求任何 pass 默认 enable 前必须有 per-pass FPS >= 0.95x baseline 的 measured_local 证据（GRX_PLAN GRX.4 出口判据 / GRX_CONTRACT G-GRX-5 口径 `single_scene_fps_ratio_min=0.95`）。当前不存在 taa_resolve 的 full baseline 对比实测，无任何 per-pass benchmark。
2. **patch 0019 writeback 仍是 scaffold（无净收益）**：单次全分辨率 resolve 结果虽 dispatch 进真实 Godot taa/temp 目标纹理，但 native Godot TAA resolve continuation 仍作 backstop，每帧照常重跑整帧 resolve **并**维护 `resolve->temp->internal->history` 物理 copy 链（`taa.cpp` 每帧 3 次 `copy_to_rect`），Rurix dispatch 目前没有净收益（甚至是额外开销）；candidate 序列因此逐帧不变（temporal LDR visual gate 全 8 帧 `max_abs=0`/`mean_abs=0`，与 GRX-011 ssao_blur 同段位）。raster/compute resolve seam（native 维护完整 history 链，本 kernel 只做单 view 单次 resolve）尚未设计。
3. **仅单 resolve 子集 + 一帧延迟**：math parity 仅覆盖单次全分辨率 TAA resolve 子集（`math_parity_evidence.json`，`status=pending_gpu_dispatch`）；history 双缓冲 copy-back / prev_velocity bookkeeping、one-frame-latency 的 draw_graph 真替代（self-queue dispatch 读上一帧 color/velocity）、hardware-sampler 亚纹素 rounding、rgba16f/rg16f half 存储量化、multiview 全部未覆盖（recorded gaps）。

## Re-evaluation Conditions

满足以下条件后由 owner 重新决策（在 full baseline + per-pass benchmark 之后）：

- full baseline benchmark 证据可用（7 场景 measured_local）；
- per-pass FPS ratio 实测 >= 0.95x baseline（对齐契约 `single_scene_fps_ratio_min=0.95` 口径）；
- draw_graph one-frame-latency 真替代已设计并接线 history 物理维护链；
- raster/compute resolve seam 已设计并实测存在净收益，或 native continuation 退役。

## Fail-Closed Invariants

- `default_enable_state` 保持 `disabled`；默认 Godot config 下 bridge 对 `RXGD_PASS_TAA_RESOLVE` 仍返回 `RXGD_STATUS_FALLBACK`，native TAA resolve 接管。
- `performance_claim=none`：本决策与 enablement success 均不构成 FPS、p95、GPU timestamp、时序稳定性或任何视觉优越性宣称；candidate 序列逐帧 bit-exact（`max_abs=0`）本身即证明尚无输出替代。
- 本决策文件存在且校验通过（顶层 `default_enable_decision` 字段非空，`ci/grx_gates/grx012_taa_resolve.py` `_decision_ready` 口径）+ enablement strict success（`strict_success=true`）有效时，probe `next_action` 才可推进到 `start_grx013_particles_copy_pass_contract`；两者任一缺失/被篡改即 fail-closed（`grx_gate_module_error`），`next_action` 保持不变。
- 只有 strict 校验通过的 `real_pass_enablement_success_evidence.json` 才允许 manifest 记录 `real_gpu_pass=true`（opt-in 口径）；手工编辑的 placeholder 永远不能推进任何 gate。

## rd_native（Route B）默认启用追加 — GRX-025 close-out（2026-07-13，append-only）

> 本段是 GRX-025 收官对 **rd_native（Route B，patch 0042，backend 三态）** 复制阶段的默认启用决策追加，**不修改上方 bridge-era 结论**。汇总矩阵：[../DEFAULT_ENABLE_MATRIX.md](../DEFAULT_ENABLE_MATRIX.md)。

- rd_native 决策 token：**`eligible_for_default_enable_gate_met`**（GRX-025 ≥0.95 门 + GRX-024 视觉 parity 两者实测达成）。
- 当前 default：**仍 `disabled`**。成因不是门未过，而是 **rd_native 启用天生是 per-project opt-in**：须把 Rurix 容器 staged 到 `target/grx/rd_containers/taa_resolve.rd_container.bin`、设 `rendering/rurix_accel/passes/taa_resolve/rd_container_path` 指向它、把 `passes/taa_resolve/backend` 置 `2`（缺容器则 fail-closed 回落 native）。**无全局默认可翻**，故如实记「门槛条件已全部实测达成，启用为集成方按需 opt-in」。
- GRX-025 门槛（`spike/godot-rurix/bench/grx025_default_enable_20260713/`）：engaged geomean **0.9906** ≥ 0.95；worst engaged scene = mixed_forward_plus / 0.9890 / noise 0.69%（含场景 noise 后仍过门）。
- engagement 子集：2/7 use_taa(many_mesh_instances, mixed_forward_plus);单 resolve dispatch 子集,history 维护留 native。
- 视觉 parity（`spike/godot-rurix/bench/grx024_visual_20260713/`）：GRX-024 ±1 LSB(many_mesh_instances,deterministic floor 0)。
- per-pass GPU µs 归因（`spike/godot-rurix/bench/rd_native_final_20260713/` §4）：+100~114 µs(~1.6-1.8x native resolve;门内 <1.1% avg_fps)；程序级 Amdahl 零成本 geomean 天花板 **1.0669x**（measured all5 0.9942）。
- fail-closed 不变式：`performance_claim=none`（本追加不含 FPS/p95/GPU-timestamp/性能提升宣称）；缺容器/preflight 失败/dispatch-eligibility 失败/visual 超阈值一律回落 native；`RXGD_ABI_VERSION` 不变、bridge-independent（不占 cap bit）。
- 收官口径：1.5x strict 门（GRX_CONTRACT G-GRX-5）在此 workload/GPU/build 上被归档为**结构性不可达**（1.50 vs 1.0669 硬上限），门的数学与阈值不变、未放宽——是「归档不可达」非「降门」。
