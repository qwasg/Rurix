# GRX-009 Luminance Reduction Pass — PASS CONTRACT

> **状态声明（2026-07-02）：准备阶段已完成；当前仍停留在 gated implementation 第二段（core call-site fallback wiring）。`segment 3a` 的离线 compile evidence 已开始，但 current artifacts 只描述 latest compile attempt 产物；任何 `artifact_kind=dxil_ir_text`、`semantic_status=entry_shell_only` 的 IR 文本只能是 debug evidence，不是真实 DXIL container；非平凡 compute body 仍未真实 lowering，因此 pass 仍未实现。**
> 第一段交付了可验证的 disabled/fallback wiring（bridge gate 恒回退、per-pass 设置默认 `disabled`）。第二段（见 §9）通过 patch 0003 把 Godot core Auto Exposure call site 接线到 opt-in gate：只有 module 设置开启且 bridge `rxgd_record_pass` 返回 OK 时才跳过 Godot 原生 luminance；否则执行原生路径。
> 因 bridge 对 `RXGD_PASS_LUMINANCE_REDUCTION` 恒返回 `RXGD_STATUS_FALLBACK`、per-pass 设置默认 `disabled`，实测仍走 Godot 原生 luminance 路径。
> 本文件不宣称视觉验证通过、fallback 真接入（引擎内实测）或性能提升；`segment 3a` 即使只拿到 IR text / entry shell / compile blocker evidence，也不等于 runtime 可用。
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

## 4. 输入 / 输出资源（占位，未接入）

- 输入：`hdr_internal_texture`（HDR internal color target）。
- 中间：`luminance_reduce_buffers`（分级降采样缓冲链）。
- 输出：`current_luminance_buffer`（供 tonemap / auto-exposure 消费）。

以上均为占位描述，尚未接入任何 Rurix 资源映射。

## 5. Dispatch 形态（占位，未定）

候选二选一，形态待定，不写真实线程组数字，不宣称性能：

- `compute_reduce_pyramid`：compute 分级降采样金字塔。
- `raster_fragment_reduce`：raster fragment 路径降采样。

## 6. Fallback

- fallback reason 枚举（对齐 GRX-008 五枚举）：
  - `compile_failed`
  - `validation_failed`
  - `unsupported_device`
  - `visual_diff_failed`
  - `manual_disabled`
- 任一 compile / validation / visual / perf 失败 → 回退到 Godot 原生 luminance 路径（`godot_native_luminance`）。

## 7. Evidence 要求

- Visual：复用 GRX-007 `visual_diff.py` 的 LDR per-channel diff；只有存在真实 reference + candidate 帧并成功计算 diff 才算数。
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
- 若离线脚本真实运行后得到 `compile_failed`：
  - 必须把 blocker 分类、stderr/stdout 摘要、artifact 缺失情况写入 `offline_compile_evidence.json`；
  - 这只算 blocker evidence complete，不算 ready；
  - `pass_manifest.json` 必须继续保持 `implementation_status.segment = 2` 与 `real_gpu_pass = false`。
- 当前最新结果：`offline_compile_evidence.json` 已记录 `status=compile_failed`，blocker 为 `body_lowering_missing`。current artifacts 必须来自 latest compile attempt；若保留 `artifact_kind=dxil_ir_text`、`semantic_status=entry_shell_only` 的 LLVM IR 文本，只能作为 debug evidence，不能作为 real DXIL luminance pass artifact 或 compile-ready evidence。
- 只有在真实 `DXIL container + root signature + descriptor layout` 三类 artifact 全部可追溯，且不再是 entry shell-only 时，manifest 才允许推进到 segment 3；否则不得把 blocker 写成 ready。

## 11. 仍未完成项

- 真实 Rurix GPU luminance pass runtime 接入。
- 真实资源映射与 Godot/Rurix 句柄对齐。
- 真实 visual diff evidence（当前 visual evidence 仍为 SKIP/placeholder）。
- 真实 measured fallback telemetry（当前 telemetry 样例仍为 scaffold）。
- full baseline / Rurix measured_local 对比数据。
- 任何性能提升声明。
