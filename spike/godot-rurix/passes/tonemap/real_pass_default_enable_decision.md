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
