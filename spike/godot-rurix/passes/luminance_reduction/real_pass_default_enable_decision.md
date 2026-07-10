# GRX-009 Luminance Reduction — Real-Pass Default-Enable Decision（stage A5）

> Evidence 伴生文件：`real_pass_default_enable_decision.json`
> 前置事实：segment 4h strict measured success（`real_pass_enablement_success_evidence.json`，`status=success`、`real_gpu_pass=true`、`real_d3d12_dispatch_recorded=true`；opt-in real dispatch 在 0001..0010 scratch Godot（NVIDIA GeForce RTX 4070 Ti）上完成，LDR visual gate `max_abs=0`，`forced_capability_downgrade` 红腿实测 `unsupported_device`，telemetry `measured_local` 通过）。

## Owner Decision

- decision: **`keep_default_disabled`**
- approved_by: `owner`
- approved_at_utc: `2026-07-08T13:30:00Z`
- machine_role: `local_test_machine`
- 适用对象：`rendering/rurix_accel/passes/luminance_reduction/*` 全部 per-pass 设置保持默认 `false`；`pass_manifest.json` 的 `default_enable_state` 保持 `disabled`。

即使 segment 4h 已取得 strict measured success（opt-in real dispatch 真正执行且完成、视觉门保持绿、红腿实测、溯源/日志审计全绿），**默认启用状态仍保持 disabled**。

## Rationale

1. **无 per-pass FPS 证据**：GRX 契约要求任何 pass 默认 enable 前必须有 per-pass FPS >= 0.95x baseline 的 measured_local 证据（GRX_PLAN GRX.4 出口判据 / GRX_CONTRACT G-GRX-5 口径）。当前不存在 full baseline 实测，GRX-005 runner 证据仅为 quick-smoke，无任何 per-pass benchmark。
2. **patch 0010 writeback 仍是 scaffold**：level-0 结果虽 dispatch 进真实 `luminance_buffers->reduce[0]`，但 native Godot luminance continuation 仍作为 backstop 重渲染全部 reduction level，Rurix dispatch 目前没有净收益（甚至是额外开销）。
3. **math parity GPU 腿不完整**：math parity 仅 level-0 CPU-proven（`math_parity_evidence.json`，`status=pending_gpu_dispatch`），multi-level pyramid / EMA feedback / WRITE_LUMINANCE clamp parity 尚未证明。

## Re-evaluation Conditions

满足以下条件后由 owner 重新决策（在 full baseline + per-pass benchmark 之后）：

- full baseline benchmark 证据可用（7 场景 measured_local）；
- per-pass FPS ratio 实测 >= 0.95x baseline（对齐契约 `single_scene_fps_ratio_min=0.95` 口径）;
- multi-level math parity 已证明（pyramid/EMA/WRITE_LUMINANCE）;
- native continuation 退役或实测存在净收益。

## Fail-Closed Invariants

- `default_enable_state` 保持 `disabled`；默认 Godot config 下 bridge 对 `RXGD_PASS_LUMINANCE_REDUCTION` 仍返回 `RXGD_STATUS_FALLBACK`（`runtime_state=fallback_only_by_default_real_pass_optin_measured` 中 `fallback_only_by_default` 是默认路径口径）。
- `performance_claim=none`：本决策与 4h success 均不构成 FPS、p95、GPU timestamp 或任何性能提升宣称。
- 本决策文件存在且校验通过 + 4h strict success 有效时，probe `next_action` 才可推进到 `start_grx010_tonemap_pass_contract`;两者任一缺失/被篡改即 fail-closed 回退。
- 只有 strict 校验通过的 `real_pass_enablement_success_evidence.json` 才允许 manifest 记录 `real_gpu_pass=true`（opt-in 口径）;手工编辑的 placeholder 永远不能推进任何 gate。
