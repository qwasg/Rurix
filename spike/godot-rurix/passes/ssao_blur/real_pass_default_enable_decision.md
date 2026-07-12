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
