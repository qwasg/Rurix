# GRX-009 Luminance Reduction Pass — PASS CONTRACT

> **状态声明追加（2026-07-08，stage A5）：`segment 4h` gated real-pass enablement 已取得 strict MEASURED success。** `real_pass_enablement_success_evidence.json` 记录：opt-in real dispatch（`rendering/rurix_accel/passes/luminance_reduction/enabled`+`dispatch_bringup`+`dispatch_real_pass`，三者均默认 `false`，显式开启后）在 FULL 0001..0010 patch stack 重建的 scratch Godot console exe（NVIDIA GeForce RTX 4070 Ti，`RURIX_GRX009_SEGMENT4H_GODOT_EXE`）上真正执行且完成——`RXGD_GODOT_RUNTIME_LUMINANCE_REAL_PASS` marker 入证、LDR visual gate 保持绿（reference/candidate 逐字节一致，max_abs=0、mean_abs=0，阈值 max<=2 / mean<=0.25）、`forced_capability_downgrade` 红腿实测 `unsupported_device` fallback（`RXGD_REAL_PASS_BLOCKED first_missing_prerequisite=runtime_binding_preflight_failed`）、measured_local telemetry 通过 GRX-008 校验、0001..0010 patch-stack identity / scratch 溯源 / runtime log 审计全绿。canonical package 为 owner-approved HLSL bridge texture package（`texture_artifact_provenance_policy.json`，per-slot texture2d/rwtexture2d binding kinds、Rurix-owned RTS0）。patch `0010-rurix-accel-luminance-real-pass-result-writeback.patch` 仍是 **scaffold**：level-0 结果 dispatch 进真实 `luminance_buffers->reduce[0]`，但 native Godot luminance continuation 仍重渲染全部级别（画面不可能变、Rurix dispatch 无净收益）。manifest 顶层已如实翻转：`status=stage_a5_real_pass_measured_success_default_disabled`、`implemented=true`、`real_gpu_pass=true`（**opt-in 实测口径**；默认路径仍 fallback-only）、`runtime_state=fallback_only_by_default_real_pass_optin_measured`。**`default_enable_state` 保持 `disabled`**：owner 决策 `real_pass_default_enable_decision.json` / `.md` 记 `keep_default_disabled`——无 per-pass FPS 证据（契约要求 per-pass FPS >= 0.95x baseline 才可默认 enable）、0010 writeback 仍是 scaffold、math parity GPU 腿仅 level-0 CPU-proven pending multi-level；full baseline + per-pass benchmark 后复评。默认 Godot config 下 bridge 对 `RXGD_PASS_LUMINANCE_REDUCTION` 仍返回 `RXGD_STATUS_FALLBACK`、native luminance path 接管；无 FPS、p95、GPU timestamp 或任何性能提升宣称。probe `next_action=start_grx010_tonemap_pass_contract`。下方 2026-07-04 状态声明为历史快照，其中 `real_gpu_pass=false` / “strict success 按设计不可达”等描述已被本段取代。
> **状态声明（2026-07-04）：`segment 4b` gated dispatch bring-up 在 `segment 4a` runtime binding preflight 之前加入一个显式、默认关闭、opt-in 的 dispatch eligibility gate：只有当 Godot opt-in 设置 `.../dispatch_bringup` 开启（经保留 flag `RXGD_CAP_LUMINANCE_DISPATCH_BRINGUP` 传入 `RxGdCaps.flags`，不改 ABI struct layout）、64-bit integer 能力、native D3D12 device/queue 与 resource handle 均非空、compiled package 的 descriptor layout 与离线 evidence 摘要匹配时才算 eligible。即便全部满足，且 `segment 4c` 已有 standalone measured D3D12 dispatch smoke（见下），explicit dispatch gate 仍关闭：因为没有 bridge-linked runtime dispatch recording path、没有 measured bridge telemetry，且不能让 `rxgd_record_pass` 返回 OK，`rxgd_record_pass` 对 `RXGD_PASS_LUMINANCE_REDUCTION` 仍返回 `RXGD_STATUS_FALLBACK`，不录制 D3D12 dispatch，不累计 estimated GPU/CPU time。`segment 4c`（见 §14）新增 real Windows D3D12 dispatch smoke harness（`ci/grx009_luminance_d3d12_dispatch_smoke.py`），其 measured evidence（`real_d3d12_dispatch_smoke.json`）记 `status=success`：tracked 离线 DXIL container + RTS0 root signature + descriptor layout 在真实 D3D12 device/queue 上完成一次最小 compute dispatch（RTS0 accept、compute PSO from tracked DXIL、SRV t0/UAV u0/b0 绑定、`Dispatch(1,1,1)`、fence 完成、dst UAV readback）。该 smoke 仅为独立 measured evidence，**不**让 bridge 录制 dispatch 或返回 OK、**不**默认启用 Godot luminance Rurix path。runtime 仍 fallback_only（manifest `runtime_state=fallback_only`、`real_gpu_pass=false`、`real_d3d12_dispatch_recorded=false`），无 visual/telemetry/性能宣称。`grx009_real_d3d12_dispatch_smoke_ready` 为 true 后 `next_action=start_grx009_bridge_real_d3d12_dispatch_recording`；smoke 缺失/SKIP/FAIL 时保持 `provide_grx009_luminance_real_d3d12_dispatch_smoke`。`segment 4a` runtime binding preflight 保持不变。canonical `segment 3a` 离线 compile evidence 当前 `status=compile_failed`（segment 4i texture-capable attempt 因 patched llc 不支持 `llvm.dx.resource.load.texture.2d` intrinsic 失败；`runtime_mappable=false`、`attempted_binding_kinds=[texture2d,rwtexture2d]`、`blocker_category=dxil_container_missing`），canonical artifacts 路径携带 raw-buffer 字节复制自 `artifacts/raw_buffer_historical/` 让 bridge `include_bytes!` 工作，manifest 顶层 `offline_compile_status=compile_failed`，runtime 仍 `fallback_only`；historical raw-buffer segment 3a success 保留在 `offline_compile_evidence_raw_buffer.json`，current first blocker 是 `kernel_binding_kind_mismatch`（bridge tracked package 仍为 raw-buffer，Godot runtime 提供 Texture2D 资源），`math_pyramid_parity_not_proven` 仅 future-only。`segment 4d`（见 §15）新增 bridge real D3D12 dispatch recording smoke（`ci/grx009_luminance_bridge_recording_smoke.py`），其 measured evidence（`bridge_dispatch_recording_evidence.json`）记 `status=success`：`rurix_godot.dll` 以**默认关闭**的 `d3d12-recording-shim` feature 编译后，经 C ABI 在真实 D3D12 device/queue 上录制一次最小 luminance compute dispatch（`rxgd_record_pass` 返回 `RXGD_STATUS_OK`、`recorded_passes=1`、`fallback_passes=0`、fence 完成、dst UAV readback）。录制路径**仅**在 test-only feature 下编译、**仅**由 harness-only flag `RXGD_CAP_LUMINANCE_DISPATCH_RECORD` 武装；Godot module 从不设置该 flag，shipping（feature-off）bridge 不变、对 `RXGD_PASS_LUMINANCE_REDUCTION` 仍返回 `RXGD_STATUS_FALLBACK`。该 bridge smoke 仅为 measured evidence，**不**启用 Godot luminance Rurix path（`godot_runtime_luminance_path_enabled=false`）、**不**完成 Godot runtime pass，保持 `runtime_state=fallback_only`、`real_gpu_pass=false`、`real_d3d12_dispatch_recorded=false`、`default_enable_state=disabled`、`gpu_timestamp_status=not_yet`（不伪造 `gpu_time_ns`）。`grx009_bridge_real_d3d12_dispatch_recording_ready` 为 true 后 `next_action=start_grx009_godot_native_resource_handle_mapping`；bridge smoke 缺失/SKIP/FAIL 时保持 `start_grx009_bridge_real_d3d12_dispatch_recording`。无 visual/telemetry/性能宣称。`segment 4e`（见 §16）新增 native D3D12 resource handle mapping（patch 0007）：Godot runtime 现在经 `RenderingDevice::get_driver_resource(DRIVER_RESOURCE_TEXTURE, RID, 0)` 把 `rb->get_internal_texture()` 与 `luminance_buffers->reduce[0]` 解析成真实 `ID3D12Resource*` native handle 并放入 `RxGdResource.native_handle`（非 logical RID id），native handle 为 0 或 `RenderingDevice` 不可用时 fallback 到 Godot 原生 luminance path。该段**只**完成 native handle mapping + preflight/evidence，**不**启用默认 Rurix luminance runtime pass、**不**让 shipping/feature-off bridge 返回 OK、`RXGD_ABI_VERSION` 不变、Godot module **仍不**设置 `RXGD_CAP_LUMINANCE_DISPATCH_RECORD`，manifest 保持 `runtime_state=fallback_only`、`real_gpu_pass=false`、`real_d3d12_dispatch_recorded=false`、`default_enable_state=disabled`。`grx009_segment4e_native_resource_handle_mapping_ready` 为 true 后 `next_action=start_grx009_godot_runtime_bridge_dispatch_recording_smoke`；0007 缺失/不可叠加时保持 `fix_grx009_luminance_segment4e_patch_0007_applyability`。无 real GPU pass、visual diff、telemetry 或性能宣称。`segment 4f`（见 §17）新增 Godot-runtime bridge dispatch recording smoke（`ci/grx009_godot_runtime_bridge_recording_smoke.py`，patch 0008）：经 FULL 0001..0008 patch stack 重建的 ignored scratch Godot console exe（`module_rurix_accel_enabled=yes d3d12=yes`，经 `RURIX_GRX009_SEGMENT4F_GODOT_EXE` 指向）在真实 D3D12 上，由 **patched Godot runtime luminance call site**（经默认关闭的 harness-only `.../dispatch_recording_smoke` opt-in 与 `d3d12-recording-shim` `rurix_godot.dll`）用它经 `RenderingDevice::get_driver_resource` 解析的真实 `ID3D12Resource*` native handle 驱动一次 bridge 录制的 `RXGD_PASS_LUMINANCE_REDUCTION` dispatch，打印 `RXGD_GODOT_RUNTIME_LUMINANCE_RECORD ... recorded=1` marker 并 `exit_code == 0`（`checks.godot_exit_code_zero=true`）时，**latest 证据** `godot_runtime_bridge_recording_evidence.json` 记 `status=success`、`godot_runtime_bridge_recorded_dispatch=true`，且**同一份 success 文档另写入 historical 证据** `godot_runtime_bridge_recording_success_evidence.json`（记录 Godot exe fingerprint、0001..0008 patch stack identity、DLL fingerprint、artifact hashes、`godot_exit_code_zero=true`、marker `recorded=1`，注明 scratch build 二进制不入 Git）。latest 证据每次运行改写、未设 `RURIX_GRX009_SEGMENT4F_GODOT_EXE` 时诚实回落为 `status=skip`；historical success 证据仅在严格 success 时写入、之后 SKIP/FAIL 运行绝不删除或覆盖它,故 readiness gate 只看 historical success 文件。**口径更正**：segment 4d/4e 的“Godot module 从不设置 `RXGD_CAP_LUMINANCE_DISPATCH_RECORD`”自本段起更正——module 仅在该默认关闭的 opt-in 显式开启时才设置该 record-arm flag；默认 Godot config 与 shipping/feature-off bridge 仍从不设置、对 `RXGD_PASS_LUMINANCE_REDUCTION` 仍恒返回 `RXGD_STATUS_FALLBACK`。即便 success 仍保持 `runtime_state=fallback_only`、`real_gpu_pass=false`、`real_d3d12_dispatch_recorded=false`、`godot_runtime_luminance_path_enabled=false`、`default_enable_state=disabled`、`gpu_timestamp_status=not_yet`。`grx009_segment4f_godot_runtime_bridge_recording_ready`（读 historical success 证据）为 true 后 `next_action=start_grx009_luminance_real_visual_diff_and_measured_fallback_telemetry`；historical success 缺失（含 latest 为 SKIP/FAIL、从未录得 success）时保持 `start_grx009_godot_runtime_bridge_dispatch_recording_smoke`，0008 不可叠加时保持 `fix_grx009_luminance_segment4f_patch_0008_applyability`。segment 4f 本身无 real GPU pass、visual diff、measured telemetry、GPU timestamp 或性能宣称。`segment 4g`（见 §18）新增**首个 real visual diff + measured fallback telemetry gate**（`ci/grx009_segment4g_visual_fallback_smoke.py`）：tracked Godot build（0001+0002+0003）以 pass 默认关闭（reference）与开启但 fallback（candidate）两条 enable-matrix 腿各跑一次同一确定性 auto-exposure 场景；candidate 腿必须实测打印 patch 0002 的 `RurixAccel: luminance_reduction fallback rc=` marker（fallback path observed），两帧 raw RGB8 必须在 pinned LDR absolute diff 阈值内（max_abs<=2 / mean_abs<=0.25；probe 从磁盘字节重算 hash/尺寸/diff 反伪造，SKIP/placeholder 永不 ready）。本机已录得 measured success（两帧逐字节一致，max_abs=0，`measured_fallback_telemetry.json` 通过 GRX-008 校验）。该 gate **只**证明 fallback path 真被走到且开启（恒回退的）pass 不改画面——两帧均由 Godot 原生 luminance 路径渲染，**不是** Rurix GPU pass 的视觉验证；pass 仍默认 disabled、runtime 仍 fallback_only、real_gpu_pass=false、无 GPU timestamp / FPS / 性能宣称。`grx009_segment4g_visual_fallback_ready` 为 true 后 `next_action=start_grx009_luminance_gated_real_pass_enablement`（下一片可开始设计 measured、opt-in 的 real pass enablement）；success 证据缺失/不严格时保持 `start_grx009_luminance_real_visual_diff_and_measured_fallback_telemetry` 并在 `next_action_reason` 中报告确切 blocker（`grx009_segment4g_visual_fallback_issue`）。**
> 第一段交付了可验证的 disabled/fallback wiring（bridge gate 恒回退、per-pass 设置默认 `disabled`）。第二段（见 §9）通过 patch 0003 把 Godot core Auto Exposure call site 接线到 opt-in gate：只有 module 设置开启且 bridge `rxgd_record_pass` 返回 OK 时才跳过 Godot 原生 luminance；否则执行原生路径。
> 因 bridge 对 `RXGD_PASS_LUMINANCE_REDUCTION` 恒返回 `RXGD_STATUS_FALLBACK`、per-pass 设置默认 `disabled`，实测仍走 Godot 原生 luminance 路径。
> 本文件不宣称视觉验证通过、fallback 真接入（引擎内实测）或性能提升；`segment 3b` 即使记录了 resource mapping scaffold，也不等于 runtime 可用。
> pass 默认 `disabled`；任何 compile / validation / visual / perf 失败都走 Godot 原生 luminance 路径。
> §3 对 `external/godot-master` 的调查只记录路径与函数名；Godot 侧改动只以 `spike/godot-rurix/patches/` 下的 patch 文件入库，不直接修改快照的 Godot 原生源文件。

## 1. Pass 标识

- `pass_id = luminance_reduction`
- Tier：Tier 1（低风险 pass 候选）
- 目标后端：`Godot 4.7-dev Windows D3D12 Forward+`
- 默认启用状态：`disabled`

## 2. 目标场景

- `post_fx_chain`
- `mixed_forward_plus`

（对齐 GRX-007 视觉 diff 与 GRX-004/005 benchmark 场景集合中的后处理相关场景。）

## 3. Godot 侧候选 hook / source 调查结果

仅记录路径与函数，**不改 `external/godot-master`**。

### 3.1 Effect 类

- 头文件：`servers/rendering/renderer_rd/effects/luminance.h`
- 源文件：`servers/rendering/renderer_rd/effects/luminance.cpp`
- 关键函数 / 类型：
  - `RendererRD::Luminance::luminance_reduction(...)`（`luminance.cpp:159`）——核心降采样入口。
  - `RendererRD::Luminance::LuminanceBuffers`——reduce 缓冲；`configure()`（`luminance.cpp:86`）、`free_data()`（`luminance.cpp:123`）。
  - `RendererRD::Luminance::get_current_luminance_buffer(...)`——取当前 luminance 缓冲（供 tonemap/exposure 消费）。

### 3.2 Shader

- `servers/rendering/renderer_rd/shaders/effects/luminance_reduce.glsl`（compute 路径）
- `servers/rendering/renderer_rd/shaders/effects/luminance_reduce_raster.glsl`（raster fragment 路径）

### 3.3 调用 / 注入候选点

- `servers/rendering/renderer_rd/renderer_scene_render_rd.cpp` 的 "Auto Exposure" 段：
  - `luminance->get_luminance_buffers(rb)`（`L558`）
  - `luminance->luminance_reduction(rb->get_internal_texture(), rb->get_internal_size(), ...)`（`L567`）
- Forward+ 消费点：`servers/rendering/renderer_rd/forward_clustered/render_forward_clustered.cpp` `L2439` / `L2486` 通过 `luminance->get_current_luminance_buffer(rb)` 取 exposure texture。

## 4. 输入 / 输出资源（tracked mapping scaffold）

- 输入：`hdr_internal_texture` / `p_source_texture`（HDR internal color target，第 0 级原生 source）。
- 中间：`p_luminance_buffers->reduce[i]`（`R32_SFLOAT` 分级降采样缓冲链，每级目标尺寸为 `max(source_size / 8, 1)`）。
- 输出：`p_luminance_buffers->current`（最终 1x1 luminance，函数末尾与最后一级 reduce texture 交换，供 tonemap / auto-exposure 消费）。
- tracked mapping：`spike/godot-rurix/passes/luminance_reduction/resource_mapping.md`。

以上为 segment 3b resource mapping scaffold；尚未接入真实 Rurix runtime pass。

## 5. Dispatch / level 形态（记录，未实现 runtime）

- Godot compute 路径使用 `BLOCK_SIZE = 8`，每个 shader workgroup 汇总一个 8x8 tile 到目标 luminance 像素。
- 第 0 级从 HDR source texture 采样；中间级从上一层 reduce image 读取；最终 write mode 在 `!p_set` 时读取 `current` 作为 previous luminance。
- 每级 dispatch 后 `source_size` 更新为 `max(source_size / 8, 1)`，直到 1x1。
- Rurix segment 3b 只记录 descriptor/resource mapping scaffold，不实现完整 pyramid、inter-level barrier、previous luminance feedback 或 runtime dispatch。

- `compute_reduce_pyramid`：已记录 Godot 原生形态，Rurix runtime 未实现。
- `raster_fragment_reduce`：仍为 Godot 原生备选路径，Rurix segment 3b 不替换。

## 6. Fallback

- fallback reason 枚举（对齐 GRX-008 五枚举）：
  - `compile_failed`
  - `validation_failed`
  - `unsupported_device`
  - `visual_diff_failed`
  - `manual_disabled`
- 任一 compile / validation / visual / perf 失败 → 回退到 Godot 原生 luminance 路径（`godot_native_luminance`）。

## 7. Evidence 要求

- Visual：`segment 4g` 起使用 per-pass 的 LDR absolute diff 证据（`visual_fallback_evidence.schema.json` + `ci/grx009_segment4g_visual_fallback_smoke.py`：真实 reference + candidate 帧、SHA-256 校验、probe 从磁盘字节重算 diff）；只有存在真实 reference + candidate 帧并成功计算 diff 才算数。segment 4g 的 diff 只覆盖 fallback path（证明开启恒回退的 pass 不改画面），**不是** Rurix pass 画面验证；GRX-007 `visual_diff.py` 的 7 场景 LDR per-channel diff 留给未来 real-pass slice 复用。
- Perf：复用 GRX-006 baseline / perf gate；在产出实测证据前，不得声称任何性能提升。

## 8. 出口判据

- pass 默认 `disabled`。
- 未通过 compile / validation / visual / perf 门禁前，保持 `disabled` 并 fallback 到 Godot 原路径。
- 准备阶段、第一段 gated scaffold 与第二段 core call-site wiring **均不**代表 pass 已完成；后续段落必须交付真实 GPU pass 与 strict evidence。

## 9. 实现状态（第二段 core call-site fallback wiring，2026-07-02）

第一段已落地（保持）：

- **Rust bridge gate**（`src/rurix-godot/src/lib.rs`，ABI v1 不变）：`LuminanceReductionGate` 默认 disabled；`request_enable()` 恒失败 `compile_failed`；`rxgd_record_pass` 对 `RXGD_PASS_LUMINANCE_REDUCTION` 恒返回 `RXGD_STATUS_FALLBACK`，累加 `fallback_passes`，**不**累加 estimated GPU/CPU time。
- **Godot module patch 0002**：仅改 `modules/rurix_accel/*`，新增默认 `false` 的 `rendering/rurix_accel/passes/luminance_reduction/enabled` 与 `try_record_luminance_reduction()`。

第二段本轮新增（可验证）：

- **Godot core call-site patch 0003**（`spike/godot-rurix/patches/0003-rurix-accel-luminance-core-callsite-wiring.patch`，栈式，基于 0001+0002）：
  - `drivers/d3d12/d3d12_hooks.h`：为基类 `D3D12Hooks` 新增默认返回 `false` 的 `virtual bool try_record_luminance_reduction()`，保证无 hooks singleton 或未 override 时走 Godot 原生路径。
  - `servers/rendering/renderer_rd/renderer_scene_render_rd.cpp` 的 Auto Exposure 段：在原生 `luminance->luminance_reduction(...)` 前加 opt-in 调用——只有 `D3D12Hooks::get_singleton()->try_record_luminance_reduction()` 返回 `true`（即 module 设置开启、bridge session/符号存在、`rxgd_record_pass` 返回 `RXGD_STATUS_OK`）时才跳过原生 luminance；否则必须执行原生 `luminance_reduction`。
  - `modules/rurix_accel/rurix_accel.h`：把 `try_record_luminance_reduction()` 标为 `override`。
  - 因 bridge 恒 `RXGD_STATUS_FALLBACK`、设置默认 `disabled`，opt-in gate 实测恒返回 `false`，Godot 原生 luminance 路径接管。
- **Bridge smoke**：`ci/godot_rurix_bridge_smoke.py` 现校验 0001/0002/0003 四态（base / 0001-only / 0001+0002 / 0001+0002+0003），drift 即红。
- **Probe gate**：`ci/godot_rurix_toolchain_probe.py` 的 `grx009_segment2_ready` 必须同时满足：
  - `samples/fallback_telemetry_luminance_callsite_wired_disabled_example.json` 能通过 `fallback_telemetry.py --validate-only`；
  - 共享 patch 栈检查真实落在 `0001+0002+0003`；
  - `LuminanceReductionGate` 与 0002/0003 的关键 marker 仍存在。
- **运行时语义保持不变**：即便 0003 已接线，当前 bridge 对 `RXGD_PASS_LUMINANCE_REDUCTION` 仍恒返回 `RXGD_STATUS_FALLBACK`，因此 `rxgd_record_pass` 不能把 luminance 宣告为 OK，Godot 原生 luminance 路径继续接管。

## 10. Segment 3a（离线 compile evidence，2026-07-02 起）

- 本段目标不是默认启用 runtime acceleration，而是开始真实 GPU luminance pass 的离线 kernel/package 编译取证。
- 本段计划在 `spike/godot-rurix/passes/luminance_reduction/` 下新增：
  - `rurix.toml`
  - `src/lib.rx`
  - `compile_offline.py`
  - `compile_evidence.schema.json`
  - `offline_compile_evidence.json`
  - `artifacts/` 下的真实 compile artifact 或明确失败证据
- package 草案必须贴近上游 Godot luminance 输入：
  - `servers/rendering/renderer_rd/effects/luminance.cpp`
  - `servers/rendering/renderer_rd/shaders/effects/luminance_reduce.glsl`
  - 资源/参数草案至少覆盖 `source texture or source luminance`、`dest luminance`、可选 `prev luminance` 与 `Params { source_size, max_luminance, min_luminance, exposure_adjust }`。
- `segment 3a` success 条件是离线 compile 真正产出并落盘：
  - 真实 `DXIL container` artifact（不得是 LLVM IR 文本）
  - `root signature` artifact
  - `descriptor layout` artifact
  三者都必须在 `offline_compile_evidence.json` 中可追溯，且 `runtime_state` 仍记为 `fallback_only`。
- 若离线脚本真实运行后得到 `compile_failed` 或 `validation_failed`：
  - 必须把 blocker 分类、stderr/stdout 摘要、artifact 缺失情况写入 `offline_compile_evidence.json`；
  - 这只算 blocker evidence complete，不算 ready；
  - `pass_manifest.json` 必须继续保持 `implementation_status.segment = 2` 与 `real_gpu_pass = false`。
- 当前最新结果（segment 4i canonical 离线 compile，fail-closed）：canonical `offline_compile_evidence.json` 当前 `status=compile_failed`（`blocker_category=dxil_container_missing`、`runtime_mappable=false`、`attempted_binding_kinds=[texture2d,rwtexture2d]`）。texture-capable kernel 源 `src/lib_texture.rx` 已就位（声明 `Texture2D<f32>`/`RWTexture2D<f32>`），编译器侧 forward-looking 改动已落地（`RWTexture2D<F>` lang item、`MirResourceType::RWTexture2D`、`derive_compute_bindings` 的 Texture2D/RWTexture2D 分支、`texture_target_ty`、`@llvm.dx.resource.load.texture.*`/`store.texture.*` emit、descriptor layout `binding_kind` 字段），但本机 `RURIX_LLC=H:\llvm-dxil\build\bin\llc.exe` 这版 patched llc 不支持 `llvm.dx.resource.load.texture.2d` intrinsic，texture-capable 离线 compile 失败，因此 canonical `artifacts/luminance_reduction.{dxil,rts0.bin,_descriptor_layout.json}` 路径回退为从 `artifacts/raw_buffer_historical/` 复制的 raw-buffer 字节，让 bridge `include_bytes!` 工作；manifest 顶层 `offline_compile_status=compile_failed`，`real_gpu_pass` 仍为 `false`，runtime 仍 `fallback_only`。historical raw-buffer segment 3a success（基于 `src/lib.rx` 的 raw-buffer kernel，`RURIX_LLC` + signed DXC suite `RURIX_DXC_DIR=H:\dxc-round7\extracted\bin\x64`，产出 `DXIL container`、root signature artifact 与 descriptor layout artifact，`dxv.exe` 接受该 container（`Validation succeeded`），descriptor layout 中的 `root_constant_layout` 记录 scalar root constants 的 `name/type/order/root_parameter_index/dword_offset/dword_size`，shader 声明 64-Bit integer 能力）保留在 `offline_compile_evidence_raw_buffer.json`，作为 historical fixture 与 bridge tracked package 的字节来源。当前 first blocker 是 `kernel_binding_kind_mismatch`（bridge tracked package 仍为 raw-buffer，Godot runtime 提供 Texture2D 资源）；`math_pyramid_parity_not_proven` 仅 future-only（tracked package 切到 texture-capable 后才会到达该分支）。
- 只有在真实 `DXIL container + root signature + descriptor layout` 三类 artifact 全部可追溯，且不再是 entry shell-only 时，manifest 才允许推进到 segment 3；否则不得把 blocker 写成 ready。

## 11. Segment 3b（resource mapping scaffold，2026-07-03）

- 本段目标是把 Godot luminance reduction 的真实资源/参数映射写入 Rurix bridge 设计，不启用 runtime acceleration。
- 新增 tracked mapping 文档：`spike/godot-rurix/passes/luminance_reduction/resource_mapping.md`。
- descriptor scaffold：
  - root constants / root-cbuffer binding：`b0 space0`，字段为 `source_width`、`source_height`、`max_luminance`、`min_luminance`、`exposure_adjust`。
  - SRV：`src_luminance = t0 space0`。
  - UAV：`dst_luminance = u0 space0`。
  - 当前 root constant layout 为 7 DWORD / 28 bytes。
- 64-bit integer shader capability 必须在目标 D3D12 device 上 gate；cap 未确认时必须 fallback。
- `src/rurix-godot` 只加入内部 mapping scaffold 校验，`RXGD_ABI_VERSION` 保持 v1；未改公开 C ABI layout 或导出签名。
- 新增 `spike/godot-rurix/patches/0004-rurix-accel-luminance-resource-mapping-scaffold.patch`，仅承载 module/bridge 级 mapping scaffold marker，不直接修改或跟踪 `external/godot-master`。
- fallback-first 语义保持：任一资源缺失、cap 不支持、ABI mismatch、descriptor 不一致均返回 fallback；`rxgd_record_pass` 不得对 `RXGD_PASS_LUMINANCE_REDUCTION` 返回 OK，Godot 原生 luminance 路径继续接管。
- 本段不产生 real visual diff、measured fallback telemetry 或性能证据。

## 12. Segment 4a（runtime binding preflight，2026-07-04）

- 本段目标是把 segment 3b 的 resource mapping scaffold 接到 runtime binding 前置校验层：Godot/Rurix 双侧能传递并校验 luminance 资源与 push constants，但 runtime 保持 `fallback_only`，不录制真实 D3D12 dispatch，不跳过 Godot 原生 luminance path。
- 新增 `spike/godot-rurix/patches/0005-rurix-accel-luminance-runtime-binding-preflight.patch`，栈在 0001+0002+0003+0004 之后：
  - `drivers/d3d12/d3d12_hooks.h`：`try_record_luminance_reduction()` 演进为带 luminance 参数的签名（source/dest 的 logical RID id 与尺寸 + `max_luminance`/`min_luminance`/`exposure_adjust`），默认实现仍返回 `false`。
  - `servers/rendering/renderer_rd/renderer_scene_render_rd.cpp` Auto Exposure call site：传入 `rb->get_internal_texture()` 与 `rb->get_internal_size()`、`luminance_buffers->reduce[0]` 与其 `max(source / 8, 1)` 尺寸、min/max sensitivity 与 exposure adjust step；gate 返回 `false` 时原生 `luminance_reduction` 照常执行。
  - `modules/rurix_accel`：组装两个 `RXGD_RESOURCE_TEXTURE` 记录（`src_luminance = t0` 在前、`dst_luminance = u0` 在后；`native_handle` 为 logical Godot RID id，非 D3D12 GPU handle）与 28 字节 `b0` root constant block（`source_width`/`source_height` 为 64-bit 整数 + 三个 f32），调 `rxgd_record_pass`；非 `RXGD_STATUS_OK` 一律返回 `false` 走原生路径。
- **Rust bridge preflight**（`src/rurix-godot/src/lib.rs`，C ABI v1 不变）：`record_runtime_binding_preflight` 依序校验 64-bit integer shader capability、2-resource descriptor 形态、28-byte push constant 尺寸、texture 资源类型、b0 中 source dimensions 非零且与 `src_luminance` 资源一致、`dst_luminance` 满足 `max(source / 8, 1)` level-0 reduce 形态。任一失败返回 `RXGD_STATUS_FALLBACK` 并记录 fallback reason（`validation_failed` / `unsupported_device`），不累计 estimated GPU/CPU time；**全部通过也仍返回 `RXGD_STATUS_FALLBACK`**（gate 恒 disabled，无真实 dispatch 通路）。
- 0005 的可叠加性校验不得污染 `external/godot-master`：`ci/godot_rurix_patch_stack.py` 的 `evaluate_stacked_patch_applyability` 在临时 scratch 副本中真实 apply 0004 后对 0005 执行 `git apply --check`；bridge smoke 与 toolchain probe 均纳入该检查（`grx009_patch_0005_applyable`）。
- probe 新增 `grx009_segment4a_runtime_binding_preflight_ready`；ready 只代表 preflight/fallback-only 就绪，`next_action` 指向 gated dispatch bring-up（`start_grx009_luminance_segment4b_gated_dispatch_bringup`），不指向 visual/perf。
- 本段不产生 real visual diff、measured fallback telemetry 或性能证据。

## 13. Segment 4b（gated dispatch bring-up，2026-07-04）

- 本段目标是建立第一个"显式 opt-in、失败即 fallback"的 D3D12 dispatch bring-up 接线通路，但 runtime 仍保持 `fallback_only`，默认 disabled，不录制真实 D3D12 dispatch，不宣称 visual/perf/measured telemetry。
- 新增 `spike/godot-rurix/patches/0006-rurix-accel-luminance-gated-dispatch-bringup.patch`，栈在 0001+0002+0003+0004+0005 之后（可叠加性经 scratch copy 校验，不污染 `external/godot-master`）：
  - `modules/rurix_accel/register_types.cpp`：新增默认 `false` 的 per-pass 设置 `rendering/rurix_accel/passes/luminance_reduction/dispatch_bringup`（与既有 `.../enabled` 分离，dispatch bring-up 必须显式 opt-in）。
  - `modules/rurix_accel/rurix_accel.h`：声明保留能力 flag `RXGD_CAP_LUMINANCE_DISPATCH_BRINGUP (1u << 1)`，承载在既有 `RxGdCaps.flags` 字段里；**不改 C ABI struct layout，`RXGD_ABI_VERSION` 保持 1**。
  - `modules/rurix_accel/rurix_accel.cpp` `try_create_session()`：仅当 `.../dispatch_bringup` 设置开启时才把 `RXGD_CAP_LUMINANCE_DISPATCH_BRINGUP` 置入 `caps.flags`。
- **Rust bridge dispatch bring-up gate**（`src/rurix-godot/src/lib.rs`，C ABI v1 不变）：`rxgd_record_pass` 对 `RXGD_PASS_LUMINANCE_REDUCTION` 改走 `record_gated_dispatch_bringup`：
  1. 先执行完整 segment 4a runtime binding preflight（`check_runtime_binding_preflight`）；
  2. 再执行 dispatch eligibility（`check_dispatch_eligibility`）：Godot opt-in flag `RXGD_CAP_LUMINANCE_DISPATCH_BRINGUP`、64-bit integer 能力、native D3D12 device/queue handle 非空、两个 resource `native_handle` 非空、compiled package 可用且 descriptor layout（2 resources、28-byte b0 root constants、SRV `t0`、UAV `u0`、64-bit integer 要求）与离线 compile evidence 的 DXIL/root signature/descriptor layout SHA-256 摘要匹配（`LuminanceDispatchPackage::verify_matches_offline_evidence`）；
  3. 任一失败返回 `RXGD_STATUS_FALLBACK` 并记录清晰 fallback reason（`manual_disabled` / `unsupported_device` / `validation_failed` / `compile_failed`），不累计 estimated GPU/CPU time；
  4. 即便全部 eligible，且 `segment 4c` 已有 standalone measured D3D12 dispatch smoke，`request_dispatch_bringup` 仍恒失败：因为没有 bridge-linked runtime dispatch recording path、没有 measured bridge telemetry，且不能让 `rxgd_record_pass` 返回 OK，explicit dispatch gate 保持关闭，仍返回 `RXGD_STATUS_FALLBACK`，`enabled` 保持 false，不录制 dispatch，不累计时间。
- **单元测试红绿**（`cargo test -p rurix-godot`）：disabled fallback（opt-in flag 未置）、missing native device/queue/resource handle fallback、layout/hash mismatch fallback（tampered package）、valid eligibility 仍受 explicit dispatch gate 控制（返回 FALLBACK、`enabled=false`、无 GPU/CPU 时间）；fake/null handle 一律不得返回 `RXGD_STATUS_OK`。
- **probe/smoke**：新增 `grx009_patch_0006_applyable` 与 `grx009_segment4b_gated_dispatch_bringup_ready`；ready 依赖 manifest 4b 字段、0006 scratch 可叠加性、bridge/patch 关键 marker 与 `cargo test` 红绿，而非仅脚本存在。
- **real local D3D12 dispatch smoke**：见 §14（segment 4c）。segment 4b 本身不含 device harness；4c 补齐真实 device smoke evidence，但 bridge 仍不录制 dispatch、不返回 OK。
- 本段不产生 real visual diff、measured fallback telemetry、真实 bridge D3D12 dispatch 录制或性能证据。

## 14. Segment 4c（real D3D12 dispatch smoke，2026-07-04）

- 本段目标是产出**可复核的 real Windows D3D12 dispatch smoke evidence**：证明 tracked 的离线 luminance DXIL container + RTS0 root signature + descriptor layout artifacts 能在**真实 D3D12 device / command queue** 上完成一次最小 compute dispatch。本段**只**产出 smoke evidence，不把 Godot runtime pass 标记为完成、不让 bridge 默认返回 OK、不宣称 visual/perf/measured telemetry。
- 新增 harness：`ci/grx009_luminance_d3d12_dispatch_smoke.py`（内联 C++/MSVC D3D12 harness，按 `ci/dxil_binding_device_smoke.py` 既有范式）。
  - device / command queue 恒为真实对象：不接受 fake/null handle。无硬件 D3D12 adapter、无 D3D12 runtime、device 无 64-bit integer（`Int64ShaderOps`）能力、缺签名 DXC suite（`dxil.dll`）或缺 MSVC 时记 `status=skip` 并写具体原因；SKIP 不推进 ready。
  - 直接用 tracked artifacts：先重算 DXIL / RTS0 / descriptor layout 的 SHA-256，必须与 segment 3a `offline_compile_evidence.json` 摘要**逐字节匹配**；descriptor layout 必须与当前 resource mapping 一致（2 resources `src_luminance=t0` SRV + `dst_luminance=u0` UAV、28-byte `b0` root constants、64-bit integer 要求）。任一 hash/layout 不匹配记 `status=fail` 并保留错误信息。
  - SRV(t0)/UAV(u0)/`b0` root constants 严格按 descriptor layout 绑定，不猜测资源形态；root signature 直接由 Rurix RTS0 字节 `CreateRootSignature`（device-parse）；compute PSO 由 tracked DXIL container 创建。
  - DXIL 加载：编译器产出的 DXIL container 仅经 dxv 校验、缺 runtime 需要的 validation hash；harness 用签名 DXC 的 `dxil.dll` validator 对**内存副本** in-place 签名（不改 shader 语义、不改 tracked 文件字节），使其在非 Developer-Mode device 上可创建 compute PSO。
  - 执行一次最小 dispatch、`Signal`+等待 fence 完成、readback dst UAV，并对 dst 字节做 checksum 作为完成/输出验证。
  - 输出 tracked evidence：`spike/godot-rurix/passes/luminance_reduction/real_d3d12_dispatch_smoke.json`。
- evidence 三态：
  - `status=success`：记录 adapter/device 信息、artifact hashes、dispatch dimensions、fence completion value、dst readback checksum，以及逐项 `checks`。
  - `status=skip`：无 adapter / 无 runtime / 无 64-bit integer 能力 / 缺签名 DXC / 缺 MSVC；写具体原因；不推进 ready。
  - `status=fail`：PSO / root signature / descriptor / resource / readback 任一失败，或 artifact hash / descriptor layout 不匹配；保留 D3D12 错误码与日志。
- 当前最新结果（2026-07-04）：本机 `NVIDIA GeForce RTX 4070 Ti` 上 `status=success`（RTS0 accept、compute PSO from tracked DXIL、SRV t0/UAV u0/b0 绑定、`Dispatch(1,1,1)`、fence 完成、dst UAV readback）。
- probe/smoke：新增 `grx009_real_d3d12_dispatch_smoke_ready`。ready 只在 segment 4b 已 ready **且** `real_d3d12_dispatch_smoke.json` 记 `status=success`、其记录的 artifact 摘要与磁盘 artifacts 及离线 evidence 摘要仍匹配、`checks` 全绿、`real_gpu_pass=false`、`runtime_state=fallback_only` 时成立。smoke 缺失 / SKIP / FAIL 时 `next_action` 保持 `provide_grx009_luminance_real_d3d12_dispatch_smoke`；success 后 `next_action` 指向 `start_grx009_bridge_real_d3d12_dispatch_recording`（bridge real dispatch recording slice，而非 visual/perf）。
- 运行时语义保持不变：即便 smoke `status=success`，manifest 仍 `runtime_state=fallback_only`、`real_gpu_pass=false`、`real_d3d12_dispatch_recorded=false`，`segment_detail` 仍为 `4b_gated_dispatch_bringup`；`rxgd_record_pass` 对 `RXGD_PASS_LUMINANCE_REDUCTION` 仍恒返回 `RXGD_STATUS_FALLBACK`、不录制 dispatch、不累计 estimated GPU/CPU time。Godot 原生 luminance 路径继续接管。在 bridge real dispatch recording slice 落地并有 measured telemetry 前，任何 dispatch 都不得返回 OK。
- 本段不产生 real visual diff、measured fallback telemetry 或性能证据。

## 15. Segment 4d（bridge real D3D12 dispatch recording smoke，2026-07-04）

- 本段目标是产出**可复核的 bridge real D3D12 dispatch recording smoke evidence**：证明 Rurix Godot bridge（`rurix_godot.dll`）能经 C ABI 在**真实 D3D12 device / command queue** 上录制一次最小 luminance compute dispatch。与 segment 4c 的 **standalone** device smoke（自建 device、完全不经 bridge）不同，本段真正驱动 bridge 的 `rxgd_create_d3d12_session` / `rxgd_record_pass` / `rxgd_collect_timestamps`。本段**只**产出 bridge smoke evidence，**不**默认启用 Godot luminance Rurix path、**不**完成 Godot runtime pass、**不**宣称 visual/perf/GPU-timestamp/measured fallback telemetry。
- **默认关闭的 bridge 录制路径**（`src/rurix-godot`）：
  - 新增 cargo feature `d3d12-recording-shim`（默认关闭）。启用时 `build.rs` 经 `cc` 编译 Windows-only C++ D3D12 shim `shim/rxgd_luminance_record.cpp` 并链接 Windows SDK 的 `d3d12`/`dxgi`（对齐 `src/uc04-demo` 既有先例，不手搓大段 COM vtable）。
  - **默认 `cargo test -p rurix-godot` 不需要 Windows SDK/D3D12 link**，且 `rxgd_record_pass` 对 `RXGD_PASS_LUMINANCE_REDUCTION` 保持 `RXGD_STATUS_FALLBACK`。
  - 新增 harness-only 保留能力 flag `RXGD_CAP_LUMINANCE_DISPATCH_RECORD (1u << 2)`，承载在既有 `RxGdCaps.flags`（**不改 C ABI struct layout，`RXGD_ABI_VERSION` 保持 1**）。截至 segment 4d/4e，Godot module 从不设置该 flag（**自 segment 4f 起口径更正**：module 仅在默认关闭的 harness-only `.../dispatch_recording_smoke` opt-in 显式开启时才设置该 flag，见 §17；默认 config 与 shipping/feature-off bridge 仍从不设置）。
  - 新增导出 `rxgd_dispatch_recording_shim_available()`：feature 开启时返回 1，否则 0。
  - tracked 离线 DXIL/RTS0 以 `include_bytes!` 内嵌 bridge,录制前用自带 SHA-256 重算并与离线 evidence 摘要逐字节核对（不匹配即 fallback）。shim 在**内存副本**上用签名 DXC 的 `dxil.dll` in-place 签名（不改磁盘 artifact 字节）以在非 Developer-Mode device 上创建 compute PSO。
  - `rxgd_record_pass` 只有在:feature 编入 **且** record-arm flag 置位 **且** dispatch bring-up opt-in + 64-bit integer cap + 非空真实 device/queue/resource handles + eligibility(layout/摘要匹配) 全部满足 **且** 内嵌 artifact 摘要匹配时,才调 shim 录制真实 dispatch 并返回 `RXGD_STATUS_OK`(`recorded_passes+=1`、累计 measured `cpu_record_ns`、`gpu_time_ns` 保持 0)。任一不满足即 `RXGD_STATUS_FALLBACK`。unit test 从不置 record-arm flag,故 feature 开启下仍恒 fallback,不会用 fake/null handle 触发 shim。
- 新增 harness：`ci/grx009_luminance_bridge_recording_smoke.py`（内联 C++/MSVC D3D12 harness）。
  - 创建真实 D3D12 device/queue 与真实 src(8x8 R32F,填 1.0,置于 `NON_PIXEL_SHADER_RESOURCE`)、dst(1x1 R32F UAV,置于 `UNORDERED_ACCESS`);fake/null handle 一律不接受。
  - `cargo build -p rurix-godot --features d3d12-recording-shim` 产出 DLL(RURIX_DXC_DIR 传给 `build.rs` 以定位 `dxcapi.h`),`LoadLibrary` 该 DLL,`GetProcAddress` 取 C ABI,置 `RXGD_CAP_SHADER_INT64 | RXGD_CAP_LUMINANCE_DISPATCH_BRINGUP | RXGD_CAP_LUMINANCE_DISPATCH_RECORD`,以真实 `ID3D12Resource*` 作 `native_handle` 调 `rxgd_record_pass(RXGD_PASS_LUMINANCE_REDUCTION)`,期望返回 `RXGD_STATUS_OK`。
  - 输出 tracked evidence:`spike/godot-rurix/passes/luminance_reduction/bridge_dispatch_recording_evidence.json`。
- evidence 三态：
  - `status=success`：`bridge_recorded_d3d12_dispatch=true`;记录 adapter/device、artifact hashes、bridge dispatch dimensions、fence completion、dst readback checksum、bridge frame stats(`recorded_passes`/`fallback_passes`/`last_error`/`cpu_record_ns`),以及逐项 `checks`。`godot_runtime_luminance_path_enabled=false`、`default_enable_state=disabled`、`gpu_timestamp_status=not_yet`(`gpu_time_ns` 不伪造,保持 0/null)。
  - `status=skip`：无 adapter / 无 runtime / 无 64-bit integer 能力 / 缺签名 DXC / 缺 MSVC;写具体原因;不推进 ready。
  - `status=fail`：DLL 构建/加载失败、缺 C ABI 符号、`rxgd_record_pass` 未返回 OK、bridge frame stats 不一致、artifact hash / descriptor layout 不匹配。
- 当前最新结果（2026-07-04）：本机 `NVIDIA GeForce RTX 4070 Ti` 上 `status=success`（`rxgd_record_pass` 返回 `RXGD_STATUS_OK`、`recorded_passes=1`、`fallback_passes=0`、`gpu_time_ns=0`、`dispatch=1,1,1`、`fence=1`、`dst=1x1`、`checksum=0x4b95f515`、`dxil_signed=yes`）。
- probe/smoke：新增 `grx009_bridge_real_d3d12_dispatch_recording_ready`。ready 只在 segment 4c smoke 已 ready **且** `bridge_dispatch_recording_evidence.json` 记 `status=success`、`bridge_recorded_d3d12_dispatch=true`、`godot_runtime_luminance_path_enabled=false`、`default_enable_state=disabled`、`gpu_timestamp_status=not_yet`、`real_gpu_pass=false`、`runtime_state=fallback_only`、其记录的 artifact 摘要与磁盘 artifacts 及离线 evidence 摘要仍匹配、`checks` 与 bridge frame stats 全绿时成立。ready 后 `next_action=start_grx009_godot_native_resource_handle_mapping`；bridge smoke 缺失/SKIP/FAIL 时保持 `start_grx009_bridge_real_d3d12_dispatch_recording`。
- **Evidence artifact hygiene（historical run artifact）**：`bridge_dispatch_recording_evidence.json` 是**某一次历史 measured run** 的产物,不是持续复算的实时状态。
  - smoke 在 feature build 成功后记录当次 feature-built DLL 的指纹 `dll_fingerprint`：`dll_path_at_run`、`dll_sha256`、`dll_size_bytes`、`dll_mtime_utc`、`build_profile=debug`、`features=["d3d12-recording-shim"]`,并把该 DLL 复制到 `target/grx009_bridge_recording_smoke/rurix_godot_d3d12_recording_shim.dll`(记 `snapshot_dll_path`/`snapshot_dll_sha256`;二进制在 `target/` 下,gitignored,**不纳入 Git**)。
  - `target/debug/rurix_godot.dll` 是 **mutable build artifact**：后续 feature-off 的 `cargo build -p rurix-godot` 会**原地覆盖**它,使其 hash 不再等于 evidence 记录的 feature-built DLL 指纹。
  - 因此 4d readiness **不**把当前 `target/debug/rurix_godot.dll` hash 纳入 gate:evidence 是历史 measured 证据,当前 DLL 被覆盖不代表历史 run 失效,`ready` 保持 `true`。
  - probe 输出 `grx009_bridge_recording_evidence_dll_sha256`(历史指纹)、`grx009_bridge_recording_current_dll_sha256`(当前磁盘 DLL)、`grx009_bridge_recording_current_dll_matches_evidence`(是否匹配);不匹配时追加 warning,提示重跑 `ci/grx009_luminance_bridge_recording_smoke.py` 可刷新当前 artifact 指纹并精确复现当次 feature-built DLL。
- 运行时语义保持不变：`segment_detail` 仍为 `4b_gated_dispatch_bringup`，manifest `real_d3d12_dispatch_recorded`（默认 Godot runtime 路径口径）仍 `false`；shipping（feature-off）bridge 对 `RXGD_PASS_LUMINANCE_REDUCTION` 仍恒返回 `RXGD_STATUS_FALLBACK`。Godot 原生 luminance 路径继续接管。**除非** Godot runtime path、visual diff、telemetry gate 同时完成，否则 `real_gpu_pass` 不改为 true。
- 本段不产生 real visual diff、measured fallback telemetry 或性能证据。

## 16. Segment 4e（native D3D12 resource handle mapping，2026-07-04）

- 本段目标是把 Godot runtime luminance pass 传给 bridge 的资源从 logical RID id 改成真实 D3D12 native handle（真实 `ID3D12Resource*`）。本段**只**完成 native handle mapping + preflight/evidence，**不**启用默认 Rurix luminance runtime pass、**不**让 shipping bridge 返回 OK、**不**宣称 real GPU pass、visual diff、telemetry 或性能提升。
- 新增 `spike/godot-rurix/patches/0007-rurix-accel-luminance-native-resource-handle-mapping.patch`，栈在 0001+0002+0003+0004+0005+0006 之后（可叠加性经 scratch copy 校验，不污染 `external/godot-master`）：
  - `drivers/d3d12/d3d12_hooks.h`：`try_record_luminance_reduction()` 的 source/dest 参数由 logical RID id（`p_source_texture_id` / `p_dest_texture_id`）改名为真实 native handle（`p_source_native_handle` / `p_dest_native_handle`），注释说明 caller 现在传真实 `ID3D12Resource*`（经 `RenderingDevice::get_driver_resource` 取得），非 RID id。
  - `servers/rendering/renderer_rd/renderer_scene_render_rd.cpp` Auto Exposure call site：用 `RenderingDevice::get_driver_resource(DRIVER_RESOURCE_TEXTURE, RID, 0)` 取得 `rb->get_internal_texture()`（source）与 `luminance_buffers->reduce[0]`（level-0 reduce dest）的真实 D3D12 native handle；`RenderingDevice` 不可用或任一 native handle 为 0 时 fallback 到 Godot 原生 luminance path。
  - `modules/rurix_accel`：`RurixAccelD3D12Hooks` override 同步改名；新增 native handle 为 0 的 guard（返回 false 走原生路径）；两个 `RXGD_RESOURCE_TEXTURE` 记录的 `RxGdResource.native_handle` 现在承载真实 `ID3D12Resource*` 而非 logical RID id。
- **底层接缝**：`RenderingDevice::get_driver_resource(DRIVER_RESOURCE_TEXTURE, RID)` 内部经 `RenderingDeviceDriverD3D12::get_resource_native_handle(DRIVER_RESOURCE_TEXTURE, TextureID)` 返回 `ID3D12Resource*`。Godot 改动仅以 patch 入队，`external/godot-master` 不纳入 Git。
- **不改 ABI / 不启用默认 pass**：`RXGD_ABI_VERSION` 保持 1；per-pass 设置仍默认 `disabled`；Godot module 截至本段（4e）**仍不**设置 `RXGD_CAP_LUMINANCE_DISPATCH_RECORD`（harness-only record-arm flag；**自 segment 4f 起口径更正**：module 仅在默认关闭的 `.../dispatch_recording_smoke` opt-in 显式开启时才设置，见 §17）；shipping（feature-off）bridge 对 `RXGD_PASS_LUMINANCE_REDUCTION` 仍恒返回 `RXGD_STATUS_FALLBACK`，Godot 原生 luminance 路径继续接管。
- **只完成 preflight，不是 runtime dispatch**：本段把 Godot runtime 传入的资源改为真实 native handle，但**没有**由 Godot runtime 驱动 bridge 录制真实 D3D12 dispatch（那属于后续 segment）。manifest 保持 `runtime_state=fallback_only`、`real_gpu_pass=false`、`real_d3d12_dispatch_recorded=false`、`godot_runtime_luminance_path_enabled=false`、`default_enable_state=disabled`。
- **probe/smoke**：新增 `grx009_patch_0007_applyable` 与 `grx009_segment4e_native_resource_handle_mapping_ready`。ready 依赖 segment 4d bridge recording smoke 已 ready、manifest 4e 字段、0007 scratch 可叠加性与 0007/callsite 关键 marker，而非仅脚本存在。ready 后 `next_action=start_grx009_godot_runtime_bridge_dispatch_recording_smoke`；0007 缺失/不可叠加时保持 `fix_grx009_luminance_segment4e_patch_0007_applyability`。
- 本段不产生 real visual diff、measured fallback telemetry、真实 Godot-runtime-驱动 D3D12 dispatch 录制或性能证据。

## 17. Segment 4f（Godot-runtime bridge dispatch recording smoke，2026-07-05）

- 本段目标是产出**可复核的 Godot-runtime bridge D3D12 dispatch recording smoke evidence**：证明 **patched Godot runtime luminance call site**（segment 4e 接线的 Auto Exposure 路径）能用它经 `RenderingDevice::get_driver_resource` 解析出的真实 `ID3D12Resource*` native handle，从 **Godot runtime**（而非 segment 4d 那种 bare C++ harness）驱动 bridge 录制一次真实 luminance compute dispatch。本段**只**产出 measured Godot-runtime smoke evidence，**不**默认启用 Godot luminance Rurix path、**不**完成 Godot runtime pass、**不**让 shipping/feature-off bridge 返回 OK、**不**宣称 real GPU pass、visual diff、measured telemetry、GPU timestamp 或性能提升。
- 新增 `spike/godot-rurix/patches/0008-rurix-accel-luminance-godot-runtime-bridge-recording-smoke.patch`，栈在 0001+0002+0003+0004+0005+0006+0007 之后（可叠加性经 scratch copy 校验，不污染 `external/godot-master`）：
  - `modules/rurix_accel/register_types.cpp`：新增默认 `false` 的 per-pass 设置 `rendering/rurix_accel/passes/luminance_reduction/dispatch_recording_smoke`（harness-only opt-in，与既有 `.../enabled`、`.../dispatch_bringup` 分离，dispatch recording smoke 必须显式 opt-in）。
  - `modules/rurix_accel`：**只有当该 `.../dispatch_recording_smoke` opt-in 开启时**，`try_create_session()` 才把 harness-only record-arm flag `RXGD_CAP_LUMINANCE_DISPATCH_RECORD` 置入 `caps.flags`（承载于既有 `RxGdCaps.flags`，**不改 C ABI struct layout，`RXGD_ABI_VERSION` 保持 1**）。默认（opt-in 关闭）时 module **不**设置该 flag。
  - Godot runtime luminance call site 在真实驱动一次 bridge 录制（rc == OK）后打印独有 marker `RXGD_GODOT_RUNTIME_LUMINANCE_RECORD ... recorded=1 pass=...`，供 smoke 判定是 **Godot runtime**（而非 bare harness）驱动了录制。
- **口径更正（相对 segment 4d/4e）**：segment 4d/4e 时期"Godot module 从不设置 `RXGD_CAP_LUMINANCE_DISPATCH_RECORD`"的旧口径在本段起被更正——segment 4f 引入 harness-only 的 `.../dispatch_recording_smoke` opt-in 后，Godot module **可以**设置该 record-arm flag，但**仅**在该默认关闭的 opt-in 显式开启时（test-only 录制取证用）。默认 Godot config（opt-in 关闭）与 shipping/feature-off bridge **仍**不设置该 flag、对 `RXGD_PASS_LUMINANCE_REDUCTION` 仍恒返回 `RXGD_STATUS_FALLBACK`。
- 新增 harness：`ci/grx009_godot_runtime_bridge_recording_smoke.py`。
  - 前置（缺任一记 `status=skip`，不推进 ready）：一个经 **FULL 0001..0008 patch stack** 重建的 Godot console exe（叠加在 ignored `external/godot-master` 快照之上、以 `module_rurix_accel_enabled=yes d3d12=yes` 重编）——tracked 的 `external/godot-master` 构建只含 0001+0002+0003，**不得**复用，caller 必须把 `RURIX_GRX009_SEGMENT4F_GODOT_EXE` 指向 full-stack console exe；含 `dxil.dll` 的签名 DXC suite 与 MSVC vcvars64；具 64-bit integer shader 能力的真实 D3D12 adapter。scratch build/artifacts 位于 ignored `target/grx009_segment4f_godot_build/`（不纳入 Git）。
  - `cargo build -p rurix-godot --features d3d12-recording-shim` 产出录制 shim DLL，生成一个最小 Godot 工程（开启 `.../enabled` + `.../dispatch_bringup` + 默认关闭的 `.../dispatch_recording_smoke` opt-in、tonemap + auto exposure 的场景），以 `--rendering-driver d3d12 --rendering-method forward_plus` 运行数帧后退出。
  - 输出**两个** tracked evidence，区分 latest 与 historical success：
    - `godot_runtime_bridge_recording_evidence.json` —— **latest 证据**，每次运行都改写，诚实可复现：未设 `RURIX_GRX009_SEGMENT4F_GODOT_EXE`（scratch Godot exe env var）时记 `status=skip`。它**本身不推进** readiness gate。
    - `godot_runtime_bridge_recording_success_evidence.json` —— **historical measured success 证据**，**仅**在严格 `status=success` 运行时写入/更新（记录 Godot exe fingerprint、0001..0008 patch stack identity、feature-built DLL fingerprint、artifact hashes、`godot_exit_code_zero=true`、marker `recorded=1`，并注明 scratch build 二进制不入 Git）。之后的 SKIP/FAIL 运行**绝不**删除或覆盖它。segment 4f readiness gate 只看此文件，因此 latest 回落为 reproducible-default SKIP 时，只要曾经录得一次 measured success，readiness 不回退。
- latest evidence 三态：
  - `status=success`：`godot_runtime_bridge_recorded_dispatch=true`；观察到 `RXGD_GODOT_RUNTIME_LUMINANCE_RECORD` marker 且 `recorded=1`，**且 Godot 进程 `exit_code == 0`**（`checks.godot_exit_code_zero=true`）。记录 adapter/session、artifact hashes、marker 字段与逐项 `checks`。即便 success 仍保持 `runtime_state=fallback_only`、`real_gpu_pass=false`、`real_d3d12_dispatch_recorded=false`、`godot_runtime_luminance_path_enabled=false`、`default_enable_state=disabled`、`gpu_timestamp_status=not_yet`（`gpu_time_ns` 不伪造）。
  - `status=skip`：缺 full-stack Godot exe / 缺签名 DXC / 缺 MSVC / session unavailable / 未观察到 marker / 超时；写具体原因；不推进 ready。
  - `status=fail`：artifact hash / descriptor layout 不匹配、DLL 构建失败、marker present 但 `recorded != 1`，**或 marker present 且 `recorded=1` 但 Godot 进程 `exit_code != 0`**（marker 出现但进程非零退出即 FAIL，绝不 success）。fake/null handle 绝不返回 OK。
- probe/smoke：`grx009_patch_0008_applyable`、`grx009_segment4f_inputs_ready`、`grx009_segment4f_godot_runtime_bridge_recording_ready`。segment 4f ready 只在 segment 4e 已 ready **且 historical success 证据** `godot_runtime_bridge_recording_success_evidence.json`（**不是** reproducible-default SKIP 的 latest 文件）记 `status=success`、`godot_runtime_bridge_recorded_dispatch=true`、discipline flag（`runtime_state=fallback_only`/`real_gpu_pass=false`/`real_d3d12_dispatch_recorded=false`/`godot_runtime_luminance_path_enabled=false`/`default_enable_state=disabled`/`gpu_timestamp_status=not_yet`）全部保持、artifact 摘要与磁盘/离线 evidence 仍匹配、`checks`（含 `godot_exit_code_zero`）与 `recording.recorded=1` 全绿时成立。probe 同时报告 latest smoke status（`grx009_segment4f_godot_runtime_bridge_recording_latest_status`，可为 reproducible-default SKIP）与 historical success readiness（`..._success_status`）。ready 后 `next_action=start_grx009_luminance_real_visual_diff_and_measured_fallback_telemetry`；success 证据缺失（含 latest 为 SKIP/FAIL、historical success 从未录得）时保持 `start_grx009_godot_runtime_bridge_dispatch_recording_smoke`；0008 不可叠加时保持 `fix_grx009_luminance_segment4f_patch_0008_applyability`。stale/hash 不匹配/被篡改的 success 证据不推进 readiness。
- 运行时语义保持不变：即便 smoke `status=success`，manifest 仍 `runtime_state=fallback_only`、`real_gpu_pass=false`、`real_d3d12_dispatch_recorded=false`（默认 Godot runtime 路径口径）；默认 Godot config 与 shipping/feature-off bridge 对 `RXGD_PASS_LUMINANCE_REDUCTION` 仍恒返回 `RXGD_STATUS_FALLBACK`，Godot 原生 luminance 路径继续接管。**除非** real visual diff、measured fallback telemetry、visual/perf gate 同时完成，否则 `real_gpu_pass` / 默认 pass 不改。
- 本段不产生 real visual diff、measured fallback telemetry、GPU timestamp 或性能证据。

## 18. Segment 4g（real visual diff + measured fallback telemetry gate，2026-07-05）

- 本段目标是落地**首个 real（非 placeholder）visual diff + measured fallback telemetry gate**：用实测证据证明（a）luminance_reduction 的 fallback path 在运行时真被走到，（b）开启这个恒回退的 pass 不会改变渲染画面。本段是 gate/scaffold 任务：**不**启用 real GPU pass、**不**改 `real_gpu_pass`/`real_d3d12_dispatch_recorded`、**不**做任何性能/FPS 宣称，也**不是** Rurix GPU pass 的画面验证（两帧均由 Godot 原生 luminance 路径渲染）。
- 新增 harness：`ci/grx009_segment4g_visual_fallback_smoke.py`。
  - 使用 **tracked** `external/godot-master` console build（patch stack 0001+0002+0003、`module_rurix_accel_enabled=yes d3d12=yes`；可经 `RURIX_GRX009_SEGMENT4G_GODOT_EXE` 覆盖）。本段测量 default / enabled-fallback 行为，0001+0002+0003 已完整承载，无需 full-stack scratch 重建。
  - bridge 为 **shipping feature-OFF** `cargo build -p rurix-godot` 产物（无 `d3d12-recording-shim`；evidence `dll_fingerprint.features` 必须为空数组）。
  - **pass enable matrix 两条腿**（同一确定性 flat-color + tonemap + auto-exposure 场景、`--rendering-driver d3d12 --rendering-method forward_plus --fixed-fps 60 --verbose`、固定第 24 帧经 `frame_post_draw` 抓帧）：
    - `disabled_default`（reference）：`rendering/rurix_accel/passes/luminance_reduction/enabled=false`（默认），call site 不得调 bridge，patch 0002 fallback marker **不得**出现；
    - `enabled_fallback`（candidate）：`.../enabled=true`，Auto Exposure call site 调 shipping bridge，bridge 回退，运行**必须**实测打印 patch 0002 marker `RurixAccel: luminance_reduction fallback rc=`（fallback path observed 的 measured 信号），双腿都必须 session ready 且 `exit_code == 0`。
  - **LDR absolute diff**：两帧 raw `R8G8B8_raw`（256x144）逐字节 |reference − candidate|，pinned 阈值 `max_abs<=2`、`mean_abs<=0.25`；超阈值即 `status=fail`（诚实记录实测数字）。tracked frame artifacts（`.rgb8` + 人眼可看的 `.png`）与 diff artifact 落盘在 `artifacts/visual/` 并入库，evidence 逐一 hash-pin。
  - 生成 GRX-008 格式 `measured_fallback_telemetry.json`（`run_mode=full` / `evidence_level=measured_local`、`luminance_reduction` 条目 `enable_state=enabled`、`fallback_reason=validation_failed`（0002 级 module 调用不带 resource binding，bridge preflight 按构造记 validation_failed）、`godot_fallback_active=true`、真实 timestamp/frame），并在发布前自验 `fallback_telemetry.py --validate-only`。
- **两份 tracked evidence（镜像 segment 4f 的 latest/success 卫生）**：
  - `visual_fallback_evidence.json` —— latest，每次运行改写；tracked exe 缺失时诚实记 `status=skip`（`RURIX_REQUIRE_REAL=1` 时 SKIP 升级为 FAIL），本身不推进 readiness。
  - `visual_fallback_success_evidence.json` —— historical measured success，仅严格 `status=success` 时写入，之后 SKIP/FAIL 运行绝不删除或覆盖；segment 4g readiness gate 只看此文件。
- **probe gate（反伪造）**：`grx009_segment4g_visual_fallback_ready` 只在 segment 4f 已 ready **且** `grx009_segment4g_visual_fallback_issue` 返回 None 时成立。issue 审计从磁盘字节出发：schema 在位、success 证据 `status=success`（SKIP 永不 ready）、discipline flag（`runtime_state=fallback_only`/`real_gpu_pass=false`/`real_d3d12_dispatch_recorded=false`/`godot_runtime_luminance_path_enabled=false`/`default_enable_state=disabled`/`performance_claim="none"`）、`visual.measured_local=true`（placeholder/estimated 永不 ready）、三个 frame artifact 存在 + SHA-256 匹配 + `size == width*height*3`（尺寸反伪造，最小 64px）、**in-process 重算** LDR diff（含 diff artifact 字节）并要求 recorded 数字与 pinned 阈值逐项匹配、fallback matrix 双腿 exit 0 + session ready + candidate 腿 marker observed（reference 腿必须未观察到）、`measured_fallback_telemetry.json` hash 匹配且过 GRX-008 校验、离线 compile artifact 摘要复核（同 4f）、DLL 指纹必须 feature-off。手改/占位 success JSON 无法推进 gate。probe 同时输出 latest/success 状态与确切 blocker（`grx009_segment4g_visual_fallback_issue`）。
- 当前最新结果（2026-07-05，本机 tracked build）：`status=success` —— 双腿 exit 0、session ready、candidate 腿实测 `fallback rc=1` marker、两帧 256x144 逐字节一致（`max_abs=0`、`mean_abs=0`）、telemetry 过校验。
- ready 后 `next_action=start_grx009_luminance_gated_real_pass_enablement`（下一片才开始设计 measured、opt-in 的 real luminance pass enablement）；success 证据缺失/不严格时保持 `start_grx009_luminance_real_visual_diff_and_measured_fallback_telemetry` 并报告确切 blocker，`next_command` 指向本 harness。
- 运行时语义保持不变：即便 4g `status=success`，manifest 仍 `runtime_state=fallback_only`、`real_gpu_pass=false`、`real_d3d12_dispatch_recorded=false`、pass 默认 `disabled`；默认 Godot config 与 shipping/feature-off bridge 对 `RXGD_PASS_LUMINANCE_REDUCTION` 仍恒返回 `RXGD_STATUS_FALLBACK`。本段无 Rurix-pass visual 宣称、无 GPU timestamp、无性能/FPS 宣称。

## 19. Segment 4h（gated real-pass enablement gate，2026-07-06）

- 本段目标是落地**首个 opt-in、fail-closed 的 real luminance pass enablement gate**：显式测试/bring-up 开关 + 桥侧全链校验 + 三腿 pass enable matrix + measured fallback red/green，**不是 default enablement**。`rendering/rurix_accel/passes/luminance_reduction/enabled` 默认仍 `false`，新增的 `.../dispatch_real_pass` opt-in 也默认 `false`，本段**不**做任何 FPS/GPU timestamp/性能宣称。
- **关键事实（binding-kind 失配，本段 fail-closed 的诚实根源）**：tracked segment 3a kernel（`src/lib.rx`）把 `View`/`ViewMut<global, f32>` lower 成 raw-buffer 视图（debug IR 中为 `target("dx.RawBuffer", float, ...)` 句柄），并在 level 0 就做 `clamp(min,max) * exposure_adjust`；而 Godot runtime（segment 4e）交给桥的是真实 **Texture2D** `ID3D12Resource*` 句柄，原生语义是完整金字塔 + 时域自适应。把 texture descriptor 绑到 raw-buffer 声明上属 descriptor-type 未定义行为（4d/4f 的「recorded dispatch」仅证明了管线 plumbing，不证明计算正确性），因此**用 tracked artifact 无法做出计算正确的 real dispatch**。本段 gate 把这一失配变成显式校验并如实上报 first missing prerequisite，而不是执行未定义行为或谎称成功。
- **bridge 侧（`src/rurix-godot/src/lib.rs`，编译进 shipping feature-off DLL，处处 fail-closed）**：
  - 新增保留 capability 位 `RXGD_CAP_LUMINANCE_REAL_PASS = 1 << 3`（复用 `RxGdCaps.flags`，ABI v1 不变）；
  - real-pass attempt 依次跑 segment 4a runtime binding preflight → segment 4b dispatch eligibility → **segment 4h kernel-binding-kind conformance check**（tracked kernel binding kind = `raw_buffer_view`，runtime texture 资源不符 → `validation_failed`）；任一失败即返回 `RXGD_STATUS_FALLBACK`、记录 fallback reason，并**每 session 一次**打印机读诊断 `RXGD_REAL_PASS_BLOCKED first_missing_prerequisite=... fallback_reason=... kernel_binding=raw_buffer_view default_enable_state=disabled`（刻意不是 `ERROR:` 行、不含 `RXGD_DIAG`，不触发 runtime log audit）；
  - 全链通过后本段也**不**接 dispatch（即便 `d3d12-recording-shim` feature 开启）：dispatch 接线与使其 well-defined 的 runtime-mappable kernel artifact round 同片落地；
  - feature-off 单测覆盖：binding-kind 阻断（`validation_failed` + `kernel_binding_kind_mismatch`）、缺 bring-up opt-in（`manual_disabled`）、能力降级（`unsupported_device`）、空句柄（`validation_failed`）、默认 4b 路径行为不变。
- **patch 0009（`0009-rurix-accel-luminance-real-pass-optin.patch`，stacked on 0001..0008）**：新增两个默认 false 设置 —— `.../dispatch_real_pass`（置 `RXGD_CAP_LUMINANCE_REAL_PASS`）与 harness-only 强制失败旋钮 `.../real_pass_force_capability_downgrade`（清 `RXGD_CAP_SHADER_INT64`，令 preflight 以 `unsupported_device` fail closed）；模块仅在 real-pass arm 下 bridge 实际返回 OK 时打印 `RXGD_GODOT_RUNTIME_LUMINANCE_REAL_PASS ... dispatched=1` marker（tracked artifact 下永不可达）。
- 新增 harness：`ci/grx009_segment4h_real_pass_enablement_smoke.py`。
  - 需要 **full 0001..0009** scratch Godot console build（`RURIX_GRX009_SEGMENT4H_GODOT_EXE` + `_GODOT_SOURCE`/`_SOURCE_PROVENANCE`/`_BUILD_COMMAND`/`_BUILD_LOG`，sidecar 溯源审计复用 segment 4f 机制、按 0001..0009 校验）；bridge 用 **shipping feature-OFF** DLL（gate 量的就是出厂 DLL 的 fail-closed 行为，evidence `dll_fingerprint.features` 必须为空）。
  - **三腿 matrix**（同 4g 的确定性场景/抓帧管线；fallback marker 为 0007 级措辞 `RurixAccel: luminance_reduction native resource handle mapping fallback rc=`）：`disabled_default`（全默认，不得出现任何 marker）、`enabled_real_pass_optin`（enabled+bringup+real_pass；必须实测 fallback marker + `RXGD_REAL_PASS_BLOCKED`，且 blocked 诊断必须命中预测的 `kernel_binding_kind_mismatch`/`validation_failed`，否则 FAIL）、`forced_capability_downgrade`（再加降级旋钮；必须实测 `runtime_binding_preflight_failed`/`unsupported_device`——这就是要求的 forced-failure fallback red/green）。
  - **visual gate（measured_local、LDR absolute diff、pinned `max_abs<=2`/`mean_abs<=0.25`）**：candidate 与 forced 两腿帧必须与 native reference 帧同阈值内一致（武装 fail-closed opt-in 不得改画面）；未来 strict success 下同一阈值裁决 native reference vs real Rurix-pass candidate，超阈值即 FAIL 且 pass 保持默认 disabled。
  - 三腿全量 stdout+stderr runtime log audit（只容忍 `Could not load global script cache`，带 rationale；其余 `ERROR:` 一律 FAIL）；生成 GRX-008 `real_pass_enablement_telemetry.json`（measured_local；blocked 结局含 candidate `validation_failed` + forced `unsupported_device` 两条，`telemetry_frame` 必须等于实测抓帧帧号）。
- **结局语义 / 两份 tracked evidence**：`real_pass_enablement_evidence.json`（latest，每次改写）+ `real_pass_enablement_success_evidence.json`（historical strict success，唯一推进 readiness 的文件）。`skip_kind=environment`（前置缺失；`RURIX_REQUIRE_REAL=1` 升级 FAIL）与 `skip_kind=measured_prerequisite_blocked`（真机全腿实测且形状与预测完全一致，如实记录 `first_missing_prerequisite`，**不**被 REQUIRE_REAL 升级、也**不**推进 gate）分开记录；任何完整性违规（marker 错腿/超阈值/telemetry 失配/意外 ERROR/非零退出/artifact 篡改）= FAIL。`status=success` 严格保留给未来：real dispatch 真正执行且完成（marker line 入证）+ visual gate 绿 + 全审计绿，**用 tracked segment 3a artifact 按设计不可达**；即便 success 也仅 evidence 内 `real_gpu_pass=true`，`default_enable_state` 仍 `disabled`、`performance_claim` 仍 `none`。
- **probe gate（反伪造）**：`grx009_segment4h_real_pass_enablement_ready` 只在 segment 4g ready **且** `grx009_segment4h_real_pass_enablement_issue` 为 None 时成立；issue 审计镜像 4f/4g（schema、discipline flags、checks、三腿 matrix 一致性、frame artifact 磁盘复核 + in-process diff 重算、telemetry hash/字段/`--validate-only`、离线 artifact 摘要复核、exe/DLL 指纹、0001..0009 patch-stack identity 与 scratch 溯源、runtime log audit + stdout 复审、success 与 candidate fallback telemetry 互斥性）。probe 同时输出 latest status/skip_kind/first_missing_prerequisite 与 `grx009_patch_0009_applyable`；4g ready 且 4h 未 ready 时 `next_action` 按 latest evidence 指向第一缺失前置（缺 exe → `start_grx009_segment4h_real_pass_enablement_smoke`；measured blocked → `design_grx009_luminance_pyramid_continuation_kernel`（segment 4i texture-capable kernel artifact round 已就位后，下一前置是 pyramid continuation + EMA + prev luminance 设计与实现）；fail → `fix_grx009_segment4h_real_pass_enablement_failure`）。
- 运行时语义保持不变：manifest 仍 `runtime_state=fallback_only`、`real_gpu_pass=false`、`real_d3d12_dispatch_recorded=false`、pass 默认 `disabled`；默认 Godot config 与 shipping/feature-off bridge 对 `RXGD_PASS_LUMINANCE_REDUCTION` 仍恒返回 `RXGD_STATUS_FALLBACK`。本段无 Rurix-pass visual 成功宣称、无 GPU timestamp、无性能/FPS 宣称。

## 20. 仍未完成项

- 默认启用的 Godot runtime bridge D3D12 dispatch 录制（segment 4f 仅在 test-only `d3d12-recording-shim` feature + 默认关闭的 `.../dispatch_recording_smoke` opt-in 下，从 Godot runtime path 驱动一次录制取证；默认 Godot config 与 shipping/feature-off bridge 仍 fallback，未默认启用）。
- **pyramid continuation + EMA + prev luminance 设计与实现**（segment 4i texture-capable kernel artifact round：texture-capable kernel 源 `src/lib_texture.rx` 与编译器侧 forward-looking 改动已就位——`RWTexture2D<F>` lang item、`MirResourceType::RWTexture2D`、`derive_compute_bindings` 的 Texture2D/RWTexture2D 分支、`texture_target_ty`、`@llvm.dx.resource.load.texture.*`/`store.texture.*` emit、descriptor layout 的 `binding_kind` 字段——但 patched llc 不支持 `llvm.dx.resource.load.texture.2d` intrinsic，canonical compile 失败、bridge tracked package 保持 raw-buffer（`LUMINANCE_KERNEL_RESOURCE_BINDING_KIND = "raw_buffer_view"`，未替换为 `"texture2d"`，fail-closed），canonical artifacts 路径携带 raw-buffer 字节复制自 `artifacts/raw_buffer_historical/`，current first blocker 是 `kernel_binding_kind_mismatch`；待 patched llc 支持 texture intrinsic 后，下一前置是 multi-level cascade、EMA feedback `prev + (cur-prev)*exposure_adjust`、previous-luminance 双缓冲、final-level clamp/min/max gating（即 `math_pyramid_parity_not_proven`，仅 future-only），才能解锁 real dispatch 接线）。
- 真实 Rurix GPU luminance pass runtime 接入（真实 D3D12 dispatch 执行及之后段落；segment 4h 已把 opt-in gate、fail-closed 校验链与 forced-failure red/green 接好，dispatch 本身仍未接线）。
- 真实 Godot/Rurix runtime 句柄绑定与 runtime descriptor 写入（segment 4a/4b 仅传递 logical RID id 与 opt-in flag 并做前置/eligibility 校验；segment 4c 只在独立 harness 里用合成资源做 device smoke；segment 4d 在 harness 里驱动 bridge C ABI 录制但非 Godot runtime path；segment 4e 把真实 native handle 映射进 hook/callsite，但仍是 preflight，不驱动 runtime 录制）。
- Godot runtime bridge D3D12 dispatch 录制（segment 4d 仅在 test-only feature + harness 下录制；shipping/feature-off bridge 与默认 Godot 路径仍 fallback）。
- **Rurix pass 的**真实 visual diff evidence（segment 4g 已录得 fallback-path 的 measured LDR diff——开启恒回退的 pass 不改画面；但 Rurix GPU pass 本身的画面验证仍未开始，reference vs Rurix-pass candidate 的对比留给 real pass enablement 之后的段落）。
- 更全面的 measured fallback telemetry（segment 4g 已录得首个 measured_local fallback telemetry——enabled-but-fallback 矩阵与 patch-0002 marker 实测；多场景 / 多 pass / 运行时长采样的 telemetry 仍未覆盖）。
- full baseline / Rurix measured_local 对比数据。
- 任何性能提升声明。

## 21. Segment 4i — Texture-Capable Kernel Artifact Round（2026-07-06，fail-closed revert）

- 本段目标是把 segment 4h real-pass gate 的诚实 blocker 从 `kernel_binding_kind_mismatch` 推进到下一真实前置 `math_pyramid_parity_not_proven`：在 Rurix DXIL/offline compile 路径中新增 texture-capable luminance kernel artifact round，使 tracked kernel 声明的是 `Texture2D<f32>`/`RWTexture2D<f32>`（而非 raw-buffer view），从而通过 segment 4h kernel-binding-kind conformance check。本段**不**完成 strict success（仍需 real dispatch wiring + visual gate + 真实硬件 measured success，属后续 slice）；`real_gpu_pass` 保持 `false`、`runtime_state` 保持 `fallback_only`、`default_enable_state` 保持 `disabled`、无任何 FPS/performance 宣称。
- **HONEST FAIL-CLOSED PATH（当前状态）**：texture-capable kernel 源 `src/lib_texture.rx` 已就位（声明 `Texture2D<f32>`/`RWTexture2D<f32>`），编译器侧 forward-looking 改动已落地（`RWTexture2D<F>` lang item、`MirResourceType::RWTexture2D`、`texture_target_ty`、`@llvm.dx.resource.load.texture.*`/`@llvm.dx.resource.store.texture.*` emit、conformance corpus `texture_param.rx`/`rwtexture_param.rx`/`texture_wrong_elem_type.rx`），但 `H:\llvm-dxil\build\bin\llc.exe` 这版 patched llc **不支持** `llvm.dx.resource.load.texture.2d` intrinsic，texture-capable 离线 compile 记录 `status=compile_failed`（blocker `dxil_container_missing`），因此 canonical `artifacts/luminance_reduction.{dxil,rts0.bin,_descriptor_layout.json}` 路径回退为从 `artifacts/raw_buffer_historical/` 复制的 raw-buffer 字节，bridge tracked package 保持 raw-buffer，probe 停在 `kernel_binding_kind_mismatch`。当更新的 patched llc 支持 texture intrinsic 后，本段 forward-looking 改动会自动激活。
- **新 kernel 源**：`src/lib_texture.rx` 声明 `kernel fn luminance_reduce_level_texture(src_luminance: Texture2D<f32>, dst_luminance: RWTexture2D<f32>, source_width: usize, source_height: usize, max_luminance: f32, min_luminance: f32, exposure_adjust: f32, t: ThreadCtx<1>)`，用 `Texture2D<f32>`/`RWTexture2D<f32>` 替代 `View`/`ViewMut`；`src/lib.rx` 保留不动作为 raw-buffer 历史 fixture 与 conformance corpus 输入。raw-buffer artifact 字节落在 `artifacts/raw_buffer_historical/`，并被复制到 canonical `artifacts/` 路径以使 bridge `include_bytes!` 工作（fail-closed：texture-capable compile 失败时）。
- **编译器侧新增（forward-looking，已保留）**：`RWTexture2D<F>` lang item（compute-kernel UAV 纹理，与已有 SRV `Texture2D<F>` 区分；`Texture2D<F>` 同时被放宽到 compute-kernel 参数位置）；`MirResourceType::RWTexture2D(PrimTy)` 变体（`class() → Uav`）；`derive_compute_bindings` 的 `Texture2D`/`RWTexture2D` head-name 分支；`require_view_global_f32` 放宽为 `require_texture_or_view_global_f32`；新辅助 `texture_target_ty(mutable: bool)` emit `target("dx.Texture2D<float>", 0, 0)` / `target("dx.RWTexture2D<float>", 0, 0)`；`render_lowered_ops` 的 `@llvm.dx.resource.load.texture.*` / `@llvm.dx.resource.store.texture.*` emit；raw-buffer lowering 路径保持不变。
- **descriptor layout 新字段**：`render_descriptor_layout_json` 的每条 resource record 增加 `"binding_kind": "texture2d"|"rwtexture2d"|"raw_buffer_view"|"sampler"|"constant_buffer"` 字段，由新辅助 `binding_kind_str(res: MirResourceType)` 决定。
- **bridge tracked package 保持 raw-buffer（fail-closed）**：`LUMINANCE_KERNEL_RESOURCE_BINDING_KIND = "raw_buffer_view"`（未替换为 texture2d）；三个 SHA-256 常量保持 segment 3a raw-buffer 值（`c77a54de...`/`f08794f9...`/`3ceee39b...`），与 `offline_compile_evidence_raw_buffer.json` 一致；`runtime_resource_binding_kind` 映射不变（`RXGD_RESOURCE_TEXTURE → "texture2d"`、`RXGD_RESOURCE_BUFFER → "raw_buffer_view"`），texture 资源（Godot runtime 实际提供的）仍 fail binding-kind check → `kernel_binding_kind_mismatch`，buffer 资源通过 binding-kind check 后才到 `check_real_pass_math_parity`。
- **新 bridge `check_real_pass_math_parity`（forward-looking，已保留）**：返回 `Err(FallbackReason::ValidationFailed)` hard-coded fail（注释记录已知 gap：single-level、无 EMA feedback、无 previous-luminance 双缓冲）；`record_real_pass_attempt` 顺序变为 preflight → eligibility → binding_kind → **math_parity** → `real_dispatch_path_not_linked`；blocked 诊断打印 `first_missing_prerequisite=math_pyramid_parity_not_proven kernel_binding=raw_buffer_view`（math_parity 为首个失败前置时，仅当 tracked package 切到 texture-capable 后才会到达此分支）。当前 tracked package 为 raw-buffer，故 blocked 诊断打印 `first_missing_prerequisite=kernel_binding_kind_mismatch kernel_binding=raw_buffer_view`。
- **4h smoke 同步**：`EXPECTED_FIRST_MISSING_PREREQUISITE = "kernel_binding_kind_mismatch"`；`EXPECTED_BLOCKED_FALLBACK_REASON = "validation_failed"`、`EXPECTED_FORCED_PREREQUISITE`/`EXPECTED_FORCED_FALLBACK_REASON` 不变；`KNOWN_GAPS` 重写（首项描述 tracked kernel 为 raw-buffer + Godot runtime 提供 Texture2D 资源 + texture-capable kernel source `lib_texture.rx` 已就位但 patched llc 不支持 `llvm.dx.resource.load.texture.2d` intrinsic）；artifact hash 三向校验 recompute from canonical paths → match raw-buffer `offline_compile_evidence_raw_buffer.json` 哈希。
- **probe `next_action` 保持**：当 `first_missing_prerequisite == "kernel_binding_kind_mismatch"` 时 `next_action = provide_grx009_runtime_mappable_luminance_kernel_artifact`（本 slice 的 main goal 仍被 patched llc 不支持 texture intrinsic 阻塞）；summary 输出键 `grx009_luminance_kernel_binding_kind` 应为 `"raw_buffer_view"`、`grx009_luminance_math_parity_status` 读自 offline evidence（forward-looking kernel 的 parity gap）、`grx009_luminance_offline_binding_kinds` 读自 offline evidence（`["texture2d", "rwtexture2d"]`，即使 compile 失败也保留作为 audit）；validation 回归测试 probe 常量 == smoke 常量断言保持绿，candidate 腿 fixture `first_missing_prerequisite` 为 `kernel_binding_kind_mismatch`，forced 腿 fixture 保持 `runtime_binding_preflight_failed`/`unsupported_device`。
- **raw-buffer artifact 保留**：作为历史 fixture 落在 `artifacts/raw_buffer_historical/`，`offline_compile_evidence_raw_buffer.json` 复制原 segment 3a raw-buffer measured state 并 `notes` 注明为 historical fixture；canonical `artifacts/` 路径在 fail-closed 状态下也携带相同的 raw-buffer 字节。
- 运行时语义保持不变：manifest 仍 `runtime_state=fallback_only`、`real_gpu_pass=false`、`real_d3d12_dispatch_recorded=false`、pass 默认 `disabled`；默认 Godot config 与 shipping/feature-off bridge 对 `RXGD_PASS_LUMINANCE_REDUCTION` 仍恒返回 `RXGD_STATUS_FALLBACK`。本段无 Rurix-pass visual 成功宣称、无 GPU timestamp、无性能/FPS 宣称。

## 22. Segment 4j — Texture Intrinsic Toolchain Blocker Evidence（2026-07-06）

- 本段不引入新的 pass 状态变更，而是为 §21（segment 4i fail-closed revert）的"patched llc 不支持 `llvm.dx.resource.load.texture.2d` intrinsic"这一陈述落地**严格的三向交叉取证证据基**（structured evidence 落在 `texture_intrinsic_toolchain_blocker.json`，与本文件同目录）。本段**不**与 §21 的状态字段冲突：双方一致 `status=compile_failed`、`runtime_mappable=false`、bridge tracked package=`raw_buffer_view`、probe first blocker=`kernel_binding_kind_mismatch`。本段**不**推进 `math_pyramid_parity_not_proven`（那是 texture artifact 成功之后的**下一** blocker）；**不**改 bridge/probe/smoke/schema 常量；**不**把 raw_buffer 伪装成 texture artifact；`real_gpu_pass` 保持 `false`、`default_enable_state` 保持 `disabled`、`runtime_mappable` 保持 `false`、bridge tracked package 保持 `raw_buffer_view`、probe first blocker 保持 `kernel_binding_kind_mismatch`。本段无 FPS / GPU timestamp / visual success / performance 宣称。
- **Three-way investigation methodology（三向交叉取证）**：
  1. **Minimal .ll scratch cases（10 例 A–J）**：在 `spike/godot-rurix/passes/luminance_reduction/artifacts/toolchain_probe/` 下手工 author 十个最小 LLVM IR 模块，每个孤立一个 intrinsic + target-ext-type 组合，以 `llc.exe <case>.ll -filetype=obj -o <case>.obj` 编译并记录 exit code / stderr / `obj_produced` 到 `probe_results.json`。
  2. **`IntrinsicsDirectX.td` source review**：读 `H:\llvm-dxil\llvm-project\llvm\include\llvm\IR\IntrinsicsDirectX.td` 枚举每个 `int_dx_resource_load_*` / `int_dx_resource_store_*` 定义；读 `H:\llvm-dxil\llvm-project\llvm\lib\Target\DirectX\DXILResourceAccess.cpp` `createLoadIntrinsic`（L389-420）与 `createStoreIntrinsic`（L153-183）检视 C++ lowering 对 `ResourceKind::Texture2D` 的分发。
  3. **`llc.exe` binary `findstr`**：对 `H:\llvm-dxil\build\bin\llc.exe` 跑 `findstr` 搜索 `llvm.dx.resource.` 与 `llvm.dx.texture.`，枚举编译进二进制的 intrinsic 名字字符串，与 `.td` 源 review 独立交叉验证。
- **Key conclusion（核心结论）**：patched llc（`H:\llvm-dxil\build\bin\llc.exe`，LLVM 22.1.7，registered targets `dxil`/`x86`/`x86-64`）对 Texture2D/RWTexture2D 的 load/store intrinsic **零支持**，三向证据互证：
  - `.td` 源：`IntrinsicsDirectX.td` 中**无** `int_dx_resource_load_texture*` / `int_dx_resource_store_texture*` 任何定义（仅有 `int_dx_resource_load_typedbuffer`、`int_dx_resource_load_rawbuffer`、`int_dx_resource_load_cbufferrow.{2,4,8}` 与 `int_dx_resource_store_typedbuffer`、`int_dx_resource_store_rawbuffer`）。
  - 二进制：`findstr` 在 `llc.exe` 中找到 14 条 `llvm.dx.resource.*` 字符串（`llvm.dx.resource.casthandle`、`getdimensions.x`、`getpointer`、`handlefrombinding`、`handlefromimplicitbinding`、`load.cbufferrow.{2,4,8}`、`load.rawbuffer`、`load.typedbuffer`、`nonuniformindex`、`store.rawbuffer`、`store.typedbuffer`、`updatecounter`），`llvm.dx.texture.*` 字符串**零**条。
  - C++ lowering：`DXILResourceAccess.cpp` `createLoadIntrinsic`（L389-420）对 `ResourceKind::Texture2D` 显式 stub `reportFatalUsageError("Load not yet implemented for resource type")`，`createStoreIntrinsic`（L153-183）同类 stub（`Texture2D` 走 `reportFatalUsageError` 路径，无真实 lowering）。
- **Accepted intrinsics（被识别的 intrinsics，非 texture）**：
  - `llvm.dx.resource.load.typedbuffer`：case C（`target("dx.TypedBuffer", float, 0, 0, 0)`）与 case D（`target("dx.Texture2D<float>", 0, 0)`）本 run 均 `obj_produced=true`、`exit=0`（注：case D 之所以通过，是 IR 层 selection 不 cross-validate resource kind 与 intrinsic 名字，**不**意味着 texture 类型已注册）。
  - `llvm.dx.resource.load.rawbuffer`：case J（`target("dx.RawBuffer", float, 0, 0)`），**non-deterministic**——本 run crash（exit 3221225477），其它 run 接受；intrinsic 名字被识别（findstr string #9，`.td` 定义 `int_dx_resource_load_rawbuffer`）。
  - `llvm.dx.resource.store.rawbuffer`：无独立 scratch case；intrinsic 名字被识别（findstr string #12，`.td` 定义 `int_dx_resource_store_rawbuffer`），未被作为 "unknown intrinsic" 拒绝。
  - `llvm.dx.resource.store.typedbuffer`：case I（`target("dx.TypedBuffer", float, 1, 0, 0)`），**non-deterministic**——本 run crash（exit 3221225477）；intrinsic 名字被识别（findstr string #13，`.td` 定义 `int_dx_resource_store_typedbuffer`），未被作为 "unknown intrinsic" 拒绝。注："recognized-by-name" **不**等于 "obj-produced"。
- **Rejected intrinsics（被拒的 texture intrinsics）**：
  - `llvm.dx.resource.load.texture.2d`：case A（`target("dx.Texture2D<float>", 0, 0)` + 完整参数 `(handle, i32 0, i32 0)`）与 case B（同 intrinsic，省略 trailing `0,0` 参数）均 `exit=1`、`obj_produced=false`，verbatim 错误：`unknown intrinsic 'llvm.dx.resource.load.texture.2d'`。这是 `rurixc` 当前 emit 的形式。
  - `llvm.dx.resource.store.texture.2d`：case H（`target("dx.RWTexture2D<float>", 0, 0)` + `(handle, i32 0, i32 0, float 1.0)`）`exit=1`、`obj_produced=false`，verbatim 错误：`unknown intrinsic 'llvm.dx.resource.store.texture.2d'`。load/store 两向 texture intrinsics 同等地不被支持。
- **target-ext type status**：
  - `target("dx.Texture2D<float>", 0, 0)`：**未注册**。当分配该类型 handle 但不使用（case E：仅 `handlefrombinding`）时，llc 在 "DXContainer Global Emitter" 崩溃，Exception Code 0xC0000005（exit 3221225477）；case F 同样崩溃。即便该类型与被识别的 intrinsic 同用（case D：`load.typedbuffer`）能产出 obj，也**不**意味着类型已注册。
  - `target("dx.RWTexture2D<float>", 0, 0)`：**未注册**。在 case H（`store.texture.2d`）中使用，但先在 intrinsic-name 层被拒，未到类型注册路径；无 scratch case 单独隔离 RWTexture2D 类型注册崩溃。
  - 对照：`target("dx.TypedBuffer", float, 0, 0, 0)` 与 `target("dx.RawBuffer", float, 0, 0)` **已注册**（case C/J 用作 handle 类型；注册形式见 `docs/DirectX/DXILResources.rst`）。即便如此，case G（`dx.TypedBuffer` handle 仅 `handlefrombinding` 不使用）仍崩 DXContainer Global Emitter，说明未使用 handle 的崩溃不限于 texture 类型，影响任何 target-ext 类型的未使用 handle。
- **Non-determinism note（非确定性）**：llc 22.1.7 在 "DXContainer Global Emitter" pass 有 memory-safety bug（Exception Code 0xC0000005 访问违例）。cases C、D、I、J 在不同 run 之间 flip 于 crash（exit 3221225477）与 accept（exit 0）之间：本 probe run 中 cases C/D 接受、cases I/J 崩溃；下一次 run 可能 cases C/D 崩溃、cases I/J 接受。这一非确定性**本身**就是 toolchain-blocker 证据：即便"被接受"的 intrinsics 也无法稳定产出 object。崩溃与 texture handle 是否使用无关，也影响 `dx.TypedBuffer`/`dx.RawBuffer` 未使用 handle（cases E、F、G）。
- **Patched llc capability list（要让 texture intrinsic 跑通所需的 4 项 patched llc 改动，详见 `texture_intrinsic_toolchain_blocker.json`）**：
  1. **Add `int_dx_resource_load_texture_2d`** in `H:\llvm-dxil\llvm-project\llvm\include\llvm\IR\IntrinsicsDirectX.td`（signature `{float, i1} ← [target("dx.Texture2D<float>", 0, 0), i32, i32]`，handle + coord_x + coord_y；DXIL opcode `textureLoad` 66）。
  2. **Add `int_dx_resource_store_texture_2d`** in same `.td`（signature `void ← [target("dx.RWTexture2D<float>", 0, 0), i32, i32, float]`，handle + coord_x + coord_y + value；DXIL opcode `textureStore` 67）。
  3. **Register target-ext types** `target("dx.Texture2D<float>", 0, 0)` 与 `target("dx.RWTexture2D<float>", 0, 0)` 为 valid target-ext types in `H:\llvm-dxil\llvm-project\llvm\lib\Target\DirectX`（参考 `docs/DirectX/DXILResources.rst` 中 `dx.TypedBuffer`/`dx.RawBuffer` 的注册形式）。
  4. **Implement C++ lowering** in `H:\llvm-dxil\llvm-project\llvm\lib\Target\DirectX\DXILResourceAccess.cpp` `createLoadIntrinsic`（L389-420）/ `createStoreIntrinsic`（L153-183），对 `ResourceKind::Texture2D` 替换 `reportFatalUsageError("Load not yet implemented for resource type")` stub 为真实 lowering（emit `dx.op.textureLoad` opcode 66 / `dx.op.textureStore` opcode 67 DXIL opcodes）。
- **Repro command**：`py -3 spike/godot-rurix/passes/luminance_reduction/artifacts/toolchain_probe/run_probe.py`（每次运行可能因 non-determinism 给出不同的 C/D/I/J 接受/崩溃组合，但 cases A/B/H 的 "unknown intrinsic" 拒绝与 `llvm.dx.texture.*` 零字符串结果在所有 run 中稳定）。
- 运行时语义保持不变：manifest 仍 `runtime_state=fallback_only`、`real_gpu_pass=false`、`real_d3d12_dispatch_recorded=false`、pass 默认 `disabled`；默认 Godot config 与 shipping/feature-off bridge 对 `RXGD_PASS_LUMINANCE_REDUCTION` 仍恒返回 `RXGD_STATUS_FALLBACK`。本段无 Rurix-pass visual 成功宣称、无 GPU timestamp、无性能/FPS 宣称。

## 23. Segment 4k — DXC Texture Artifact Bridge Design（design-only）

- Segment 4k 只证明一个最小 HLSL `Texture2D<float>` / `RWTexture2D<float>` compute shader 可以由 DXC 编译并由 DXV 验证通过（`texture_dxc_feasibility_evidence.json`: `status=success`, `ready=true`, `validation.status=pass`）。该 HLSL bridge output 缺少 Rurix-owned `RTS0` root signature、Rurix descriptor layout contract 与 Rurix source provenance，因此不是 canonical luminance artifact。
- 新设计契约落在 `dxc_texture_artifact_bridge.md`，design-only evidence 落在 `dxc_texture_artifact_bridge_design.json`。契约要求后续 texture package 明确 root signature 策略、descriptor layout synthesis、binding_kind mapping、DXIL validation metadata、Rurix provenance、canonical switch 条件与 fail-closed 条件。
- Root signature 策略：未来 runtime-mappable package 必须产出 Rurix-owned `RTS0` bytes。DXC container reflection/extraction 可作为 cross-check，但不能单独替代 Rurix descriptor-layout synthesis；root parameter count/order、descriptor range type、register、space、visibility、root constant byte/DWORD packing 任一 mismatch 都必须 fail-closed。
- Descriptor layout synthesis：texture package 的 source 必须是 `src_luminance=t0 space0 binding_kind=texture2d`，destination 必须是 `dst_luminance=u0 space0 binding_kind=rwtexture2d`。若携带 luminance constants，必须沿用 `b0 space0` 28-byte layout：`source_width`/`source_height` 两个 `i64` 后接 `max_luminance`/`min_luminance`/`exposure_adjust` 三个 `f32`。无 constants 的最小 feasibility shader 必须记录 `root_constants=none`，不得作为 canonical luminance artifact。
- Binding-kind 语义：Godot `Texture2D` `ID3D12Resource*` 必须映射为 texture resource，不能再绑定到 raw-buffer declaration。当前 canonical descriptor layout 仍是 raw-buffer fallback bytes，bridge tracked package 仍 `raw_buffer_view`，所以当前 real-pass first blocker 仍是 `kernel_binding_kind_mismatch`。
- DXIL validation metadata：后续 package 必须记录 dxc/dxv path 与 version、compile/validation argv、container hash、stdout/stderr evidence、profile、entry point、source hash、root signature hash 与 descriptor layout hash；DXV fail、hash mismatch、metadata missing 都 fail-closed。
- Provenance 与 canonical switch：只有 Rurix source + Rurix compiler/offline pipeline 生成的 artifact 才能声明 `rurix_owned=true`。若仍是 HLSL bridge workaround，即使 DXC/DXV pass，也必须显式标为非最终 Rurix-owned artifact。只有 DXIL container、Rurix-owned RTS0、descriptor layout、DXV pass、binding-kind conformance、Rurix provenance、visual/fallback evidence 与 probe red/green 全部满足后，才允许把 canonical raw-buffer fallback 切到 texture artifact package。
- Scaffold exit criteria：`dxc_texture_artifact_bridge_scaffold_evidence.json` 可以记录 `status=success` 与 `scaffold_ready=true`，但只代表 `artifacts/dxc_texture_bridge/` 下的独立 DXIL container copy、descriptor layout artifact、root signature scaffold/unavailable reason、binding_kind mapping、DXC/DXV metadata、DXIL validation metadata 与 provenance 字段可审计。它必须保持 `provenance="hlsl_bridge_workaround"`、`rurix_owned=false`、`runtime_mappable=false`、`real_gpu_pass=false`、`canonical_artifact_replaced=false`、`offline_compile_status_changed=false`、`design_or_scaffold_only=true`；canonical `artifacts/luminance_reduction.{dxil,rts0.bin,_descriptor_layout.json}` 继续保持 raw-buffer fallback bytes，canonical descriptor 继续为 `raw_buffer_view`。scaffold ready 后唯一允许的下一步是 Rurix provenance 或 RTS0/root-signature integration，不得推进 real pass enablement、default enablement、visual success、GPU timestamp、FPS 或性能声明。
- RTS0 integration scaffold：`ci/grx009_dxc_texture_rts0_integration_smoke.py` 可从 `artifacts/dxc_texture_bridge/descriptor_layout.json` 调用 `rurixc::binding_layout::{infer_root_signature, serialize_rts0}` 生成独立的 Rurix-owned `artifacts/dxc_texture_bridge/root_signature.rts0.bin`，并在 `root_signature_scaffold.json` / scaffold evidence / manifest 中记录 RTS0 hash、size 与输入 descriptor hash。该 RTS0 只证明 root-signature 准备层可由 Rurix 逻辑合成；它不改变 HLSL DXIL container 的 `provenance="hlsl_bridge_workaround"`，不允许把顶层 `rurix_owned` 改为 true，不替换 canonical raw-buffer artifact，也不让 runtime 变为 mappable。RTS0 integration ready 后唯一允许的下一步是 descriptor/RTS0 cross-check 或 provenance policy，不得推进 real pass enablement、visual success、GPU timestamp、FPS 或性能声明。
- Descriptor/RTS0 cross-check：`ci/grx009_dxc_texture_descriptor_rts0_crosscheck_smoke.py` 重新读取 `artifacts/dxc_texture_bridge/descriptor_layout.json`，严格校验 `src_luminance` 为 SRV `t0 space0 count=1 binding_kind=texture2d`、`dst_luminance` 为 UAV `u0 space0 count=1 binding_kind=rwtexture2d`、`root_constants=none`，再经 Rurix binding layout path 重新 serialize RTS0。当前 evidence `dxc_texture_descriptor_rts0_crosscheck_evidence.json` 记录 `cross_check_status=success`、`descriptor_rts0_crosscheck_ready=true`、descriptor sha256 `65cd6b3012e08332de897f9a0cc222011e587f8e2ad8ba718472347e731b72d9`、tracked RTS0 sha256 `bed820418c0a7cf9d1e9d160de9f42ac56fee8f067c1880bd3af28f9d62fec0c`、re-serialized RTS0 sha256 `bed820418c0a7cf9d1e9d160de9f42ac56fee8f067c1880bd3af28f9d62fec0c`、`byte_for_byte_match=true`。该 cross-check 只推进 provenance-policy 下一步，仍保持 `provenance="hlsl_bridge_workaround"`、`rurix_owned=false`、`runtime_mappable=false`、`real_gpu_pass=false`、`canonical_artifact_replaced=false`、canonical raw-buffer artifact 不变。
- 本段不改 runtime 语义：canonical `offline_compile_evidence.json` 仍为 `status=compile_failed`、`runtime_mappable=false`，manifest 仍 `runtime_state=fallback_only`、`real_gpu_pass=false`、`default_enable_state=disabled`，不生成 `real_pass_enablement_success_evidence.json`，不声明 visual/GPU timestamp/FPS/performance success。
