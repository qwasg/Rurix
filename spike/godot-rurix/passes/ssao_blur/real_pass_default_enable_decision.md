# GRX-011 SSAO Blur — Real-Pass Default-Enable Decision（close-out）

> Evidence 伴生文件：`real_pass_default_enable_decision.json`
> 前置事实：ssao_blur real-pass enablement strict measured success（`real_pass_enablement_success_evidence.json`，`status=success`、`strict_success=true`、`real_gpu_pass=true`、`real_d3d12_dispatch_recorded=true`；opt-in real dispatch 在 0001..0016 scratch Godot（Windows D3D12 Forward+，NVIDIA GeForce RTX 4070 Ti）上完成，三腿 pass enable matrix 全绿——candidate 腿 `real_pass_marker_observed=true` + `writeback_marker_observed=true`（`RXGD_GODOT_RUNTIME_SSAO_BLUR_REAL_PASS recorded=1`），forced 腿实测 `first_missing_prerequisite=runtime_binding_preflight_failed`/`fallback_reason=unsupported_device`（`RXGD_SSAO_BLUR_REAL_PASS_BLOCKED`），LDR visual gate `max_abs=0`/`mean_abs=0`，telemetry `measured_local` 通过 GRX-008 校验，`0001..0016` patch-stack/溯源/日志审计全绿）。standalone dispatch smoke（`real_d3d12_dispatch_smoke.json`）CPU parity `max_abs_diff≈1.19e-07`（~1 ULP）、`mismatched=0`。

## Owner Decision

- decision: **`keep_default_disabled`**
- approved_by: `owner`
- approved_at_utc: `2026-07-12T00:00:00Z`
- machine_role: `local_test_machine`
- 适用对象：`rendering/rurix_accel/passes/ssao_blur/*` 全部 per-pass 设置保持默认 `false`；`pass_manifest.json` 的 `default_enable_state` 保持 `disabled`。

即使 ssao_blur real-pass enablement 已取得 strict measured success（opt-in real dispatch 真正执行且完成、视觉门保持绿、红腿实测、溯源/日志审计全绿），**默认启用状态仍保持 disabled**。

## Rationale

1. **无 per-pass FPS 证据**：GRX 契约要求任何 pass 默认 enable 前必须有 per-pass FPS >= 0.95x baseline 的 measured_local 证据（GRX_PLAN GRX.4 出口判据 / GRX_CONTRACT G-GRX-5 口径 `single_scene_fps_ratio_min=0.95`）。当前不存在 ssao_blur 的 full baseline 对比实测，无任何 per-pass benchmark。
2. **patch 0016 writeback 仍是 scaffold（无净收益）**：MODE_SMART 结果虽 dispatch 进真实 Godot deinterleaved ping-pong slice，但 native Godot SSAO blur continuation 仍作为 backstop，每帧照常重跑完整 `blur_passes × 4 slices` 的 ping-pong 链，Rurix dispatch 目前没有净收益（甚至是额外开销）；candidate 图像因此逐字节不变（LDR visual gate `max_abs=0`/`mean_abs=0`，与 GRX-010 tonemap 0013 同段位）。raster/compute ping-pong output seam（native 走完整 4-slice 链，本 kernel 只做单次单 slice SMART blur）尚未设计。
3. **仅 MODE_SMART 单遍单 slice 子集**：math parity 仅覆盖 `MODE_SMART` 单次 blur pass、单 deinterleaved slice 的 edge-aware 十字子集（`math_parity_evidence.json`，`status=pending_gpu_dispatch`）；`MODE_WIDE`（±2 texel、双向 edge 乘、0.8 起始权重）、`MODE_NON_SMART`、多 pass ping-pong 链（`ssao_blur_passes` 默认 2）、4-slice 循环、SSIL blur（`RXGD_PASS_SSIL_BLUR`，rgba16 值 + 独立 r8 edges image，未接线）、mirror-sampler 边界寻址、rg8 unorm 存储量化全部未覆盖。

## Re-evaluation Conditions

满足以下条件后由 owner 重新决策（在 full baseline + per-pass benchmark 之后）：

- full baseline benchmark 证据可用（7 场景 measured_local）；
- per-pass FPS ratio 实测 >= 0.95x baseline（对齐契约 `single_scene_fps_ratio_min=0.95` 口径）；
- 更完整的 ssao_blur 模式覆盖已证明（超出 MODE_SMART 单遍单 slice 子集：MODE_WIDE / MODE_NON_SMART / 多 pass ping-pong / 4-slice 循环 / SSIL blur）；
- raster/compute ping-pong output seam 已设计并实测存在净收益，或 native continuation 退役。

## Fail-Closed Invariants

- `default_enable_state` 保持 `disabled`；默认 Godot config 下 bridge 对 `RXGD_PASS_SSAO_BLUR` 仍返回 `RXGD_STATUS_FALLBACK`（`runtime_state=fallback_only_by_default_real_pass_optin_measured` 中 `fallback_only_by_default` 是默认路径口径），native SSAO blur 循环接管。
- `performance_claim=none`：本决策与 enablement success 均不构成 FPS、p95、GPU timestamp 或任何性能提升宣称；candidate 图像 bit-exact（`max_abs=0`）本身即证明尚无输出替代。
- 本决策文件存在且校验通过（顶层 `default_enable_decision` 字段非空，`ci/grx_gates/grx011_ssao_blur.py` `_decision_ready` 口径）+ enablement strict success（`strict_success=true`）有效时，probe `next_action` 才可推进到 `start_grx012_taa_resolve_pass_contract`；两者任一缺失/被篡改即 fail-closed（`grx_gate_module_error`），`next_action` 保持不变。
- 只有 strict 校验通过的 `real_pass_enablement_success_evidence.json` 才允许 manifest 记录 `real_gpu_pass=true`（opt-in 口径）；手工编辑的 placeholder 永远不能推进任何 gate。

## rd_native（Route B）默认启用追加 — GRX-025 close-out（2026-07-13，append-only）

> 本段是 GRX-025 收官对 **rd_native（Route B，patch 0041，backend 三态）** 复制阶段的默认启用决策追加，**不修改上方 bridge-era 结论**。汇总矩阵：[../DEFAULT_ENABLE_MATRIX.md](../DEFAULT_ENABLE_MATRIX.md)。

- rd_native 决策 token：**`eligible_for_default_enable_gate_met`**（GRX-025 ≥0.95 门 + GRX-024 视觉 parity 两者实测达成）。
- 当前 default：**仍 `disabled`**。成因不是门未过，而是 **rd_native 启用天生是 per-project opt-in**：须把 Rurix 容器 staged 到 `target/grx/rd_containers/ssao_blur.rd_container.bin`、设 `rendering/rurix_accel/passes/ssao_blur/rd_container_path` 指向它、把 `passes/ssao_blur/backend` 置 `2`（缺容器则 fail-closed 回落 native）。**无全局默认可翻**，故如实记「门槛条件已全部实测达成，启用为集成方按需 opt-in」。
- GRX-025 门槛（`spike/godot-rurix/bench/grx025_default_enable_20260713/`）：engaged geomean **0.9950** ≥ 0.95；worst engaged scene = mixed_forward_plus / 0.9904 / noise 0.69%（含场景 noise 后仍过门）。
- engagement 子集：2/7 SSAO on + MODE_SMART blur 单-slice 子集(post_fx_chain, mixed_forward_plus)。
- 视觉 parity（`spike/godot-rurix/bench/grx024_visual_20260713/`）：GRX-024 byte-exact(post_fx_chain)。
- per-pass GPU µs 归因（`spike/godot-rurix/bench/rd_native_final_20260713/` §4）：≈0 delta(替换片是 gather-dominated bucket 内小切片)；程序级 Amdahl 零成本 geomean 天花板 **1.0669x**（measured all5 0.9942）。
- fail-closed 不变式：`performance_claim=none`（本追加不含 FPS/p95/GPU-timestamp/性能提升宣称）；缺容器/preflight 失败/dispatch-eligibility 失败/visual 超阈值一律回落 native；`RXGD_ABI_VERSION` 不变、bridge-independent（不占 cap bit）。
- 收官口径：1.5x strict 门（GRX_CONTRACT G-GRX-5）在此 workload/GPU/build 上被归档为**结构性不可达**（1.50 vs 1.0669 硬上限），门的数学与阈值不变、未放宽——是「归档不可达」非「降门」。
