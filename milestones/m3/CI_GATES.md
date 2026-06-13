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
| 15 | borrowck conformance 批跑:`cargo test -p rurixc --test borrowck_corpus`——`conformance/borrowck/reject/<category>/` 反例全拦截(逐文件断言产生预期 4xxx 诊断)+ `accept/` 正例 0 诊断 + 七类目录覆盖核对(契约 G-M3-1 通道;M3.3 WP4 接入 pr-smoke 工作流) | 是 |
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
2. **MIR 文本 golden 激活**(14 §2 常驻集,M2 CI_GATES §4 预告的评估时点到期):M3.2 做预评估(MIR 形态随 TBIR 重排/drop elaboration 定型),M3.3 激活——golden 基线入库 + 核对入 CI,激活必须经真实红绿验证;基线变更纪律对齐 UI bless(变更须审批留痕),具体形态(独立脚本 vs cargo test 快照升格)在激活 PR 中裁决留痕。**M3.3 WP6 已激活**(本表 v1.4 修订行):形态 = cargo test 磁盘 golden(`src/rurixc/tests/mir_golden.rs`,`tests/mir/*.rx` + 同名 `.mir`),三代表语料(无 drop / drop 顺序 / 条件初始化 drop flag);bless = `RURIX_BLESS=1` 重写 + 独立 `tests/mir/bless_log.md` 审批留痕(`ci/check_guardrails.py` `check_mir_bless` 机器核对);CI 接入 pr-smoke `cargo test -p rurixc --test mir_golden`。

   **M3.2 预评估结论**(本表 v1.2 修订行):
   - **形态定型程度**:M3.2 后 MIR 形态新增 `Operand::Move`(Copy/move 区分)与 `TerminatorKind::Drop`(drop elaboration 产物,含 drop flag 守卫的 SwitchBool 展开)。host 子集 MIR 的语句/终结子集合在 M3.2 结束时**基本定型**;M3.3 NLL 借用检查不改 MIR 形态(只读 pass + region 标注,产物在 borrowck 侧),故 golden 基线可在 M3.3 安全入库。
   - **基线范围建议**:覆盖三类形态代表——(a) hello-world(无 drop,回归网底线);(b) drop 顺序程序(`conformance/borrowck/accept/drop_order_run.rx`,含 Move + 无条件 Drop + drop 消去);(c) 条件初始化 drop flag 程序(含 SwitchBool 守卫 + flag set/clear)。基线 = `rurixc --emit=mir` 文本逐字节。
   - **核对脚本形态**:倾向**升格既有 cargo test 快照**(`mir_build::tests` 已有 `hello_world_mir_snapshot` / `drop_elaboration_orders_and_elides` 等 inline 快照先例),而非新建独立脚本——复用 cargo test 通道、基线随源码版本化、变更即 diff;bless 纪律对齐 UI(`RURIX_BLESS` 式重写 + 审批留痕表)。最终形态在 M3.3 激活 PR 裁决。
   - **激活前置**:M3.3 region 推断落地、`mir_borrowck` query 化后,MIR 形态再核一次无漂移,即入库基线 + 接核对入 CI,经真实红绿验证(故意改 MIR 输出不更新基线 → 红;按审批更新 → 绿,run URL 归档,§5 第 3 项)。
   - **未提前激活理由**:M3.2 仍在动 MIR 形态(Move/Drop 本 PR 刚落),提前锁基线会与 M3.3 借用检查接入期的潜在微调冲突;按 M2 CI_GATES §4 "随 MIR 定型评估"的口径,评估在 M3.2、激活在 M3.3(M3_PLAN §3 任务 6)。

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
| v1.1 | 2026-06-12 | §4 第 1 项落地:`ci/check_guardrails.py` 本地/push 回退基准 `m1-closed → m2-closed`(PR 路径 GITHUB_BASE_REF 优先不变);切换前双基准核对均 PASS(`py -3 ci/check_guardrails.py m1-closed` = PASS 101 changed paths;`m2-closed` = PASS 6 changed paths) |
| v1.2 | 2026-06-13 | §4 第 2 项 M3.2 预评估结论入档(MIR 文本 golden):MIR 形态经 M3.2 Move/Drop 落地后基本定型,基线范围(hello-world / drop 顺序 / drop flag 三代表)、核对脚本形态(倾向 cargo test 快照升格)、激活前置与时点(M3.3,M3_PLAN §3 任务 6)裁决留痕;M3.2 不激活(理由:借用检查接入期 MIR 可能微调)。新增 drop 真跑冒烟 `ci/hello_smoke.py drop-smoke`(drop_order_run/temp_drop_stmt 顺序核对,RXS-0055/0056;CI 工作流接入随步骤 15 一并裁决) |
| v1.3 | 2026-06-13 | §2 步骤 15 落地(M3.3 WP4):实测命令 = `cargo test -p rurixc --test borrowck_corpus`,接入 pr-smoke 工作流(名为 "borrowck conformance batch")。`conformance/borrowck/reject/` 补齐契约 §4 七类(use_after_move/use_before_init/double_mut_borrow/shared_mut_conflict/move_while_borrowed/assign_while_borrowed/dangling_reference)+ 借用 accept 正例;`m3.counter.borrowck_conformance_categories` 由 SKIP 转 PASS(≥7)。红绿真跑核验(§5 第 1 项)随首个 PR 归档 close-out |
| v1.4 | 2026-06-13 | §4 第 2 项落地(M3.3 WP6):MIR 文本 golden guardrail 激活。形态 = cargo test 磁盘 golden(`src/rurixc/tests/mir_golden.rs`,语料 `tests/mir/{hello_world,drop_order,conditional_drop_flag}.rx` + 同名 `.mir`,基线 = 全管线 `mir::pretty` 文本逐字节);三代表覆盖无 drop / Move+drop 消去 / 条件初始化 drop flag(SwitchBool 守卫)。bless 纪律对齐 UI:`RURIX_BLESS=1` 重写 + 独立 `tests/mir/bless_log.md` 留痕,`ci/check_guardrails.py` 新增 `check_mir_bless`(第 9 项)机器核对。CI 接入 pr-smoke 步骤 "MIR golden guardrail"(`cargo test -p rurixc --test mir_golden`)。**本地红绿验证(§5 第 3 项)均过**:(a) golden 漂移检测——篡改 `hello_world.mir` → 测试红(exit 101)→ `RURIX_BLESS=1` 重写 → 绿;(b) bless 守卫——`.mir` 变更删 bless 行 → `check_guardrails m2-closed` FAIL(列出 3 个未审批 .mir)→ 补行 → PASS。真实 CI run URL 待推送补入契约 §8 close-out |
| v1.5 | 2026-06-13 | §4 第 2 项 / §5 第 3 项 WP6 真实 CI 绿跑归档:pr-smoke run [27458630302](https://github.com/qwasg/Rurix/actions/runs/27458630302)(PR #7,event=pull_request,HEAD `5126970`)conclusion=**success**,其中 "MIR golden guardrail" 步骤(`cargo test -p rurixc --test mir_golden`,3 测试)与 guardrails 步骤(含 `check_mir_bless`)真实通过;`tests/mir/bless_log.md` 人工终审已转正(qwasg 会话授权)。失败(red)路径已本地真实执行验证(见 v1.4 (a)(b),非 YAML-only);专门的 CI red-run URL 随 M3 close-out 归档(对齐 §5 第 1 项 borrowck 同口径,v1.3 先例)。**WP6 关闭** |
