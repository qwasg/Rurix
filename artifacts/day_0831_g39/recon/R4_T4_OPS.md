# R4 — T4 profiling 门与战役运维交接单(2026-08-31)

## T4 现状(`ci/g31_profiling_smoke.py`,门 `g31.waveC.profiling`)

- 流程:schema 检查 → `cargo build --release -p rurix-render --features vendor-upscale --bin g31_window_present --bin g14_3_pipeline_perf --quiet` → dev-env 探针 → 锁内四腿:g31 off/on(`--hidden --quality off`,24f/warmup6)+ g14 off/on(`--bench bistro-interior t100 tsr_device`,24f/warmup6)。**identity 每 bin 只判 1 次(on 腿 profile 单值 mean)——无多轮。**
- 判据(L167-177 `identity_ok`):`gs <= rw + 0.10 and -0.10 <= hr <= 2.00`;容差常量 L92-94(`IDENTITY_GPU_TOL_MS=0.10`/`IDENTITY_HOST_TOL_MS=2.00`)。**容差四面同源硬编码,不在 g31_budget.json**:①脚本常量 ②`milestones/g31/g31_profile_output_schema.json` identity const(selftest L921-924 互核 0.1/2.0)③双 bin identity JSON 字面(g31_window_present.rs L2049 / lane_body L15890:`"rule":"gpu_sum_mean<=render_wall_mean+0.10 && -0.10<=host_residual_mean<=2.00"`)④docs/renderer/profiling_debugging.md。**门 evidence schema 钉死 `profiles/g14/identity_ok: True`**(改判据须 schema 同步 `_patch`)。
- 残差口径:g14 腿 `host_residual = production_wall − (cpu_record+cpu_submit+cpu_fence_wait)`(readback_convert 属 tail 不入,lane_body L15757-15762);g31 腿含 readback_convert(L2060)。
- 7 facts 闭集(L124-132):唯一红 = `identity_sum_matches_frame`;恒绿 6 = profile_schema_compliant/pass_decomposition_measured/debug_labels_recorded/profiler_zero_render_drift/capture_compat_verified/tool_probe_registered。
- 三轮红形态(day_0830_delivery/CAMPAIGN_LOG L111):轮1 g14 −0.117433 / 轮2 g14 −0.288 / 轮3 换腿 g31 +2.250(wall 5.29→6.40 抖动);两腿各有全绿轮;历史绿 +0.070/+0.134 贴边。轮1 evidence = `.tmp/g31_gates/profiling/gate_fail_20260830T074237Z.json`。处置出路已登记:「容差重标定/identity 判据鲁棒化(多轮中位)归 budget 程序窗」。
- evidence:PASS → `evidence/g31_profiling_<ts>.json`;FAIL → `.tmp/g31_gates/profiling/gate_fail_<ts>.json`。命令:`py -3 ci/g31_profiling_smoke.py --selftest` / `--gate g31.waveC.profiling`。
- `_patch` 幂等纯追加范式:`ci/_patch_g31_wp_hlod_schemas.py`(锚文本唯一、token 驻留 0/1 判定、插入后 compile 自检、io.open newline="" 字节面保全);check_schemas 三处注册(load/validator/route,route 例:`f.name.startswith("g31_profiling_") → g31_profiling_evidence_schema.json` L5922-5928)。

## T4 处置(本役)

多轮中位:单门跑内 identity 采样腿(g31 on / g14 on)跑 N=5 轮,`identity_sum_matches_frame` 消费逐腿 gpu_sum/render_wall/host_residual mean 的**中位数**;容差 [−0.10,2.00] 字面不动(四面同源零触碰);off 腿/其余 facts 口径不变(zero_drift 断言各轮 on digest 位级恒值,顺带强化);evidence 追加 rounds 明细 + median 消费块(schema `_patch` 纯追加 + check_schemas PASS + selftest 同步)。若中位仍红:如实维持红 + 重标定提案登记(budget 程序窗,禁改判据凑绿)。

## 运维(主 agent)

- CPU 守卫 7/7 逐字:`py -3 ci/check_schemas.py` / `py -3 ci/budget_eval.py`(期望 330 pass 0 skip)/ `py -3 ci/gpu_device_lock.py --selftest` / `py -3 ci/g31_encode_parity_smoke.py --selftest` / `py -3 ci/g31_texture_sampling_smoke.py --selftest` / `py -3 ci/g31_vendor_license_smoke.py --selftest` / `py -3 ci/g31_blocked_probes_smoke.py --selftest`。
- GPU 锁:`sys.path.insert(0, str(ROOT/"ci")); from gpu_device_lock import gpu_device_lock; with gpu_device_lock(purpose=...)`;锁文件 `%TEMP%\rurix-gpu-device.lock`;缺省 timeout 3600s(soak 用 4h);g36 门首跑锁超时红=排队非门语义先例。
- 门回归命令:`py -3 ci/g31_profiling_smoke.py --gate g31.waveC.profiling` / fif probe(`g31_fif_dyn_probe.exe --selftest`,`--frames 48 --rays 96x72 [--action refit] --out <ev>`)+ `py -3 ci/calibrate_fif_budget.py --check` / frame_cut probe(`g31_frame_cut_probe.exe --cluster-pack .tmp/g36_gates/wave1_geo_composition/bistro.rxcp --error-px 2.0 --frames 16 --step-m 0.15 --res 96x54 --refit-copy incr|full [--min-level 1] --evidence <ev>`,判读 incr==full 16 帧逐字节)/ `py -3 ci/g31_wp_hlod_smoke.py --gate g31.wave95.wp_hlod` / `py -3 ci/g36_geo_composition_smoke.py --gate g36.wave1.geo_composition` / `py -3 ci/g31_restir_wiring_smoke.py --gate g31.waveB.restir`。
- 锚/soak:`G38_ANCHORS.json` 全串已核(见 CAMPAIGN_LOG 锚表);soak 形 = `g38_soak.py`(MIN_WALL_S=1800,32f 迭代,it4/it9 Stage A 探针,BUDGET_MS=11.111;G39 版锚断言钉死 full19 现锚非自举)。
- TODO 表:现最新修订行 v1.2.1(表头版本行 v1.1.3 陈旧不回写);G39 追加 v1.2.2 行,修订记录表只追加。
