# GRX CI 门禁增量

> 所属契约:[GRX_CONTRACT.md](GRX_CONTRACT.md)
> 版本:v1.14(2026-07-02)
> 基线:沿用现有 Rurix host/device 回归网;本文只规定 Godot/Rurix 集成增量门禁。
> 铁律:建设期 SKIP 必须写明原因;close-out strict 不允许 SKIP、estimated 或无证据性能声明。

---

## 1. PR Smoke

| # | 步骤 | 命令 | 失败处理 |
|---|---|---|---|
| GRX-S1 | Rust 格式 | `cargo fmt --check` | 失败即红 |
| GRX-S2 | Rurix Godot ABI 单测 | `cargo test -p rurix-godot` | 失败即红 |
| GRX-S3 | DXIL/shader-stages 回归 | `cargo test -p rurixc --features "dxil-backend shader-stages"` | 失败即红 |
| GRX-S4 | Godot/Rurix bridge smoke | `py -3 ci/godot_rurix_bridge_smoke.py` | 失败即红 |
| GRX-S5 | ignored source/state 检查 | `git check-ignore -v --no-index external/godot-master .cursor/settings.json .kiro/state.json .kimi/state.json .trae/state.json .claude/state.json .vscode/settings.json .idea/workspace.xml .windsurf/state.json .zed/settings.json` | 失败即红 |
| GRX-S6 | Godot source 不入 Git | `git status --porcelain -- external/godot-master`; `git ls-files external/godot-master .cursor .kiro .kimi .trae .claude .vscode .idea .windsurf .zed` | 任一输出非空即红 |

## 2. Local Godot Gate

`GRX.0` 只固化文档基线;本节及 `GRX-001` detector / `GRX-002` build / `GRX-003` load smoke 的实际执行都属于 `GRX.1`。本地推荐先跑 wrapper gate 记录稳定命令与产物 evidence:

```powershell
py -3 ci\godot_rurix_scons_build.py
```

裸 SCons 仍是规范目标命令:

```powershell
scons platform=windows target=template_debug d3d12=yes module_rurix_accel_enabled=yes disable_path_overrides=no
```

建设期规则:

- 若 `scons` 或 Python `SCons` 不存在,GRX-001 detector 必须输出明确 SKIP reason。
- 若 detector 未发现可用 SCons launcher,必须输出 `recommended_scons_command: null` 与明确下一步(如 `install_or_enable_scons`),不得给出已知会失败的 `scons ...` 推荐命令。
- 不允许安装或修改全局工具链作为 CI 步骤副作用。
- detector 默认以 workspace-local `LOCALAPPDATA=H:\rurix\target\grx\localappdata` 解析 Godot Windows build deps。
- `build_ready PASS` 必须同时覆盖 Godot 自身配置阶段会检查的 workspace-local AccessKit 与 D3D12 Mesa/NIR deps,不能只看 SCons/MSVC/Windows SDK/DXC。
- 若 AccessKit deps 缺失,detector 必须保持 `build_ready = false`,并给出 workspace-local AccessKit 安装命令;默认口径下不得一边维持 `build_ready = false` 一边推荐默认 SCons build。若未来要支持 `accesskit=no`,必须新增单独 readiness 字段,不能复用 `build_ready`。
- 若 D3D12 Mesa/NIR deps 缺失,detector 必须保持 `build_ready = false`,并给出 workspace-local 安装命令;`d3d12=yes` 目标下不允许建议 `d3d12=no`。
- `dxv.exe` 缺失默认记为 warning/optional tool missing,服务于后续 DXIL/device validation;它不阻塞 `GRX-002` Godot SCons build。
- 仅当 detector 输出 `build_ready PASS` 时才实际运行 `scons platform=windows target=template_debug d3d12=yes module_rurix_accel_enabled=yes disable_path_overrides=no`。
- 裸 SCons 是规范目标;当前本机 MSVC 14.44 因 C1001 ICE,推荐通过 `py -3 ci\godot_rurix_scons_build.py` 固定 wrapper 内部 `num_jobs=1 verbose=yes angle=no silence_msvc=no` workaround 记录 evidence。
- wrapper summary 必须归档 Godot build log,并在 `target/grx/godot_scons_build_summary.json` 中记录 Godot exe、console exe、module lib 的 `path / exists / size_bytes / mtime_utc / sha256`。
- wrapper summary 的 `command` / `ice_workaround_command` 必须包含 `disable_path_overrides=no`,并输出 `required_scons_args_satisfied` 或 `path_overrides_ready` 之类的 readiness 字段。
- probe 的 `build_artifacts_ready` 不得只看文件存在;必须同时要求 summary 中的 required artifacts evidence 完整且 `disable_path_overrides=no` 条件满足。若命令证据缺失,必须输出 `next_action=rebuild_godot_with_path_overrides` 与 `next_command=py -3 ci\godot_rurix_scons_build.py`。
- 若 wrapper 最新一次非零退出,但 required artifacts 与 path-overrides evidence 已齐备,probe 可以把该状态降为 warning,而不是阻塞 `GRX-003` / `GRX-004` 接续。
- 任一关键产物缺失时,即使 SCons exit 0,也不得写 `status: success`。
- `rurix_godot.dll` present/missing 两种启动路径都必须验证;`godot_rurix_load_smoke.py` 生成的项目文件只允许落在 `target/grx/godot-load-smoke`,Godot 启动必须通过 `--path` 指向该目录。
- 若需清理 `external/godot-master/bin` 的旧残留,`project.godot` / `main.gd` 只允许按 marker 删除;`main.tscn` 只允许按 marker 或精确 legacy fingerprint 删除,不得泛删 Godot bin 文件。
- missing DLL case 必须保持 fallback,且若日志出现 `RurixAccel: D3D12 Forward+ bridge session ready.` 必须判失败并在 summary 记录 `unexpected_markers`。

## 3. Benchmark Gate

建设期 normal gate:

- `GRX-004` 已以 fresh per-scene smoke 通过:`target/grx/godot_bench_project_smoke_summary.json` 当前应记录 `scene_count=7`、`failure_count=0`,且 7 个 scene 都有独立 Godot load evidence。
- `GRX-005` 建设期 runner 的目标产物是 raw frame sample JSON 与 `target/grx/godot_bench_runner_summary.json`,而不是 baseline/Rurix 对比结果。
- `GRX-005` runner 必须扫描 Godot 日志 failure marker(对齐 `bench_project_smoke.py`):允许已知 `ERROR: Could not load global script cache.`(含其 `at: ProjectSettings::get_global_class_list` 上下文行)作为 warning,但其它 `ERROR:` / `SCRIPT ERROR:` / `Parser Error:` / `Parse Error:` / `Failed loading resource:` / `Failed loading script` 必须让该 scene fail;`per_scene_results` 必须记录 `failure_markers` / `warnings`,summary 必须给出聚合 `warning_count`。runner 不带 `--verbose`,因此不复用 smoke 的 “缺 Loading resource 载入证据即失败” 规则。
- `GRX-006` 已交付 baseline/perf evidence schema 与 strict perf gate 输入格式:`spike/godot-rurix/bench/schemas/baseline_evidence.schema.json`、`spike/godot-rurix/bench/schemas/perf_gate_input.schema.json`(draft-07);`perf_gate.py` 支持 `--kind {perf_gate,baseline}`、`--strict`、`--validate-only`,并把 `FORMAT FAIL` 与 `PERF FAIL` 分开输出。
- strict perf gate(`--strict`)必须拒绝:任何 SKIP / estimated 标记、`quick_smoke` 或非 full run_mode、缺任一固定 scene、缺 `*_raw_artifact_path`、`warmup/sample != 300/2000`、backend/resolution/vsync/evidence_level 不符。
- `GRX-006` hardening 后的三条强化规则:
  - strict forbidden marker 采用词边界正则(下划线视作词字符),命中文档任意字符串或 key 中独立出现的 `skip` / `skipped` / `estimated`;必须命中 `SKIP: missing`、`skip-reason`、`status=SKIP`、`estimated:true`、`estimated local`,不得误伤 `spike`、`quick_smoke`、普通路径分隔片段。
  - baseline reader(`--kind baseline`)必须校验每个 scene 的 `sample_count` 为正整数,且当顶层 `sample_frames` 为正整数时要求 `sample_count == sample_frames`,与 `baseline_evidence.schema.json` 对齐。
  - strict perf gate 若输入含 `thresholds`,其三项必须等于固定值 `geomean_fps_ratio_min=1.5` / `p95_frame_time_reduction_min=0.3` / `single_scene_fps_ratio_min=0.95`,任一被覆盖为其它值即 FORMAT FAIL。
- GRX-006 红测样例(必须可复现):`samples/perf_gate_forbidden_skip_example.json`(含独立 `SKIP: missing` 标记,`--strict` 必 FORMAT FAIL);`samples/baseline_missing_sample_count_example.json`(缺一个 scene 的 `sample_count`,`--kind baseline --validate-only` 必 FORMAT FAIL)。
- `quick_smoke` 证据(含 baseline schema 的 quick_smoke 文档)只能作为 smoke evidence,不能作为 strict perf gate 输入。
- benchmark runner 可 SKIP,但必须说明缺少哪一项:Godot build、scene project、D3D12 runtime、GPU timestamp、visual capture、baseline JSON。
- `gpu_timestamps_available=false` 在 `GRX-005` 建设期可接受,但必须显式写入 raw JSON;禁止伪造 GPU timestamp 或把占位值写成真实测量值。
- quick/full 模式都只能声明采样 runner 已完成并给出帧数、路径与 raw artifacts,不能宣称 perf gate、baseline 对比或任何性能提升。
- 任何已有 evidence JSON 必须可被 schema/reader 解析。
- 不允许把 estimated 写入 close-out 输入。

Close-out strict gate(`GRX-006+ / close-out`,本次不触发):

```powershell
py -3 spike/godot-rurix/bench/perf_gate.py --strict <results.json>
```

`perf_gate.py` 输入格式见 `spike/godot-rurix/bench/schemas/perf_gate_input.schema.json`;strict close-out 必须用 `--strict` 且提供 full-mode measured_local 输入。必须同时满足:

| 指标 | 阈值 | 字段名 |
|---|---:|---|
| 几何平均 FPS ratio | >= 1.5 | `geomean_fps_ratio_min` |
| p95 frame time reduction | >= 0.30 | `p95_frame_time_reduction_min` |
| 单场景 FPS ratio 下限 | >= 0.95 | `single_scene_fps_ratio_min` |

Close-out 证据必须包含:

- scene-level baseline FPS / Rurix FPS。
- scene-level baseline p95 ms / Rurix p95 ms。
- raw sample path 或 run artifact path。
- `measured_local` evidence level。
- 同画质、同分辨率、同 Godot scene、同 D3D12 Forward+ 后端说明。

## 4. Visual And Fallback Gate

每个新增 pass 必须过:

| 门 | 要求 |
|---|---|
| visual reference | reference frame capture 与 Rurix frame capture 均存在 |
| diff policy | LDR pass 用 per-channel absolute diff; HDR/temporal pass 用 SSIM/PSNR + temporal stability |
| fallback telemetry | compile_failed、validation_failed、unsupported_device、visual_diff_failed、manual_disabled 至少覆盖实际失败路径 |
| pass matrix | pass 默认 enabled/disabled、适用场景、失败原因可查询 |
| red/green | 至少一种真实红绿:pass 输出错误、禁用 fallback、视觉 diff 超阈值 |

任一 visual 或 validation 失败时,该 pass 默认 disabled,Godot 原 pass 接管。

`GRX-007` scaffold 建设期规则:

- visual scaffold 产物:`spike/godot-rurix/bench/capture_reference_frames.py`、`visual_diff.py`、`schemas/visual_diff_evidence.schema.json`、`samples/visual_diff_placeholder.json`。
- schema 覆盖 7 个固定 scene,每 scene 至少一个 capture frame;`status ∈ {pass, skip}`。
- LDR 支持 per-channel absolute diff:仅当 `reference_frame_path` 与 `candidate_frame_path` 指向真实存在的帧文件时才计算逐通道 max/mean 绝对差;否则输出 SKIP。
- HDR/temporal 只在 schema 中声明字段(`ssim`/`psnr`/`temporal_stability` 等),值保持 null,建设期不产出结果、不伪造。
- 建设期允许 SKIP,但每个 SKIP 的 capture frame 必须写明具体缺失原因之一:`missing capture backend`、`missing Godot full run`、`missing frame artifact`。
- 除非真的有 reference 与 candidate frame 文件以及计算出的 diff,否则不得写“视觉验证已通过”;`visual_diff.py` 在无真实帧时打印 `SCAFFOLD ... visual verification is NOT done`。
- `visual_diff.py --validate-only` 只校验格式(7 scene、每 scene ≥1 frame、`status=skip` 必须有非空 `skip_reason`、`status=pass` 必须有 reference+candidate path 与 `ldr_diff`),格式通过(含全 SKIP)退出码 0,格式错误退出码 1。
- `GRX-007` hardening:`status=skip` 的 capture frame 必须为纯 skip,不得携带伪造 diff/帧路径——`reference_frame_path`、`candidate_frame_path`、`ldr_diff`、`hdr_diff`、`temporal_diff` 必须为 null 或缺省;任一非 null 即 FORMAT FAIL(`visual_diff.py` 与 schema `else` 分支双侧约束)。红测样例:`samples/visual_diff_skip_with_fake_ldr_example.json`(skip 带伪造 `ldr_diff`,`--validate-only` 必 FORMAT FAIL)。已有红绿保持:`samples/visual_diff_pass_missing_ldr_example.json`(pass 缺 `ldr_diff` FORMAT FAIL)、`samples/visual_diff_mismatch_example.json`(pass 真实帧但记录 diff 不一致 DIFF FAIL)、`samples/visual_diff_ldr_pass_example.json`(pass 记录 diff 与计算一致 PASS);`--write-output` 生成带 computed `ldr_diff` 的 evidence JSON。
- `GRX-007` 收尾 hardening:`status=pass` 的 capture frame 承诺存在可比对的真实帧对,若其 `reference_frame_path`/`candidate_frame_path` 帧文件在磁盘上缺失、不可读、不是合法 channel 文档、或两帧 channel 数量不一致,`visual_diff.py` 必须 `DIFF FAIL` 且非零退出,不得降级为 SKIP;`--write-output` 模式在任一 `status=pass` 帧算不出 diff 时必须失败且不写出 evidence。红测样例:`samples/visual_diff_pass_missing_frame_artifact_example.json`(pass 帧带合法 `ldr_diff` 但帧文件不存在,运行 `visual_diff.py` 必 DIFF FAIL / 非零退出)。`ci/godot_rurix_toolchain_probe.py` 的 `grx007_visual_ready` 已把该红测纳入红绿证据。

`GRX-008` scaffold/hardening 建设期规则:

- fallback telemetry scaffold 产物:`spike/godot-rurix/bench/schemas/fallback_telemetry.schema.json`、`fallback_telemetry.py`、`samples/fallback_telemetry_placeholder.json`。`fallback_reason` 枚举五值:`compile_failed`、`validation_failed`、`unsupported_device`、`visual_diff_failed`、`manual_disabled`。
- `fallback_telemetry.py` 与 schema 区分 scaffold 与 full/measured_local:
  - scaffold(`run_mode=scaffold` / `evidence_level=scaffold`)允许 `telemetry_timestamp=null`、`telemetry_frame=null`,但每个 pass 必须 `enable_state=disabled` 且 `godot_fallback_active=true`。
  - full(`run_mode=full` 或 `evidence_level=measured_local`)要求 `telemetry_timestamp` 非空、`telemetry_frame` 为非负整数;null 即 FORMAT FAIL。
  - `evidence_level=measured_local` 不允许 `pass_id` 以 `placeholder_` 开头(`fallback_telemetry.py` 与 `fallback_telemetry.schema.json` 双侧约束,schema-only 校验不再比脚本松)。
- `GRX-008` 红测样例(必须可复现):`samples/fallback_telemetry_full_null_timestamp_example.json`(full/measured_local 但 timestamp/frame 为 null,`--validate-only` 必 FORMAT FAIL);`samples/fallback_telemetry_scaffold_fallback_inactive_example.json`(scaffold 但 `godot_fallback_active=false`,`--validate-only` 必 FORMAT FAIL)。`samples/fallback_telemetry_placeholder.json` 仍 FORMAT PASS,且明确它不是实际 fallback telemetry(不代表任何 pass 已接入或发生真实 fallback)。

`GRX-009` luminance reduction pass 准备期 + gated implementation 分段规则(pass 本体仍未完成):

- 准备产物:`spike/godot-rurix/passes/luminance_reduction/PASS_CONTRACT.md` 与 `pass_manifest.json`。二者仅记录 `pass_id=luminance_reduction`、目标场景(`post_fx_chain`、`mixed_forward_plus`)、Godot 侧候选 hook/source 调查(路径 + 函数,不改 `external/godot-master`)、输入/输出资源占位、dispatch 形态占位、fallback reason(对齐 GRX-008 五枚举)、visual/perf evidence 要求;pass 默认 `disabled`,任何 compile/validation/visual/perf 失败走 Godot 原生 luminance 路径。
- `ci/godot_rurix_toolchain_probe.py` 的 `grx009_prep_ready` 只负责准备产物与 Godot manifest path 证据判定:两个准备产物存在、`pass_manifest.json` 可解析且 `pass_id==luminance_reduction`、`implemented==false`、`default_enable_state==disabled`、`target_scenes` 含 `post_fx_chain` 与 `mixed_forward_plus`;manifest `godot_hook_investigation` 记录的 effect_class header/source、全部 shaders、全部 call_sites file 必须为相对路径且确实存在于 `external/godot-master` 之下(resolve 后不得逃逸快照根;检查只读,不修改快照),任一缺失即 not ready。未就绪时 `next_action=start_grx009_luminance_reduction_pass_contract`;准备就绪后仅表示可以进入实现阶段,**不代表任何实际 pass 已实现或任何性能/视觉结论已达成**。
- `grx009_segment1_ready` 代表第一段 gated scaffold 的历史交付点:manifest `implementation_status.segment == 1`、`real_gpu_pass == false`、`godot_core_call_site_wired == false`,且 0002 patch、`samples/fallback_telemetry_luminance_disabled_example.json`、`src/rurix-godot` 中的 `LuminanceReductionGate` 关键标记齐备,并且 disabled sample 必须能通过 `fallback_telemetry.py --validate-only`。当 manifest 已推进到 segment 2 或更后段时,该布尔值可能为 `false`;这表示当前状态已越过 segment 1,**不**表示回归。命中时 `next_action=start_grx009_luminance_core_callsite_fallback_wiring`。
- gated implementation 第一段(gated scaffold)规则:`src/rurix-godot` 的 `LuminanceReductionGate` 默认 disabled,`request_enable` 恒失败 `compile_failed`(无已编译 Rurix DXIL luminance kernel),`rxgd_record_pass` 对 `RXGD_PASS_LUMINANCE_REDUCTION` 恒返回 `RXGD_STATUS_FALLBACK` 且不得累加 estimated GPU/CPU time(C ABI v1 不变);红绿由 `cargo test -p rurix-godot` 承担(default-disabled 回退 + enable 失败)。Godot 侧栈式 `0002-rurix-accel-luminance-pass-gate.patch` 基于 0001、仅改 `modules/rurix_accel/*`;per-pass 设置 `rendering/rurix_accel/passes/luminance_reduction/enabled` 默认 false,`try_record_luminance_reduction()` 在设置关闭、session/符号缺失或 bridge 返回非 `RXGD_STATUS_OK` 时必须返回 false(调用方走 Godot 原生 luminance 路径)。
- `grx009_segment2_ready` 代表第二段 core call-site fallback wiring 已交付:manifest `implementation_status.segment == 2`、`real_gpu_pass == false`、`godot_core_call_site_wired == true`,且 0002/0003 patch、`samples/fallback_telemetry_luminance_callsite_wired_disabled_example.json` 与 `LuminanceReductionGate` 关键标记齐备。这里不允许只靠 patch 文本或 JSON 可解析来判 ready:callsite-wired sample 必须真能通过 `fallback_telemetry.py --validate-only`,同时 patch stack 必须通过共享四态检查并真实落在 `0001+0002+0003`。命中后 `next_action=start_grx009_luminance_reduction_real_gpu_pass`。
- gated implementation 第二段(core call-site fallback wiring)规则:Godot core 修改只能通过 `spike/godot-rurix/patches/0003-rurix-accel-luminance-core-callsite-wiring.patch` 管理,不得把 `external/godot-master` 纳入 Git。0003 只在 Auto Exposure `luminance_reduction` 调用点前加入 opt-in gate:只有 module 设置开启、bridge session/record symbol 可用且 `rxgd_record_pass` 返回 `RXGD_STATUS_OK` 时才跳过 Godot 原生 luminance;否则必须执行原来的 `luminance_reduction`。当前 bridge 对 `RXGD_PASS_LUMINANCE_REDUCTION` 恒返回 `RXGD_STATUS_FALLBACK`,所以 segment 2 实测语义仍是 Godot 原生路径接管。
- `ci/godot_rurix_bridge_smoke.py` 必须校验 patch 栈四态(base 未应用 / 仅 0001 / 0001+0002 / 0001+0002+0003),任何 drift 即红;并校验 0002 与 0003 的关键标记(setting key、`RXGD_PASS_LUMINANCE_REDUCTION`、`rxgd_record_pass`、`try_record_luminance_reduction`、`D3D12Hooks::get_singleton`、`renderer_scene_render_rd.cpp`)。
- GRX-009 disabled/fallback wiring 状态记录样例:`samples/fallback_telemetry_luminance_disabled_example.json` 与 `samples/fallback_telemetry_luminance_callsite_wired_disabled_example.json` 均为 scaffold 级,`--validate-only` 必 FORMAT PASS;它们**不是** measured telemetry,不得当作真实 fallback 事件或接入证据。
- `grx009_segment3a_compile_ready` 代表离线 compile artifact 已真正齐备:manifest 与 `offline_compile_evidence.json` 必须一致记录 `status=success`,且 latest compile attempt 真实产生并可追溯的 `DXIL container + root signature + descriptor layout` current artifacts 三者齐备,同时 `runtime_state` 仍为 `fallback_only`。`.dxil` 若是 LLVM IR 文本、`artifact_kind=dxil_ir_text`、`semantic_status=entry_shell_only`,或 stderr 记录 patched llc / validator SKIP,只能作为 debug/non-ready evidence,一律不得 ready。若离线脚本真实运行后得到 `compile_failed` 或 `toolchain_missing`,这只能算 blocker evidence complete,不得把 manifest 推进到 segment 3,也不得把 blocker 写成 ready;`next_action` 应继续指向修复真实 DXIL container/body lowering blocker,不得进入 resource mapping。

## 5. ABI Gate

`RXGD_ABI_VERSION` 是稳定边界。以下任一变化必须同 PR 更新 Rust、header、smoke 和 Godot patch:

- exported function signature。
- `RxGdCaps`、`RxGdResource`、`RxGdFrameStats` layout。
- status code、backend/render method 常量、pass id 常量。
- resource ownership 或 lifetime 约定。

验证命令:

```powershell
cargo test -p rurix-godot
py -3 ci/godot_rurix_bridge_smoke.py
cargo build -p rurix-godot
```

Godot 侧不得静默接受 ABI mismatch;必须 fallback 并记录 telemetry。

## 6. Red/Green 程序

每个 pass PR 至少执行一条:

1. **pass 输出错误**:注入错误输出或资源映射错误 -> visual/perf/validation gate 红 -> 复原绿。
2. **禁用 fallback**:强制 pass 返回 fallback -> Godot 原 pass 接管且 telemetry 记录 -> 复原绿。
3. **视觉 diff 超阈值**:调低阈值或篡改 capture -> diff gate 红 -> 复原绿。

close-out 前必须汇总 red/green evidence URL 或本地 evidence 路径。

## 7. 当前验证状态

截至本文修订时,`GRX.0` 文档任务本身不要求 Godot benchmark,也不能据此宣称任何性能提升。`GRX-001/002/003` 已按 fresh evidence 在本地通过并分别产出 detector/build/load evidence;`GRX-004` 已以 fresh per-scene smoke 通过,`GRX-005` runner 已存在并硬化,`GRX-006` 已交付并硬化 baseline/perf schema,`GRX-007` 已完成 visual scaffold + hardening,`GRX-008` 已完成 fallback telemetry scaffold + hardening。GRX-009 现仍停留在第二段 core call-site fallback wiring 完成态:probe 保留 `grx009_prep_ready=true` 作为准备就绪信号,同时以 `grx009_segment2_ready` 标识当前 manifest segment 2 + `godot_core_call_site_wired=true` 的接线态。`segment 3a` 当前 blocked:`offline_compile_evidence.json` 记录 `compile_failed/body_lowering_missing`;current artifacts 只描述 latest compile attempt 产物,`artifact_kind=dxil_ir_text`、`semantic_status=entry_shell_only` 的 IR 只能作为 debug/non-ready evidence。这不是真实 DXIL container,也不是 real luminance pass compile success。`grx009_segment3a_compile_ready=false`,next action 必须指向修复真实 DXIL container/body lowering blocker,不得进入 resource mapping。**当前默认语义仍是 disabled/fallback:bridge 对 `RXGD_PASS_LUMINANCE_REDUCTION` 恒返回 `RXGD_STATUS_FALLBACK`,所以即使 0003 已接线,实测仍由 Godot 原生 luminance 路径接管。baseline full 实测对比、真实 visual diff、真实 fallback 接入(引擎内实测)、任何实际加速 pass 与性能提升声明仍属未完成;visual/telemetry evidence 仍不是 measured_local close-out 证据。** 当前已知可用验证包括:

- `cargo fmt --check`
- `cargo test -p rurix-godot`
- `cargo build -p rurix-godot`
- `py -3 ci/godot_rurix_bridge_smoke.py`
- `py -3 ci/godot_rurix_scons_build.py`
- `py -3 ci/godot_rurix_load_smoke.py`
- `cargo test -p rurixc --features "dxil-backend shader-stages"`
- `py -3 spike\godot-rurix\bench\generate_benchmark_project.py`
- `py -3 spike\godot-rurix\bench\bench_project_smoke.py`
- `py -3 spike\godot-rurix\bench\run_benchmark_scenes.py --quick-smoke`
- `py -3 spike\godot-rurix\bench\perf_gate.py --kind baseline --validate-only spike\godot-rurix\bench\samples\baseline_smoke_example.json`(格式校验 PASS)
- `py -3 spike\godot-rurix\bench\perf_gate.py --strict spike\godot-rurix\bench\samples\perf_gate_failing_example.json`(格式可解析,性能门 PERF FAIL,属预期红)
- `py -3 spike\godot-rurix\bench\perf_gate.py --strict spike\godot-rurix\bench\samples\perf_gate_forbidden_skip_example.json`(含 `SKIP: missing`,FORMAT FAIL,预期红)
- `py -3 spike\godot-rurix\bench\perf_gate.py --kind baseline --validate-only spike\godot-rurix\bench\samples\baseline_missing_sample_count_example.json`(缺 `sample_count`,FORMAT FAIL,预期红)
- `py -3 spike\godot-rurix\bench\visual_diff.py --validate-only spike\godot-rurix\bench\samples\visual_diff_placeholder.json`(格式 PASS,7 场景全部 SKIP)
- `py -3 spike\godot-rurix\bench\visual_diff.py --validate-only spike\godot-rurix\bench\samples\visual_diff_skip_with_fake_ldr_example.json`(skip 带伪造 ldr_diff,FORMAT FAIL,预期红)
- `py -3 spike\godot-rurix\bench\visual_diff.py --validate-only spike\godot-rurix\bench\samples\visual_diff_pass_missing_ldr_example.json`(pass 缺 ldr_diff,FORMAT FAIL,预期红)
- `py -3 spike\godot-rurix\bench\visual_diff.py spike\godot-rurix\bench\samples\visual_diff_mismatch_example.json`(pass 真实帧但记录 diff 不一致,DIFF FAIL,预期红)
- `py -3 spike\godot-rurix\bench\visual_diff.py spike\godot-rurix\bench\samples\visual_diff_ldr_pass_example.json`(pass 记录 diff 与计算一致,PASS)
- `py -3 spike\godot-rurix\bench\fallback_telemetry.py --validate-only spike\godot-rurix\bench\samples\fallback_telemetry_placeholder.json`(scaffold placeholder,FORMAT PASS)
- `py -3 spike\godot-rurix\bench\fallback_telemetry.py --validate-only spike\godot-rurix\bench\samples\fallback_telemetry_full_null_timestamp_example.json`(full/measured_local 但 timestamp/frame 为 null,FORMAT FAIL,预期红)
- `py -3 spike\godot-rurix\bench\fallback_telemetry.py --validate-only spike\godot-rurix\bench\samples\fallback_telemetry_scaffold_fallback_inactive_example.json`(scaffold 但 `godot_fallback_active=false`,FORMAT FAIL,预期红)
- `py -3 spike\godot-rurix\bench\visual_diff.py spike\godot-rurix\bench\samples\visual_diff_pass_missing_frame_artifact_example.json`(pass 帧指向不存在的帧文件,DIFF FAIL,预期红)
- `py -3 spike\godot-rurix\bench\fallback_telemetry.py --validate-only spike\godot-rurix\bench\samples\fallback_telemetry_luminance_disabled_example.json`(GRX-009 disabled/fallback wiring 状态记录,FORMAT PASS;非 measured telemetry)
- `py -3 spike\godot-rurix\bench\fallback_telemetry.py --validate-only spike\godot-rurix\bench\samples\fallback_telemetry_luminance_callsite_wired_disabled_example.json`(GRX-009 segment 2 callsite-wired disabled/fallback 状态记录,FORMAT PASS;非 measured telemetry)

当前未完成:

- 7 场景 full baseline benchmark 实测。
- baseline / Rurix 两组 full measured_local 对比数据。
- 真实 visual diff evidence(当前 visual evidence 全部为 SKIP/placeholder)。
- 真实 fallback 接入与 measured telemetry(当前 telemetry 全部为 scaffold placeholder)。
- 任何实际加速 pass(含 GRX-009 Tier 1 luminance reduction pass;当前虽已完成第二段 core call-site fallback wiring,但 bridge 恒 FALLBACK、默认 disabled、pass 本体未实现)。
- 达成 1.5x / p95 -30% 的性能提升声明(当前 runner evidence 仅 quick-smoke,不可用于 close-out)。

## 8. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-07-01 | 初版。定义 PR smoke、Local Godot gate、Benchmark strict gate、Visual/fallback gate、ABI gate、red/green 程序和当前验证状态。 |
| v1.1 | 2026-07-01 | 收紧 GRX.0 文档基线口径。把 strict 指标中文解释统一为 `p95 frame time reduction >= 30%`,并明确当前阶段只是文档任务,不跑 Godot build/benchmark,也不宣称性能提升。 |
| v1.2 | 2026-07-01 | 统一 `GRX.0` / `GRX.1` 边界:明确 detector/build/load 执行项属于 `GRX.1`,避免将 `GRX-001` 误读为 GRX.0 交付。 |
| v1.3 | 2026-07-01 | 收紧 GRX.1 detector/build 口径:缺少 SCons launcher 时 `recommended_scons_command` 必须为 `null` 并给出下一步;明确 `dxv.exe` 属于后续 DXIL/device validation warning,不阻塞 `GRX-002` Godot SCons build。 |
| v1.4 | 2026-07-01 | 统一 GRX.1 probe readiness 语义:默认只在 workspace-local AccessKit 与 D3D12 deps 都齐备时才允许 `build_ready PASS` 并推荐默认 SCons;不再用 `accesskit=no` 作为 `build_ready=false` 状态下的默认推荐命令。 |
| v1.5 | 2026-07-01 | 将 `py -3 ci\godot_rurix_scons_build.py` 提升为推荐 Local Godot Gate;明确裸 SCons 是规范目标、当前本机 MSVC 14.44 使用 wrapper single-job/ANGLE-disabled workaround;要求 GRX-002 summary 记录 exe/console exe/module lib 的 artifact evidence,并把 `godot_rurix_load_smoke.py` 纳入当前验证状态。 |
| v1.6 | 2026-07-01 | 修正 `GRX.1` close-out hardening 口径:明确 `GRX-001/002/003` 已本地通过;要求 load smoke 项目文件仅落在 `target/grx/godot-load-smoke`,missing-DLL case 需对 session-ready marker 做反向断言并记录 `unexpected_markers`;probe 在 build + load evidence 完整后推进到 `start_grx2_tier0_benchmark_skeleton`,但不得据此宣称 benchmark 已开始或性能已提升。 |
| v1.7 | 2026-07-01 | 以 fresh path-overrides rebuild / smoke 收口更新 Local Godot Gate:要求 build summary 显式给出 `disable_path_overrides=no` readiness 证据,probe 的 `build_artifacts_ready` 同时检查 required artifacts 与 path-overrides 条件,并允许把 latest wrapper 非零退出降为 warning;同时把 `main.tscn` 旧残留清理口径收紧为 marker 或精确 fingerprint。 |
| v1.8 | 2026-07-01 | 收口 `GRX-004`/`GRX-005` 建设期 benchmark gate:明确 `GRX-004` 已以 fresh per-scene smoke 通过,当前接续任务是固定 `warmup 300 / sample 2000 / vsync off / 1920x1080` 的 tracked runner;允许 `gpu_timestamps_available=false` 但必须显式记录,同时写死本次不触发 baseline schema/perf gate、visual diff 或任何性能提升声明。 |
| v1.9 | 2026-07-01 | 收口 `GRX-005` 硬化 / `GRX-006` 交付:Benchmark Gate 新增 runner failure marker 扫描规则(allowlist global script cache warning、记录 `failure_markers` / `warnings` / `warning_count`)与 GRX-006 schema/`--strict`/`--validate-only` 说明;strict close-out 命令改为 `perf_gate.py --strict <results.json>` 并指向 `perf_gate_input.schema.json`;当前验证状态纳入两个样例 JSON 校验命令,并写死 full baseline 实测、visual diff、加速 pass 与性能提升仍未完成、quick-smoke 不可用于 close-out。 |
| v1.10 | 2026-07-01 | 收口 `GRX-006` hardening / 接续 `GRX-007` scaffold:Benchmark Gate 新增 strict forbidden marker 词边界正则、baseline `sample_count`(且 `== sample_frames`)、strict `thresholds` 固定值(1.5/0.3/0.95)三条强化规则与两个红测样例说明;Visual/Fallback Gate 新增 `GRX-007` scaffold 建设期规则(7 scene × ≥1 frame、LDR per-channel absolute diff 需真实帧、HDR/temporal 仅声明、SKIP 需具体原因、无真实帧不得写视觉验证通过);当前验证状态更新为 probe `next_action=start_grx007_visual_diff_scaffold`,纳入两个红测与 `visual_diff.py --validate-only` 命令,并重申 full baseline / 真实 visual diff / Rurix 加速 pass / 性能提升仍未完成。 |
| v1.11 | 2026-07-01 | 收口 `GRX-007` hardening / `GRX-008` scaffold hardening / 接续 `GRX-009` 准备:Visual/Fallback Gate 新增 `GRX-007` skip-禁止伪造 diff/帧路径规则与红测 `visual_diff_skip_with_fake_ldr_example.json`,并补齐 pass 缺 ldr_diff / diff 不一致 / diff 一致 / `--write-output` 红绿说明;新增 `GRX-008` scaffold/full 区分规则(scaffold 允许 timestamp/frame=null 但必须 disabled + `godot_fallback_active=true`;full/measured_local 要求 timestamp 非空、frame 非负整数;measured_local 禁止 `placeholder_` pass_id)与两个红测样例说明,placeholder 仍 FORMAT PASS;当前验证状态更新为 probe `grx007_visual_ready=true`/`grx008_telemetry_ready=true`、`next_action=start_grx009_luminance_reduction_pass_prep`,纳入新增 visual/fallback 红绿命令,并把“未完成”补充真实 fallback 接入/telemetry 与 GRX-009 仅为准备。本轮不实现任何实际 Rurix 加速 pass,不宣称视觉验证、fallback 真接入或性能提升已完成。 |
| v1.12 | 2026-07-01 | 收尾 `GRX-007`/`GRX-008` hardening / 产出 `GRX-009` 准备:Visual/Fallback Gate 新增 `GRX-007` 收尾规则(`status=pass` 帧在帧文件缺失/不可读/非合法 channel 文档/channel 数量不一致时必 DIFF FAIL 且非零退出,`--write-output` 拒绝写出)与红测 `visual_diff_pass_missing_frame_artifact_example.json`;注明 `GRX-008` `measured_local` 禁止 `placeholder_` pass_id 现为 schema 与脚本双侧约束;新增 `GRX-009` luminance reduction pass 准备期规则(准备产物 `PASS_CONTRACT.md` + `pass_manifest.json`、`grx009_prep_ready` 证据判定、`next_action=start_grx009_luminance_reduction_pass_contract`/就绪后 `start_grx009_luminance_reduction_pass_implementation`);当前验证状态同步更新。GRX-009 仍只是准备;本轮不实现任何实际 Rurix 加速 pass,不宣称视觉验证、fallback 真接入或性能提升已完成。 |
| v1.13 | 2026-07-02 | 收口 GRX-009 准备 / 交付 gated implementation 第一段:标题版号补齐至修订行(v1.12 后续);`grx009_prep_ready` 加强为校验 manifest 记录的 Godot source/header/shader/call-site 文件存在于 `external/godot-master`(相对路径、不逃逸快照根、只读);新增 GRX-009 第一段 gated 规则(bridge `LuminanceReductionGate` 恒 `RXGD_STATUS_FALLBACK` 且不累加 estimated time、栈式 0002 module patch 默认关且非 OK 走原生路径、bridge smoke patch 栈三态检查、disabled telemetry 样例 FORMAT PASS 但非 measured);当前验证状态纳入 pass-缺帧红测与 luminance disabled 样例命令,并把 GRX-009 未完成口径改为第一段 gated scaffold/pass 本体未实现。 |
| v1.14 | 2026-07-02 | 收口 GRX-009 第二段 core call-site fallback wiring:Visual/Fallback Gate 的 GRX-009 规则改为分段 gate 语义,保留 `grx009_prep_ready` 只做准备产物/manifest path 校验,新增 `grx009_segment1_ready` / `grx009_segment2_ready` 与 segment-aware `next_action`;新增 0003 core call-site patch 规则,bridge smoke 改为 patch 栈四态检查,并补入 callsite-wired disabled scaffold 样例命令。当前验证状态同步改为 segment 2 完成态,同时明确默认仍是 disabled/fallback,真实 GPU pass、真实 visual diff、measured telemetry 与性能提升声明仍未完成。 |
| v1.15 | 2026-07-02 | 收口 GRX-009 review findings 并接续 `segment 3a`:明确 `grx009_segment1_ready` 是历史交付点而非当前 segment 2 的必经真值;把 `grx009_segment2_ready` 收紧为必须同时通过 callsite-wired telemetry sample 的 `fallback_telemetry.py --validate-only` 与共享 patch 栈四态检查(`0001+0002+0003`),不再接受仅 patch 文本/JSON 可解析;新增 `grx009_segment3a_compile_ready` 规则,要求真实 `DXIL + root signature + descriptor layout` artifact 三者齐备且 runtime 仍 `fallback_only`,compile_failed 仅算 blocker evidence complete,不得推进 segment 3。 |
| v1.16 | 2026-07-02 | 修复 GRX-009 segment 3a artifact gate:ready gate 现在拒绝 LLVM IR 文本、`artifact_kind=dxil_ir_text`、`semantic_status=entry_shell_only` 与 toolchain SKIP stderr;当前 evidence 为 `compile_failed/body_lowering_missing`,current artifacts 只代表 latest compile attempt 产物,manifest 保持 segment 2,next action 指向真实 DXIL container/body lowering blocker,不进入 resource mapping。 |
