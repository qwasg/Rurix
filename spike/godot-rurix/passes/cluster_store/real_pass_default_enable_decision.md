# GRX-014 Cluster Store — Real-Pass Default-Enable Decision（close-out）

> Evidence 伴生文件：`real_pass_default_enable_decision.json`
> 前置事实：cluster_store real-pass enablement strict measured success（`real_pass_enablement_success_evidence.json`，`status=success`、`strict_success=true`、`real_gpu_pass=true`、`real_d3d12_dispatch_recorded=true`；opt-in real dispatch 在 0001..0026 scratch Godot（Windows D3D12 Forward+，NVIDIA GeForce RTX 4070 Ti）上完成，三腿 pass enable matrix 全绿——candidate 腿 `real_pass_marker_observed=true` + `writeback_marker_observed=true` + `record_marker_observed=true`（`RXGD_GODOT_RUNTIME_CLUSTER_STORE_REAL_PASS recorded=1`；Wave 4 print 门控下 candidate/forced 腿开 `dispatch_recording_smoke` 以驱动 per-dispatch instrumentation marker），forced 腿实测 `first_missing_prerequisite=dispatch_eligibility_failed`/`fallback_reason=unsupported_device`（`RXGD_CLUSTER_STORE_REAL_PASS_BLOCKED`；cluster_store 的 b0 全 u32 无 i64 字段，int64 不在 preflight，故 forced 降级在**下一级 dispatch-eligibility** 关口 fail-closed，GRX-013 同型），LDR visual gate `max_abs=0`/`mean_abs=0`，telemetry `measured_local` 通过 GRX-008 校验，`0001..0026` patch-stack/溯源/日志审计全绿）。场景 = 确定性 clustered-lights（5×5 静态 lit box 网格 + 9 盏静态 `OmniLight3D`（无阴影）保 `render_element_count > 0`，`ClusterBuilderRD::bake_cluster()` 每帧驱动 store compute merge dispatch——patch 0023 call site；三 structured buffer：cluster_render SRV t0 + render_elements SRV t1 输入、cluster_store UAV u0 输出，32 字节 b0 `ClusterStore::PushConstant` 镜像）。standalone dispatch smoke（`real_d3d12_dispatch_smoke.json`）`real_d3d12_dispatch_recorded=true`、`cpu_reference_match=true`（整数逐字零容差）。

## Owner Decision

- decision: **`keep_default_disabled`**
- approved_by: `owner`
- approved_at_utc: `2026-07-12T00:00:00Z`
- machine_role: `local_test_machine`
- 适用对象：`rendering/rurix_accel/passes/cluster_store/*` 全部 per-pass 设置保持默认 `false`；`pass_manifest.json` 的 `default_enable_state` 保持 `disabled`。

即使 cluster_store real-pass enablement 已取得 strict measured success（opt-in real dispatch 真正执行且完成、视觉门保持绿、红腿实测、溯源/日志审计全绿），**默认启用状态仍保持 disabled**。

## Rationale

1. **无 per-pass FPS 证据**：GRX 契约要求任何 pass 默认 enable 前必须有 per-pass FPS >= 0.95x baseline 的 measured_local 证据（GRX_PLAN GRX.4/GRX.5 出口判据 / GRX_CONTRACT G-GRX-5 口径 `single_scene_fps_ratio_min=0.95`）。当前不存在 cluster_store 的 full baseline 对比实测，无任何 per-pass benchmark。
2. **patch 0025 writeback 仍是 scaffold（无净收益）**：cluster_store merge 结果虽 dispatch 进真实 Godot `cluster_buffer` 目标 `ID3D12Resource*`，但 shim submit 在 Godot 帧命令序之外执行（先于本帧的 `buffer_clear`），且 native Godot cluster store dispatch 仍作 continuation/backstop 每帧照常重打包整张 cluster 表，Rurix dispatch 目前没有净收益（甚至是额外开销）；candidate 图像因此逐字节不变（LDR visual gate `max_abs=0`/`mean_abs=0`，与 GRX-011/012/013 同段位）。
3. **仅 compute merge（store）段**：cluster_render 光栅段（proxy-mesh draw）、两个 `buffer_clear` 与 `render_element_count == 0` early-out 永久留 native（`resource_mapping.md` 范围裁定）；GPU 侧 math parity 虽 CPU 整数精确参照已证，但 in-engine 观测仍 pending（`math_parity_evidence.json` `status=pending_gpu_dispatch`），且录进 Godot 自身 compute list 的帧内命令序集成（替代 shim 的带外 submit）尚未设计。

## Re-evaluation Conditions

满足以下条件后由 owner 重新决策（在 full baseline + per-pass benchmark 之后）：

- full baseline benchmark 证据可用（7 场景 measured_local）；
- per-pass FPS ratio 实测 >= 0.95x baseline（对齐契约 `single_scene_fps_ratio_min=0.95` 口径）；
- native continuation 退役或实测存在净收益；
- 帧内命令序集成（录进 Godot compute list 而非 shim 带外 submit）已设计并证明。

## Fail-Closed Invariants

- `default_enable_state` 保持 `disabled`；默认 Godot config 下 bridge 对 `RXGD_PASS_CLUSTER_STORE` 仍返回 `RXGD_STATUS_FALLBACK`，native cluster store 接管。
- `performance_claim=none`：本决策与 enablement success 均不构成 FPS、p95、GPU timestamp 或任何性能提升宣称；candidate 图像 bit-exact（`max_abs=0`）本身即证明尚无输出替代。
- 本决策文件存在且校验通过（顶层 `default_enable_decision` 字段非空，`ci/grx_gates/grx014_cluster_store.py` `_decision_ready` 口径）+ enablement strict success（`strict_success=true`）有效时，probe `next_action` 才可推进到 `start_grx015_gpu_culling_pass_contract`；两者任一缺失/被篡改即 fail-closed（`grx_gate_module_error`），`next_action` 保持不变。
- 只有 strict 校验通过的 `real_pass_enablement_success_evidence.json` 才允许 manifest 记录 `real_gpu_pass=true`（opt-in 口径）；手工编辑的 placeholder 永远不能推进任何 gate。
