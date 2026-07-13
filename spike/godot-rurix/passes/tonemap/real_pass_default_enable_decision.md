# GRX-010 Tonemap — Real-Pass Default-Enable Decision（close-out）

> Evidence 伴生文件：`real_pass_default_enable_decision.json`
> 前置事实：tonemap real-pass enablement strict measured success（`real_pass_enablement_success_evidence.json`，`status=success`、`real_gpu_pass=true`、`real_d3d12_dispatch_recorded=true`；opt-in real dispatch 在 0001..0013 scratch Godot（Windows D3D12 Forward+）上完成，22 checks 全绿含 `forced_capability_downgrade` 红腿，LDR visual gate `max_abs=0`/`mean_abs=0`，telemetry `measured_local` 通过）。

## Owner Decision

- decision: **`keep_default_disabled`**
- approved_by: `owner`
- approved_at_utc: `2026-07-11T00:00:00Z`
- machine_role: `local_test_machine`
- 适用对象：`rendering/rurix_accel/passes/tonemap/*` 全部 per-pass 设置保持默认 `false`；`pass_manifest.json` 的 `default_enable_state` 保持 `disabled`。

即使 tonemap real-pass enablement 已取得 strict measured success（opt-in real dispatch 真正执行且完成、视觉门保持绿、红腿实测、溯源/日志审计全绿），**默认启用状态仍保持 disabled**。

## Rationale

1. **无 per-pass FPS 证据**：GRX 契约要求任何 pass 默认 enable 前必须有 per-pass FPS >= 0.95x baseline 的 measured_local 证据（GRX_PLAN 出口判据 / GRX_CONTRACT G-GRX-5 口径）。当前不存在 full baseline 实测，无任何 per-pass benchmark。
2. **仅 TONEMAPPER_LINEAR + sRGB-only 子集**：math parity 仅覆盖 `TONEMAPPER_LINEAR` + `linear_to_srgb` 的 SDR 子集（`math_parity_evidence.json`，`status=pending_gpu_dispatch`），Reinhard/Filmic/ACES/AgX、auto exposure、glow、FXAA、BCS、color correction、debanding、multiview、HDR output 全部未覆盖。
3. **patch 0013 writeback 仍是 scaffold**：LINEAR 结果虽 dispatch 进真实 Godot tonemap destination 资源，但 native Godot tonemapper continuation 仍作为 backstop 重渲染每帧，Rurix dispatch 目前没有净收益（甚至是额外开销）；raster-vs-compute output seam（native 走 fullscreen fragment pass 写 LDR，本 kernel 写 full-res UAV）尚未设计。

## Re-evaluation Conditions

满足以下条件后由 owner 重新决策（在 full baseline + per-pass benchmark 之后）：

- full baseline benchmark 证据可用（7 场景 measured_local）；
- per-pass FPS ratio 实测 >= 0.95x baseline（对齐契约 `single_scene_fps_ratio_min=0.95` 口径）；
- 更完整的 tonemapper 模式 / HDR output math parity 已证明（超出 LINEAR + sRGB 子集）；
- raster-vs-compute output seam 已设计并实测存在净收益，或 native continuation 退役。

## Fail-Closed Invariants

- `default_enable_state` 保持 `disabled`；默认 Godot config 下 bridge 对 `RXGD_PASS_TONEMAP` 仍返回 `RXGD_STATUS_FALLBACK`（`runtime_state=fallback_only_by_default_real_pass_optin_measured` 中 `fallback_only_by_default` 是默认路径口径）。
- `performance_claim=none`：本决策与 enablement success 均不构成 FPS、p95、GPU timestamp 或任何性能提升宣称。
- 本决策文件存在且校验通过 + enablement strict success 有效时，probe `next_action` 才可推进到 `start_grx011_ssao_blur_godot_patch_0014`；两者任一缺失/被篡改即 fail-closed 回退。
- 只有 strict 校验通过的 `real_pass_enablement_success_evidence.json` 才允许 manifest 记录 `real_gpu_pass=true`（opt-in 口径）；手工编辑的 placeholder 永远不能推进任何 gate。

## rd_native（Route B）默认启用追加 — GRX-025 close-out（2026-07-13，append-only）

> 本段是 GRX-025 收官对 **rd_native（Route B，patch 0040，backend 三态）** 复制阶段的默认启用决策追加，**不修改上方 bridge-era 结论**。汇总矩阵：[../DEFAULT_ENABLE_MATRIX.md](../DEFAULT_ENABLE_MATRIX.md)。

- rd_native 决策 token：**`eligible_for_default_enable_gate_met`**（GRX-025 ≥0.95 门 + GRX-024 视觉 parity 两者实测达成）。
- 当前 default：**仍 `disabled`**。成因不是门未过，而是 **rd_native 启用天生是 per-project opt-in**：须把 Rurix 容器 staged 到 `target/grx/rd_containers/tonemap.rd_container.bin`、设 `rendering/rurix_accel/passes/tonemap/rd_container_path` 指向它、把 `passes/tonemap/backend` 置 `2`（缺容器则 fail-closed 回落 native）。**无全局默认可翻**，故如实记「门槛条件已全部实测达成，启用为集成方按需 opt-in」。
- GRX-025 门槛（`spike/godot-rurix/bench/grx025_default_enable_20260713/`）：engaged geomean **0.9959** ≥ 0.95；worst engaged scene = many_mesh_instances / 0.9888 / noise 2.17%（含场景 noise 后仍过门）。
- engagement 子集：4/7 LINEAR-only(clustered_lights, many_mesh_instances, material_variants, particles;FILMIC 三场景 0040 mode-guard fail-closed)。
- 视觉 parity（`spike/godot-rurix/bench/grx024_visual_20260713/`）：GRX-024 byte-exact(clustered_lights, material_variants)。
- per-pass GPU µs 归因（`spike/godot-rurix/bench/rd_native_final_20260713/` §4）：+13~31 µs(~1.8-1.9x 其 native tonemap bucket)；程序级 Amdahl 零成本 geomean 天花板 **1.0669x**（measured all5 0.9942）。
- fail-closed 不变式：`performance_claim=none`（本追加不含 FPS/p95/GPU-timestamp/性能提升宣称）；缺容器/preflight 失败/dispatch-eligibility 失败/visual 超阈值一律回落 native；`RXGD_ABI_VERSION` 不变、bridge-independent（不占 cap bit）。
- 收官口径：1.5x strict 门（GRX_CONTRACT G-GRX-5）在此 workload/GPU/build 上被归档为**结构性不可达**（1.50 vs 1.0669 硬上限），门的数学与阈值不变、未放宽——是「归档不可达」非「降门」。
