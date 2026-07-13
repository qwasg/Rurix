# GRX-015 gpu_culling — rd_native default-enable decision（close-out，2026-07-13）

> Evidence 伴生文件：`rd_native_default_enable_decision.json`
> 诊断全文：[rd_native_device_removal_diagnosis.md](rd_native_device_removal_diagnosis.md) §6
> 汇总矩阵：[../DEFAULT_ENABLE_MATRIX.md](../DEFAULT_ENABLE_MATRIX.md) §4

本 pass 无 bridge-era `real_pass_default_enable_decision`（gpu_culling rd_native 从未取得 enablement success），故本文件是其**唯一**默认启用决策，记录 GRX-025 收官下的 mechanism-blocked 终态。

## Decision

- decision: **`mechanism_blocked`**（default_enable_state 保持 `disabled`）
- machine_role: `local_test_machine`（RTX 4070 Ti，Windows D3D12 Forward+）
- performance_claim: `none`

gpu_culling rd_native **不满足**任何默认启用前置：它在真机上 **device-remove**，连 opt-in 都不可用。

## Rationale — R1 终裁

R1 已实现到位（patch 0046 修订：Rurix-owned scratch indirect buffer、`buffer_copy` live→scratch 为唯一 touch live buffer、clear/dispatch 限于 scratch、`draw_list_draw_indirect` retarget scratch），交付其预注册终裁 `ci/grx_rb_gpu_culling_rd_native_enablement_smoke.py`（rb4 exe，0001-0029+0040-0048）。

**Verdict: candidate 腿仍 device-remove（`0x887A0005`）。** 按预注册决策树，这定罪 GENERAL 假设：此 Godot D3D12 后端上「compute 写 `DISPATCH_INDIRECT` buffer 被同帧 indirect draw 消费」的模式，无论涉及哪个 buffer、kernel 写什么（空 kernel 复现）、instance 数（1 复现）都移除设备；debug 层与 GPU-Based Validation 全静默（CPU 侧 barrier/state 链验证干净，故障在 GPU 时间线）。

R2（用上帧 cull 结果 + 帧边界 copy）被拒：1 帧陈旧 visibility 在相机运动下违背 pass 的保守画面保持不变式，视觉门只在静态场景通过 = 不诚实覆盖。

**R3 终态**：`pass_manifest.json` 已翻转到 `grx015_rd_native_r1_final_verdict_mechanism_blocked_rdg_gap`；GRX-015/016/018 在此 engine/driver 组合上均 blocked；default 保持 disabled；无性能宣称。

## 依赖收口（GRX-016 / GRX-018）

GRX-016 instance_compaction 与 GRX-018 indirect_args 都依赖同一 compute→indirect 机制，故同 RDG 缺口下 mechanism-blocked，收官归档为 blocked（各自 offline kernel + math parity 已落地、default disabled）。

## Upstream

这是 upstream Godot bug-report 候选（RDG / D3D12 driver：一帧 frame-graph submission 内缺 UAV→INDIRECT_ARGUMENT 的 GPU-timeline sync）；report recipe = 本诊断 note + 三阶段 evidence 链（shim / in-graph live / in-graph scratch）。

## Fail-Closed 不变式

- `default_enable_state` 保持 `disabled`；backend 默认 `0`（native）；缺容器/preflight 失败一律回落 native。
- `performance_claim=none`：本决策不构成任何 FPS、p95、GPU timestamp 或性能提升宣称。
- 1.5x strict 门（GRX_CONTRACT G-GRX-5）在收官归档为**结构性不可达**（1.50 vs Amdahl 1.0669x 硬上限）——门数学与阈值不变、未放宽。
