# M4 CI 门禁增量

> 所属契约:[M4_CONTRACT.md](M4_CONTRACT.md)
> 版本:v1.0(2026-06-13)
> 基线:[../m0/CI_GATES.md](../m0/CI_GATES.md) + [../m1/CI_GATES.md](../m1/CI_GATES.md) + [../m2/CI_GATES.md](../m2/CI_GATES.md) + [../m3/CI_GATES.md](../m3/CI_GATES.md)(全部沿用:runner 约定、PR Smoke 1–16 步、guardrail 含 M1.1/M1.2/M1.4/M3.3 激活项、nightly 工作流);本文只规定 M4 期的**增量**。
> 铁律不变:任何新增门禁必须在真实 PR 上以真实失败/通过路径验证过(反 YAML-only,H06 D11.8-2)。

---

## 1. Runner

沿用 M0 §1(自托管 RTX 4070 Ti 开发机)。M4 首次出现 **device 路径门禁**:

- CPU 任务(着色/地址空间检查、NVPTX codegen、ptxas 干验证、launch conformance、黄金路径 4)不占 GPU 队列。
- **GPU 任务**(SAXPY 端到端真跑 + measured 基准)占 GPU 队列,沿用 M0 基准 harness 的锁频/环境画像/进程隔离纪律(BENCH_PROTOCOL §2)。
- 新增 runner 预置项:CUDA Toolkit(含 `ptxas`)+ Driver API 装载环境。探测复用运行时探测器(NVML / `CUDA_PATH` 枚举,**禁硬编码版本文件名**——r6 的 `CUDA 13.2.props` 教训,07 §10);预置项落地时在本表修订行留痕。

## 2. PR Smoke 追加步骤(编号接 M3 §2 的 15–16)

| # | 步骤 | 失败即红 |
|---|---|---|
| 17 | ptxas 干验证关卡:示例 kernel 经 rurixc device codegen 产 PTX → `ptxas -arch=sm_89` 干验证通过;构造拒绝场景 → RX6xxx 编译期诊断(契约 G-M4-4 通道;M4.2 落地接入,落地回填实测命令) | 是 |
| 18 | launch 类型契约 conformance 批跑:`conformance/launch/reject/<category>/` 反例全拦截(逐文件断言产生预期 3xxx/6xxx 诊断)+ `accept/` 正例 0 诊断 + 四类目录覆盖核对(契约 G-M4-2 通道;M4.3 落地接入,落地回填实测命令) | 是 |
| 19 | 黄金路径 4 snapshot 核对:`tests/ui/` 目标后端错误 .stderr snapshot(3xxx/6xxx)全绿 + bless 守卫(契约 G-M4-3 通道,复用 M1.4 UI 通道与 check_ui_bless;M4.3 落地接入) | 是 |
| 20 | (GPU)Rurix SAXPY 端到端冒烟:Rurix kernel 全管线产 EXE → launch → 拷回逐元素核对 exit 0(契约 G-M4-1 真跑通道,对齐步骤 12/16 真跑形态;M4.4 落地接入,GPU 队列) | 是 |

预算 evaluator(M0 步骤 6)自动合并加载 [m4_budget.json](m4_budget.json)(命名空间冲突即红)。**M4 期 PR Smoke 跑 normal 模式**:`m4.counter.*` 建设期未达标 SKIP 属预期;`m4.ratio.saxpy_vs_m0_baseline` estimated 占位在 M4.4 回填前继续 SKIP。**M4 close-out 必须跑 `--strict` 且全局零 estimated 残留**(契约 G-M4-1;本占位在 M4 内生灭,不跨里程碑欠债,14 §3)。

## 3. Nightly 追加

- 既有 nightly 全保留(lexer/parser/SAXPY(手写基线)冒烟 + budget normal + self-profile 归档,M2/M3 实体化)。
- **Rurix SAXPY measured 基准纳入 nightly**(M4.4 落地后):全量 bench 含 Rurix kernel SAXPY 采样 + 对 M0 手写基线的回归判定(BENCH_PROTOCOL §5,Mann-Whitney U + 效应量门,1% Warning / 5% Critical)。
- **Compute Sanitizer 评估**:device 运行时出现后,memcheck nightly 评估接入(全量 racecheck 随 M5 scoped atomics/barrier,r5/08 §5);M4 期结论入修订行。
- self-profile 归档自然覆盖 M4 新增阶段计数器(着色/codegen/ptxas 布点随实现扩列,非门禁,趋势参考)。
- Release 层仍不建(RD-001,承接 M8 不变)。

## 4. Guardrail

沿用 M0 五项 + M1 三项(spec 档位 / 错误码冻结 / UI bless)+ M3 一项(MIR golden bless,check_mir_bless)。三项 M4 期动作:

1. **基准 ref 切换**:`m3-closed` tag 已随 M3 终审打出(2026-06-13);M4.1 第 1 项将 `ci/check_guardrails.py` 本地/push 回退基准 `m2-closed → m3-closed`(PR 路径仍以 GITHUB_BASE_REF 为准),切换前双基准核对,落地留痕本表修订行。
2. **NVIDIA 再分发白名单审计激活评估**(14 §2 常驻集,M0~M3 CI_GATES 标注"device 路径 M4 起评估"的到期时点):M4 产物为 **PTX-only**(开发期),运行时经已安装驱动 JIT 装载(`cuModuleLoadDataEx`),**不打包任何 NVIDIA 再分发二进制**(libdevice 链接随 M5、cubin/fatbin 分发随 G1)。故 M4 期审计结论 = 无再分发物需白名单核对,formal 审计门(再分发清单逐项核对)随 libdevice/cubin 引入时激活(M5/G1)。评估结论于 M4 期实体化(本表修订行 + close-out)。
3. **PTX 文本 golden / NVPTX 雷区回归集**(14 §2 IR golden 机制,07 §11):M4.2 做预评估(PTX 形态随 device codegen 定型),M4.2/M4.3 激活——PTX golden 基线入库 + 核对入 CI,激活必须经真实红绿验证;基线变更纪律对齐 UI/MIR bless(变更须审批留痕),具体形态(独立脚本 vs cargo test 快照,bless 留痕文件)在激活 PR 中裁决。NVPTX 雷区(shfl 选择失败/sqrt 近似约束类)遇雷登记 pin 绕行并挂此机制回归。

14 §2 常驻集其余项的 M4 期评估结论:

| 项 | 结论 |
|---|---|
| MIR 文本 golden | M3.3 WP6 已激活(check_mir_bless),M4 沿用;device MIR 形态变更纳入既有 golden 核对 |
| stable API 快照 | M4 无 stable 面,不激活 |
| unsafe-audit 完整性 | rurixc 实现侧 `unsafe_code = deny` 维持;运行时 Driver API FFI 边界出现首个 unsafe 块时按 AGENTS 硬规则 9 激活 unsafe-audit |
| Compute Sanitizer | M4 device 运行时落地后 memcheck nightly 评估(§3);全量 racecheck 随 M5 |

m0~m3 历史预算的回填/冻结走 `check_guardrails.py` 既有机制("estimated 条目只允许回填为 measured_local";measured_local 条目 0-byte),不属新增激活项。

## 5. 验证程序(对应契约 G-M4-1/G-M4-2/G-M4-3/G-M4-4 与步骤 17–20)

1. 步骤 17 落地后,构造 ptxas 拒绝场景(注入非法 PTX 或破坏 codegen 产出)→ rurixc 必须报 RX6xxx、step 红;修复后同 PR 转绿,两次 run URL 随 close-out 归档。
2. 步骤 18 落地后,构造**故意放行某 launch 反例类别**(或篡改 reject 语料预期)的 PR → 必须红;修复后转绿,run URL 归档。
3. 步骤 19(黄金路径 4)走 bless 审批:篡改 .stderr 不附 bless 行 → `check_ui_bless` FAIL;补 bless 留痕 → 绿,run URL 归档。
4. 步骤 20(GPU SAXPY)落地后,构造拷回核对失败(篡改 kernel 语义)→ exit 1 红;复原 → exit 0 绿,run URL 归档。
5. PTX golden 激活时(§4 第 3 项),故意改 PTX 输出不更新基线 → 红;按审批更新基线 → 绿,run URL 归档。
6. close-out 附 `budget_eval --strict` 输出原文(契约 G-M4-1 比值 ≥0.95 与全局零 estimated 残留判定)+ SAXPY measured_local 证据 JSON 路径 + NVIDIA 白名单评估结论。

## 6. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-06-13 | 初版(M4 契约配套;步骤 17–20 为 M4.2/M4.3/M4.4 计划项,落地时回填实测命令;guardrail 三项动作:基准切换 m2-closed→m3-closed、NVIDIA 白名单评估、PTX golden 评估均为计划项) |
| v1.1 | 2026-06-13 | §4 第 1 项落地(M4.1 任务 1):`ci/check_guardrails.py` 本地/push 回退基准 `m2-closed → m3-closed`(`resolve_base()` 默认值 + 文件头 docstring;PR 路径仍以 GITHUB_BASE_REF 优先,既有逻辑不变)。切换前双基准核对均 PASS:`py -3 ci/check_guardrails.py m2-closed` = PASS(118 changed paths);`py -3 ci/check_guardrails.py m3-closed` = PASS(2 changed paths)。配套 `ci/budget_eval.py` 新增 `m4.counter.launch_conformance_categories`(≥4)/ `m4.counter.ui_golden_path4_snapshots`(≥10)evaluator 分支(目录缺失 → 0 → normal SKIP,对齐 M3 计数器先例);`py -3 ci/budget_eval.py` = PASS(19 pass, 3 skip, normal mode),M4 期 `m4.*` 占位/计数器 SKIP 属预期。`tests/test_budget_namespace.py` 17 passed;`check_schemas` / `trace_matrix --check`(65/65)/ `check_structure` 全 PASS |
