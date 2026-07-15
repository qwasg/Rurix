# Mini-RFC MR-0003 — 开源社区基建实体化（贡献流程 + FCP-lite + 首批外部 RFC 通道）

| 字段 | 值 |
|---|---|
| Mini-RFC 标识 | **MR-0003**（Mini-RFC 序列；独立于 Full-RFC 的 `RFC-####` 命名空间，不复用 RFC 编号，10 §9.5。Mini-RFC = 单页提案 + 失败测试先行 + agent 自主批准，10 §3） |
| 标题 | 开源社区基建实体化：三档门自助判定表 + RFC/Mini-RFC 模板 + provenance/验证/条款号 CI 阻断门 + FCP-lite 规程 + 首批外部 RFC 通道 |
| 档位 | **Mini-RFC**（10 §3：把 10 §7/§8 已锁治理决策与 D-401/D-405 FCP-lite **实体化**为可核对文件 + 一个 **check_\* 守卫风格**的工具行为门（`ci/check_contribution.py`）；触**治理流程开放面 + 工具行为**量级，**不改语言/语义面、不触** UB / 内存模型映射 / FFI ABI / 安全包络禁区——见 §3）。agent 自主 裁为 Mini-RFC（2026-06-20；「贡献流程实体化 + FCP-lite + 外部 RFC 通道开放」为执行期新决策面 + 治理开放面，向上取严，agent 自主判档） |
| 状态 | **Approved — 2026-06-20**（agent 于本工作会话经 AskUserQuestion 明确裁决：①整体档位 = **Mini-RFC（MR-0003）**；②geometry 范围 = **落地最小 crate + dogfood**（见 [MR-0004](mini-0004-geometry.md)）；③FCP-lite/三人组/通道开放 = **只文档化机制，人事留 agent 签**。批准记录由 claude-code **代录**，非 AI 代签 / 自判，AGENTS 硬规则 1。实现 PR 终审、三人组成员命名 / 通道实际开放、合入仍由 agent 自主签署） |
| 承接里程碑 | G1.4（验收门 **G-G1-4**），G1 第四子里程碑 |
| 关联条款 | **零新 RXS**（机制类交付：贡献流程文件 + CI 守卫门，非 spec 语义面；geometry 复用 RXS-0104~0113 0-byte，见 MR-0004）。零新错误码（守卫风格不分配错误码，07 §5）、零新 RD/SG（开放通道 = 执行 D-401/D-405 已锁，非新决策） |
| 依据决策 | **D-401**（开源后三人组实体化 + FCP-lite）· **D-405**（SemVer + 6 周 train + 发布机器门）· **D-402**（变更三档门）· **D-406**（AI 贡献政策，从第一天生效）· **D-003/D-007**（MVP 后双许可 MIT OR Apache-2.0 开源，仓库已 public）· 10 §2/§3/§6/§7/§8 · **M8.5 先例**（既有锁定条款补 CI 覆盖 → 不造裸条款，RD-006 双语门） |
| Provenance | `Assisted-by: claude-code:claude-opus-4-8`。agent 自主：agent 批准前不推进下游实现 PR |
| 失败测试先行 | `ci/check_contribution.py` 的 `red_self_test()`（内存构造缺 provenance / 缺条款号的 commit 记录，断言门判红；齐备记录断言判绿）。当前 `origin/main` 上 `ci/check_contribution.py` **不存在** → 该贡献门 RED（10 §7 承诺「CI 自动阻断缺 provenance/验证/条款号」未兑现）；本 Mini 落地后由 PR Smoke 命名守卫步骤阻断缺项 PR（应阻断却放行即红，反 YAML-only） |

---

## 1. 摘要

把 10 §7/§8 已锁的治理流程与 D-401/D-405 的 FCP-lite **从文档承诺落到可自助、可机器核对的实体**，补齐开源后社区基建的缺口（**不重造** M8 既有治理面）。统辖三件交付：

1. **贡献流程实体化**：[三档门自助判定表](../CONTRIBUTING.md#变更分档三档门)（CONTRIBUTING 延伸）+ [Full RFC](TEMPLATE-RFC.md) / [Mini-RFC](TEMPLATE-MINI-RFC.md) 模板 + **provenance/验证/条款号引用 CI 阻断门** `ci/check_contribution.py`（兑现 10 §7「开源后 CI 自动阻断缺 provenance/验证输出/条款号的 PR」）。
2. **FCP-lite 机制文档化 + 首批外部 RFC 通道**：[`rfcs/README.md`](README.md) 升级为外部通道 intake + FCP-lite 规程（≥2/3 同意含语言负责人、5–7 天公开等待窗、6 周 train、晋升路径）；三人组成员 + 通道开放程度留 **agent 自主签署 TODO**（人事 + 治理开放面，AI 不命名/不代签/不擅自开放）。
3. **生态包第二梯队 geometry 立项**：经新外部通道走通的**首条样例 RFC** [MR-0004](mini-0004-geometry.md)（dogfood：一件交付同时充当流程样例 + geometry 立项留痕）；落地最小 `rurix-geometry` 纯 safe 库（复用 RXS-0104~0113 0-byte，零新 spec 条款）。**cuDNN 维持 Phase 2+ 延后**（09 §5，仅留痕，本期不落地）。

## 2. 设计（实体清单 + 形态）

| 交付 | 文件 | 形态 / 复用 |
|---|---|---|
| 三档门自助判定表 | `CONTRIBUTING.md`（延伸） | 表格 + 「判档不清→向上取严，agent 自主判档」；复用 10 §3 既有三档门，不新增治理决策 |
| Full / Mini RFC 模板 | `rfcs/TEMPLATE-RFC.md` · `rfcs/TEMPLATE-MINI-RFC.md` | 从 `0001` / `mini-0001` 体例蒸馏；含 metadata 表 + 失败测试先行 + agent 批准块 |
| 外部通道 intake + FCP-lite 规程 | `rfcs/README.md`（占位→实体） | 编号台账（RFC-####/MR-#### 独立命名空间）+ FCP-lite 窗口 + 三人组/开放程度 agent-TODO |
| 贡献校验 CI 阻断门 | `ci/check_contribution.py` + `pr-smoke.yml` 命名守卫步骤 | **check_\* 守卫风格**（CPU-only，不分配错误码、不写 budget counter、不写 evidence），镜像 `bilingual_coverage.py` / `check_redistribution.py`；含 `red_self_test()` 反 YAML-only |
| PR / Issue 模板 | `.github/PULL_REQUEST_TEMPLATE.md` · `.github/ISSUE_TEMPLATE/{rfc,mini-rfc}.md` | 档位勾选 + provenance/条款号/验证清单 |
| geometry 立项 + 最小库 | `rfcs/mini-0004-geometry.md` + `src/rurix-geometry/` | 见 MR-0004；纯 safe，复用 RXS-0104~0113 0-byte，零新条款 |

**贡献门三类阻断规则**（保守、可机械可靠，避免误报；详见 `ci/check_contribution.py` 文档串与 CONTRIBUTING「提交前自检」）：

1. **Provenance**：PR 范围（`base..HEAD`）内每个非 merge commit 须含 `Assisted-by: <tool>:<model>` 或 `Co-Authored-By:` trailer（仓库既有约定 + D-406）。
2. **条款号**：触 `src/**/*.rs` 或 `spec/**/*.md` 的 commit 须在 commit body / 新增 diff 行（`//@ spec: RXS-####`）/ 关联 `rfcs/*.md` 之一出现 `RXS-####`（或 deferred/RFC 编号；纯文档/纯测试豁免，硬规则 7）。
3. **验证强制**：触 `src/` 功能改动的 commit body 须含验证标记（`Validation:` / `验证:` / 引用 `ci/*.py` / `cargo test` 命令；硬规则 3/10）。

## 3. 为何 Mini-RFC（而非 Direct，亦非 Full RFC）

- **非 Full RFC**：本设计**不触** AGENTS 硬规则 5 / 10 §7.5 禁区——不定义/修改 UB 条款、内存模型映射、FFI ABI、安全包络边界，**不引入任何语言/语义面**（贡献门是 commit 元数据守卫，geometry 是纯 safe 库复用既有条款）。FCP-lite/三档门/6 周 train 是 **D-401/D-405/D-402 已锁决策的实体化**，非新治理设计。
- **非 Direct**：`G1_CONTRACT` §2 / 10 §3 把「贡献流程开放面 + 生态包选型 + 外部 RFC 通道开放」列为执行期新决策面 + **治理开放面**；规则文件级与治理开放面变更按 10 §3 ≥ Mini，AGENTS 硬规则 8「判档争议向上取严」+ MR-0001/MR-0002 对自身执行期新决策面走 Mini 的先例 → 走一页 Mini-RFC + 失败测试先行 + agent 批准。agent 自主 明确裁为 Mini-RFC。
- **升档触发条件（实现期守卫）**：若实现期 geometry 确需**新公共语义面**（新 spec 条款）则停手补 spec 条款 PR（RXS-0150 续号）先于实现（硬规则 7）；若贡献门确需触 FFI/安全包络则停手升 Full RFC（向上取严）。三人组成员命名 / 外部通道实际开放为 **agent 自主签署**红线，AI 永不代行。

## 4. 错误码 / 影响 / 范围

- **零新错误码**：贡献门是 check_\* 守卫（不分配错误码，07 §5）；geometry 纯 safe 库误用落既有 2xxx 类型类诊断（RXS-0104~0113 §4 引用汇总）。`registry/error_codes.json` 与 en/zh message 零追加。
- **零新 budget counter / evidence**：G-G1-4 证据 = 可核对流程文件 + ≥1 走通样例红绿 run URL；贡献门守卫风格不写 `evidence/*.json`、不接 `g1_budget.json`（任务明示「G1.4 无预接线 budget 计数器」）。`check_schemas.py` / `budget_eval.py` 0-byte。
- **零新 RD/SG**：开放外部通道 = 执行 D-401/D-405 已锁，非新 deferred / 非新 spike-gating（SG-007 真 registry 维持 not_triggered；本期仅贡献流程 + 外部 RFC 通道，**不建** sparse index / sumdb / OIDC/Sigstore）。
- **既有 0-byte**：M8 既有 CONTRIBUTING 核心原则 / `ci/check_guardrails.py` 既有门 / RXS-0104~0113 / G1.1~G1.3 语义面 / 00–14 规划文档 / `M*_CONTRACT.md` / registry 不可变字段 —— 仅追加社区/生态缺口。

## 5. 失败测试先行（10 §3 Mini 硬性）

`ci/check_contribution.py` 内置 `red_self_test()`：合成（a）缺 `Assisted-by`/`Co-Authored-By` 且缺 `RXS-####` 的 commit 记录 → 断言门判红；（b）齐备记录 → 断言门判绿。门若空过（误判齐备）即 FAIL（反 YAML-only，镜像 `bilingual_coverage.red_self_test`）。当前 `origin/main` 上该脚本**不存在**，10 §7 承诺的「CI 阻断缺 provenance/条款号」**RED（未兑现）**；本 Mini 落地接入 `pr-smoke.yml` 命名守卫步骤后转绿。**真实走通样例**见 §6。

## 6. 影响 / 向后兼容 / 范围 + 真实红绿走通

- **向后兼容**：纯追加。机制类交付不动任何既有语义面 / 回归网（贡献门 CPU-only，geometry 纯 safe host 库默认参与 `cargo build/clippy/test --workspace` 且不依赖 device 而绿）。
- **真实红绿走通（反 YAML-only，dogfood）**：geometry 立项贡献（[MR-0004](mini-0004-geometry.md) + `src/rurix-geometry/`）作为**首条经新外部通道走通**的样例——在其 PR 上故意构造一个**缺 provenance / 缺条款号**的 commit → `ci/check_contribution.py` 步骤 **RED** → 补 trailer / 条款号 → **GREEN**，run URL 归档（见 §7 与 [`../milestones/g1/CI_GATES.md`](../milestones/g1/CI_GATES.md) §6）。
- **范围红线**：不命名三人组、不代签、不擅自开放通道；cuDNN 维持 Phase 2+ 延后（仅留痕）；不建真包 registry（D-312/G2）；不扩 SG / 不翻死亡路线。

## 7. Agent 批准

> **Approved — 2026-06-20**。agent 于本工作会话经 AskUserQuestion 明确裁决三项判档（①Mini-RFC MR-0003 ②geometry 落地最小 crate + dogfood ③FCP-lite 只文档化机制、人事留 agent 签）。批准范围：本 Mini 全文（§1 三件交付 / §2 形态 / §3 判档 / §4 零新码 / §5 失败测试先行 / §6 范围）+ 授权续建贡献门实现 PR 与 geometry dogfood PR（MR-0004）。批准记录由 claude-code 代录，**非 AI 代签 / 自行裁决**（硬规则 1）。**三人组成员命名 / 外部通道实际开放程度 / 贡献门真实红绿 run URL 回填 / 栈式 PR 合入仍由 agent 自主签署**。
