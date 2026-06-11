---
contract: M2
title: HIR、类型检查与 host 编译闭环
status: active            # active → closed(close-out 只追加,既有条款 0-byte 修改)
version: v1.0
date: 2026-06-11
timebox: "M+3 ~ M+5(约 8 周,两级结构见 M2_PLAN.md)"
rfc_required: none        # 初版 spec 语义条款是对 05/07/13 已锁定决策(D-1xx/D-2xx)的条款化:纯追加、尚无 stable 面;任何偏离已锁定决策的语义动作按 10 §3 升档,判档争议向上取严
upstream_docs:
  - "11 §3 (M2 定义)"
  - "07 §1 §2 §3 §5 §6 §8 (管线分层 D-202 / query D-203 / 类型推断 / 诊断 / 编译性能预算 / host codegen D-209)"
  - "05 §2 §3 §8 §9 §10 (trait 子集 D-104 / host 所有权 D-105 / Result D-110 / 泛型 D-111 / 模块 D-112)"
  - "08 §5 (host 调试 PDB/WinDbg D-237)"
  - "14 (契约/预算/deferred/证据分级/测试纪律)"
in_scope:
  - name_resolution_hir        # 名称解析 + HIR lowering(item/body 分离,D-202)
  - typeck_host_subset         # 类型收集/HM 局部推断/检查(函数/struct/enum/泛型单态化雏形,D-104/D-111;签名全标注)
  - query_skeleton             # query 风格 API + 进程内 memo(D-203 第一天形态)
  - mir_host_codegen           # MIR 雏形 + LLVM(pin 22.1.x)→ COFF → link.exe → hello-world EXE + PDB(D-205/D-209)
  - ui_golden_path2            # 黄金路径 2 = 类型错误(2xxx;名称/模块 1xxx 同期分配,07 §5)
  - self_profile               # query 级计时 + 阶段计数器(07 §6,-Z self-profile 式)
  - spec_semantic_clauses      # spec/names.md + spec/types.md(RXS-0032 起,规范先行)
out_of_scope:
  - tbir_nll_const_eval        # TBIR 窄门/NLL 借用检查/const eval → M3 路线图项(11 §3),非 deferred
  - device_codegen_coloring    # device codegen 与着色检查深度 → M4 路线图项(11 §3)
  - mlir_kernel_island         # SG-001 维持 not_triggered(D-208)
  - dyn_trait_specialization   # dyn/特化/HKT/async 永久裁剪(D-104),不进任何里程碑
  - proc_macros                # 过程宏/声明宏 MVP 裁剪(D-111;声明宏 gating 见 SG-006)
  - cross_session_incremental  # 跨会话红绿增量与并行前端(D-203 Phase 2+)
  - lld_link_default           # lld-link 仅 opt-in,默认 link.exe 不动(D-209)
deferred_refs: []              # M2 开工无预造 deferred;执行期按需登记 RD-###(14 §4)
deliverables:
  - id: D-M2-1
    name: 名称解析 + HIR lowering(item/body 分离 + desugar;错误码 1xxx 首批)
  - id: D-M2-2
    name: spec 语义条款首批(spec/names.md + spec/types.md,RXS-0032 起)
  - id: D-M2-3
    name: query 骨架(type_of 等纯函数 API + 进程内 memo,D-203 第一天形态)
  - id: D-M2-4
    name: 类型检查 host 子集 + 黄金路径 2(类型错误 snapshot,2xxx 错误码)
  - id: D-M2-5
    name: MIR 雏形 + LLVM host codegen 闭环(hello-world EXE + PDB)
  - id: D-M2-6
    name: self-profile 阶段计时 + 编译性能预算布点(m2_budget.json 占位,阈值 M3 实测回填)
acceptance_gates:
  - id: G-M2-1
    check: "hello-world 编译闭环:rurixc 驱动产出 EXE,运行退出码/输出验证 + PDB 产物存在(CI 自动核对)"
  - id: G-M2-2
    check: "断点验证:cdb/WinDbg 脚本化断点命中(bp + g),命令输出留痕入 close-out(真跑铁律,对齐 G-M0-2)"
  - id: G-M2-3
    check: "黄金路径 2(类型错误)snapshot ≥10(m2.counter.ui_golden_path2_snapshots),复用 M1 UI 通道与 bless guardrail"
  - id: G-M2-4
    check: "self-profile 阶段计时输出可解析且阶段计数器非零(D-235 二里程碑非零规则布点)"
  - id: G-M2-5
    check: "M2 新增 RXS 条款 ≥1 测试锚定(trace_matrix 全局口径,沿用 m1.counter.spec_clause_test_anchoring)"
guardrails:
  - "milestones/m0/m0_budget.json 与 milestones/m1/m1_budget.json 的 measured_local 既有条目 git diff 0-byte(新增条目允许)"
  - "milestones/m0/M0_CONTRACT.md(closed)与 milestones/m1/M1_CONTRACT.md 既有条款非 close-out 区 0-byte(M1 close-out 区只追加)"
  - "registry/deferred.json 与 registry/spike_gating.json 只追加(既有条目修改触发人工审查)"
  - "evidence/ 只增不删不改"
  - "00–14 共 15 份规划文档不被执行 PR 改写(勘误走 00 §6.3 追加式修订)"
  - "tests/ui/ 的 .stderr snapshot 变更必须经审批 bless(M1.4 已激活,check_ui_bless)"
  - "spec/ 变更必须携带变更档位标记(M1.2 已激活)"
  - "registry/error_codes.json 错误码语义可加不可改(M1.1 已激活)"
  - "本契约 in_scope/acceptance_gates 等既有条款 0-byte 修改,close-out 只追加"
---

# M2 契约 — HIR、类型检查与 host 编译闭环

> 所属:[../../11_ROADMAP.md](../../11_ROADMAP.md) §3 M2 / 契约机制见 [../../14_ENGINEERING_DISCIPLINE.md](../../14_ENGINEERING_DISCIPLINE.md) §1
> 规范先行延续(AGENTS.md 硬规则第 7 条):名称/类型语义 PR 必须引用 RXS-#### 条款号;缺条款先补 spec(spec/names.md、spec/types.md 自 M2.1/M2.2 实体化)。
> 基准 ref 过渡:`m1-closed` tag 由人类随 M1 终审打出;落地前 guardrail 核对仍以 PR base / `m0-baseline` 为基准(M1 close-out 见 [../m1/M1_CONTRACT.md](../m1/M1_CONTRACT.md) §8)。

---

## 1. 目标

建成 rurixc 的第一条**端到端 host 编译闭环**:从源码经名称解析、HIR、类型检查、MIR 到 LLVM host codegen,产出第一个 hello-world EXE 且 PDB 断点可命中。同时落下语义层地基纪律:query 风格 API(D-203 第一天形态)、spec 语义条款(names/types)、黄金路径 2(类型错误)与 self-profile 计时——为 M3(借用检查/const eval)与编译性能预算实测回填铺路。

## 2. 范围

### 2.1 in-scope

| 项 | 说明 | 对应交付物 |
|---|---|---|
| 名称解析 + HIR | 作用域/可见性(`pub`/`pub(package)`,D-112)/use 解析;HIR lowering:item/body 分离(增量依赖边界,D-202)+ desugar(for/`?` 等);错误码 1xxx 段首批 | D-M2-1 |
| spec 语义条款 | `spec/names.md`(名称/模块/可见性)+ `spec/types.md`(类型/推断/trait 子集),RXS-0032 起,FLS 体例;**条款 PR 先于实现 PR** | D-M2-2 |
| query 骨架 | 全部语义分析 API 写成 query 纯函数(`type_of(def_id)` 等),provider 只经 query context 互访;进程内 memoization;无全局可变状态(D-203"接口第一天、存储最后一天") | D-M2-3 |
| typeck host 子集 | 类型收集 → HIR body 内 HM 局部推断(签名强制全标注,07 §3)→ 检查;范围 = 函数/struct/enum/泛型单态化雏形(D-104 trait 单态化子集/D-111);2xxx 错误码 + 黄金路径 2 | D-M2-4 |
| MIR + host codegen | MIR 雏形(CFG 化、显式类型)+ 单态化收集;LLVM pin 22.1.x(D-205)→ x86-64 COFF .obj → link.exe → EXE;CodeView/PDB(D-209/D-237) | D-M2-5 |
| self-profile | query 级计时 + 阶段计数器,`-Z self-profile` 式输出(07 §6);编译性能预算占位布点(阈值 M3 实测回填,见 §4 诚实声明) | D-M2-6 |

### 2.2 out-of-scope(显式排除)

- TBIR 窄门 / NLL 借用检查 / const eval——M3 路线图项(11 §3 M3),非 deferred,不登记编号。
- device 路径 codegen 与着色/地址空间检查深度——M4 路线图项(着色语法形态 M1 已可解析,语义检查随 M4)。
- MLIR kernel-island(SG-001)、dyn trait/特化/HKT/async(D-104 永久裁剪)、过程宏/声明宏(D-111/SG-006)。
- 跨会话增量编译与并行前端(D-203 Phase 2+);lld-link 默认化(D-209:opt-in 维持)。
- 11 §2 MVP 红线清单全部不触碰([../../registry/spike_gating.json](../../registry/spike_gating.json) SG-001 ~ SG-009 维持 not_triggered)。

## 3. 交付物清单

| ID | 交付物 | 形态 | 完成判据 |
|---|---|---|---|
| D-M2-1 | 名称解析 + HIR | `src/rurixc/` resolve/hir 模块 + 1xxx 错误码 | cargo test 绿 + 条款锚定(G-M2-5) |
| D-M2-2 | spec 语义条款 | `spec/names.md` + `spec/types.md`(RXS-0032+) | G-M2-5 |
| D-M2-3 | query 骨架 | query context + memo,语义 API 全走 query | 单测(memo 命中/纯函数纪律) |
| D-M2-4 | typeck + 黄金路径 2 | typeck 模块 + 2xxx 错误码 + `tests/ui/typeck/` snapshot | G-M2-3 |
| D-M2-5 | MIR + host codegen | MIR 模块 + codegen/链接驱动 + hello-world EXE/PDB | G-M2-1 + G-M2-2 |
| D-M2-6 | self-profile + 预算布点 | 计时输出 + [m2_budget.json](m2_budget.json) 占位 | G-M2-4 |

## 4. 验收门(完整版,YAML 头为可提取摘要)

1. **G-M2-1(编译闭环)**:`conformance/` 的 hello-world 程序经 rurixc 全管线产出 EXE;CI 自动核对:进程退出码与预期输出匹配 + 同名 `.pdb` 产物存在。
2. **G-M2-2(断点真跑)**:cdb(WinDbg 命令行)脚本化验证——对 hello-world EXE 设源行断点(`bp`),运行(`g`)命中并打印栈;命令与输出原文留痕 close-out(反 YAML-only,对齐 G-M0-2 真跑铁律)。
3. **G-M2-3(黄金路径 2)**:`tests/ui/typeck/` 类型错误 snapshot ≥10 条(`m2.counter.ui_golden_path2_snapshots`),走 M1.4 已激活的 bless 审批 guardrail;数量为 `estimated` 性质工程选择,调整经 Direct PR 留痕(对齐 G-M1-1 先例)。
4. **G-M2-4(self-profile)**:阶段计时输出机器可解析(JSON 行或等价),编译 hello-world 时各阶段计数器非零;为 D-235"计数器合入后 2 里程碑内非零证据"布点。
5. **G-M2-5(traceability 延续)**:M2 新增 RXS 条款(names/types)每条 ≥1 测试锚定;`ci/trace_matrix.py` 全局口径核对(`m1.counter.spec_clause_test_anchoring` 为全局断言,无需另立 m2 计数器)。

**预算口径诚实声明(与 M1 G-M1-3 的差异)**:编译性能预算(冷编译 hello-world / 增量 check 延迟,07 §6)按 11 §3 在 **M3 首次实测回填**;M2 仅交付 self-profile 基础设施 + `m2_budget.json` `estimated` 占位,**本契约不设"关闭时零 estimated 残留"门**。占位存活 ≤2 个里程碑(14 §3),即必须在 M3 close-out 前转 `measured_local`,逾期即预算门 FAIL。

## 5. Guardrails(字节级,机器核对)

见 YAML 头 `guardrails` 字段。核对方式:`ci/check_guardrails.py <基准ref>`(M1 关闭前以 PR base / `m0-baseline` 为基准;`m1-closed` tag 落地后切换,切换记录追加于 [CI_GATES.md](CI_GATES.md) 修订表)。M2 无新增 guardrail 激活项;14 §2 常驻集中 IR golden / stable API 快照等随 M3 MIR 定型再评估(见 [CI_GATES.md](CI_GATES.md) §4)。

## 6. Deferred 引用

M2 开工时无预造 deferred(`deferred_refs: []`);M1 遗留 RD-001(M8)/RD-002(M5)/RD-003(M6)/RD-004(M6)/RD-005(M6)/RD-006(M8)不属 M2 范围,维持原承接。执行期做不完的事按 14 §4 追加 `RD-###` 并双侧标注。

## 7. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-06-11 | 初版契约固化 |

---

## 8. Close-out(只追加区 — 开工时为空)

<!-- 验收记录、guardrail 核对输出、deferred 继承/关闭记录追加于此;上方条款 0-byte 修改。 -->
