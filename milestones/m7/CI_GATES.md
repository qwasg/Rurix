# M7 CI 门禁增量

> 所属契约:[M7_CONTRACT.md](M7_CONTRACT.md)
> 版本:v1.0(2026-06-15)
> 基线:[../m0/CI_GATES.md](../m0/CI_GATES.md) + [../m1/CI_GATES.md](../m1/CI_GATES.md) + [../m2/CI_GATES.md](../m2/CI_GATES.md) + [../m3/CI_GATES.md](../m3/CI_GATES.md) + [../m4/CI_GATES.md](../m4/CI_GATES.md) + [../m5/CI_GATES.md](../m5/CI_GATES.md) + [../m6/CI_GATES.md](../m6/CI_GATES.md)(全部沿用:runner 约定、PR Smoke 1–28 步、guardrail 含 M1.1/M1.2/M1.4/M3.3/M4.2/M4.3/M5.4/M6 激活项、nightly 工作流含 Compute Sanitizer racecheck+memcheck + measured 基准 + rx test 子进程隔离);本文只规定 M7 期的**增量**。
> 铁律不变:任何新增门禁必须在真实 PR 上以真实失败/通过路径验证过(反 YAML-only,H06 D11.8-2)。

---

## 1. Runner

沿用 M0 §1(自托管 RTX 4070 Ti 开发机)+ M4 §1(device 路径门禁:CUDA Toolkit 含 `ptxas` + Driver API 装载环境)+ M5 §1(Compute Sanitizer + libdevice bc 探测)+ M6 §1(离线重建干净环境 + LSP server)。M7 新增 runner 预置项(随实现落地时本表修订行留痕):

- **core 数学库 conformance**(G-M7-4)host+device 双路径——host CPU 路径 + device 经 `ptxas`/Driver API,沿用 M4 device 门禁环境。
- **软光栅 L3 基准实测**(G-M7-2)在 RTX 4070 Ti 开发机采(L0 锁频前置,帧时间/吞吐主指标),沿用 BENCH_PROTOCOL §2 环境画像/进程隔离纪律,与既有 GPU 基准互斥队列。
- **UC-03 demo 单 EXE 构建**(G-M7-1)经 `rx build` 产单 EXE + 运行出图;图像序列落盘逐帧 content SHA-256 核对,CPU/GPU 混合路径。
- **rx watch 热重载**联调为人工/半自动功能冒烟,不入 PR Smoke 必经硬门(开发期迭代体验)。

## 2. PR Smoke 追加步骤(编号接 M6 §2 的 25–28)

| # | 步骤 | 失败即红 |
|---|---|---|
| 29 | core 数学库 conformance 冒烟:Vec/Mat/swizzle/几何原语 host+device 双路径端到端真跑(契约 G-M7-4 通道;M7.1 落地接入)。**实测命令(M7.1 回填)**:计数核对 `py -3 ci/budget_eval.py`(`m7.counter.math_primitives ≥` 预设核心集,计数源 = `evidence/stdlib_math_*.json` 的 `primitives_passed` 去重基数)。建设期数学库未落地 → 0 → normal SKIP 属预期 | 是 |
| 30 | image-io 确定性图像序列输出门(契约 D-M7-2;M7.2 落地接入,CPU-only):image-io 编解码 + 落盘逐字节确定性。**实测命令(M7.2 回填)**:`cargo test`(image-io 包编解码/确定性单测)+ 离线构建冒烟(经 M6 包管理 rurix.toml 集成);门为 check_* 守卫风格,失败即红(反 YAML-only) | 是 |
| 31 | 软光栅 kernel 冒烟 + safe 覆盖(契约 G-M7-3 通道;M7.3 落地接入):binning/tile 光栅/深度/tonemap kernel 确定性帧像素 + safe 覆盖计数。**实测命令(M7.3 回填)**:`py -3 ci/soft_raster_smoke.py`(软光栅 kernel device codegen IR 产出 + host CPU 参考 `softraster_repro` 固定输入两次落盘逐帧 content SHA-256 逐字节一致 + 内建篡改帧像素红绿,写唯一 `evidence/soft_raster_smoke.json` 的 `safe_kernels`);计数核对 `py -3 ci/budget_eval.py`(`m7.counter.soft_raster_kernels_safe ≥` 预设软光栅 kernel 数 4,计数源 = `evidence/soft_raster_*.json` 的 `safe_kernels` 去重基数);软光栅 device kernel 纳入 Compute Sanitizer nightly(`bench/sr_*_bench.py --smoke`)+ PTX/IR golden bless(`tests/ptx/sr_*.nvptx` + `bless_log.md`)。建设期软光栅未落地 → 0 → normal SKIP 属预期 | 是 |
| 32 | UC-03 demo 单 EXE 端到端 + 确定性图像序列(契约 G-M7-1;M7.4 落地接入):SPH 仿真 + 软光栅出图经 `rx build` 产单 EXE,运行输出确定性图像序列,逐帧 content SHA-256 两次运行逐字节一致。**实测命令(M7.4 回填)**:UC-03 demo 端到端脚本(写唯一 `evidence/uc03_demo_*.json`,`image_sequence_ok=true`;内建篡改一帧像素红绿断言,应红却绿即脚本 FAIL,反 YAML-only)。计数核对 `py -3 ci/budget_eval.py`(`m7.counter.uc03_demo_image_sequence ≥1`)。建设期 demo 未落地 → 0 → normal SKIP 属预期 | 是 |
| 33 | rx watch 热重载冒烟(契约 D-M7-6 子项;M7.4 落地接入,功能冒烟非硬阈门):源变更→重编译→重载往返冒烟。**实测命令(M7.4 回填)**:rx watch 单次热重载触发冒烟脚本;门为 check_* 守卫风格(不写 budget counter) | 是 |

预算 evaluator(M0 步骤 6)自动合并加载 [m7_budget.json](m7_budget.json)(命名空间冲突即红;evaluator 已配 `m7.counter.math_primitives`/`m7.counter.soft_raster_kernels_safe`/`m7.counter.uc03_demo_image_sequence` 分支,目录/证据缺失 → 0 → normal SKIP,对齐 M4/M5/M6 计数器先例)。**M7 期 PR Smoke 跑 normal 模式**:`m7.counter.*` 建设期未达标 SKIP 属预期;`m7.bench.soft_raster_l3_frame_ms` estimated 占位在 M7.5 回填前继续 SKIP。**M7 close-out 必须跑 `--strict` 且全局零 estimated 残留**(契约 G-M7-2;本占位在 M7 内生灭,不跨里程碑欠债,14 §3)。

## 3. Nightly 追加

- 既有 nightly 全保留(lexer/parser/SAXPY 冒烟 + budget normal + self-profile 归档 + **Compute Sanitizer racecheck+memcheck**(M5.4)+ **gpu 并行基元 measured 基准**(M5.3/M5.4)+ **rx test GPU 子进程隔离**(M6.3))。
- **软光栅 device kernel**(M7.3 落地):软光栅 binning/tile 光栅/深度/tonemap device kernel 纳入既有 Compute Sanitizer racecheck+memcheck nightly 全跑(全 safe 代码目标,unsafe 落点留痕)。
- **软光栅 L3 基准趋势**(M7.5):L3 规模软光栅帧时间/吞吐纳入 nightly 趋势归档(经 `rx bench` 入口,RD-003 已收编;门禁判定在 close-out `--strict`,nightly 为趋势参考)。
- self-profile 归档自然覆盖 M7 新增阶段计数器(数学库 / 软光栅 codegen 布点随实现扩列,非门禁,趋势参考)。
- Release 层仍不建(RD-001,承接 M8 不变)。

## 4. Guardrail

沿用 M0 五项 + M1 三项(spec 档位 / 错误码冻结 / UI bless)+ M3 一项(MIR golden bless,check_mir_bless)+ M4 一项(PTX/IR golden bless,check_ptx_bless)+ M4 unsafe-audit(rurix-rt `undocumented_unsafe_blocks=deny`)+ M5 一项(NVIDIA 再分发白名单审计 check_redistribution / Compute Sanitizer nightly)+ M6(rx fmt 幂等 / rx test 子进程隔离 / 新工具链 crate unsafe_code=deny)。M7 期动作:

1. **基准 ref 维持 `m6-closed`**:M6 close-out 已完成 `m5-closed → m6-closed` 切换(M6 CI_GATES §6 v1.9 / M6_CONTRACT §8.11),`ci/check_guardrails.py` 无参默认 = `m6-closed`,**M7 开工无需再切**;PR 路径仍以 `GITHUB_BASE_REF` 为准。若 M7 期需再切按 `check_*` 守卫风格 + 双基准核对,留痕本表修订行。
2. **新段位错误码首批分配**(stdlib/数学库/image-io/软光栅工具链诊断):随 M7.1+ 诊断 PR 留痕,段位按 07 §5 语义分配(stdlib 类归既有段位续接或经裁决新段位),分配制递增、含义冻结(10 §6,`check_error_codes` 既有冻结机制延续)。**开工脚手架不预造错误码**(无实现即无诊断可锚)。
3. **软光栅 unsafe-audit**(G-M7-3,全 safe 代码目标):软光栅/数学库/image-io/demo 新 crate 维持 `unsafe_code=deny`;凡落 unsafe 须按 AGENTS 硬规则 9 注册条目,每 unsafe 块 `// SAFETY:`,并在 safe 覆盖率报告留痕原因(反哺 views 扩展清单)。
4. **软光栅 kernel codegen 形态纳入既有 PTX/IR golden bless**(M4.2 机制延续):软光栅 device kernel codegen 变更须经 check_ptx_bless 审批。
5. **软光栅 device kernel 纳入既有 Compute Sanitizer nightly**(M5.4 机制延续):激活经真实验证(软光栅 kernel device 路径落地后 racecheck/memcheck 全跑)。

14 §2 常驻集其余项的 M7 期评估结论:

| 项 | 结论 |
|---|---|
| MIR/PTX/IR 文本 golden | M3.3/M4.2 已激活;M7 软光栅 device kernel codegen 纳入既有 PTX/IR golden 核对 |
| stable API 快照 | M7 无 stable 面(core 数学库/image-io/软光栅均 MVP 演进中);stable 面冻结随 MVP 收口(M8)评估激活 |
| unsafe-audit 完整性 | M4.3 已激活(rurix-rt);M7 新 crate 维持 `unsafe_code=deny`,软光栅全 safe 目标,新增 unsafe 边界按硬规则 9 注册 + safe 覆盖率报告 |
| Compute Sanitizer | M5.4 已激活;M7 软光栅 device kernel 落地后纳入既有 nightly 全跑 |
| NVIDIA 再分发白名单审计 | M5.4 formal 激活(事实层,`check_redistribution`);M7 维持 PTX-only 开发期产物,再分发面持续为空;UC-03 demo 单 EXE 为本地构建产物不打包再分发,真分发(G1 cubin/fatbin)随 M8/G1 |
| registry sumdb(D-312) | M7 不做 registry;SG registry 方向触发条件(D-312)未满足,维持 not_triggered(M7 期如评估则追加 spike_gating decisions) |

m0~m6 历史预算的回填/冻结走 `check_guardrails.py` 既有机制(measured_local 条目 0-byte;estimated 只允许回填为 measured_local),不属新增激活项。

## 5. 验证程序(对应契约 G-M7-1/G-M7-2/G-M7-3/G-M7-4 与步骤 29–33)

1. 步骤 32(UC-03 demo 图像序列)落地后,构造**篡改一帧像素 / 破坏 demo 管线**的 PR → 图像序列校验必须红;复原后转绿,run URL 归档(反 YAML-only)。
2. 步骤 29(数学库 conformance)落地后,构造数学库正确性失败(篡改 Vec/Mat 运算预期)→ 红;复原 → 绿,run URL 归档。
3. 步骤 31(软光栅 kernel)落地后,构造软光栅确定性帧像素失败(篡改 kernel 输出)→ 红;复原 → 绿,run URL 归档。
4. 步骤 30(image-io)落地后,构造图像序列非确定性(篡改编码字节序)→ 红;复原 → 绿,run URL 归档。
5. close-out 附 `budget_eval --strict` 输出原文(契约 G-M7-2 软光栅 L3 measured_local 与全局零 estimated 残留判定)+ UC-03 demo 图像序列复现证据路径 + demo 图像序列红绿 run URL + safe 覆盖率报告 + RD-007 处置留痕。

## 6. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-06-15 | 初版(M7 契约配套;步骤 29–33 为 M7.1/M7.2/M7.3/M7.4 计划项,落地时回填实测命令;guardrail 动作:基准 ref 维持 m6-closed 无需再切、新段位错误码首批分配随 M7.1+ 诊断 PR、软光栅 unsafe-audit / PTX golden / Compute Sanitizer nightly 延续均为计划项)。配套 `ci/budget_eval.py` 新增 `m7.counter.math_primitives`/`m7.counter.soft_raster_kernels_safe`/`m7.counter.uc03_demo_image_sequence` evaluator 分支(目录/证据缺失 → 0 → normal SKIP,对齐 M4/M5/M6 计数器先例);`m7_budget.json` 含三计数器 + `m7.bench.soft_raster_l3_frame_ms` estimated 占位(M7.5 回填 measured_local)。`py -3 ci/budget_eval.py`(normal)= PASS(m7.* 占位/计数器 SKIP 属预期) |
