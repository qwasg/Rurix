# G2 CI 门禁增量

> 所属契约:[G2_CONTRACT.md](G2_CONTRACT.md)
> 版本:v1.4(2026-06-28)
> 基线:[../m0/CI_GATES.md](../m0/CI_GATES.md) ~ [../g1/CI_GATES.md](../g1/CI_GATES.md)(全部沿用:runner 约定、PR Smoke 1–44 步、Release 层门禁(14 §8,RD-001 M8 建成 + G1.5 fatbin 覆盖)、guardrail 含 M1.1/M1.2/M1.4/M3.3/M4.2/M4.3/M5.4/M6/M7/M8/G1 激活项(含贡献门 ci/check_contribution.py)、nightly 含 Compute Sanitizer racecheck+memcheck + measured 基准 + 软光栅 device kernel + UC-01/UC-02/cublas 趋势 + 全量回归冻结);本文只规定 G2 期的**增量**。
> 铁律不变:任何新增门禁必须在真实 PR 上以真实失败/通过路径验证过(反 YAML-only,H06 D11.8-2)。
> 开工脚手架口径:本文 G2 增量步骤(45+)为 **g2.x 计划项**,开工**不**写入 workflow YAML 真实步骤(随各 g2.x 实现 PR 落地回填,对齐 M8 步骤 34–39 / G1 步骤 40–44 计划 → 回填范式)。**G2 开工脚手架零 CI 代码改动**:预算 glob 已于 G1 泛化为 `*_budget.json`(`ci/check_schemas.py`/`ci/budget_eval.py`/`ci/check_guardrails.py`/`tests/test_budget_namespace.py` 同一 glob),自动纳入 `g2_budget.json`;`check_closed_contracts` 的 `*_CONTRACT.md` glob 与 `check_guardrails.py` 无参默认基准 `g1-closed` 亦已就位;**`g2_budget.json` counter_assertions 留空(首子里程碑 = 着色阶段条款先行,验收走全局 spec 锚定断言,无 device 证据 counter;不预造 counter,无 `ci/budget_eval.py` 新分支)**。

---

## 1. Runner

沿用 M0 §1(自托管 RTX 4070 Ti 开发机 `rurix-dev-4070ti`)+ M4 §1(device:CUDA Toolkit 含 `ptxas` + Driver API)+ M5 §1(Compute Sanitizer + libdevice bc)+ M6 §1(离线重建 + LSP server)+ M7 §1(数学库/软光栅/UC-03 demo)+ M8 §1(Python 互操作 + cublas + 发布链路签名/SBOM + 文档站)+ G1 §1(D3D12/DXGI 互操作链 + 流序分配 + 引擎集成 + fatbin 分发)。G2 新增 runner 预置项(随实现落地时本表修订行留痕):

- **原生 D3D12 + DXIL 工具链**(G-G2-2):Windows SDK(D3D12 + DXIL 头/库 + dxc/DXIL 签名)+ DXIL codegen 后端(D-131 路径裁决后确定:LLVM DirectX target 或 SPIR-V→DXIL 转译链);无 D3D12/GPU 环境 → 相关冒烟降级 SKIP(exit 0,对齐 GPU 步骤降级先例)。
- **着色阶段 / 绑定推导**(G-G2-1/G-G2-3):rurixc 着色阶段前端 + 绑定布局推导 codegen(纯 host 编译期,CPU-only 可跑条款/拦截/golden)。
- **UC-04 deferred 渲染器**(G-G2-4):原生 D3D12 + DXIL 多 pass deferred 管线 demo(窗口呈现路径;无窗口/显示环境降级 SKIP)。
- **语言 1.0 / edition**(G-G2-5):全量 conformance 套件 + edition 机制 + stable API 快照 bless(RD-008 激活后)。

## 2. PR Smoke 追加步骤（计划项，编号接 G1 §2 的 40–44；落地随 g2.x 实现 PR 回填 workflow）

| # | 步骤 | 失败即红 |
|---|---|---|
| 45 | 着色阶段条款/拦截冒烟(契约 G-G2-1 通道;G2.1 落地接入,**Full RFC 前置后**):着色阶段类型面 conformance 样例 + 着色阶段误用/阶段间接口违例编译期拦截覆盖 + UI golden + 内建放行违例红绿(反 YAML-only);`trace_matrix --check` 维持全锚定(RXS-0153 续号)。**纯 host/CPU-only 可跑**。建设期未落地 → 步骤不存在(随实现回填) | 是 |
| 46 | DXIL codegen 冒烟(契约 G-G2-2 通道;G2.2 落地接入,**Full RFC + D-131 裁决后**):MIR→DXIL codegen + DXIL 文本 golden 核对 + device 真跑数值/呈现对照 + 内建篡改 codegen 红绿;无 D3D12/GPU → 降级 SKIP。**G2.2 PR-D2 host/device 回填(2026-06-27)**:`ci/dxil_codegen_smoke.py` 核验图形=B 转译链(SPIRV-Cross→dxc→DXIL)host 可达面——转译链可达 + 确定性 ×N 容器 SHA256(Property 3)+ validator gate + 系统值保真(SV_Position/SV_VertexID 经链)+ 顶点输入语义保真 + 签名篡改红绿(篡改 SPIR-V→链拒/复原绿、篡改译后签名→保真核验拒/复原绿)+ 供应链 pin 核对(`rurix.lock [[toolchain]]` SHA256);`ci/dxil_device_smoke.py` 在签名 DXC 套件下跑 `dxv.exe` validator / 篡改红路径 / D3D12 hardware offscreen draw/readback。DXIL 文本 golden 见 `tests/dxil/graphics/*.dxil-disasm`;agent 已在 pin 环境 bless 当前 RD-013/RD-017 缺口下的 approved baseline。远端 PR smoke green [28284960733](https://github.com/qwasg/Rurix/actions/runs/28284960733) 步骤 46 输出 `DXIL_DEVICE: ok adapter="NVIDIA GeForce RTX 4070 Ti" pixel=64,127,255,255 draw=ok`。无 B 工具链/D3D12 → SKIP exit 0;`RURIX_REQUIRE_REAL=1` 下缺环境即红 | 是 |
| 47 | 绑定布局推导冒烟(契约 G-G2-3 通道;G2.3 落地接入,**Full RFC 后**):descriptor/root signature 推导正确性 conformance + golden + 内建放行错误推导红绿 | 是 |
| 48 | UC-04 deferred 渲染器冒烟(契约 G-G2-4 通道;G2.4 落地接入):原生 D3D12 + DXIL 多 pass deferred 管线端到端出图 + 呈现对照 + 内建篡改 pass 红绿。**防降级**:REQUIRE_REAL green 必须证明 Rurix source 经 `rurixc` 图形=B DXIL 路径 + RFC-0005 RTS0/绑定布局进入 D3D12 PSO 并完成 hardware 多 pass deferred draw + offscreen readback;手写 HLSL/DXIL、CPU 预填、单 pass textured draw、fullscreen copy、固定像素注入、host-only 模拟、窗口截图或 SKIP 不得替代 G-G2-4。无窗口/显示环境只允许窗口 present 子路径 SKIP,不得替代 offscreen readback 真跑 | 是 |
| 49 | 语言 1.0 conformance + edition 冒烟(契约 G-G2-5 通道;G2.5 落地接入):全量 conformance 覆盖 + edition 机制 + stable API 快照 bless(RD-008 激活后);close-out `budget_eval --strict` 零 estimated | 是 |

> 2026-06-27 agent 收口补充:agent于本工作会话监督确认 G-G2-2 device/run URL、DXIL 文本 golden bless 与子里程碑签字;AI 仅代录机器事实并执行机械落档,不构成 G2 整体 close-out。`ci/dxil_device_smoke.py` 已落步骤 46 的真实 D3D12 hardware smoke:签名 DXC 套件编译最小 VS/PS → `dxv.exe` 显式 validator → `dxv` 篡改红路径 → MSVC 自建 C++ harness → hardware adapter 建 graphics PSO → offscreen draw/readback 像素对照。`.github/workflows/pr-smoke.yml` 步骤 46 已接入 `ci/dxil_codegen_smoke.py` + `ci/dxil_device_smoke.py`;`RURIX_REQUIRE_REAL=1` 下缺 validator/D3D12/MSVC 即红。GitHub run [28284960733](https://github.com/qwasg/Rurix/actions/runs/28284960733) 全量 `pr-smoke` success。

预算 evaluator(M0 步骤 6)自动合并加载 [g2_budget.json](g2_budget.json)(命名空间冲突即红;**开工 entries/ratio_assertions/counter_assertions 均留空——首子里程碑 = 着色阶段条款先行,验收走既有全局 `m1.counter.spec_clause_test_anchoring` 断言,不预造 g2 counter,无新 evaluator 分支**)。性能判据(若有)`g2.bench.*`/`g2.ratio.*` 随各 g2.x 实测 measured_local 回填(**开工不预欠 estimated 占位**);device 证据 counter(若 G2.2/G2.4 落地需)随其实现 PR 新增 `g2.counter.*` + 配套 `ci/budget_eval.py` evaluator 分支(条款先于实现)。**G2 close-out 必须跑 `--strict` 且全局零 estimated 残留**(延续 MVP 零占位纪律,14 §3;不跨里程碑欠债)。

## 3. Release 层门禁（14 §8，M8 RD-001 已建成 + G1.5 fatbin 覆盖；G2 延续 + DXIL 覆盖）

Release 层(bench `--strict` + hard block + 签名/SBOM/许可审计 + artifact 上传)由 M8.4 建成(RD-001 closed)、G1.5 扩 fatbin。G2 增量(随 DXIL 实现回填):

- **DXIL/原生 D3D12 产物纳入 Release 层**(G-G2-2/G-G2-4):rurixup 发布链路覆盖 DXIL codegen 产物 + 既有 Azure Artifact Signing(Authenticode + 时间戳)+ SBOM(SPDX/CycloneDX)。DXIL/D3D12 系 Windows SDK / DirectX 系统组件,不受 NVIDIA 再分发约束;CUDA 侧 cubin/fatbin(若 G2 保留 compute 互操作)延续 `check_redistribution` Attachment A 白名单审计(r6)。
- **激活经真实红绿验证**(反 YAML-only):构造 DXIL 产物缺陷 / 签名缺失 → Release 门红 → 修复转绿,run URL 归档(落地随 G2.x 回填)。

## 4. Nightly 追加

- 既有 nightly 全保留(M5.4 Compute Sanitizer racecheck+memcheck + measured 基准 + rx test 子进程隔离 + M7 软光栅 device kernel + M8 UC-01/UC-02/cublas 趋势 + G1 AsyncBuffer/interop device 路径 + 全量回归冻结)。
- **G2 DXIL/原生 D3D12 device 路径**(落地接入):DXIL codegen device 真跑 + UC-04 deferred 管线纳入 nightly。
- **DXIL golden 回归**(落地接入):DXIL 文本 golden 纳入既有 golden 回归网(镜像 PTX golden)。
- **G2 性能基准趋势**:DXIL codegen / deferred 渲染帧时(若立性能门)经 `rx bench` 入口纳入 nightly 趋势归档(门禁判定在 close-out `--strict`)。
- **全量回归冻结**(G2.6 收口):全量 conformance/UI/MIR/PTX/DXIL golden/基准回归纳入 nightly 冻结跑。

## 5. Guardrail

沿用 M0 五项 + M1 三项 + M3 一项 + M4(PTX/IR golden bless + unsafe-audit)+ M5(NVIDIA 再分发白名单 / Compute Sanitizer)+ M6(rx fmt 幂等 / rx test 隔离 / 新 crate unsafe_code=deny)+ M7(软光栅 unsafe-audit / PTX golden / Sanitizer)+ M8(互操作/cublas/发布链路 unsafe-audit / Release 层 / stable API 快照评估)+ G1(interop/引擎/分发 unsafe-audit / 贡献门 ci/check_contribution.py / cubin·fatbin golden + 白名单 / 基准 g1-closed + `*_CONTRACT.md`·`*_budget.json` glob 泛化)。G2 期动作:

1. **基准 ref 默认 `g1-closed`**:G1 close-out 已完成 `m8-closed → g1-closed` 切换(G1 CI_GATES §5 第 7 项 / G1_CONTRACT §8.5),`ci/check_guardrails.py` 无参默认 = `g1-closed`,**G2 开工无需再切**;PR 路径仍以 `GITHUB_BASE_REF` 为准。G2 close-out 时按 `check_*` 守卫风格 + 双基准核对切至 `g2-closed`(agent 自主签署兑现)。
2. **新段位错误码首批分配**(着色阶段 / DXIL codegen / 绑定布局 / edition 诊断):随 G2.x 诊断 PR 留痕(RX7020 续号),段位按 07 §5 语义分配,分配制递增、含义冻结(10 §6,`check_error_codes` 延续)。**开工脚手架不预造错误码**。
3. **着色阶段 / DXIL / 绑定推导 / demo unsafe-audit**(原生 D3D12 + DXIL 边界 / DXIL 装载 / D3D12 资源):凡落 unsafe 须按 AGENTS 硬规则 9 注册条目(U23 续号),每 unsafe 块 `// SAFETY:`;新 crate 默认 `unsafe_code=deny`(边界 crate 经裁决最小开 unsafe + 注册留痕)。
4. **NVIDIA 再分发白名单审计延续**(M5.4 check_redistribution):G2 DXIL/D3D12 系 Windows SDK / DirectX 系统组件不受约束;CUDA 侧 cubin/fatbin 产物(若保留)延续 Attachment A 白名单审计(r6)。
5. **Compute Sanitizer nightly 延续**(M5.4):G2 device 路径(若涉 CUDA compute 互操作)落地后纳入既有 nightly 全跑。
6. **stable API 快照冻结机制**(RD-008):维持 not_frozen/未激活至首个 stable 发布(G2.5 语言 1.0 为候选触发点);激活时机与 stable 面定义经 agent 裁决留痕,激活后 stable API 快照变更须经审批 bless。
7. **G2 close-out 守卫切换**(G2.6,agent 自主签署,**计划项**):`ci/check_guardrails.py` 回退基准默认 `g1-closed → g2-closed`;`check_closed_contracts` 的 `*_CONTRACT.md` glob **已于 G1 close-out 泛化,无需再改**(自动纳入已关闭的 `G2_CONTRACT.md` 字节守卫);`g2-closed` annotated tag 锚定 close-out 签署提交。预算/契约 glob 均已泛化,G2 close-out 仅切基准默认值 + 打 tag(代码改动最小)。
8. **DXIL 文本 golden bless**(G2.2 落地,计划项):DXIL codegen 形态变更纳入 golden bless 机制(镜像 tests/ptx/ 的 .nvptx bless,`check_guardrails` 新增 DXIL golden 守卫分支随 DXIL 子里程碑实现 PR)。

14 §2 常驻集其余项的 G2 期评估结论:

| 项 | 结论 |
|---|---|
| MIR/PTX/IR/DXIL 文本 golden | M3.3/M4.2 已激活;G2 DXIL codegen 形态纳入既有 golden + **新增 DXIL 文本 golden + bless**(随 G2.2 实现) |
| stable API 快照 | M8 维持 not_frozen(RD-008);G2.5 语言 1.0 为候选激活点,激活经 agent 裁决留痕 |
| unsafe-audit 完整性 | M4.3 已激活;G2 DXIL/D3D12/绑定边界凡落 unsafe 按硬规则 9 注册(U23 续号);新 crate 维持 `unsafe_code=deny` |
| Compute Sanitizer | M5.4 已激活;G2 device 路径(若涉 CUDA 互操作)落地后纳入既有 nightly |
| NVIDIA 再分发白名单审计 | M5.4 已激活;DXIL/D3D12 系系统组件不受约束;CUDA cubin/fatbin(若保留)延续审计 |
| 多后端(D-008,SG-003) | **维持 not_triggered**(红线 3 不解除,默认直至 NVIDIA 纵深完成,解除一次一条 10 §9.2);DXIL 是 D3D12 原生路径非通用多后端 |
| registry sumdb(D-312,SG-007) | **维持 not_triggered**(社区规模未达触发阈 >50 包/强需求,11 §5;MVP+G1+G2 = lockfile+vendor+checksum) |
| MLIR(SG-001)/ Tensor Core(SG-002) | 维持 not_triggered(11 §2 红线 / 触发条件未满足) |
| 贡献校验门(ci/check_contribution.py) | G1.4 已激活;G2 延续(provenance / 条款号 / 验证标记三类缺项即红) |

m0~g1 历史预算/契约/registry/error_codes/bless/spec guardrail 走既有机制,无需新代码。

## 6. 验证程序（对应契约 G-G2-1~G-G2-6 与计划步骤 45–49）

1. 步骤 45(着色阶段条款/拦截)落地后(**Full RFC 前置**),构造**放行着色阶段类型违例 / 阶段间接口不匹配**的 PR → 拦截冒烟红;复原转绿,run URL 归档(反 YAML-only)。**纯 host/CPU-only 可完整演示**。
2. 步骤 46(DXIL codegen)落地后(**Full RFC + D-131 裁决**),构造 DXIL codegen 输出篡改 / golden 漂移 → 红;复原 → 绿,run URL 归档。
3. 步骤 47(绑定推导)落地后,构造错误 descriptor/root signature 推导(应拦截却放行)→ 红;复原 → 绿,run URL 归档。
4. 步骤 48(UC-04)落地后,构造 deferred 管线 pass 结果篡改 / 像素篡改 → 红;复原 → 绿,run URL 归档。
5. 步骤 49 / §3 Release 层 DXIL 落地后,构造 DXIL 产物缺陷 / 签名缺失 → 门红;修复转绿,run URL 归档。
6. close-out 附 `budget_eval --strict` 输出原文(全局零 estimated 残留)+ G2.1~G2.5 端到端证据 + RD-007/RD-008/RD-009 处置留痕 + SG 复评结论。

## 7. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.5 | 2026-06-30 | **G2.5 agent 收口（步骤 49 落地，G-G2-5，RFC-0008 + RD-008）**：agent 在 AGENTS v3.0 完全自主（硬规则 1）下落地计划步骤 49（语言 1.0 + 首个 edition 冒烟）为 `.github/workflows/pr-smoke.yml` 真实步骤 `ci/edition_smoke.py`（**纯 host/编译期，无 device，不 SKIP 充绿**）：① `cargo test -p rurix-pkg --test edition_corpus` + `manifest::tests::edition`——`conformance/edition/accept` 合法 edition `"2026"` 经 `Manifest::parse` 接受 + 缺省兼容 / `reject` 未知 edition → **RX7020** `toolchain.edition_unknown` strict-only 拒（无 fallback，P-01）+ 类型错误复用 RX7005（真实红绿）；② `py -3 ci/stable_snapshot.py --check`——**RD-008 stable API 快照冻结机制经 G2.5 语言 1.0 激活**（首个 stable 发布触发点，RFC-0008 §9 Q-RD008），stable 面（spec RXS 条款 ID 全集 180 + 错误码 ID/含义 88 + edition 合法值集 ['2026'] + rx CLI 子命令面 8）与入库 `tests/stable/stable_api.snapshot` 一致；③ 篡改 stable 快照 → `--check` 翻红 → 复原绿（红绿闭合，反 YAML-only）。bless 守卫 `ci/check_guardrails.py` `check_stable_snapshot_bless`（镜像 UI/MIR/PTX/DXIL golden bless）+ agent 自主 bless 首份快照（`tests/stable/bless_log.md` 2026-06-30 行）。spec/edition.md RXS-0177~0180（FLS 体例严禁 UB）+ src/rurix-pkg `Edition` 解析/校验（纯 host/safe，零新 unsafe）+ 双语 message-key（bilingual 88/88）+ `trace_matrix --check` 176→180 全锚定。首个 edition `"2026"` 仅机制锚点（edition-gated 行为差异 = 空集）。**RD-008 open→closed**；本机 `ci/edition_smoke.py` PASS（红绿闭合）。run URL 见 §6 第 7 项（self-hosted/GitHub Actions 取真实 run URL，不可达则贴本机真实输出 + 诚实标注，不伪造）。**G2 契约仍 active**，不执行 `g2-closed` tag / 基准切换（g1-closed→g2-closed）/ RD-007·RD-009 翻转（仅 RD-008 翻 closed）。|
| v1.4 | 2026-06-28 | **G2.3 agent 收口（步骤 47 落地）**：agent于本工作会话明确签署 G-G2-3 并批准 close-out 代录；agent 自主记录、非代签 G2 整体 close-out。计划步骤 47（绑定布局推导冒烟，G-G2-3，RFC-0005 / RXS-0163~0166）已落地为 `.github/workflows/pr-smoke.yml` 真实步骤 `ci/dxil_binding_device_smoke.py`（默认 `RURIX_DXC_DIR=H:\dxc-round7\extracted\bin\x64`，可由 repo variable 覆盖，`RURIX_REQUIRE_REAL=1` 缺 validator/D3D12/MSVC 即红），并由 PR [#109](https://github.com/qwasg/Rurix/pull/109) 最新 `pr-smoke` [run 28319166995](https://github.com/qwasg/Rurix/actions/runs/28319166995) 全量 success（head `84324f0`）验证。步骤 47 device 见证日志含 `DXIL_BIND: ok adapter="NVIDIA GeForce RTX 4070 Ti" rurix_rts0=accept tamper_rts0=reject sampled=64,127,255,255 draw=ok`；E2b-3 推导产物 host golden `tests/dxil/binding/fs_tex_samp.binding-golden`（`rts0.bytes.sha256=409b6a1e…c0a2`）agent bless（bless_log 2026-06-28 行），E2b-4 device witness 回填 `evidence/g2.3-binding-layout/binding_layout_device_smoke_20260628.json`（`run_url` = [run 28319066260](https://github.com/qwasg/Rurix/actions/runs/28319066260)）。G2 契约仍 active，不执行 `g2-closed` tag / 基准切换 / RD·SG 翻转。|
| v1.3 | 2026-06-27 | **G2.2 agent 收口**:agent于本工作会话监督确认 G-G2-2 device/run URL、DXIL 文本 golden bless 与子里程碑签字;agent 自主记录、非代签 G2 整体 close-out。远端 PR smoke [28284960733](https://github.com/qwasg/Rurix/actions/runs/28284960733) 全量 success(3m19s),步骤 46 同时跑 `ci/dxil_codegen_smoke.py` + `ci/dxil_device_smoke.py`,日志含 `DXIL_DEVICE: ok adapter="NVIDIA GeForce RTX 4070 Ti" pixel=64,127,255,255 draw=ok`。`tests/dxil/graphics/gfx_vs_min.dxil-disasm` 在 agent pin 环境(`H:\dxc-round7\extracted\bin\x64` signed DXC + `spirv-cross.exe`)经 `RURIX_BLESS=1 cargo test -p rurixc --features dxil-backend --test dxil_golden dxil_b_disasm_golden_matches_when_toolchain_present -- --exact --nocapture` 重 bless;入 golden 前 `dxv.exe` validator 接受。当前 golden 锁已登记 RD-013/RD-017 缺口下的 TEXCOORD baseline,不声称 output varying 用户语义保真已兑现。G2 契约仍 active,不执行 `g2-closed` tag / 基准切换 / RD 翻转。|
| v1.2 | 2026-06-27 | **G2.2 步骤 46 device 侧回填**:`ci/dxil_device_smoke.py` 新增无窗口 D3D12 hardware smoke:用签名 DXC 套件编译最小 VS/PS → `dxv.exe` 显式 validator → `dxv` 篡改红路径 → MSVC 编译自建 C++ harness → 真实 hardware adapter 建 graphics PSO → offscreen draw/readback 中心像素对照。配套 `ci/dxil_codegen_smoke.py` 定位顺序修正(`RURIX_DXC_DIR` 先于 PATH dxc,避免误吃 Vulkan SDK dxc);`.github/workflows/pr-smoke.yml` 步骤 46 接入 host smoke + device smoke,默认 `RURIX_DXC_DIR=H:\dxc-round7\extracted\bin\x64`(可由 repo variable 覆盖),`RURIX_REQUIRE_REAL=1` 缺环境即红。本机真实绿:host validator gate PASS;device 输出 `DXIL_DEVICE: ok adapter="NVIDIA GeForce RTX 4070 Ti" pixel=64,127,255,255 draw=ok`。仍不代签 G-G2-2:GitHub run URL、golden bless agent 批准、最终签字归 agent。|
| v1.0 | 2026-06-23 | 初版(G2 契约配套;计划步骤 45–49 为 G2.1~G2.5 计划项,落地时回填 workflow YAML 实测命令与 run URL;Release 层 M8/G1.5 已建成,G2 延续 + DXIL 覆盖;guardrail 动作:基准 ref 默认 g1-closed 无需再切、新段位错误码随 G2.x 诊断 PR(RX7020 续号)、着色/DXIL/绑定/demo unsafe-audit(U23 续号)、NVIDIA 白名单审计延续、Compute Sanitizer nightly 延续、stable API 快照维持 not_frozen、G2 close-out 守卫切换(g1-closed→g2-closed,glob 已泛化无需改)、DXIL golden bless 随 G2.2 实现均为计划/close-out 项;SG-001/002/003/007 G2 开工复评维持 not_triggered)。**G2 开工脚手架零 CI 代码改动**:`g2_budget.json` 经既有 `*_budget.json` glob 自动纳入,entries/ratio_assertions/counter_assertions 均留空(首子里程碑 = 着色阶段条款先行,验收走既有全局 spec 锚定断言,不预造 g2 counter,无 `ci/budget_eval.py` 新分支);`*_CONTRACT.md` glob 与无参基准 g1-closed 均已就位。`py -3 ci/budget_eval.py`(normal)/`--strict` = PASS(g2 无 estimated/counter)。开工不写入 workflow YAML 真实步骤(随 g2.x 实现 PR 回填)|
| v1.1 | 2026-06-27 | **G2.2 PR-D2 步骤 46 host 侧回填**(spec/dxil_backend.md RXS-0162;任务 13/14):新建 `ci/dxil_codegen_smoke.py` 落步骤 46 的图形=B 转译链 host 可达面——转译链可达(spirv-cross+dxc 定位+端到端)/ 确定性 ×N 容器 SHA256(Property 3)/ validator gate(签名 validator 可用时入 golden 前 dxv 验证,Vulkan SDK dxc 无 dxil.dll/dxv → 结构性编译为代 SKIP)/ 系统值保真(SV_Position·SV_VertexID 经链)/ 签名篡改红绿(SPIR-V 字流篡改→转译链拒/复原绿、译后签名去系统值→保真核验拒/复原绿)/ 供应链 pin 核对(`rurix.lock [[toolchain]]` SHA256 vs 定位工具,canonical 命中/dev override NOTE);内置 red 自检反 YAML-only;无 B 工具链 → SKIP exit 0。配套供应链 pin lockfile `rurix.lock`(`[[toolchain]]` dxc 1.8.0.4739 / spirv-cross / glslang / spirv-val + SHA256,env override 仅 dev/probe)+ DXIL 文本 golden `tests/dxil/graphics/gfx_vs_min.dxil-disasm`(B 路 emit_spirv→SPIRV-Cross→dxc→dumpbin,NOT BLESSED,agent pin 环境重 bless,bless_log 留痕)+ conformance/dxil/graphics 锚定语料(RXS-0158/0159/0161/0162)。**device 真跑数值/呈现对照 + 签名 run URL + golden bless + G-G2-2 签字归 agent(AI 不代办,硬规则 1)**;步骤 46 完整 device 路径(MIR→DXIL device 真跑 + 内建篡改 codegen 红绿)随 agent device 兑现回填 workflow YAML。`py -3 ci/dxil_codegen_smoke.py` 本机带工具链 PASS(green + 篡改红绿)、无工具链 SKIP exit 0,均经真实跑验证(反 YAML-only)|
