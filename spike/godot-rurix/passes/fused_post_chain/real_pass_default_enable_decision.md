# GRX-019 Fused Post Chain — Real-Pass Default-Enable Decision（close-out）

> Evidence 伴生文件：`real_pass_default_enable_decision.json`
> 前置事实：fused_post_chain real-pass enablement strict measured success（`real_pass_enablement_success_evidence.json`，`status=success`、`strict_success=true`、`real_gpu_pass=true`、`real_d3d12_dispatch_recorded=true`；opt-in real dispatch 在 `0001..0026 + 0036..0038` scratch Godot（Windows D3D12 Forward+）上完成，三腿 pass enable matrix 全绿——candidate 腿 `real_pass_marker_observed=true` + `writeback_marker_observed=true` + `record_marker_observed=true`（`RXGD_GODOT_RUNTIME_FUSED_POST_CHAIN_REAL_PASS recorded=1`），forced 腿实测 `first_missing_prerequisite=dispatch_eligibility_failed`/`fallback_reason=unsupported_device`（`RXGD_FUSED_POST_CHAIN_REAL_PASS_BLOCKED`；注意 fused 的 runtime binding preflight **不**校验 int64——只有 bridge dispatch eligibility 校验，故 forced 降级在**下一级 dispatch-eligibility** 关口 fail-closed，与其它纹理 pass 的 preflight 关口不同），reference 腿零 marker，LDR visual gate `max_abs=0`/`mean_abs=0`，telemetry `measured_local` 通过 GRX-008 校验，`0001..0026 + 0036..0038`（0027-0035 为 GRX-015/016/018 预留孔）patch-stack/溯源/日志审计全绿）。场景 = 确定性 3D 场景，`CameraAttributesPractical.auto_exposure_enabled=true`（触发原生 luminance_reduction,使 1×1 current luminance buffer 有效——fusion call site 的 luminance 输入前置）+ FILMIC tonemapper,`fixed_fps` 使 auto-exposure 时域 EMA 确定性收敛。standalone dispatch smoke（`real_d3d12_dispatch_smoke.json`）`real_d3d12_dispatch_recorded=true`、`cpu_reference_match=true`。

## Owner Decision

- decision: **`keep_default_disabled`**
- approved_by: `owner`
- approved_at_utc: `2026-07-12T00:00:00Z`
- machine_role: `local_test_machine`
- 适用对象：`rendering/rurix_accel/passes/fused_post_chain/*` 全部 per-pass 设置保持默认 `false`；`pass_manifest.json` 的 `default_enable_state` 保持 `disabled`。

即使 fused_post_chain real-pass enablement 已取得 strict measured success（opt-in real dispatch 真正执行且完成、视觉门保持绿、红腿实测、溯源/日志审计全绿），**默认启用状态仍保持 disabled**。

## Rationale

1. **无 per-pass FPS 证据**：GRX 契约要求任何 pass 默认 enable 前必须有 per-pass FPS >= 0.95x baseline 的 measured_local 证据（GRX_PLAN GRX.4 出口判据 / GRX_CONTRACT G-GRX-5 口径 `single_scene_fps_ratio_min=0.95`）。当前不存在 fused_post_chain 的 full baseline 对比实测,无任何 per-pass benchmark。
2. **patch 0038 writeback 仍是 scaffold（无净收益）**：fused kernel 虽 dispatch 进真实 LDR tonemap 目标（dst_color）+ scaffold dst_luminance（同为该 LDR 目标,被后续 native 完全覆写）,但 native Godot luminance_reduction + tonemap chain continuation 仍作 backstop,每帧照常重渲染,Rurix dispatch 目前没有净收益（甚至是额外开销）；candidate 图像因此逐字节不变（LDR visual gate `max_abs=0`/`mean_abs=0`,与 GRX-010/013 同段位）。
3. **scaffold fused binding（不宣称数学正确性）**：fused luminance 段的 distinct ≤8×8 reduce 级与双缓冲 1×1 prev/dst luminance 目标**未被 Godot 公开 Luminance API 在 tonemap call site 暴露**,故 patch 0037 把唯一可取的公开 1×1 current luminance buffer 别名给 lum_source 与 prev_luminance（只读）,把 LDR tonemap 目标别名给 dst_color 与 dst_luminance。strict success 只断言「一次真实 D3D12 fused dispatch 被录制且 native-continuation 视觉门保持逐字节稳定」,**不**断言 fused 输出正确或是净收益。真正 distinct/correct 的 fused binding（真正满足 auto-exposure-on / glow-off / LINEAR / SDR 融合前置）需要延迟的 Luminance-API 扩展 / draw_graph 路线 B（PASS_CONTRACT.md §3.4/§5）。
4. **仅 LINEAR + sRGB + auto-exposure 子集**：math parity 仅覆盖 fused EMA×tonemap（TONEMAPPER_LINEAR + linear_to_srgb）子集（`math_parity_evidence.json`,`status=pending_gpu_dispatch`）；glow、LINEAR 之外的 tonemapper 模式（Reinhard/FILMIC/ACES/AgX）、auto-exposure 纹理链其余消费者、以及当帧 tonemap 替代（一帧延迟）全部未覆盖（recorded gaps）。

## Re-evaluation Conditions

满足以下条件后由 owner 重新决策（在 full baseline + per-pass benchmark 之后）：

- full baseline benchmark 证据可用（measured_local）；
- per-pass FPS ratio 实测 >= 0.95x baseline（对齐契约 `single_scene_fps_ratio_min=0.95` 口径）；
- 经 Luminance-API 扩展 / draw_graph 路线 B 提供 distinct/correct 的 fused binding；
- native continuation 退役或实测存在净收益；
- 更完整的 fused_post_chain 覆盖已证明（超出 LINEAR + SDR + auto-exposure 子集）。

## 不蕴含（does not imply）

本决策与其前置 strict measured success **不**蕴含：默认 pass 启用；性能/FPS/p95/GPU-timestamp 宣称；fused 输出的数学正确性（binding 为 scaffold）；净收益 Rurix dispatch；超出 LINEAR+SDR+auto-exposure 子集的更广覆盖。default_enable_state 保持 `disabled`,shipping feature-off bridge 仍以 `real_dispatch_path_not_linked` fail-closed。
