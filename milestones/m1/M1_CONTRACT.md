---
contract: M1
title: 词法、语法与诊断地基
status: closed            # active → closed(close-out 只追加,既有条款 0-byte 修改)
version: v1.0
date: 2026-06-11
timebox: "M+2 ~ M+3(约 8 周,两级结构见 M1_PLAN.md)"
rfc_required: none        # 初版 spec 条款是对 05/13 已选定决策(D-1xx/D-206)的条款化:纯追加、尚无 stable 面;任何偏离已锁定决策的语义动作按 10 §3 升档,判档争议向上取严
upstream_docs:
  - "11 §3 (M1 定义)"
  - "07 §1 §5 §9 (管线分层 D-202 / 诊断架构 D-206 / parser 事件流预留)"
  - "05 §12 (语法基调 D-114)"
  - "10 §4 §5 (spec/conformance 一等公民、feature gate 生命周期 D-403/D-404)"
  - "14 (契约/预算/deferred/测试纪律)"
in_scope:
  - diag_foundation            # Span/SourceMap/DiagCtxt/错误码注册表,先于 lexer(r1 顺序)
  - lexer_and_lexical_spec
  - parser_ast_feature_gate
  - ui_golden_path1
  - rx_fmt_prototype
  - frontend_bench_evidence    # M1 的真实硬件证据交付物(H06 §6 反纯骨架规则)
out_of_scope:
  - name_resolution_hir_typeck   # M2 路线图项(11 §3 M2),非 deferred
  - lossless_syntax_tree         # rowan 式无损语法树完整通道(RD-004)
  - rx_fmt_full_tooling          # RD-005
  - diag_full_bilingual          # RD-006
  - grammar_fuzz_gate            # 里程碑级差分测试,非 M1 门禁(14 §6)
deferred_refs: [RD-004, RD-005, RD-006]
deliverables:
  - id: D-M1-1
    name: 诊断地基(rurixc Rust workspace + Span/SourceMap/DiagCtxt + 错误码注册表 + message-key 骨架)
  - id: D-M1-2
    name: lexer + spec 词法条款(RXS-0xxx 首批)
  - id: D-M1-3
    name: 手写递归下降 parser + AST + feature gate 骨架 + spec 语法条款 + 语法样例集
  - id: D-M1-4
    name: UI golden 测试框架(受控 bless)+ 黄金路径 1(解析错误)
  - id: D-M1-5
    name: rx fmt 雏形
  - id: D-M1-6
    name: 前端性能基准(lexer/parser 吞吐,measured_local 入 m1_budget.json)
acceptance_gates:
  - id: G-M1-1
    check: "语法样例集 100% 解析:conformance/syntax/ 全量样例 0 失败,样例数 ≥100(m1.counter.syntax_corpus_size)"
  - id: G-M1-2
    check: "UI golden 通道全自动:bless 流程可用 + 黄金路径 1 snapshot ≥10 条(m1.counter.ui_golden_path1_snapshots)+ CI 红绿验证过(附 run URL)"
  - id: G-M1-3
    check: "m1_budget.json 中 m1.bench.* 全部条目 evidence=measured_local(三次进程级独立运行 trimmed mean,关闭时零 estimated 残留)"
  - id: G-M1-4
    check: "spec 条款 ↔ 测试 traceability 首版:每条 RXS-0xxx 条款 ≥1 测试锚定(矩阵工具生成,10 §4)"
  - id: G-M1-5
    check: "rx fmt 雏形在语法样例集上幂等:fmt(fmt(x)) == fmt(x) 全量成立"
guardrails:
  - "milestones/m0/m0_budget.json 既有条目 git diff 0-byte(新增条目允许)"
  - "milestones/m0/M0_CONTRACT.md(status: closed)非 close-out 区 0-byte"
  - "registry/deferred.json 与 registry/spike_gating.json 只追加(既有条目修改触发人工审查)"
  - "evidence/ 只增不删不改"
  - "00–14 共 15 份规划文档不被执行 PR 改写(勘误走 00 §6.3 追加式修订)"
  - "tests/ui/ 的 .stderr snapshot 变更必须经审批 bless(bless 是审批动作不是日常操作,14 §6;M1 起激活)"
  - "spec/ 变更必须携带变更档位标记(Direct / Mini-RFC / Full RFC,14 §2;M1 起激活)"
  - "registry/error_codes.json 错误码语义可加不可改(既有码含义字段冻结,10 §6;文件自 M1.1 存在起激活)"
  - "本契约 in_scope/acceptance_gates 等既有条款 0-byte 修改,close-out 只追加"
---

# M1 契约 — 词法、语法与诊断地基

> 所属:[../../11_ROADMAP.md](../../11_ROADMAP.md) §3 M1 / 契约机制见 [../../14_ENGINEERING_DISCIPLINE.md](../../14_ENGINEERING_DISCIPLINE.md) §1
> 顺序不可调换:诊断基础设施(Span/SourceMap/DiagCtxt)先于 lexer 落地——r1 顺序,一切诊断与工具的地基。
> 自本里程碑起,AGENTS.md 硬规则第 7 条(规范先行)实操生效:`src/` 与 `spec/` 实体化,语义 PR 必须引用 RXS-#### 条款号。

---

## 1. 目标

建成 rurixc 前端的第一段:从源码到 AST 的完整通道,以及承载后续一切编译器工作的诊断地基与测试纪律实体。M1 结束时,项目具备:可恢复错误的手写递归下降 parser(语法样例集 100% 解析)、RX#### 错误码与结构化诊断的第一批实例、全自动 UI golden 测试通道(受控 bless)、防风格漂移的 `rx fmt` 雏形,以及前端吞吐的 `measured_local` 基线。

## 2. 范围

### 2.1 in-scope

| 项 | 说明 | 对应交付物 |
|---|---|---|
| 诊断地基 | rurixc Rust workspace(D-201,`src/` 实体化);Span 携带 edition(D-404 预埋)/SourceMap/DiagCtxt/Diag(emit-or-cancel,泄漏即 ICE);错误码注册表 `registry/error_codes.json`(0xxx 词法/语法段,07 §5);message-key 骨架 | D-M1-1 |
| lexer + 词法条款 | spec 词法条款首批(RXS-0xxx,FLS 风格分节)→ lexer 实现(TokenStream,span 全保留)——规范先行(10 §4) | D-M1-2 |
| parser/AST/feature gate | 手写递归下降 parser(错误恢复优先;事件流接口预留,RD-004 双侧标注);AST(贴近用户语法,不做类型/数据流,D-202);feature gate 骨架(10 §5);spec 语法条款 + 语法样例集(conformance/syntax/) | D-M1-3 |
| UI golden 通道 | compiletest 风格:`//~ ERROR RX####` 注释 + `.stderr` snapshot + 路径/行号规范化 + 受控 bless;黄金路径 1 = 解析错误(07 §5 四路径之首) | D-M1-4 |
| rx fmt 雏形 | 语法定型即跟进,防风格漂移(11 §3 M1);MVP 判据为幂等性,完整工具化 → RD-005 | D-M1-5 |
| 前端基准 | lexer/parser 吞吐在开发机 `measured_local` 入 m1_budget.json,为 07 §6 编译性能预算(M2 回填)提前布点;Nightly 同时维持 M0 GPU 基准回归(回归判定 M1+ 生效,BENCH_PROTOCOL §5) | D-M1-6 |

### 2.2 out-of-scope(显式排除)

- 名称解析 / HIR lowering / 类型检查——M2 路线图项(11 §3 M2),非 deferred,不登记编号。
- 无损语法树(rowan 式)完整通道——M1 parser 仅预留事件流接口(07 §9)→ **RD-004**。
- `rx fmt` 完整工具化(配置面/稳定性承诺/rx CLI 收编)→ **RD-005**。
- 诊断消息中英双语全量覆盖——M1 仅 message-key 骨架 + 单语基线(07 §5 第 7 条,首发双语挂 M8)→ **RD-006**。
- grammar-based fuzz 差分测试——里程碑级机制(14 §6),不进 M1 门禁,启用时点随 conformance 体量评估。
- 11 §2 MVP 红线清单全部不触碰([../../registry/spike_gating.json](../../registry/spike_gating.json) SG-001 ~ SG-009 维持 not_triggered)。

## 3. 交付物清单

| ID | 交付物 | 形态 | 完成判据 |
|---|---|---|---|
| D-M1-1 | 诊断地基 | `src/` Rust workspace + `registry/error_codes.json` + message-key 骨架 | cargo test 绿 + error_codes schema 校验 PASS |
| D-M1-2 | lexer + 词法条款 | `spec/` RXS-0xxx 首批 + lexer 实现与单测 | G-M1-4(词法条款部分) |
| D-M1-3 | parser/AST/feature gate + 语法样例集 | parser 实现 + `conformance/syntax/` 样例集 | G-M1-1 + G-M1-4 |
| D-M1-4 | UI golden 通道 + 黄金路径 1 | `tests/ui/` harness + 解析错误 snapshot | G-M1-2 |
| D-M1-5 | rx fmt 雏形 | 独立二进制或 rurixc 子命令(M6 收编前形态自由) | G-M1-5 |
| D-M1-6 | 前端基准 | bench harness + 证据 JSON + [m1_budget.json](m1_budget.json) 回填 | G-M1-3 |

## 4. 验收门(完整版,YAML 头为可提取摘要)

1. **G-M1-1(语法样例集)**:`conformance/syntax/` 全量样例 100% 解析(0 失败);样例数 ≥100。样例数为 `estimated` 性质的工程选择(无历史数据),close-out 前允许经 Direct PR 调整,调整记录留痕(参照 M0 BENCH_PROTOCOL §4.1 调档先例)。
2. **G-M1-2(UI 通道真跑)**:bless 流程可用(受控审批动作);黄金路径 1(解析错误)snapshot ≥10 条;通道必须在 CI 上完成红绿验证——构造一个未审批 bless / snapshot 不匹配的 PR 必须红,修复后转绿,close-out 附 run URL(对齐 G-M0-2 真跑铁律,反 YAML-only)。
3. **G-M1-3(证据锚点)**:`m1_budget.json` 中 `m1.bench.lexer.*` 与 `m1.bench.parser.*` 全部条目回填为 `measured_local`(三次进程级独立运行 trimmed mean,统计协议复用 `bench/stats.py`;阈值 = 实测 × 0.95,对齐 M0 先例);**M1 关闭时本预算文件 `m1.bench.*` 零 `estimated` 残留**。
4. **G-M1-4(traceability 首版)**:spec 条款 ↔ 测试锚定矩阵工具首版可执行;每条 RXS-0xxx 条款 ≥1 条 conformance 或 UI 测试锚定(10 §4:每条款 ≥1 测试)。
5. **G-M1-5(fmt 幂等)**:`rx fmt` 雏形对语法样例集全量满足 `fmt(fmt(x)) == fmt(x)`(脚本核对,字节级比较)。

## 5. Guardrails(字节级,机器核对)

见 YAML 头 `guardrails` 字段。核对方式:`ci/check_guardrails.py <上一里程碑 close tag>`(M1 期基准 = `m0-closed`)。M1 新激活三项(UI bless 审批 / spec 档位标记 / 错误码注册表只追加)对应 14 §2 常驻集的逐项激活,激活留痕见 [../m0/CI_GATES.md](../m0/CI_GATES.md) 修订记录与 [CI_GATES.md](CI_GATES.md)。

## 6. Deferred 引用

| 编号 | 内容摘要 | 承接 |
|---|---|---|
| RD-004 | 无损语法树(rowan 式)完整通道 | M6 |
| RD-005 | rx fmt 完整工具化 | M6 |
| RD-006 | 诊断中英双语全量覆盖 | M8 |

详情以 [../../registry/deferred.json](../../registry/deferred.json) 为唯一事实源,本表仅引用。M0 遗留 RD-001(M8)/RD-002(M5)/RD-003(M6)不属 M1 范围,维持原承接。

## 7. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-06-11 | 初版契约固化 |

---

## 8. Close-out(只追加区 — 开工时为空)

<!-- 验收记录、guardrail 核对输出、deferred 继承/关闭记录追加于此;上方条款 0-byte 修改。 -->

### 8.1 Close-out 草案(2026-06-11,M1.4;状态:**已人工批准**,见 §8.3)

> 本节由 M1.4 收尾工作追加(Assisted-by: cursor:fable-5);全部数字来自命令真实输出。
> 关闭判定与 `status: active → closed` 已由 qwasg 终审落笔,签字与 run URL 见 §8.3。

#### 8.1.1 验收门证据

| 门 | 判据 | 证据(命令真实输出) | 状态 |
|---|---|---|---|
| G-M1-1 | conformance/syntax/ 100% 解析,≥100 样例 | `cargo test --test syntax_corpus` 4 passed;`budget_eval`:`m1.counter.syntax_corpus_size: PASS — 104 个语法样例(要求 ≥100)` | 达成 |
| G-M1-2 | bless 流程可用 + snapshot ≥10 + CI 红绿验证 | `cargo test --test ui_golden` 4 passed;`m1.counter.ui_golden_path1_snapshots: PASS — 12 条 .stderr snapshot(要求 ≥10)`;CI 红验证 PR #2 run 27356303114 → `[check_guardrails] FAIL ... .stderr 变更未附 bless 审批行`;同 PR 补 bless 审批并改为自洽 fixture 后 run 27356777420 → `success`;详见 §8.3 | 达成 |
| G-M1-3 | m1.bench.* 全部 measured_local,零 estimated 残留 | `py -3 ci/budget_eval.py --strict` → `PASS (14 pass, 0 skip, strict mode)`;lexer 213.183 MB/s(阈值 202.52)、parser 3485.183 kloc/s(阈值 3310.92);证据 `evidence/frontend_{lexer,parser}_20260611_{1..3,agg}.json` | 达成 |
| G-M1-4 | traceability 首版,每条款 ≥1 锚定 | `py -3 ci/trace_matrix.py` → `PASS (31/31 clauses anchored, 133 test files scanned)`;产物 `conformance/traceability_matrix.json` | 达成 |
| G-M1-5 | fmt(fmt(x)) == fmt(x) 全量字节级 | `py -3 ci/check_fmt_idempotent.py` → `PASS (104 files, fmt(fmt(x)) == fmt(x) byte-exact)` | 达成 |

#### 8.1.2 guardrail 核对输出(2026-06-11)

```
[check_structure] PASS (11 dirs, 6 files)
[check_schemas] PASS
[check_guardrails] PASS (base=m0-baseline, 233 changed paths)
[budget_eval] PASS (14 pass, 0 skip, strict mode)
[trace_matrix] PASS (31/31 clauses anchored, 133 test files scanned)
[check_fmt_idempotent] PASS (104 files, fmt(fmt(x)) == fmt(x) byte-exact)
cargo test: 116 passed 0 failed(106 lib + 2 fmt_corpus + 4 syntax_corpus + 4 ui_golden)
pytest: 23 passed
```

#### 8.1.3 交付物落位

| 交付物 | 落位 |
|---|---|
| D-M1-1 | `src/rurixc/`(span/source_map/diag/messages)+ `registry/error_codes.json`(RX0001~RX0011) |
| D-M1-2 | `spec/lexical.md`(RXS-0001~0010)+ `src/rurixc/src/lexer.rs` |
| D-M1-3 | `spec/syntax.md`(RXS-0011~0031)+ `parser.rs`/`ast.rs`/`feature_gate.rs` + conformance 104 样例 |
| D-M1-4 | `render.rs` + `tests/ui_golden.rs` + `tests/ui/parse/` 12 对 snapshot + bless guardrail |
| D-M1-5 | `fmt.rs` + `rx_fmt` 二进制 + `ci/check_fmt_idempotent.py` |
| D-M1-6 | `bench/{lexer,parser}_bench.py` + `frontend_triple_run.py` + 证据 8 份 + m1_budget 回填 |

#### 8.1.4 deferred 处置

RD-004(无损语法树,M6)/ RD-005(rx fmt 完整工具化,M6)/ RD-006(诊断双语,M8)维持原承接,无新增 deferred;parser 事件流接口已以 `// STUB(RD-004)` 双侧标注预留。

### 8.2 人工待办清单(已完成,证据见 §8.3)

1. **G-M1-2 红绿程序(CI 真跑)**:已构造"篡改 snapshot / 未 bless"PR → CI 红;同 PR 补 `tests/ui/bless_log.md` 审批行并改为自洽 fixture 后转绿;两次 run URL 见 §8.3。
2. **CI_GATES §5.3**:spec 档位违规与错误码冻结红验证均已完成,run URL 见 §8.3。
3. **bless_log.md 首条记录批签**:已按只追加方式补 qwasg 批签行,见 §8.3。
4. **m1_budget.json 回填审查**:M1.4-E 对既有 estimated 条目的 measured_local 回填已复核,见 §8.3。
5. 终审本草案 → 人工落笔 `status: active → closed` 与 §8.3 验收签字已完成。

### 8.3 Run URL 与签字(人工追加区)

<!-- 红绿 run URL、批准记录由人类追加于此。 -->

#### 8.3.1 Run URL

- G-M1-2 红验证(未审批 bless / snapshot 篡改):PR #2 `https://github.com/qwasg/Rurix/pull/2`, run `https://github.com/qwasg/Rurix/actions/runs/27356303114` — `failure`,命中 `tests/ui/bless_log.md: .stderr 变更未附 bless 审批行`。
- G-M1-2 绿验证(补 bless 审批后转绿):PR #2 `https://github.com/qwasg/Rurix/pull/2`, run `https://github.com/qwasg/Rurix/actions/runs/27356777420` — `success`;`guardrails`/pytest/GPU smoke/budget/cargo fmt/clippy/test 全绿。
- CI_GATES §5.3 spec 档位违规红验证:PR #3 `https://github.com/qwasg/Rurix/pull/3`, run `https://github.com/qwasg/Rurix/actions/runs/27356546351` — `failure`,命中 `spec/syntax.md: spec 变更未新增修订行`。
- CI_GATES §5.3 error_codes 冻结红验证:PR #4 `https://github.com/qwasg/Rurix/pull/4`, run `https://github.com/qwasg/Rurix/actions/runs/27356330152` — `failure`,命中 `registry/error_codes.json RX0001: 含义字段 title 被修改`。

#### 8.3.2 批准信息

- bless_log.md 首条记录批签:已按只追加方式补 `2026-06-11 | tests/ui/parse/ 初始 12 条 snapshot | 人工终审批准首批 snapshot bless;用于 M1 close-out §8.2/§8.3 留痕 | qwasg`。
- m1_budget.json 回填审查:M1.4-E `estimated → measured_local` 回填已复核;当前 `py -3 ci/budget_eval.py` 输出 `PASS (14 pass, 3 skip, normal mode)`,其中 M1 条目全部 PASS,M2 开工占位按契约 skip。
- close-out 终审批准人:`qwasg`。
- close-out 终审日期:`2026-06-11`。
- 人工落笔 `status: active -> closed`:已完成;闭环 tag 为 `m1-closed`。
