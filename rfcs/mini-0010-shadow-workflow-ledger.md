# Mini-RFC MR-0010 — 影子/off-tree 编号工作流登记机制（number_ledger + 跨分支保留号守卫）

| 字段 | 值 |
|---|---|
| Mini-RFC 标识 | **MR-0010**（Mini-RFC 序列；独立于 Full-RFC 的 `RFC-####` 命名空间，不复用 RFC 编号，10 §9.5。Mini-RFC = 单页提案 + 失败测试先行，10 §3。MR-0006/0007 已被 GRX 影子分支 claim，本号 MR-0010 由 [EA1_CONTRACT](../milestones/ea1/EA1_CONTRACT.md) §7 明记不占用可用） |
| 标题 | 建立 off-main 编号工作流的**结构化登记台账** `registry/number_ledger.json` + **跨分支保留号 CI 守卫** `ci/check_number_ledger.py`，把 GRX 影子分支对共享命名空间的消费从散落 prose 升级为可机核事实源 |
| 档位 | **Mini-RFC**（10 §3：新增一个登记机制文件（registry）+ 一个 **check_\* 守卫风格**工具行为门 + 台账补录，触**治理卫生 / 工具行为**量级；**不改语言/语义面、不触** UB / 内存模型映射 / FFI ABI / 安全包络禁区——见 §3。直接先例 [MR-0003](mini-0003-oss-community.md) 亦以 Mini-RFC 确立 `ci/check_contribution.py` 门）。agent 自主 裁为 Mini-RFC（2026-07-17） |
| 状态 | **Approved — 2026-07-17**（agent 自主批准并记录；P1-1 = 治理卫生/工具机制，不触 owner 权力面、不触红线、不触 P-13，D-406 v2.0 下 agent 完全自主） |
| 承接里程碑 | 治理卫生支线（无独立子里程碑；镜像 MR-0003 贡献门以 Mini-RFC 承载 CI 守卫机制的先例） |
| 关联条款 | **零新 RXS**（机制类交付：registry 台账 + CI 守卫，非 spec 语义面）。零新错误码（守卫风格不分配错误码，07 §5）、零新 RD/SG（登记既发生的 off-tree 工作，非新决策 / 非新延期 / 非扩张方向 gating） |
| 依据决策 | **10 §9.5**（编号永不复用，本机制补齐其跨分支执行面）· **10 §4**（一等公民目录 / 事实源）· 先例 **MR-0003**（check_\* 守卫机制以 Mini-RFC 承载）· **EA1_CONTRACT §7**（MR-0010 不占用 / RXS-0214 / RD-033 / U29 / SG-010 / 步骤 59-60 在途保留号原料） |
| Provenance | `Assisted-by: claude-code:claude-opus-4-8`。agent 自主决策，批准后推进实现 commit |
| 失败测试先行 | `tests/test_number_ledger.py`（合成一个树内同号异义 fixture → 断言 `ci/check_number_ledger.py` 判红；正常树 → 判绿；一个 shadow-reserved 号新出现为树内定义 → 断言判红）。当前 `origin/main` 上 `ci/check_number_ledger.py` **不存在** → 跨分支保留号无任何门拦截（编号永不复用 10 §9.5 的跨分支执行面**未兑现**）；本 Mini 落地接入 PR Smoke 守卫步骤后转为有意义的拦截。 |

---

## 1. 摘要

把 GRX 影子分支（`codex/grx-godot-dxil-workspace`，milestone **closed** v1.31，close-out 提交 `95d5af43`）对**共享编号命名空间**的消费，从当前散落在 5+ spec 文件 + 1 行 `rfcs/README.md` 的**临时 prose 跳号注**，升级为一个**结构化、可机器核对的事实源** `registry/number_ledger.json`，并配一个 **check_\* 守卫风格**的 CI 门 `ci/check_number_ledger.py` 强制两条可靠判定（树内同号异义碰撞 + 已登记保留号被尊重）。

**要解决的机制漏洞（10 §9.5 的跨分支执行面缺口）**：`10_GOVERNANCE.md` §9 第 5 条「编号永不复用」原文只列『错误码、spec 条款号、deferred 编号、RFC 编号』；`SG-###`、私有 `GRX-0xx`/`D-GRX`/`G-GRX`/`patch-00xx` 命名空间未被显式覆盖；且 CI 只见当前分支树，**无法枚举其它未合分支**——故一个 off-main 分支消费了共享号（如 GRX 消费 MR-0006/0007 与 RXS-0181~0184），main 侧只能靠**人工警觉逐处跳号**，无结构化登记、无机器门。本机制补齐这个执行面（在 CI 能力边界内），**不重新发明** 既有 `rfcs/README.md` 台账 / `registry/*.json` 事实源，只补**跨分支消费**这一未覆盖维度。

## 2. 设计（实体清单 + 形态）

| 交付 | 文件 | 形态 / 复用 |
|---|---|---|
| 结构化编号台账 | `registry/number_ledger.json`（新增） | `schema_version` + `namespaces`（每命名空间记 `on_tree_max` / `next_free` / `shadow_reserved` 数字化字段 + `notes` prose）+ `off_tree_workflows`（GRX 分支/commit/契约版本/消费区间/main 追踪现状）+ `known_collisions`（诚实登记跨分支处置）+ `reserved_in_flight`（EA1 在途 claim）+ `revision_log`。**只追加**（镜像 `deferred.json` 纪律） |
| 跨分支保留号守卫 | `ci/check_number_ledger.py`（新增） | **check_\* 守卫风格**（CPU-only，纯 stdlib，不分配错误码、不写 evidence、不接 budget counter；镜像 `check_structure.py`/`check_contribution.py`）；三查见 §2.1；内置 `red_self_test()` 反 YAML-only |
| 失败测试先行 | `tests/test_number_ledger.py`（新增） | pytest 合成 fixture 红绿：伪造同号异义输入 → 红；正常 → 绿；shadow-reserved 号新出现为树内定义 → 红 |
| PR Smoke 接线 | `.github/workflows/pr-smoke.yml`（改：check_\* 守卫块加一步） | 在 `check_contribution` 后加一步跑 `check_number_ledger.py`（**无步骤编号声明**——守卫类不占数字步骤号，与 structure/schema/guardrails/redistribution/contribution 一致；数字步骤 59/60 已由 EA1 保留） |
| README 台账结构化引用 | `rfcs/README.md` §5（改：既有跳号注升级） | 把既有「MR-0006/0007 已被 GRX claim」临时注升级为指向 `registry/number_ledger.json` 的结构化引用（既有措辞尽量少动） |

### 2.1 守卫三查（保守、可机械可靠，避免误报）

1. **树内同号异义碰撞检测（blocking）**：扫 `spec/**/*.md` 的 `### RXS-####` 条款头——同一 RXS 号出现 ≥2 个 heading 定义即红；扫 `registry/{deferred,spike_gating,error_codes,number_ledger}.json` 的 `id` 集——单文件内 id 重复即红。（当前 main 树：209 个 distinct `### RXS` 头零重复、各 registry id 零重复 → **PASS**。）
2. **保留号被尊重（blocking，仅对可靠判定项）**：对 `number_ledger.json` 中 `shadow_reserved` 标注的号——若该号在当前树**新出现**为条款定义（如 shadow-reserved 的 `RXS-0181` 突然获得 `### RXS-0181` 头）即红（有人复用了 burned 号）；并断言 ledger 声明的 `next_free` ≥ max(树内已用, shadow_reserved 最大) + 1（防 ledger 滞后 + 强制下一个自由号跳过两者）。仅对**树内可机检**命名空间（RXS via spec 头）做强制；私有 `GRX-0xx`/`patch-00xx` 本就不在 main 树，只做「不得新出现」的平凡校验。
3. **台账引用存在性（advisory，打印不阻断）**：对 ledger 的 `off_tree_workflows` 分支/commit ref 做 `git rev-parse --verify` 存在性核验，**打印 exists/missing 但绝不 exit 非零**（CI 可能是浅 clone / 不含该分支——不可因此误红）。

### 2.2 能力诚实边界（写入守卫 docstring + 本 §）

**CI 只见当前分支树，无法枚举/扫描其它未合分支**——故本守卫**无法真正「自动发现 untracked 编号工作流」**。新影子/off-tree 工作流的登记，仍需一次**人工/agent 前置动作**把它录入 `number_ledger.json`。守卫只强制两件事：（a）树内同号异义碰撞；（b）**已登记**的保留号被当前树尊重。宣称「完全自动化发现」将违 14 §5 证据分级 + 反 extractive 纪律，故明确不宣称。

## 3. 为何 Mini-RFC（而非 Direct，亦非 Full RFC）

- **非 Full RFC**：本设计**不触** AGENTS 硬规则 5 / 10 §7.5 禁区——不定义/修改 UB、内存模型映射、FFI ABI、安全包络边界，**不引入任何语言/语义面**（台账是 JSON 事实源，守卫是编号元数据核对）。「编号永不复用」是 10 §9.5 **已锁**决策的执行面实体化，非新治理设计。
- **非 Direct**：新增一个 registry 事实源文件 + 一个不可绕过的 CI 守卫门 + 跨分支治理台账补录，属**工具行为变更 + 治理机制**面（10 §3 ≥ Mini）；直接先例 [MR-0003](mini-0003-oss-community.md) 以 Mini-RFC 确立 `ci/check_contribution.py` 门。硬规则 8「判档争议向上取严」→ 走一页 Mini + 失败测试先行 + agent 批准。
- **升档触发条件（实现期守卫）**：若实现期发现确需**修改 `10_GOVERNANCE.md` §9 条款体本身**（如把 SG/私有段显式写入「永不复用」列举），则停手走 **00 §6.3 独立 errata PR**（规划文档冻结集，check_planning_docs 预期红）而非在本机制 PR 夹带；本 Mini 只**在 CI 层实体化** 既有 §9.5 精神，不改宪法条款字面。

## 4. 错误码 / 影响 / 范围

- **零新错误码**：守卫是 check_\* 风格（不分配错误码，07 §5）。`registry/error_codes.json` 与 en/zh message 零追加。
- **零新 budget counter / evidence**：守卫 CPU-only，不写 `evidence/*.json`、不接 budget。`check_schemas.py` / `budget_eval.py` 对 `number_ledger.json` 0-byte（`check_schemas` 只校验 `deferred`/`spike_gating`/`error_codes`/`*_budget`/`evidence`，本文件不在其列）。
- **零新 RD/SG**：登记既发生的 off-tree 工作 = 事实补录，非新延期（RD）、非拒绝扩张方向（SG——语义错配，见 §6 范围红线）。
- **evidence 入库范围红线**：**只入编号台账 + 分支/commit 指针**，NOT GRX 源码/patch/Godot 快照（`external/` 永久 ignore；GRX 契约 out_of_scope 明禁 vendoring Godot）。GRX 蒸馏的 Godot 上游发现已由 **EA1 upstream_report_packs 单独承接**（[EA1_CONTRACT](../milestones/ea1/EA1_CONTRACT.md) §out_of_scope grx_merge + 上游备包面），**本 Mini 不重复搬运**避免双写。

## 5. 失败测试先行（10 §3 Mini 硬性）

`tests/test_number_ledger.py` + `ci/check_number_ledger.py` 内置 `red_self_test()`：合成（a）同一 RXS 号两个 heading → 断言碰撞检测判红；（b）一个 shadow-reserved 号新出现为树内条款定义 → 断言保留号检测判红；（c）干净台账 + 干净树 → 断言判绿。门若空过（漏检碰撞）即 FAIL（反 YAML-only，镜像 `check_contribution.red_self_test`）。当前 `origin/main` 上 `ci/check_number_ledger.py`**不存在** → 编号永不复用（10 §9.5）的跨分支执行面**RED（未兑现）**；本 Mini 落地接入 PR Smoke 守卫步骤后，对当前 main 树判绿（无同树碰撞）、对未来复用 burned 号的 PR 判红。

## 6. 影响 / 向后兼容 / 范围

- **向后兼容**：纯追加。不动任何既有语义面 / 回归网（守卫 CPU-only，不改既有 registry 文件的 schema，只新增 `number_ledger.json` + 只改 `rfcs/README.md` 一处跳号注措辞 + `pr-smoke.yml` 加一步）。
- **历史碰撞的诚实登记（核实要点）**：经 git 核对（`grep`/`git log`/`git ls-tree` 逐项），GRX 影子分支消费了共享 **MR-0006/0007**（提交 `cc059daa`）+ **RXS-0181~0184**（提交 `cc059daa`，条款体入该分支 `spec/dxil_backend.md`）+ 私有 **GRX-001~026**（GRX_PLAN 区间）/ **patch-0001~0048**（patches/ 目录）/ **D-GRX-1~6** / **G-GRX-1~5**。**关键诚实修正**：main 侧对 MR-0006/0007 与 RXS-0181~0184 是**跳号避撞**（deliberate skip），当前 main 树**零** `### RXS-0181~0184` 条款定义（每处出现均为「已被 GRX claim、跳号避撞」的注，非复用）——即 10 §9.5 目前**未 materialize 为同树同号异义碰撞**，而是**仅靠人工跨分支警觉**避开的**近失/潜在风险**。故 `known_collisions` 登记为「**已知跨分支共享命名空间消费 + main 跳号处置；影子(GRX)侧号为 closed 分支上的 definition of record；这些号对 main 永久 burned（10 §9.5），main 永不复用；未 materialize 为同树同号异义**」——**绝不假装可回收、不改写任何既有条款、不虚构已发生的违反**。
- **范围红线**：不用 `SG` 载体（SG = 拒绝扩张诱惑方向机制，00 §71 / 14 §7；GRX 是已发生并 closed 的工作，语义错配；且 SG-010 已长期软保留给「窗口/UI 框架 / 通用异步宿主运行时」方向，占用会冲突）；不搬 GRX 代码/patch/Godot 快照；不改 `10_GOVERNANCE.md` §9 条款体（如需显式扩列举走 00 §6.3 errata）；不宣称守卫自动发现未知分支。

## 7. Agent 批准

> **Approved — 2026-07-17**。agent 自主批准本 Mini-RFC（§1 摘要 / §2 形态 + 三查 + 能力边界 / §3 判档 / §4 零新码 + evidence 范围 / §5 失败测试先行 / §6 历史碰撞诚实登记 + 范围红线）+ 授权推进实现 commit（`registry/number_ledger.json` + `ci/check_number_ledger.py` + `tests/test_number_ledger.py` + PR Smoke 接线 + README 台账升级）。P1-1 = 治理卫生/工具机制，不触 owner 权力面、不触红线、不触 P-13，D-406 v2.0 下 agent 完全自主签署。批准记录由 claude-code 代录（硬规则 1）。
