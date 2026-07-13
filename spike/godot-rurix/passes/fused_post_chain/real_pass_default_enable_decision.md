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

## rd_native（Route B）默认启用追加 — GRX-025 close-out（2026-07-13，append-only）

> 本段是 GRX-025 收官对 **fused_post_chain rd_native（Route B，patch 0045 Design 1 shadow-recompute）** 的默认启用决策追加，**不修改上方 bridge-era 结论**。汇总矩阵：[../DEFAULT_ENABLE_MATRIX.md](../DEFAULT_ENABLE_MATRIX.md) §3。

- rd_native 决策 token：**`keep_disabled`**（双重独立成因，任一足以禁用）。
- 成因一 — **bench 7 场景从不 engage**：fused gate 需 LINEAR-tonemap 子集 AND auto-exposure 产出的独立 current/previous luminance buffer；7 个 bench 场景两者互斥（4 个 LINEAR 无 AE、2 个 AE 是 FILMIC），交集 ∅。`rd_native_final_20260713` §2 记 fused 0/7 non-engagement，all5_fused leg 仅作 A/A 控制，fused kernel 在 bench 上 perf-unmeasured。
- 成因二 — **专属 AE 场景上 engage 但两条 honest boundary**（`ci/grx_rb_fused_post_chain_rd_native_enablement_smoke.py` 5-leg 矩阵，CameraAttributesPractical 驱动 AE）：
  1. **parity out-of-tolerance**：candidate 腿 `RXGD_RD_NATIVE_FUSED_POST_CHAIN active` genuinely engage，但 fused AE/EMA+tonemap 数学对 native auto-exposure 的 LDR 输出发散 `max_abs=85 / mean_abs=66`（阈值 max≤4/mean≤1.0），因 b0 auto-exposure 标量仍是占位、待 Luminance-API 扩展；`pass_manifest.rd_native.enablement.status=measured_engaged_parity_out_of_tolerance`、`real_gpu_pass=false`、`first_missing_prerequisite=fused_tonemap_parity_out_of_tolerance`，**不写 success evidence、不推进 gate**。
  2. **结构净零**：fused 的 luminance-final 写是 shadow-recompute（自有 scratch UAV，永不读回），native luminance_reduction pyramid 仍每帧全跑，`honest_boundary.net_dispatch_saving=0`、`status_tokens=[engaged, shadow-luminance-write, dispatch-savings-not-claimed]`、`structural_fusion_claimed=false`。真结构收益要 Design 2（glow-off gate + 跳 native final reduce + luminance 双缓冲外部 SWAP），属未来批次。
  3. cascade 实测：AE-off + tonemap backend=2 腿上 fused gate 在 invalid-lum-RID 关口 fail-closed → `RXGD_RD_NATIVE_TONEMAP active`、帧与非 AE reference 逐字节一致（fused→tonemap→native 两级回退真机打通）。
- fail-closed 不变式：`performance_claim=none`；default 保持 `disabled`；`RXGD_ABI_VERSION` 不变、bridge-independent。
- 历史勘误：早前「fused 在 aliasing guard blocked、byte-identical」是误判（专属 smoke 曾设不受支持的 `auto_exposure_enabled` → SCRIPT ERROR、AE 从未 engage）；现 smoke 经 CameraAttributesPractical 驱动 AE 且对 SCRIPT ERROR 失败，上述 parity 是首次诚实 engage+parity 测量（manifest `historical_misattribution_note`）。
