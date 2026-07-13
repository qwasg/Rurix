# GRX-015 gpu_culling — rd_native default-enable decision（close-out，2026-07-13）

> Evidence 伴生文件：`rd_native_default_enable_decision.json`
> 诊断全文：[rd_native_device_removal_diagnosis.md](rd_native_device_removal_diagnosis.md) §6
> 汇总矩阵：[../DEFAULT_ENABLE_MATRIX.md](../DEFAULT_ENABLE_MATRIX.md) §4

本 pass 无 bridge-era `real_pass_default_enable_decision`（gpu_culling rd_native 从未取得 enablement success），故本文件是其**唯一**默认启用决策，记录 GRX-025 收官下的 mechanism-blocked 终态。

## Decision（2026-07-13 翻案后现值）

- decision: **`keep_disabled`**（default_enable_state 保持 `disabled`）
- manifest_status: `grx015_rd_native_r1c_picture_preserving_first_strict_success`
- machine_role: `local_test_machine`（RTX 4070 Ti，Windows D3D12 Forward+）
- performance_claim: `none`

**翻案(retraction)**：下方「Rationale — R1 终裁」的 `mechanism_blocked` 定罪（RDG 同帧 sync 缺口）**已撤销**——真凶是 Godot D3D12 驱动的 misaligned `buffer_clear`（16B 对齐 bug），R1b + R1c 修复后 gpu_culling rd_native 取得 **strict success**（candidate engage + 无任何腿设备移除 + byte-exact vs native reference `max_abs=0`，manifest `grx015_rd_native_r1c_picture_preserving_first_strict_success`）。现决策 = `keep_disabled`，理由**不再是 blocked**，而是：在此 gate 场景 high-water-mark == 满 count（可见尾），**net draw 削减 = 0**，且**无 per-pass bench ratio 证据**——门达成 ≠ 净收益。完整叙述见文末「§ 修订（2026-07-13 culling 翻案）」。旧正文保留作审计。

## Rationale — R1 终裁

**[SUPERSEDED — 2026-07-13 翻案撤销此结论,见文末「§ 修订」;以下为历史审计正文,未改]**

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

## § 修订（2026-07-13 culling 翻案）

> 本段 append-only，供 close-out reconcile；上方「Decision」metadata 已同步现值,「Rationale — R1 终裁」历史正文保留未改并标注 SUPERSEDED。

**撤销的定罪**：R1(0046)终裁把 candidate 腿的设备移除（`0x887A0005`）定罪为一个 general RDG 缺口——「compute 写 `DISPATCH_INDIRECT` buffer 被同帧 indirect draw 消费」在此后端无条件移除设备。该 GENERAL 假设**已被证伪并撤销**。

**真凶（misaligned buffer_clear）**：清每-surface 计数 dword 于 byte offset `(s*command_stride_dwords + instance_count_dword_index)*4` = 4/24/44…（从不是 16 的倍数）会 lower 成 `RenderingDeviceDriverD3D12::command_clear_buffer` 的一个 RAW buffer UAV（`D3D12_BUFFER_UAV_FLAG_RAW`, `FirstElement = offset/4`）+ `ClearUnorderedAccessViewUint`；D3D12 要求 RAW buffer UAV byte offset 为 16 的倍数（`D3D12_RAW_UAV_SRV_BYTE_ALIGNMENT`）——越界 UAV 移除设备。每个此前崩溃的腿（shim / in-graph live / R1 scratch）都共享这个 misaligned clear。

**两 reproducer 定罪为驱动 bug**：
- misaligned buffer_clear（真凶）：`spike/godot-rurix/upstream-repro/rd-buffer-clear-misaligned-offset/`（offset 4/8/12/20/36 第 1 帧移除设备，0/16/32/48 干净，完美 offset % 16 律，无 compute/draw 参与）。upstream issue DRAFT `spike/godot-rurix/upstream-repro/ISSUE_DRAFT.md`（待 owner 发）。
- 纯 compute→indirect 假说 FALSIFIED：`spike/godot-rurix/upstream_bug_repro/`（compute UAV-写 DISPATCH_INDIRECT buffer 同帧被 `draw_list_draw_indirect` 消费,跑 300 帧干净）。

**修复与 strict success**：
- **R1b**（patch 0046 修订）：保留 Rurix-owned scratch indirect 解耦 + 把 misaligned 计数-dword `buffer_clear` 换成对齐的 `buffer_copy`（from persistent 16-byte 全零 SSBO，`CopyBufferRegion` 无 RAW-UAV 对齐约束）→ 破了设备移除墙（`rb5`）；暴露一个下游 over-cull（picture-preservation）correctness 缺陷。
- **R1c**（container-only 内核改动，无 0046/b0/RTS0/descriptor/exe 变更）：per-surface 计数写从 `InterlockedAdd(+1)` count-of-visible 改成 **high-water-mark** `InterlockedMax(instance+1)`，使 prefix draw `[0..InstanceCount-1]` 覆盖每个可见实例、永不 over-cull。
- 终裁 `ci/grx_rb_gpu_culling_rd_native_enablement_smoke.py`（rb5 exe + R1c container）= candidate engage + 无任何腿设备移除 + byte-exact vs native reference（`max_abs=0`），gate 三准则全绿，`real_gpu_pass_recorded=true`。诊断 [rd_native_device_removal_diagnosis.md](rd_native_device_removal_diagnosis.md) §7（R1b）+ §8（R1c）；manifest `rd_native_r1b_verdict` / `rd_native_r1c_verdict`。

**为何 strict success 仍 `keep_disabled`（诚实 net-zero）**：此 gate 场景最深角实例（index 4095）可见,故 high-water-mark == 满 count（4096）:candidate scratch command buffer 与 live 逐字节相同，**net draw-count 削减 = 0**（场景性质=可见尾，非 fix 缺陷）。要可测的 draw-count 削减需 (a) instance-array 尾在视锥外的场景，或 (b) GRX-016 transform compaction。故 `default_enable_state` 保持 `disabled`、`performance_claim=none`，且无 per-pass bench ratio 证据支撑默认启用。

**依赖收口修订（GRX-016/018）**：下方「依赖收口」旧正文把 016/018 归为 mechanism_blocked;设备移除墙既破，该判定**撤销**——016/018 = `unblocked_deferred`（解封但运行时未实现，归后续里程碑候选）。

**对 1.5x 收官判定的影响**：无。culling 翻案不改 G-GRX-5 判定——compaction 于当前 bench 量级评估 = 个位数百分点（indirect 子集 40k/260k + 实例多在视锥内），Amdahl 零成本 geomean 硬上限仍 1.0669x « 1.50。详 `milestones/grx/GRX_PLAN.md` §8「1.5x strict 门收官修订」。
