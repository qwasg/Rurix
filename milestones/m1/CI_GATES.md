# M1 CI 门禁增量

> 所属契约:[M1_CONTRACT.md](M1_CONTRACT.md)
> 版本:v1.0(2026-06-11)
> 基线:[../m0/CI_GATES.md](../m0/CI_GATES.md)(M0 版全部沿用:runner 约定、PR Smoke 六步、guardrail 五项);本文只规定 M1 期的**增量**与 14 §2 常驻集的逐项激活。
> 铁律不变:任何新增门禁必须在真实 PR 上以真实失败/通过路径验证过(反 YAML-only,H06 D11.8-2)。

---

## 1. Runner

沿用 M0 §1(自托管 RTX 4070 Ti 开发机,GPU 任务串行,无 GPU 显式 fail)。M1 新增的 cargo 步骤为 CPU 任务,不占 GPU 队列。

## 2. PR Smoke 追加步骤(编号接 M0 §3 的 1–6)

| # | 步骤 | 失败即红 |
|---|---|---|
| 7 | `cargo fmt --check` + `cargo clippy -- -D warnings`(rurixc workspace,自 M1.1 存在起) | 是 |
| 8 | `cargo test`(rurixc 单测:Span/SourceMap/DiagCtxt/lexer/parser) | 是 |
| 9 | 语法样例集跑批:`conformance/syntax/` 全量解析,任一失败即红(契约 G-M1-1 通道;样例计数走预算 counter) | 是 |
| 10 | UI golden 运行:`tests/ui/` snapshot 比对,diff 即红(自 M1.4 框架存在起) | 是 |
| 11 | `registry/error_codes.json` schema 校验(并入 `ci/check_schemas.py`,自 M1.1 存在起) | 是 |

M0 步骤 6 的预算 evaluator 自动合并加载 [m1_budget.json](m1_budget.json)(命名空间冲突即红);M1 关闭前 `m1.bench.*` 允许 `estimated` skip,skip_reason 必须输出留痕(14 §3)。

## 3. Nightly 追加

- 前端吞吐基准短协议(lexer/parser,1 次运行冒烟,不更新预算)。
- M0 GPU 基准回归继续跑(SAXPY/bandwidthTest 短协议);**回归判定自 M1 起生效**:Mann-Whitney U(p<0.05)+ Cohen's r > 0.3,报警阈值 1% Warning / 5% Critical([../m0/BENCH_PROTOCOL.md](../m0/BENCH_PROTOCOL.md) §5"M0 建机制,M1+ 生效")。
- Release 层仍不建(RD-001,承接 M8 不变)。

## 4. Guardrail 激活(14 §2 常驻集 → M1 落位三项)

接 M0 §4 五项(全部沿用,其中第 1 项的对比基准自 M1 起为 `m0-closed` tag),新激活:

| # | 项 | 核对方式 |
|---|---|---|
| 6 | `tests/ui/` 的 `.stderr` snapshot 变更必须经审批 bless:snapshot diff 的 PR 必须携带 bless 审批标记,否则 FAIL(bless 是审批动作不是日常操作,14 §6) | guardrail 脚本扩展(M1.4 随 UI 框架落地) |
| 7 | `spec/` 目录变更必须携带变更档位标记(Direct / Mini-RFC / Full RFC,10 §3),无标记即 FAIL | guardrail 脚本扩展(M1.2 随首批条款落地) |
| 8 | `registry/error_codes.json` 只追加:既有错误码的含义字段冻结(可加不可改,10 §6 稳定面) | guardrail 脚本扩展(M1.1 随注册表落地) |

14 §2 常驻集其余项(stable API 快照 / IR golden / unsafe-audit 完整性 / NVIDIA 白名单)在 M1 仍无对应实体,随 M2+ 逐项激活;激活记录继续追加于 [../m0/CI_GATES.md](../m0/CI_GATES.md) 修订记录(该文件 §4 自身约定)与对应里程碑 CI_GATES 文档。

## 5. 验证程序(对应契约 G-M1-2 与新激活 guardrail)

1. UI 框架落地后,提交一个**篡改 snapshot / 未审批 bless** 的 PR → 必须红(guardrail 第 6 项)。
2. 修复后同 PR 转绿 → 合入。
3. spec 档位标记(第 7 项)与错误码冻结(第 8 项)各构造一次故意违规验证红。
4. close-out 附全部 run URL 与关键命令输出。

## 6. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-06-11 | 初版(M1 契约配套) |
