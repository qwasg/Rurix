---
contract: M3
title: MIR 完整化与借用检查
status: closed            # active → closed(close-out 只追加,既有条款 0-byte 修改);M3.4 终审关闭 2026-06-13
version: v1.0
date: 2026-06-12
timebox: "M+5 ~ M+7(约 8 周,两级结构见 M3_PLAN.md)"
rfc_required: none        # 借用/const eval 语义条款是对 05/07/13 已锁定决策(D-105/D-111/D-202/D-204)的条款化:纯追加、尚无 stable 面;任何偏离已锁定决策的语义动作按 10 §3 升档,判档争议向上取严
upstream_docs:
  - "11 §3 (M3 定义)"
  - "07 §1 §4 §5 §6 (IR 四层与 TBIR 窄门 D-202 / NLL 借用检查 D-204 / 错误码段位 / 编译性能预算)"
  - "05 §3 §4 §9 (host 所有权 D-105 / affine 资源与 Drop / const 泛型与 const eval D-111)"
  - "08 §3 (telemetry 计数器非零规则 D-235,预算回填数据源)"
  - "14 (契约/预算/deferred/证据分级/测试纪律)"
in_scope:
  - desugar_closeout           # for/`?` desugar 收口 + codegen"暂不支持"诊断面收窄(M2_PLAN v1.1/v1.2/v1.3 留痕的 M3 承接项)
  - tbir_narrow_gate           # TBIR 窄门:模式穷尽性/方法糖显式化/drop scope,临时存在、构造 MIR 后释放(D-202,07 §1)
  - move_init_drop             # move/init 数据流 + drop elaboration(affine 语义闭环,D-105;4xxx 错误码首批)
  - nll_borrowck               # NLL 借用检查 host 全量(region 推断 → in-scope borrows → 报错 walk,D-204;明确不做 Polonius)
  - const_eval_interp          # const eval MIR 解释器 + const 泛型可用(D-111:整数/bool + 简单算术;5xxx 错误码首批)
  - ui_golden_path3            # 黄金路径 3 = 借用错误(4xxx;const eval 5xxx 同期入 UI 通道,07 §5)
  - spec_semantic_clauses      # spec/borrow.md + spec/consteval.md(RXS-0048 起,规范先行;含 RXS-0041 预留的"子类型仅限生命周期"条款化)
out_of_scope:
  - device_codegen_coloring    # device codegen 与着色检查深度 → M4 路线图项(11 §3),非 deferred
  - views_disjoint_barrier     # views 不相交证明 / barrier 一致性 = MIR 借用检查的 device 扩展 pass → 随 M4 device 路径(07 §4)
  - polonius                   # D-204 明确不做(r1:2026 仍未 stable 且有 soundness issue)
  - borrow_diag_polish         # 借用诊断打磨:先正确性后诊断(07 §4 NLL migration 教训),MVP 诊断允许保守粗糙,UI golden 锁底线后逐步打磨
  - const_eval_heap_trait      # 堆分配 const eval / trait 调度 const eval(D-111 明确不做)
  - cross_session_incremental  # 跨会话红绿增量与并行前端(D-203 Phase 2+)
  - mlir_kernel_island         # SG-001 维持 not_triggered(D-208);dyn/特化/HKT/async(D-104)与宏(D-111/SG-006)永久裁剪,11 §2 红线全部不触碰
deferred_refs: []              # M3 开工无预造 deferred;执行期按需登记 RD-###(14 §4)
deliverables:
  - id: D-M3-1
    name: desugar 收口 + TBIR 窄门(for/`?` desugar;模式穷尽性/方法糖显式化/drop scope;HIR→TBIR→MIR 重排)
  - id: D-M3-2
    name: spec 语义条款(spec/borrow.md + spec/consteval.md,RXS-0048 起,规范先行)
  - id: D-M3-3
    name: move/init 数据流 + drop elaboration(affine 语义闭环;4xxx 错误码首批)
  - id: D-M3-4
    name: NLL 借用检查 host 全量 + 黄金路径 3(conformance/borrowck/ 语料 + tests/ui/borrowck/ snapshot)
  - id: D-M3-5
    name: const eval MIR 解释器 + const 泛型可用(5xxx 错误码首批 + 真跑验证程序)
  - id: D-M3-6
    name: 编译性能预算实测回填(m2.bench.* estimated → measured_local)+ MIR golden guardrail 评估激活
acceptance_gates:
  - id: G-M3-1
    check: "借用检查 conformance 初版:conformance/borrowck/ 预设 7 类错误类别(§4 清单)反例全拦截 + 正例 0 诊断(m3.counter.borrowck_conformance_categories ≥7,CI 批跑)"
  - id: G-M3-2
    check: "黄金路径 3(借用错误)snapshot ≥10(m3.counter.ui_golden_path3_snapshots),复用 UI 通道与 bless guardrail"
  - id: G-M3-3
    check: "预算实测回填:m2.bench.cold_compile_hello_world_ms 与 m2.bench.check_latency_ms 转 measured_local(三次进程级独立运行 trimmed mean,bench/stats.py 协议),close-out 跑 budget_eval --strict 零 estimated 残留"
  - id: G-M3-4
    check: "const eval 真跑:const 泛型程序经全管线产出 EXE,运行退出码/输出验证(CI 自动核对,对齐 G-M2-1 真跑铁律);5xxx 错误 snapshot 入 UI 通道"
  - id: G-M3-5
    check: "M3 新增 RXS 条款 ≥1 测试锚定(trace_matrix 全局口径,沿用 m1.counter.spec_clause_test_anchoring)"
guardrails:
  - "milestones/m0/m0_budget.json 与 milestones/m1/m1_budget.json 的 measured_local 既有条目 git diff 0-byte(新增条目允许)"
  - "milestones/m2/m2_budget.json 既有 estimated 条目仅允许回填为 measured_local(G-M3-3 通道,check_guardrails 既有机制)+ revision_log 追加;其余既有条目 0-byte"
  - "milestones/m0/M0_CONTRACT.md、milestones/m1/M1_CONTRACT.md、milestones/m2/M2_CONTRACT.md(均 closed)既有内容只追加不修改"
  - "registry/deferred.json 与 registry/spike_gating.json 只追加(既有条目修改触发人工审查)"
  - "evidence/ 只增不删不改"
  - "00–14 共 15 份规划文档不被执行 PR 改写(勘误走 00 §6.3 追加式修订)"
  - "tests/ui/ 的 .stderr snapshot 变更必须经审批 bless(M1.4 已激活,check_ui_bless)"
  - "spec/ 变更必须携带变更档位标记(M1.2 已激活)"
  - "registry/error_codes.json 错误码语义可加不可改(M1.1 已激活)"
  - "MIR 文本 golden:随 M3 MIR 定型激活为 guardrail(14 §2 常驻集,M2 CI_GATES §4 预告;激活时点与红绿验证见 CI_GATES.md §4)"
  - "本契约 in_scope/acceptance_gates 等既有条款 0-byte 修改,close-out 只追加"
---

# M3 契约 — MIR 完整化与借用检查

> 所属:[../../11_ROADMAP.md](../../11_ROADMAP.md) §3 M3 / 契约机制见 [../../14_ENGINEERING_DISCIPLINE.md](../../14_ENGINEERING_DISCIPLINE.md) §1
> 规范先行延续(AGENTS.md 硬规则第 7 条):借用/const eval 语义 PR 必须引用 RXS-#### 条款号;缺条款先补 spec(spec/borrow.md、spec/consteval.md 自 M3.1/M3.3 实体化)。
> 基准 ref:`m2-closed` tag 已随 M2 终审打出(2026-06-12);guardrail 核对基准自 M3 开工切换 `m1-closed → m2-closed`,切换留痕 [CI_GATES.md](CI_GATES.md) 修订表。

---

## 1. 目标

把 rurixc 的静态语义补完到 **host 全量安全检查**:经 TBIR 窄门(模式/方法糖/drop scope)定型 MIR,落下 move/init 数据流与 drop elaboration(affine 语义闭环),建成 NLL 借用检查(host 全量,D-204),并交付 const eval MIR 解释器使 const 泛型可用(D-111)。同时兑现 M2 的两笔承接:for/`?` desugar 收口(M2_PLAN v1.1/v1.2 推迟项)与编译性能预算首次实测回填(m2.bench.* 占位 ≤2 里程碑的硬约束)。M3 结束时,Rurix host 子集的"安全"承诺第一次由编译器静态强制——为 M4 device 路径(views 不相交证明实现为 MIR 借用检查扩展 pass)备好地基。

## 2. 范围

### 2.1 in-scope

| 项 | 说明 | 对应交付物 |
|---|---|---|
| desugar 收口 | `for` → loop+match、`?` → match desugar 落地(依赖 Iterator/Result lang-item 最小面);MIR lowering 的"暂不支持"诊断面收窄(M2_PLAN v1.3 附带口径的承接) | D-M3-1 |
| TBIR 窄门 | typed-body IR:模式匹配穷尽性、autoderef/方法糖显式化、drop scope 显式化;**临时存在、构造 MIR 后即释放**(控峰值内存,D-202/r1);HIR→TBIR→MIR 管线重排 | D-M3-1 |
| spec 语义条款 | `spec/borrow.md`(所有权/move/借用/生命周期/drop)+ `spec/consteval.md`(const fn 子集/const 泛型求值),RXS-0048 起,FLS 体例;**条款 PR 先于实现 PR**;含 RXS-0041 预留的"子类型仅限生命周期"条款化 | D-M3-2 |
| move/init + drop | MIR 数据流框架;move/init 分析(use-after-move/use-before-init);drop elaboration(条件初始化的 drop flag);affine 语义闭环(05 §3.1/§4) | D-M3-3 |
| NLL 借用检查 | region 推断变量替换 → MIR type check 收集 region constraints → region inference → 逐点 in-scope borrows → 报错 walk(D-204 流程照搬 r1);`mir_borrowck(body_id)` query 化(D-203);4xxx 错误码 + 黄金路径 3 | D-M3-4 |
| const eval | MIR 解释器(算术/分支/循环/数组构造,05 §9);const 泛型(整数/bool + 简单算术表达式)接入类型系统与单态化;5xxx 错误码(溢出/越界/非 const 操作) | D-M3-5 |
| 预算实测回填 | `m2.bench.cold_compile_hello_world_ms` / `m2.bench.check_latency_ms` 由 estimated 转 measured_local(07 §6;数据源 = M2 self-profile 基础设施);MIR 文本 golden 的 guardrail 化评估(14 §2 常驻集) | D-M3-6 |

### 2.2 out-of-scope(显式排除)

- device 路径 codegen 与着色/地址空间检查深度——M4 路线图项(11 §3),非 deferred,不登记编号。
- views 不相交证明与 barrier 一致性——实现为 MIR 借用检查的 device 扩展 pass(07 §4),随 M4 device 路径,本里程碑只保证 host 借用检查的 pass 结构可扩展。
- Polonius——D-204 明确不做(永久,r1 最强警告)。
- 借用诊断打磨——先正确性、后诊断对齐(07 §4 NLL migration 教训):MVP 借用诊断允许保守粗糙,黄金路径 3 snapshot 锁住质量底线后逐步打磨,不在本契约设诊断质量门。
- 堆分配 const eval / trait 调度 const eval(D-111 明确不做)。
- 跨会话增量编译与并行前端(D-203 Phase 2+);MLIR kernel-island(SG-001)、dyn trait/特化/HKT/async(D-104 永久裁剪)、过程宏/声明宏(D-111/SG-006)。
- 11 §2 MVP 红线清单全部不触碰([../../registry/spike_gating.json](../../registry/spike_gating.json) SG-001 ~ SG-009 维持 not_triggered)。

## 3. 交付物清单

| ID | 交付物 | 形态 | 完成判据 |
|---|---|---|---|
| D-M3-1 | desugar 收口 + TBIR 窄门 | `src/rurixc/` desugar/tbir 模块 + 管线重排;模式穷尽性检查 | cargo test 绿 + 条款锚定(G-M3-5);hello-world 冒烟(CI 步骤 12/13/14)持续绿 = 回归网 |
| D-M3-2 | spec 语义条款 | `spec/borrow.md` + `spec/consteval.md`(RXS-0048+) | G-M3-5 |
| D-M3-3 | move/init + drop elaboration | MIR 数据流框架 + drop flag;4xxx 首批 | 单测 + conformance 正例 0 诊断 |
| D-M3-4 | NLL 借用检查 + 黄金路径 3 | borrowck 模块(query 化)+ `conformance/borrowck/` + `tests/ui/borrowck/` snapshot | G-M3-1 + G-M3-2 |
| D-M3-5 | const eval + const 泛型 | MIR 解释器 + 5xxx 错误码 + const 泛型真跑程序 | G-M3-4 |
| D-M3-6 | 预算实测回填 + MIR golden 评估 | [../m2/m2_budget.json](../m2/m2_budget.json) 回填(revision 留痕)+ 证据 JSON;MIR golden guardrail 化记录 | G-M3-3 |

## 4. 验收门(完整版,YAML 头为可提取摘要)

1. **G-M3-1(借用检查 conformance 初版)**:`conformance/borrowck/` 按类别组织(`reject/<category>/*.rx` + 正例 `accept/*.rx`),预设 **7 类错误类别**全拦截、正例 0 诊断,CI 批跑核对(步骤 15)。预设类别清单(`m3.counter.borrowck_conformance_categories` 计数对象,类别即 reject/ 下子目录):
   1. `use_after_move` — 使用已被 move 的值;
   2. `use_before_init` — 使用未初始化/可能未初始化的局部;
   3. `double_mut_borrow` — 两个 `&mut` 借用同时存活;
   4. `shared_mut_conflict` — `&` 与 `&mut` 借用冲突;
   5. `move_while_borrowed` — 借用存活期间 move 所有者;
   6. `assign_while_borrowed` — 借用存活期间写入被借用位置;
   7. `dangling_reference` — 引用活过其指代物(返回局部引用/借用活过作用域)。
   类别数为 `estimated` 性质工程选择(对齐 G-M1-1/G-M2-3 先例),增删类别经 Direct PR 留痕。
2. **G-M3-2(黄金路径 3)**:`tests/ui/borrowck/` 借用错误 snapshot ≥10 条(`m3.counter.ui_golden_path3_snapshots`),走 M1.4 已激活的 bless 审批 guardrail;诊断措辞允许保守粗糙(§2.2 诊断打磨排除项),snapshot 的作用是锁行为底线。
3. **G-M3-3(预算实测回填)**:[../m2/m2_budget.json](../m2/m2_budget.json) 两条 `m2.bench.*` 占位转 `measured_local`——冷编译 hello-world 端到端耗时与全量 check 延迟各做**三次进程级独立运行取 trimmed mean**(统计协议复用 `bench/stats.py`,数据源 = `rurixc --self-profile` + 进程计时),证据 JSON 入 `evidence/`;阈值 = 实测值加余量,设定数值经人工批准留痕(硬规则 1)。close-out 跑 `py -3 ci/budget_eval.py --strict` 输出**全局零 estimated 残留**(14 §3 占位存活 ≤2 里程碑的硬约束,本契约是 m2.bench.* 的到期里程碑,逾期即 FAIL)。
4. **G-M3-4(const eval 真跑)**:const 泛型程序(数组长度/`const fn` 求值类,入 `conformance/consteval/`)经 rurixc 全管线产出 EXE,CI 自动核对运行退出码与预期输出(对齐 G-M2-1 真跑铁律,步骤 16);5xxx 错误码(const 求值溢出/越界/非 const 操作)snapshot 入 `tests/ui/consteval/`(计入全局 snapshot 计数,不另立计数器)。
5. **G-M3-5(traceability 延续)**:M3 新增 RXS 条款(borrow/consteval)每条 ≥1 测试锚定;`ci/trace_matrix.py` 全局口径核对(`m1.counter.spec_clause_test_anchoring` 为全局断言,无需另立 m3 计数器)。

## 5. Guardrails(字节级,机器核对)

见 YAML 头 `guardrails` 字段。核对方式:`ci/check_guardrails.py <基准ref>`(默认基准自 M3 开工切换 `m1-closed → m2-closed`,切换留痕 [CI_GATES.md](CI_GATES.md) 修订表)。M3 期计划新增激活项一项:**MIR 文本 golden**(14 §2 常驻集;M2 CI_GATES §4 预告"随 M3 MIR 定型评估"),激活时点 = TBIR/drop elaboration 落地、MIR 形态定型之后(M3_PLAN §4),激活必须经真实红绿验证(反 YAML-only 铁律)。m2_budget.json 的回填通道是 `check_guardrails.py` 既有机制("estimated 条目只允许回填为 measured_local"),无需新代码。

## 6. Deferred 引用

M3 开工时无预造 deferred(`deferred_refs: []`);M0/M1 遗留 RD-001(M8)/RD-002(M5)/RD-003(M6)/RD-004(M6)/RD-005(M6)/RD-006(M8)不属 M3 范围,维持原承接。M2 的 desugar 推迟与 codegen"暂不支持"口径(M2_PLAN v1.1/v1.2/v1.3 留痕,非 deferred)由本契约 `desugar_closeout` 范围项正式承接收口。执行期做不完的事按 14 §4 追加 `RD-###` 并双侧标注。

## 7. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-06-12 | 初版契约固化 |

---

## 8. Close-out(只追加区 — 开工时为空)

<!-- 验收记录、guardrail 核对输出、deferred 继承/关闭记录追加于此;上方条款 0-byte 修改。 -->

### 8.1 验收门核验(G-M3-1 ~ G-M3-5)

> 记于 M3.4 close-out 草拟(M3_PLAN §4 任务 6);关闭判定人工。本节为 0-byte 上方条款的只追加区。

| 门 | 状态 | 证据 |
|---|---|---|
| G-M3-1 借用检查 conformance(7 类反例全拦截 + 正例 0 诊断) | 达成(M3.3) | `m3.counter.borrowck_conformance_categories` = 7(`conformance/borrowck/reject/` 七类目录);CI 步骤 15 `cargo test -p rurixc --test borrowck_corpus` 接入 pr-smoke;红绿真跑见 [CI_GATES.md](CI_GATES.md) v1.3 |
| G-M3-2 黄金路径 3(借用错误)snapshot ≥10 | 达成(M3.3) | `m3.counter.ui_golden_path3_snapshots` = 10(`tests/ui/borrowck/`);bless 留痕 [../../tests/ui/bless_log.md](../../tests/ui/bless_log.md) |
| G-M3-3 预算实测回填(零 estimated 残留) | 达成(M3.4 WP4) | `milestones/m2/m2_budget.json` 两条 `m2.bench.*` 转 measured_local(冷编译 122.652 ms / check 延迟 6.912 ms,三次进程级独立运行 trimmed mean,证据 `evidence/compile_cold_20260613_agg.json` / `evidence/compile_check_20260613_agg.json`);`py -3 ci/budget_eval.py --strict` = **PASS (19 pass, 0 skip, strict mode)**。阈值余量 ×1.5(cold 上界 183.98 ms / check 上界 10.37 ms)经**人工终审批准**(硬规则 1,qwasg 会话授权,m2_budget.json revision_log v1.3 留痕) |
| G-M3-4 const eval 真跑(全管线产 EXE + 运行核对) | 达成(M3.4 WP2/WP3,const fn 求值类) | `conformance/consteval/const_eval_run.rx` 经全管线产 EXE → 运行 stdout `consteval-ok` / exit 0(const fn 算术·分支 + const item 引用链编译期求值驱动);CI 步骤 16 `py -3 ci/consteval_smoke.py compile-run` 接入 pr-smoke;5xxx snapshot(RX5001/RX5003)入 `tests/ui/consteval/`。**const 泛型值的运行期单态化随 M4+,登记 RD-007**(标量优先,见 §8.3) |
| G-M3-5 traceability 延续(新 RXS 条款 ≥1 锚定) | 达成(M3.4 WP5) | `py -3 ci/trace_matrix.py --check` = PASS(65/65 条款全锚定,含新增 RXS-0062 ~ RXS-0065);`m1.counter.spec_clause_test_anchoring` 全局口径 PASS |

### 8.2 guardrail / 门禁核验输出(M3.4 WP6 留痕)

- `py -3 ci/check_guardrails.py m2-closed` = **PASS (base=m2-closed, 95 changed paths)**(规划文档 0-byte;registry 只追加含 RX5001~RX5003 / RD-007;m2_budget.json estimated→measured_local 经既有机制;spec/ 档位标记齐;UI bless 留痕齐)。
- `py -3 ci/check_schemas.py` = PASS(error_codes 5xxx message-key 交叉校验过;compile 证据经新增 `milestones/m3/compile_evidence_schema.json` 校验)。
- `py -3 ci/budget_eval.py --strict` = **PASS (19 pass, 0 skip)** —— G-M3-3 全局零 estimated 残留判定通过(14 §3 占位存活 ≤2 里程碑硬约束,m2.bench.* 本里程碑到期清偿;阈值余量 ×1.5 经人工终审,qwasg 会话授权)。
- `cargo test --workspace` = 全绿(rurixc 243 lib 单测含 const_eval 9 项 + mir_build const 内联 1 项;集成测试 borrowck/mir_golden/ui_golden 等全过);`cargo fmt --all --check` / `cargo clippy --workspace --all-targets -- -D warnings` 干净。

### 8.3 deferred 登记/继承

- **RD-007**(执行期登记,M3.4 WP3):const 泛型值的运行期单态化(turbofish const 实参 → 实例值代入 + codegen)随 M4+ 接入。理由:turbofish 实参在 HIR 降级处丢弃、无 const 值的类型级表示、单态化 substs 为纯类型向量,跨层改造与 M3.4 标量优先预算不成比例(07 §4 保守先行)。const eval MIR 解释器核心(D-111,RXS-0062~0065)与 const fn/const item 求值真跑(G-M3-4 通道)已交付;语义已于 `spec/consteval.md` RXS-0064 条款化(规范先行),回填仅补实现侧。见 [../../registry/deferred.json](../../registry/deferred.json) RD-007。
- M0/M1 遗留 RD-001 ~ RD-006 维持原承接里程碑,M3 不变更。

### 8.4 红绿真跑 run URL 归档(对齐 CI_GATES §5)

- 步骤 15(borrowck conformance)红绿:M3.3 已归档([CI_GATES.md](CI_GATES.md) v1.3/v1.5 口径)。
- MIR golden guardrail 红绿:M3.3 WP6 已归档真实 CI 绿跑([CI_GATES.md](CI_GATES.md) v1.5,run [27458630302](https://github.com/qwasg/Rurix/actions/runs/27458630302))。
- 步骤 16(const eval smoke)红绿:真实 CI 绿跑已归档——pr-smoke run [27460135247](https://github.com/qwasg/Rurix/actions/runs/27460135247)(PR #7,event=pull_request,HEAD `88b6820`)conclusion=**success**,其中 "const eval smoke" 步骤(`py -3 ci/consteval_smoke.py compile-run`)与 guardrails / budget evaluator / borrowck conformance / MIR golden 步骤均真实通过。失败(red)路径本地真实执行验证(篡改 `const SIDE` 4→5 → const eval 算出 SUM=33≠24 → stdout `consteval-bad` → smoke exit 1 红;复原 → exit 0 绿,非 YAML-only,CI_GATES §5 第 2 项,对齐 v1.3/v1.5 先例)。

> M3 关闭综述:rurixc host 子集静态语义补完——desugar 收口 + TBIR 窄门(M3.1)、move/init + drop elaboration(M3.2)、NLL 借用检查(M3.3)、const eval MIR 解释器 + 5xxx + 预算实测回填(M3.4)。M3 期 5xxx const eval 段位首次启用;编译性能预算占位清偿(零 estimated 残留)。M4 device 路径承接项:RD-007(const 泛型值单态化)+ 运行期数组 aggregate codegen(spec/consteval.md RXS-0064 范围裁决)。关闭判定人工。
