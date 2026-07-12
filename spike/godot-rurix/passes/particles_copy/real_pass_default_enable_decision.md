# GRX-013 Particles Copy — Real-Pass Default-Enable Decision（close-out）

> Evidence 伴生文件：`real_pass_default_enable_decision.json`
> 前置事实：particles_copy real-pass enablement strict measured success（`real_pass_enablement_success_evidence.json`，`status=success`、`strict_success=true`、`real_gpu_pass=true`、`real_d3d12_dispatch_recorded=true`；opt-in real dispatch 在 0001..0022 scratch Godot（Windows D3D12 Forward+，NVIDIA GeForce RTX 4070 Ti）上完成，三腿 pass enable matrix 全绿——candidate 腿 `real_pass_marker_observed=true` + `writeback_marker_observed=true`（`RXGD_GODOT_RUNTIME_PARTICLES_COPY_REAL_PASS recorded=1`），forced 腿实测 `first_missing_prerequisite=dispatch_eligibility_failed`/`fallback_reason=unsupported_device`（`RXGD_PARTICLES_COPY_REAL_PASS_BLOCKED`；注意 particles_copy 的 b0 无 i64 字段，int64 不在 preflight，故 forced 降级在**下一级 dispatch-eligibility** 关口 fail-closed，与纹理 pass 的 preflight 关口不同），LDR visual gate `max_abs=0`/`mean_abs=0`，telemetry `measured_local` 通过 GRX-008 校验，`0001..0022` patch-stack/溯源/日志审计全绿）。场景 = 确定性 GPUParticles3D（fixed seed + `fixed_fps`，4096 粒子，`TRANSFORM_ALIGN_Z_BILLBOARD` 触发 cull-stage `particles_set_view_axis`，默认 `DRAW_ORDER_INDEX` 保 `do_sort=false` 选中在册 plain COPY_MODE_FILL_INSTANCES 子集，dispatch=64×1×1 = ceil(4096/64)，`dst_bytes=327680`=4096×80）。standalone dispatch smoke（`real_d3d12_dispatch_smoke.json`）`real_d3d12_dispatch_recorded=true`、`cpu_reference_match=true`。

## Owner Decision

- decision: **`keep_default_disabled`**
- approved_by: `owner`
- approved_at_utc: `2026-07-12T00:00:00Z`
- machine_role: `local_test_machine`
- 适用对象：`rendering/rurix_accel/passes/particles_copy/*` 全部 per-pass 设置保持默认 `false`；`pass_manifest.json` 的 `default_enable_state` 保持 `disabled`。

即使 particles_copy real-pass enablement 已取得 strict measured success（opt-in real dispatch 真正执行且完成、视觉门保持绿、红腿实测、溯源/日志审计全绿），**默认启用状态仍保持 disabled**。

## Rationale

1. **无 per-pass FPS 证据**：GRX 契约要求任何 pass 默认 enable 前必须有 per-pass FPS >= 0.95x baseline 的 measured_local 证据（GRX_PLAN GRX.4 出口判据 / GRX_CONTRACT G-GRX-5 口径 `single_scene_fps_ratio_min=0.95`）。当前不存在 particles_copy 的 full baseline 对比实测，无任何 per-pass benchmark。
2. **patch 0022 writeback 仍是 scaffold（无净收益）**：COPY_MODE_FILL_INSTANCES 结果虽 dispatch 进真实 Godot Transforms SSBO 目标，但 native Godot particles copy continuation 仍作 backstop，每帧照常重填全部 instance，Rurix dispatch 目前没有净收益（甚至是额外开销）；candidate 图像因此逐字节不变（LDR visual gate `max_abs=0`/`mean_abs=0`，与 GRX-011/012 同段位）。
3. **仅 COPY_MODE_FILL_INSTANCES 3D 子集**：math parity 仅覆盖 3D 模式、align mode ALIGN_DISABLED (0) 与 ALIGN_BILLBOARD (1) 的 fill-instances 子集（`math_parity_evidence.json`，`status=pending_gpu_dispatch`）；align mode 2/3/4、2D copy mode、`COPY_MODE_FILL_INSTANCES_WITH_SORT_BUFFER`（VIEW_DEPTH sort）与 FILL_SORT_BUFFER、`ORDER_BY_LIFETIME`/`REVERSE_LIFETIME` reindex、trail 插值（`trail_size>1`）、userdata 通道全部未覆盖（recorded gaps）。

## Re-evaluation Conditions

满足以下条件后由 owner 重新决策（在 full baseline + per-pass benchmark 之后）：

- full baseline benchmark 证据可用（7 场景 measured_local）；
- per-pass FPS ratio 实测 >= 0.95x baseline（对齐契约 `single_scene_fps_ratio_min=0.95` 口径）；
- 更完整的 particles_copy 覆盖已证明（超出 fill-instances 3D ALIGN_DISABLED/ALIGN_BILLBOARD 子集：2D copy / VIEW_DEPTH sort / lifetime reindex / trail / userdata / align mode 2/3/4）；
- native continuation 退役或实测存在净收益。

## Fail-Closed Invariants

- `default_enable_state` 保持 `disabled`；默认 Godot config 下 bridge 对 `RXGD_PASS_PARTICLES_COPY` 仍返回 `RXGD_STATUS_FALLBACK`，native particles copy 接管。
- `performance_claim=none`：本决策与 enablement success 均不构成 FPS、p95、GPU timestamp 或任何性能提升宣称；candidate 图像 bit-exact（`max_abs=0`）本身即证明尚无输出替代。
- 本决策文件存在且校验通过（顶层 `default_enable_decision` 字段非空，`ci/grx_gates/grx013_particles_copy.py` `_decision_ready` 口径）+ enablement strict success（`strict_success=true`）有效时，probe `next_action` 才可推进到 `start_grx014_cluster_store_pass_contract`；两者任一缺失/被篡改即 fail-closed（`grx_gate_module_error`），`next_action` 保持不变。
- 只有 strict 校验通过的 `real_pass_enablement_success_evidence.json` 才允许 manifest 记录 `real_gpu_pass=true`（opt-in 口径）；手工编辑的 placeholder 永远不能推进任何 gate。
