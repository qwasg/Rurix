# G-G2-5 CI run URL 回填 + 第二会话独立复核取证（2026-06-30）

> 地位：G2.5（语言 1.0 + 首个 edition）G-G2-5 已于 `milestones/g2/G2_CONTRACT.md` §8.7 由 agent 完全自主签署（commit `ba3c773`，PR #116）。本报告承 §8.7 末「CI run URL 待 runner 上线回填（不伪造）」诚实缺口，回填真实绿 CI run URL，并记录第二会话对全套验收门的独立复跑（AI 不轻信前会话「备绿」报告，AGENTS v3.0 硬规则 3/10）。
> Provenance：`Assisted-by: cursor:claude-opus-4.8`。所有数字来自命令真实输出。**追加件**：不改 §8.7 原文、不翻 RD/SG status（RD-008 已于 §8.7 翻 closed）、不执行任何 G2.6 close-out 动作。

## 1. CI 步骤 49 真实绿（run URL 回填）

- PR：[#116](https://github.com/qwasg/Rurix/pull/116) `feat/g2.5-edition` → `main`，状态 OPEN / MERGEABLE。
- `gh pr checks 116`：`smoke  pass  5m23s  https://github.com/qwasg/Rurix/actions/runs/28447171962/job/84299473197`
- run URL：**https://github.com/qwasg/Rurix/actions/runs/28447171962**
- 步骤名：`language 1.0 + edition smoke (G2 CI_GATES §2.49, G-G2-5, RFC-0008 + RD-008)`（= `py -3 ci/edition_smoke.py`，GitHub-hosted Windows runner，含 MSVC 14.44 / Windows SDK 10.0.26100 / CUDA v13.3 环境）

CI 日志逐行（真实摘录）：

```
[edition] cargo test -p rurix-pkg --test edition_corpus
[edition] OK edition_corpus (accept 解析 OK + reject RX7020/RX7005 strict-only 拦截)
[edition] cargo test -p rurix-pkg manifest::tests::edition
[edition] OK edition unit tests (RXS-0177~0180)
[edition] py -3 ci/stable_snapshot.py --check
[edition] OK stable snapshot --check (stable 面与入库快照一致)
[edition] OK red (篡改 stable 快照 → --check 翻红)
[edition] OK green-restored (复原 stable 快照 → --check 复绿,红绿闭合)
[edition] PASS (edition 解析/校验真实红绿:accept 解析 OK + 未知 edition RX7020 / 类型错误 RX7005 strict-only 拦截;stable API 快照 RD-008 激活:匹配 + 篡改红绿闭合)
```

结论：step 49 在 GitHub Actions **真跑转绿**（非 SKIP、非本机替代）；§8.7 host-only run URL 缺口闭合，本机 measured_local 与 CI 见证一致。

## 2. 第二会话独立复核（measured_local，逐条真实输出）

| 验证命令 | 结果 |
|---|---|
| `py -3 ci/trace_matrix.py --check` | PASS（180/180 clauses anchored，453 test files scanned） |
| `py -3 ci/check_schemas.py` | PASS |
| `py -3 ci/budget_eval.py --strict` | PASS（69 pass, 0 skip, strict mode，全局零 estimated；`m1.counter.spec_clause_test_anchoring`=180） |
| `py -3 ci/stable_snapshot.py --check` | PASS（spec_clauses=180, error_codes=88, editions=['2026'], subcommands=8） |
| `py -3 ci/edition_smoke.py` | PASS（accept + RX7020/RX7005 reject + 快照篡改红绿闭合） |
| `py -3 ci/bilingual_coverage.py` | PASS（en/zh 88/88） |
| `cargo fmt --check` | 干净（exit 0） |
| `cargo clippy --all-targets --features "dxil-backend shader-stages" -- -D warnings` | 干净（exit 0） |
| `cargo test -p rurix-pkg` | 34/0 |
| `cargo test -p rurix-pkg --test edition_corpus` | 2/0 |
| `cargo test -p rurixc --features "dxil-backend shader-stages" --lib` | 404/0 |
| `cargo test -p rurixc --features "dxil-backend shader-stages" --test dxil_corpus` | 7/0 |
| `cargo test -p rurixc --features "dxil-backend shader-stages" --test dxil_golden` | 5/0（+1 ignored） |
| `cargo test --workspace`（全量 conformance） | 全 ok，零 failed |
| `py -3 ci/check_contribution.py` | exit 0（ADVISORY） |
| `py -3 ci/check_guardrails.py`（base=g1-closed） | exit 0（ADVISORY；flagged 均为 G2 分支 vs g1-closed 既有差异，本任务 append 项未被标红） |

deferred.json 状态复核：`RD-008=closed`、`RD-007=inherited`、`RD-009=open`（仅 RD-008 翻转，revision_log v1.43）；`RD-013/RD-017/RD-021=closed`；`RD-019/RD-020/RD-022/RD-023/RD-024=open`。

## 3. 判定

G-G2-5 的 CI 步骤 49 run URL 缺口已由真实绿 CI run `28447171962` 闭合；G2.5 全套验收门经第二会话独立复跑确认仍全绿，与 §8.7 签字记载一致。**G2 契约整体仍 `active`**——本回填不执行 `g2-closed` tag / 基准切换（g1-closed→g2-closed）/ 契约 status active→closed / G2 整体 close-out / RD-007·RD-009 翻转（均属 G2.6，本任务范围外）。
