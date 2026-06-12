# M3 CI 门禁增量

> 所属契约:[M3_CONTRACT.md](M3_CONTRACT.md)
> 版本:v1.0(2026-06-12)
> 基线:[../m0/CI_GATES.md](../m0/CI_GATES.md) + [../m1/CI_GATES.md](../m1/CI_GATES.md) + [../m2/CI_GATES.md](../m2/CI_GATES.md)(全部沿用:runner 约定、PR Smoke 1–14 步、guardrail 含 M1.1/M1.2/M1.4 激活项、nightly 工作流);本文只规定 M3 期的**增量**。
> 铁律不变:任何新增门禁必须在真实 PR 上以真实失败/通过路径验证过(反 YAML-only,H06 D11.8-2)。

---

## 1. Runner

沿用 M0 §1(自托管 RTX 4070 Ti 开发机)。M3 新增步骤均为 CPU 任务,不占 GPU 队列;无新增 runner 预置项(LLVM 22.1.7 / VS BuildTools / WinDbg 已随 M2.3 预置)。

## 2. PR Smoke 追加步骤(编号接 M2 §2 的 12–14)

| # | 步骤 | 失败即红 |
|---|---|---|
| 15 | borrowck conformance 批跑:`conformance/borrowck/reject/<category>/` 反例全拦截(逐文件断言产生预期 4xxx 诊断)+ `accept/` 正例 0 诊断(契约 G-M3-1 通道;自 M3.3 存在起,实测命令落地时回填本表修订行) | 是 |
| 16 | const eval 冒烟:`conformance/consteval/` const 泛型程序经全管线产出 EXE → 运行核对退出码/输出(契约 G-M3-4 通道,对齐步骤 12 真跑形态;自 M3.4 存在起) | 是 |

预算 evaluator(M0 步骤 6)自动合并加载 [m3_budget.json](m3_budget.json)(命名空间冲突即红)。**M3 期 PR Smoke 跑 normal 模式**:`m3.counter.*` 建设期未达标 SKIP 属预期;`m2.bench.*` estimated 占位在 M3.4 回填前继续 SKIP。**M3 close-out 必须跑 `--strict` 且全局零 estimated 残留**(契约 G-M3-3;14 §3 占位存活 ≤2 里程碑,m2.bench.* 在本里程碑到期)。

## 3. Nightly 追加

- 既有 nightly 全保留(lexer/parser/SAXPY 冒烟 + budget normal + self-profile 归档,M2 CI_GATES v1.3 实体化)。
- self-profile 归档自然覆盖 M3 新增阶段计数器(TBIR/borrowck/const eval 布点随实现扩列,非门禁,趋势参考)。
- 预算回填(M3.4)落地后,nightly 的 budget 步骤对 `m2.bench.*` 自动转为 measured_local 实测核对(evaluator 既有逻辑,无需改 workflow)。
- Release 层仍不建(RD-001,承接 M8 不变)。

## 4. Guardrail

沿用 M0 五项 + M1 三项(spec 档位 / 错误码冻结 / UI bless)。两项 M3 期动作:

1. **基准 ref 切换**:`m2-closed` tag 已随 M2 终审打出(2026-06-12);M3.1 第 1 项将 `ci/check_guardrails.py` 本地/push 回退基准 `m1-closed → m2-closed`(PR 路径仍以 GITHUB_BASE_REF 为准),切换前双基准核对,落地留痕本表修订行。
2. **MIR 文本 golden 激活**(14 §2 常驻集,M2 CI_GATES §4 预告的评估时点到期):M3.2 做预评估(MIR 形态随 TBIR 重排/drop elaboration 定型),M3.3 激活——golden 基线入库 + 核对入 CI,激活必须经真实红绿验证;基线变更纪律对齐 UI bless(变更须审批留痕),具体形态(独立脚本 vs cargo test 快照升格)在激活 PR 中裁决留痕。

14 §2 常驻集其余项的 M3 期评估结论:

| 项 | 结论 |
|---|---|
| stable API 快照 | M3 无 stable 面,不激活 |
| unsafe-audit 完整性 | rurixc 实现侧 `unsafe_code = deny` 维持;出现首个 unsafe 块时按 AGENTS 硬规则 9 激活 |
| NVIDIA 白名单 | device 路径 M4 起评估 |

m2_budget.json 的 G-M3-3 回填走 `check_guardrails.py` 既有机制("estimated 条目只允许回填为 measured_local"),不属新增激活项。

## 5. 验证程序(对应契约 G-M3-1/G-M3-4 与步骤 15/16)

1. 步骤 15 落地后,构造一个**故意放行某反例类别**(或篡改 reject 语料预期)的 PR → 必须红;修复后同 PR 转绿,两次 run URL 随 close-out 归档。
2. 步骤 16 落地后,同法构造 const eval 失败路径(篡改预期输出或基线)→ 红 → 绿,run URL 归档。
3. MIR golden 激活时(§4 第 2 项),故意改动 MIR 输出不更新基线 → 红;按审批流程更新基线 → 绿,run URL 归档。
4. close-out 附 `budget_eval --strict` 输出原文(契约 G-M3-3 零 estimated 残留判定)。

## 6. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-06-12 | 初版(M3 契约配套;步骤 15/16 为 M3.3/M3.4 计划项,落地时回填实测命令) |
