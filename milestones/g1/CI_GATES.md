# G1 CI 门禁增量

> 所属契约:[G1_CONTRACT.md](G1_CONTRACT.md)
> 版本:v1.0(2026-06-18)
> 基线:[../m0/CI_GATES.md](../m0/CI_GATES.md) ~ [../m8/CI_GATES.md](../m8/CI_GATES.md)(全部沿用:runner 约定、PR Smoke 1–39 步、Release 层门禁(14 §8,RD-001 M8 建成)、guardrail 含 M1.1/M1.2/M1.4/M3.3/M4.2/M4.3/M5.4/M6/M7/M8 激活项、nightly 含 Compute Sanitizer racecheck+memcheck + measured 基准 + rx test 子进程隔离 + 软光栅 device kernel + UC-01/UC-02/cublas 趋势 + 全量回归冻结);本文只规定 G1 期的**增量**。
> 铁律不变:任何新增门禁必须在真实 PR 上以真实失败/通过路径验证过(反 YAML-only,H06 D11.8-2)。
> 开工脚手架口径:本文 G1 增量步骤(40+)为 **g1.x 计划项**,开工**不**写入 workflow YAML 真实步骤(随各 g1.x 实现 PR 落地回填,对齐 M8 步骤 34–39 计划 → 回填范式);开工仅 (a) `ci/budget_eval.py` 接 `g1.counter.*` evaluator 分支(证据缺失 → 0 → normal SKIP)、(b) `ci/check_schemas.py`/`ci/budget_eval.py`/`ci/check_guardrails.py`/`tests/test_budget_namespace.py` 预算 glob 泛化纳入 `g1_budget.json`。

---

## 1. Runner

沿用 M0 §1(自托管 RTX 4070 Ti 开发机)+ M4 §1(device 路径:CUDA Toolkit 含 `ptxas` + Driver API)+ M5 §1(Compute Sanitizer + libdevice bc)+ M6 §1(离线重建 + LSP server)+ M7 §1(数学库/软光栅/UC-03 demo 路径)+ M8 §1(Python 互操作链 + cublas runtime DLL + 发布链路签名/SBOM + 文档站)。G1 新增 runner 预置项(随实现落地时本表修订行留痕):

- **D3D12/DXGI 互操作链**(G-G1-1):Windows SDK(D3D12 + DXGI 头/库)+ external memory/semaphore 互操作(`cuImportExternalMemory`/`cuImportExternalSemaphore`,CUDA Driver API)+ 窗口呈现路径;无窗口/无显示环境 → 实时呈现冒烟降级 SKIP(exit 0,对齐 GPU 步骤降级先例)。
- **流序分配 + Graph API 评估**(G-G1-2):`cuMemAllocAsync` + `CUmemoryPool`(Driver API,CUDA Toolkit 已含);AsyncBuffer device 路径纳入既有 Compute Sanitizer racecheck+memcheck nightly。
- **引擎集成**(G-G1-3):C++/D3D12 宿主框架(MSVC 2022 已含;宿主框架选型 g1.3 裁决留痕)+ Rurix DLL(`#[export(c)]` C ABI + 内建头文件)链接。
- **生产分发 fatbin**(G-G1-5):`ptxas` 按架构预编 cubin + fatbin 打包(CUDA Toolkit 已含)+ rurixup 发布链路覆盖 fatbin + Release 层签名/SBOM/NVIDIA 白名单审计延续。

## 2. PR Smoke 追加步骤（计划项，编号接 M8 §2 的 34–39；落地随 g1.x 实现 PR 回填 workflow）

| # | 步骤 | 失败即红 |
|---|---|---|
| 40 | CUDA–D3D12 interop 冒烟(契约 G-G1-1 通道;G1.1 落地接入):`ExternalBuffer`/`ExternalSemaphore` import D3D12 共享堆/信号量 → Rurix kernel 写 backbuffer 等价纹理数值对照 + 句柄生命周期/跨 context/信号时序违例编译期拦截 + 内建篡改同步时序红绿;写 `evidence/d3d12_interop_*.json` 的 `interop_ok`;计数核对 `g1.counter.d3d12_interop ≥1`。无 D3D12/GPU → 降级 SKIP(exit 0)。建设期未落地 → 0 → normal SKIP 属预期 | 是 |
| 41 | 软光栅实时窗口呈现冒烟(契约 G-G1-1 通道;G1.1 落地接入):G0 kernel(RXS-0118~0121 语义 0-byte)写 backbuffer → 信号量同步 present 端到端;写 `evidence/realtime_present_*.json` 的 `present_ok`;计数核对 `g1.counter.realtime_present ≥1`。无窗口/显示环境 → 降级 SKIP。建设期未落地 → 0 → normal SKIP 属预期 | 是 |
| 42 | 流序分配 AsyncBuffer 冒烟(契约 G-G1-2 通道;G1.2 落地接入):`AsyncBuffer<'stream,T>` 三 stream 流序分配端到端 + 三类生命周期错误(分配未完成/释放后/跨 stream 未同步)编译期拦截覆盖 + 内建放行违例红绿;写 `evidence/async_buffer_*.json` 的 `pipeline_ok`;计数核对 `g1.counter.async_buffer_pipeline ≥1`。建设期未落地 → 0 → normal SKIP 属预期 | 是 |
| 43 | 首个引擎集成冒烟(契约 G-G1-3 通道;G1.3 落地接入):Rurix DLL(C ABI)嵌入 C++/D3D12 宿主框架承担 compute pass 端到端数值/呈现对照 + 内建篡改 pass 结果红绿;写 `evidence/engine_integration_*.json` 的 `integration_ok`;计数核对 `g1.counter.engine_integration ≥1`。建设期未落地 → 0 → normal SKIP 属预期 | 是 |
| 44 | 生产分发 fatbin 冒烟(契约 G-G1-5 通道;G1.5 落地接入,check_* 守卫风格):按架构预编 cubin + 保守 PTX fallback 装载协商往返 + manifest/lockfile [[artifact]] digest 校验 + cubin/fatbin codegen 形态纳入 PTX/IR golden + NVIDIA 白名单审计(check_redistribution 延续)。门为 check_* 守卫风格(不写 budget counter,功能冒烟);Release 层覆盖 fatbin 产物签名 | 是 |

预算 evaluator(M0 步骤 6)自动合并加载 [g1_budget.json](g1_budget.json)(命名空间冲突即红;evaluator 开工已配 `g1.counter.d3d12_interop`/`g1.counter.realtime_present`/`g1.counter.async_buffer_pipeline`/`g1.counter.engine_integration` 四分支,证据缺失 → 0 → normal SKIP,对齐 M4~M8 计数器先例)。**G1 期 PR Smoke 跑 normal 模式**:`g1.counter.*` 建设期未达标 SKIP 属预期;性能判据(若有)`g1.bench.*`/`g1.ratio.*` 随各 g1.x 实测回填(**开工 entries 留空,不预欠 estimated 占位**)。**G1 close-out 必须跑 `--strict` 且全局零 estimated 残留**(延续 MVP 零占位纪律,14 §3;不跨里程碑欠债)。

## 3. Release 层门禁（14 §8，M8 RD-001 已建成；G1 延续 + fatbin 覆盖）

Release 层(bench `--strict` + hard block + 签名/SBOM/许可审计 + artifact 上传)由 M8.4 建成(RD-001 closed)。G1 增量:

- **fatbin 产物纳入 Release 层**(G-G1-5):rurixup 发布链路覆盖按架构预编 cubin + fatbin,产物经 Azure Artifact Signing(Authenticode + 时间戳)+ SBOM(SPDX/CycloneDX)+ NVIDIA 再分发白名单审计(`check_redistribution`:cubin 产物经 Attachment A 白名单最小集,完整 Toolkit/驱动/Nsight 永不捆绑,r6)。
- **激活经真实红绿验证**(反 YAML-only):构造白名单外 cubin 组件 / 缺 [[artifact]] digest → Release 门红 → 修复转绿,run URL 归档(落地随 G1.5 回填)。

## 4. Nightly 追加

- 既有 nightly 全保留(M5.4 Compute Sanitizer racecheck+memcheck + measured 基准 + rx test 子进程隔离 + M7 软光栅 device kernel + M8 UC-01/UC-02/cublas 趋势 + 全量回归冻结)。
- **G1.2 AsyncBuffer device 路径**(落地接入):流序分配 device 路径纳入既有 Compute Sanitizer racecheck+memcheck nightly 全跑(**CUDA.jl #780 事故类永久回归项**)。
- **G1.1 interop device 写路径**(落地接入):backbuffer 等价纹理写路径纳入 Sanitizer nightly。
- **G1 性能基准趋势**:interop 呈现帧时 / fatbin 装载首启延迟(若立性能门)经 `rx bench` 入口纳入 nightly 趋势归档(门禁判定在 close-out `--strict`)。
- **全量回归冻结**(G1.6 收口):全量 conformance/UI/MIR/PTX golden/基准回归纳入 nightly 冻结跑。

## 5. Guardrail

沿用 M0 五项 + M1 三项 + M3 一项 + M4(PTX/IR golden bless + unsafe-audit)+ M5(NVIDIA 再分发白名单 / Compute Sanitizer)+ M6(rx fmt 幂等 / rx test 隔离 / 新 crate unsafe_code=deny)+ M7(软光栅 unsafe-audit / PTX golden / Sanitizer 延续)+ M8(互操作/cublas/发布链路 unsafe-audit / Release 层 / stable API 快照评估)。G1 期动作:

1. **基准 ref 默认 `m8-closed`**:M8 close-out 已完成 `m7-closed → m8-closed` 切换(M8 CI_GATES §7 v1.7 / M8_CONTRACT §8.2),`ci/check_guardrails.py` 无参默认 = `m8-closed`,**G1 开工无需再切**;PR 路径仍以 `GITHUB_BASE_REF` 为准。G1 close-out 时按 `check_*` 守卫风格 + 双基准核对切至 `g1-closed`(owner 人工签署兑现)。
2. **新段位错误码首批分配**(interop 呈现/流序分配/引擎集成/分发诊断):随 G1.x 诊断 PR 留痕,段位按 07 §5 语义分配,分配制递增、含义冻结(10 §6,`check_error_codes` 延续)。**开工脚手架不预造错误码**。
3. **interop / 引擎 / 分发 unsafe-audit**(D3D12 external memory/semaphore + DXGI 边界 / C ABI 引擎边界 / fatbin 装载):凡落 unsafe 须按 AGENTS 硬规则 9 注册条目,每 unsafe 块 `// SAFETY:`;interop/引擎/分发新 crate 默认 `unsafe_code=deny`(边界 crate 经裁决最小开 unsafe + 注册留痕)。
4. **NVIDIA 再分发白名单审计延续**(M5.4 check_redistribution):G1 cubin/fatbin 真分发产物须经 Attachment A 白名单最小集审计;完整 Toolkit/驱动/Nsight 永不捆绑(许可红线 r6)。D3D12/DXGI 系 Windows SDK 系统组件,不受 NVIDIA 再分发约束。
5. **Compute Sanitizer nightly 延续**(M5.4):G1.2 AsyncBuffer device 路径 + G1.1 interop device 写路径落地后纳入既有 nightly 全跑。
6. **stable API 快照冻结机制**(RD-008):维持 not_frozen/未激活至首个 stable 发布;激活时机与 stable 面定义经 owner 裁决留痕,激活后 stable API 快照变更须经审批 bless。
7. **G1 close-out 守卫切换**(G1.6,owner 人工签署):`ci/check_guardrails.py` 回退基准默认 `m8-closed → g1-closed`;`check_closed_contracts` 的 `M*_CONTRACT.md` glob 泛化为 `*_CONTRACT.md`(纳入已关闭的 `G1_CONTRACT.md` 字节守卫;`milestones/TEMPLATE_CONTRACT.md` 在 `milestones/` 根、不在 `milestones/*/` 子目录,泛化后不误匹配);`g1-closed` annotated tag 锚定 close-out 签署提交。

14 §2 常驻集其余项的 G1 期评估结论:

| 项 | 结论 |
|---|---|
| MIR/PTX/IR 文本 golden | M3.3/M4.2 已激活;G1 interop/流序分配 device codegen 形态变更纳入既有 PTX/IR golden;**cubin/fatbin 真分发产物形态纳入 golden + 白名单审计**(脱离 M8 PTX-only) |
| stable API 快照 | M8 MVP 收口评估维持 not_frozen(RD-008);G1 维持未激活至首个 stable 发布,激活经 owner 裁决留痕 |
| unsafe-audit 完整性 | M4.3 已激活(rurix-rt);G1 interop/引擎/fatbin 边界凡落 unsafe 按硬规则 9 注册;新 crate 维持 `unsafe_code=deny` |
| Compute Sanitizer | M5.4 已激活;G1 AsyncBuffer + interop device 路径落地后纳入既有 nightly 全跑 |
| NVIDIA 再分发白名单审计 | M5.4 已激活(`check_redistribution`);G1 cubin/fatbin 真分发产物经 Attachment A 白名单审计 |
| registry sumdb(D-312) | 维持 not_triggered(SG-007;MVP+G1 = lockfile+vendor+checksum,真 registry 留 D-312/G2) |
| 声明宏(SG-006) | 触发条件 = G1 后真实样板痛点 ≥3 类且 derive 不可覆盖;**G1 期满后复评**,本期不触发 |

m0~m8 历史预算/契约/registry/error_codes/bless/spec guardrail 走既有机制,无需新代码。

## 6. 验证程序（对应契约 G-G1-1~G-G1-6 与计划步骤 40–44）

1. 步骤 40(CUDA–D3D12 interop)落地后,构造**篡改 interop 同步时序 / 放行跨 context 误用**的 PR → interop 冒烟红;复原转绿,run URL 归档(反 YAML-only)。
2. 步骤 41(实时呈现)落地后,构造 present 同步缺失 / 帧像素篡改 → 红;复原 → 绿,run URL 归档。
3. 步骤 42(AsyncBuffer)落地后,构造流序分配生命周期违例(应编译期拦截却放行)→ 红;复原 → 绿,run URL 归档。
4. 步骤 43(引擎集成)落地后,构造 compute pass 数值结果篡改 → 红;复原 → 绿,run URL 归档。
5. 步骤 44 / §3 Release 层 fatbin 落地后,构造白名单外 cubin 组件 / 缺 [[artifact]] digest → 门红;修复转绿,run URL 归档。
6. close-out 附 `budget_eval --strict` 输出原文(全局零 estimated 残留)+ G1.1~G1.5 端到端证据 + Graph API spike report 结论 + RD-007/RD-008 处置留痕。

## 7. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-06-18 | 初版(G1 契约配套;计划步骤 40–44 为 G1.1~G1.5 计划项,落地时回填 workflow YAML 实测命令与 run URL;Release 层 M8 已建成,G1 延续 + fatbin 覆盖;guardrail 动作:基准 ref 默认 m8-closed 无需再切、新段位错误码随 G1.x 诊断 PR、interop/引擎/分发 unsafe-audit、NVIDIA 白名单审计延续、Compute Sanitizer nightly 延续、stable API 快照维持 not_frozen、G1 close-out 守卫切换(m8-closed→g1-closed + check_closed_contracts 口径泛化)均为计划/close-out 项;SG-006 G1 期满复评 / SG-007 维持 not_triggered)。配套 `ci/budget_eval.py` 新增 `g1.counter.d3d12_interop`/`g1.counter.realtime_present`/`g1.counter.async_buffer_pipeline`/`g1.counter.engine_integration` 四 evaluator 分支(证据缺失 → 0 → normal SKIP,对齐 M4~M8 计数器先例);`g1_budget.json` entries 留空(不预欠 estimated 占位)+ 四计数器;`ci/check_schemas.py`/`ci/budget_eval.py`/`ci/check_guardrails.py`/`tests/test_budget_namespace.py` 预算 glob `m*_budget.json → *_budget.json` 泛化纳入 g1。`py -3 ci/budget_eval.py`(normal)= PASS(g1.* 计数器 SKIP 属预期)。开工不写入 workflow YAML 真实步骤(随 g1.x 实现 PR 回填)|
| v1.1 | 2026-06-18 | G1.1 步骤 40/41 设备闭环与真实红绿归档(`#67`,交互式 `rurix-dev-4070ti`,RTX 4070 Ti):`cargo test -p rurix-rt --features d3d12-interop-real interop::tests::real_interop_numeric_roundtrip -- --exact --nocapture` 完成 D3D12 shared resource/fence import→CUDA 写入→数值回读→Present;`cargo run -p uc03-demo --features d3d12-present-real -- --present` 完成 G0 `sr_tonemap` 写共享 RGB buffer→偶/奇 fence handoff→窗口连续 Present(有限帧 smoke=8,手动命令真跑=120 帧)。baseline green [27760906828](https://github.com/qwasg/Rurix/actions/runs/27760906828);步骤 40 篡改 interop pattern 后 red [27761138655](https://github.com/qwasg/Rurix/actions/runs/27761138655);复原 interop、篡改 present 像素后步骤 40 green/步骤 41 red [27761328446](https://github.com/qwasg/Rurix/actions/runs/27761328446);全部复原 restored green [27761630419](https://github.com/qwasg/Rurix/actions/runs/27761630419)。证据:[d3d12_interop_smoke.json](../../evidence/d3d12_interop_smoke.json)、[realtime_present_smoke.json](../../evidence/realtime_present_smoke.json) |
| v1.2 | 2026-06-19 | G1.2 步骤 42 落地接入 workflow(`ci/async_buffer_smoke.py`,`RURIX_REQUIRE_REAL=1`):host 段默认构建 rurix-rt(`AsyncBuffer<'stream,T>` 随 rurix-rt 始终编译,无 feature 门控)+ 对 `src/rurix-rt/compile-fail/async_buffer_*.rs` 三类流序分配错误(分配未完成访问 E0599 / 释放后访问 E0382 / 跨 stream 未同步 E0599,RXS-0145~0148)逐个断言 rustc 拒绝 + red 自检(反 YAML-only)。device 段三 stream 流序分配 + 两条 `share_with` 跨 stream 时序边 + 往返数值对照 → `pipeline_ok`,写 `evidence/async_buffer_*.json` 计入 `g1.counter.async_buffer_pipeline`。`ci/check_schemas.py` 接 `async_buffer_` 路由 + `milestones/g1/async_buffer_evidence_schema.json`。§4 AsyncBuffer device 路径并入 Compute Sanitizer racecheck+memcheck nightly(CUDA.jl #780 事故类永久回归项,随 PR-4 接线)。本机 device 段真跑 `pipeline_ok=true`(三 stream 流序分配 `cuMemAllocAsync` 往返数值对照通过,`rurix-dev-4070ti` RTX 4070 Ti);**evidence 与 device 真实红绿 run URL 归档 + 计数器兑现随 owner 交互桌面会话(AI 不提交 evidence,对齐 #67)**。unsafe-audit U19/U20(流序分配 FFI + RAII)注册 |
| v1.3 | 2026-06-19 | G1.2 人工收尾(`#69`,owner 本工作会话授权,Codex 代录):交互式 `rurix-dev-4070ti`(RTX 4070 Ti,driver 591.86,CUDA Toolkit 13.3)步骤 42 device 真跑 `pipeline_ok=true`,三类生命周期错误 3/3 编译期拦截,证据 [async_buffer_smoke.json](../../evidence/async_buffer_smoke.json) 入库使 `g1.counter.async_buffer_pipeline=1` PASS。Compute Sanitizer 专项 racecheck [clean](../../evidence/compute_sanitizer_racecheck_async_buffer_20260619.json)(0 hazards/0 errors)+ memcheck [clean](../../evidence/compute_sanitizer_memcheck_async_buffer_20260619.json)(0 errors)。真实红绿:baseline green [27833847240](https://github.com/qwasg/Rurix/actions/runs/27833847240) → 临时放行 `async_buffer_alloc_incomplete.rs` 后步骤 42 red [27834392530](https://github.com/qwasg/Rurix/actions/runs/27834392530) → 恢复编译期拦截 restored green [27834580448](https://github.com/qwasg/Rurix/actions/runs/27834580448)。Graph API owner 裁决为 **G1.2 不立项**,defer 至 G1.3 有实测 launch-overhead 瓶颈时或 G2 重评估;不登记 SG-010、不起 Full RFC,见 [graph_api_spike.md §7](graph_api_spike.md#7-裁决留痕owner-人工裁决)。G-G1-2 子里程碑验收要件闭环 |
| v1.4 | 2026-06-20 | G1.3 步骤 43 落地接入 workflow(`ci/engine_integration_smoke.py`,`RURIX_REQUIRE_REAL=1`,MR-0002 / RXS-0149)。host 段(总跑,无 MSVC/GPU):`cargo build -p rurix-engine` 产 cdylib(`rurix_engine.dll` + import lib,复用 M8.1 既有 `extern "C"` C ABI rurix-interop RXS-0125 语义 0-byte)+ 校验随附头文件 `include/rurix_engine.h` 声明集 == `ffi.rs` `extern "C"` 导出集 == `EXPORTED_C_ABI`(头↔ABI 逐一对应,RXS-0149)+ red 自检(导出集漂移即红,反 YAML-only)。device 段(交互桌面 MSVC + Windows SDK D3D12 + CUDA Toolkit + GPU):`cl` 编译 `src/rurix-engine/harness/engine_host.cpp` 链接 `rurix_engine.dll.lib` + cudart + d3d12/dxgi → 自建最小 C++/D3D12 render-graph 上下文(LUID 匹配 adapter)调 Rurix DLL SAXPY compute pass → 设备数值对照 out==a*x+y → `integration_ok`,写 `evidence/engine_integration_*.json` 计入 `g1.counter.engine_integration`。配套:`ci/check_schemas.py` 接 `engine_integration_` 路由 + `milestones/g1/engine_integration_evidence_schema.json`;`ci/trace_matrix.py` 纳入 `src/rurix-engine` 扫描(RXS-0149 锚定,149/149);unsafe-audit U21(C ABI 导出属性豁免,前向 rurix-interop safe API 本层无 unsafe 块)。host 段本机绿(头↔ABI 1:1 + red 自检)。**device 段真跑收尾(owner 白栀 本工作会话授权,claude-code 代录机器事实,非 AI 代签 G1 整体 close-out)**:交互式本机 RTX 4070 Ti(driver 591.86,CUDA Toolkit v13.3,VS 2022 MSVC 14.44 + Windows SDK)`RURIX_REQUIRE_REAL=1 py -3 ci/engine_integration_smoke.py` 真跑 `integration_ok=true`——`cl` 编译 `harness/engine_host.cpp` 链接 `rurix_engine.dll.lib` + cudart + d3d12/dxgi,自建最小 C++/D3D12 render-graph 上下文(LUID 匹配 adapter)调 Rurix DLL SAXPY compute pass,设备数值对照 out==a*x+y 通过(n=4096,checksum=`85d1316b4d754b25`)。证据 [engine_integration_smoke.json](../../evidence/engine_integration_smoke.json) 入库使 `g1.counter.engine_integration=1` PASS。真实红绿(local interactive runner,退出码):baseline green(exit 0)→ 临时篡改 compute pass(`rurix_engine_compute_saxpy` 用 `a+1.0`)→ device 数值对照失败 4095 mismatch、步骤 43 red(exit 1)→ 复原 restored green(exit 0,同 checksum),功能源码 0-byte 恢复。GitHub Actions CI run URL 待自托管 runner `rurix-dev-4070ti` 上线回填(对齐 G1.1/G1.2 runner-offline 先例);采纳判据增量 check `<5s` measured_local 留 G1 close-out 回填(`rx bench` 暂无 incremental-check 基准) |
| v1.5 | 2026-06-20 | G1.3 人工收尾(`#70`→`#71`,owner 白栀本工作会话授权,Codex 代录机器事实,非 AI 代签 G1 整体 close-out):条款先行 PR #70 全量 smoke green [27864248795](https://github.com/qwasg/Rurix/actions/runs/27864248795) 后合入 `main`;实现 PR #71 重定向 `main`,自托管 `rurix-dev-4070ti` 以 VS 2022 x64 + CUDA Toolkit 13.3 环境执行全量 PR smoke green [27865635269](https://github.com/qwasg/Rurix/actions/runs/27865635269)。步骤 43 真实设备输出 `ENGINE_INTEGRATION: ok pass=saxpy numeric=ok n=4096 checksum=85d1316b4d754b25 present=false`,随附头文件↔导出 ABI 逐一对应,`integration_ok=true`,证据 [engine_integration_smoke.json](../../evidence/engine_integration_smoke.json) 使 `g1.counter.engine_integration=1` PASS。真实红绿沿 v1.4 的 local interactive baseline→篡改 compute pass 后 4095 mismatch red→复原 green 留痕;GitHub 最终 green URL 已回填。采纳判据增量 check `<5s` 不伪造 measured_local,按 v1.4 留 G1 close-out 在基准入口具备后统一回填。D-G1-3 / G-G1-3 子里程碑要件闭环,G1 契约仍为 `active` |
| v1.6 | 2026-06-22 | G1.5 步骤 44 条款先行脚手架接入(Mini-RFC/MR-0005,RXS-0150~0152,**PR-1 条款先行**;owner 2026-06-22 经 AskUserQuestion 裁决档位 Mini-RFC + 落点 release.md + `[[artifact]]` 落 rurix.lock + **不立装载首启延迟性能门**):新建 `ci/fatbin_dist_smoke.py`(步骤 44,**check_* 守卫风格,不写 budget counter**,功能冒烟 + nightly 趋势)——host 段三类构造缺陷红绿自检(白名单外 cubin 组件 / 缺 `[[artifact]]` digest / cubin↔PTX golden 漂移)+ 真实 `ci/check_redistribution.py` 延续;device 段(cubin 预编 + fatbin 装载命中 + 篡改强制 PTX fallback 协商 + 数值往返)经 `RURIX_REQUIRE_REAL` 门控,**真跑接入随 PR-2 实现 PR**。配套:spec/release.md §2.5 落 RXS-0150(分发产物变体模型与按架构预编 cubin + 保守 PTX fallback)/ RXS-0151(fatbin 装载协商序 `select_load_variant`,降级而非 reject,复用 RXS-0076/0077 语义 0-byte)/ RXS-0152(lockfile `[[artifact]]` 变体 digest 内容寻址锁定,复用 RXS-0090/0093);`src/rurix-rt/src/fatbin.rs`(`DeviceArtifactSet`/`select_load_variant`,RXS-0150/0151 锚定)+ `src/rurix-pkg/src/lock.rs`(`LockArtifact` `[[artifact]]`,RXS-0152 锚定),trace_matrix 维持全锚定(149→152);`milestones/g1/fatbin_dist_evidence_schema.json` + `ci/check_schemas.py` 接 `fatbin_dist_` 路由。host 段本机绿(三类红绿自检 + 真实再分发审计;`cargo test -p rurix-rt fatbin:: / -p rurix-pkg lock::` 全绿)。**PR-2 实现 PR 接入**:rurixc/rurix-rt cubin codegen(`ptxas` 保留字节)+ rurix-rt fatbin 装载边界(`sys.rs` `cuModuleLoadData` + unsafe-audit U22 + `lib.rs` `load_module` 协商)+ rurixup 发布链路覆盖 + `check_redistribution` 扩 cubin/fatbin + golden 结构核对 + workflow YAML 步骤 44 + Release 层 fatbin + device 真跑 / run URL 归档。**evidence 与 device 真实红绿 run URL 归档随 owner 交互桌面会话(AI 不提交 evidence,对齐 #67/#69)** |
| v1.7 | 2026-06-22 | **G1.5 人工收尾**(`#75`→`#76`,owner 白栀本工作会话授权,Codex 代录机器事实,非 AI 代签 G1 整体 close-out):条款先行 PR #75 全量 smoke green [27945406280](https://github.com/qwasg/Rurix/actions/runs/27945406280) 后合入 `main`;实现 PR #76 随后重定向 `main`。自托管 `rurix-dev-4070ti`(RTX 4070 Ti,compute capability 8.9,driver 591.86,CUDA Toolkit/ptxas 13.3)执行步骤 44 全量 PR smoke green [27946692960](https://github.com/qwasg/Rurix/actions/runs/27946692960):按 `sm_89` 预编 cubin 后装载协商命中 `variant=cubin`,SAXPY 设备数值对照通过(`n=1048576`);PTX fallback 同包保留,`artifact_variants` 的 PTX/cubin SHA-256 digest 与 `[[artifact]]` 覆盖一致。内建三类构造缺陷红绿自检在同一远端步骤逐项阻断白名单外 cubin、缺 digest、cubin↔PTX golden 漂移;健全集 + `check_redistribution` 恢复绿。Release pipeline 签名/SBOM/许可审计步骤同 run 通过,rurixup 泛型 LanguageCore 组件链路吸收 cubin/fatbin,无需专用代码分支。证据 [fatbin_dist_smoke.json](../../evidence/fatbin_dist_smoke.json) `distribution_ok=true` / `device_path_run=true` / `manifest_lockfile_coverage=true` / `release_layer_passed=true`;owner 既定不立装载首启延迟性能门,故无新增 budget 条目。D-G1-5 / G-G1-5 子里程碑验收要件闭环,G1 契约仍为 `active` |
