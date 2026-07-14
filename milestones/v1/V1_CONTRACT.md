---
contract: V1
title: V1 期——语言 1.0 正式发布（稳定化收尾）：stabilization report + FCP-lite 公示 + 最小 stable channel 清单 + v1.0.0 首个 stable 发行 + 首个 GitHub Release
status: active            # active → closed（V1.4 close-out agent 自主签署;close-out 只追加,上方条款 0-byte;基准 g2-closed→v1-closed 切换 + v1-closed tag 随 close-out 落档）
version: v1.0
date: 2026-07-14
timebox: "短期收尾（约 1–2 周,V1.1~V1.4 严格串行见 V1_PLAN.md;周为相对刻度,非日历承诺）"
rfc_required: none        # 开工脚手架取 rfc_required: none（结构件,对齐 M4~G2 先例）:stabilization/edition 的 Full RFC 面已由 RFC-0008（Agent Approved 2026-06-30,G2.5）承载,V1 是其 §6 stabilization 路径「后续两里程碑无重大修订 → stabilization report → FCP-lite → 进入 stable」的**执行收尾**,不新开 Full RFC、不重造机制。**唯一新机制面 = 最小 stable channel 清单,须经 Mini-RFC 前置（MR-0008）**,脚手架只登记 + 标 gating,不实现。agent 自主判档,判档争议向上取严（硬规则 8）;触及红线 / UB / 内存模型映射 / FFI ABI / 安全包络须 Full RFC（硬规则 5）——V1 各项均不触
upstream_docs:
  - "11 §5 (语言 1.0 = spec 全量条款化 + conformance 覆盖全部 stable 特性 + 首个 edition 机制就绪——三要件已于 G2.5 达成,G2_CONTRACT §8.8;V1 为其正式发布收尾)"
  - "10 §5 (特性生命周期:稳定化条件 = spec 条款齐 + conformance 齐 + UI 测试齐 + 两个里程碑无重大修订 → stabilization report → FCP-lite → 进入 stable) / §6 (D-405:SemVer;1.0 = 开源后第一个 LTS 质量版本,stable 面破坏只能走 edition;工具链发布门 = conformance 全绿 + UI golden 全绿 + L1 基准无 Critical 回归 + SBOM/签名齐备 = 机器门) / §2.2 (开源后 FCP-lite 为 advisory:走 RFC 流程并公开等待窗,不强制人工同意数,AI agent 可自主推进)"
  - "rfcs/0008-edition-stabilization.md §6 (stabilization 路径锚点) / §9 (Q-RD008:RD-008 已激活 open→closed,stable 面已定义并 bless) / §8 (不引入第二 edition / 不冻结快照字节为 ABI)"
  - "08 §9 (D-241 分发与签名:rurixup 发布链路;rustup 式 install/update/channel 前端标注 MVP 后期,V1 不实现)"
  - "spec/release.md RXS-0135~0139 (原子分发/分区打包/签名/SBOM/发布门,M8.4) + RXS-0150~0152 (fatbin/lockfile artifact,G1.5) — V1 channel 清单条款（RXS-0185 续号）延伸本文件"
  - "spec/edition.md RXS-0177~0180 (edition 机制 + stable 面关系;RXS-0180 L2 同 edition 内 stable 面只增不破坏 / L3 不冻结 register·字节布局·工具版本为 ABI)"
  - "13 D-405 (稳定性与版本政策,已锁) / D-406 v2.0 (agent 完全自主) / D-008 (多后端红线,维持不解除) / D-312 (registry,维持休眠)"
  - "14 §1 §3 §4 §5 (契约 / 预算零占位 / deferred / 证据分级)"
  - "10 §3 (变更三档) / agents/AGENTS.md (硬规则十条)"
in_scope:
  - stabilization_report    # V1.1 稳定化报告:stable 面盘点（快照真实输出）+ 观察期判定三段式（10 §5「两个里程碑无重大修订」,裁决载体 = STABILIZATION_REPORT.md §3）+ 已知缺口诚实列举 → Direct（RFC-0008 已 Approved,report 是其 §6 明列产物）
  - fcp_lite_announcement   # V1.1 FCP-lite 公示:公开 GitHub Issue（label fcp-lite）,advisory 语义、通告即推进、保持开放 → Direct（治理动作,10 §2.2）
  - stable_channel_manifest # V1.2 最小 stable channel 清单:rurixup release 追加产出确定性 channel_manifest.json（channel=stable）+ Release 层第 8 子门 → **Mini-RFC 前置（MR-0008）**;条款先行（RXS-0185 续号,条款体与 ≥1 测试锚定随实现 PR 同落,commit 序条款在前);不实现 install/update/channel 切换
  - v1_0_0_release          # V1.3 首个 stable 发行:workspace 版号 0.1.0→1.0.0 + annotated tag v1.0.0 + release.yml 机器发布门全绿（10 §6 / 08 §9）→ Direct（发布元数据 + 发布工程,无语义变更）
  - github_release          # V1.3 首个 GitHub Release:gh release create v1.0.0 附测试签名产物 + SBOM + 签名清单 + SHA256SUMS,发布说明诚实标注签名状态 → Direct
out_of_scope:
  - azure_production_signing # of-record Azure Artifact Signing 生产签名:维持 secret+人工门（spec/release.md §4）,显式不作 1.0 发布阻断门（用户 2026-07-14 裁决:测试证书签名产物 + 诚实标注）
  - rustup_frontend          # rurixup install/update/channel 切换 rustup 式前端（08 §9 r6,MVP 后期）:V1 只落 channel 身份锚,前端为后续里程碑按档处置
  - second_edition           # 第二 edition / 任何 edition-gated 行为差异:RFC-0008 §8 范围红线,首期差异集 = 空集维持
  - registry                 # 包 registry（D-312/SG-007）:维持休眠 not_triggered,1.0 发布不构成触发
  - multi_backend            # 多后端（D-008/SG-003 红线 3）:维持不解除——**1.0 发布 ≠ NVIDIA 纵深完成**,解除属独立 one-at-a-time 决策（10 §9.2,对齐 G2 §8.8.4 口径）
  - grx_merge                # GRX showcase 分支合入 main:独立轨道,V1 期间不合入（撞号与快照面串行化约束,§7 ⑦）
  - ecosystem_criteria       # 生态成功判据（≥3 非作者维护项目,01 §6 第二层）:时间驱动社会判据,维持 G2 §8.8.5 carve-out,不作 V1 验收门
  - rd_implementation        # RD-007（const 泛型运行期单态化）/ RD-009（#[export(c)] codegen）的实现:仅账面承接（§6）,V1 无 device codegen / FFI ABI 工作
deferred_refs: [RD-007, RD-009]   # RD-007（inherited）/ RD-009（open）owner_milestone G2→V1 顺延（deferred.json v1.44「待后续阶段顺延」兑现,v1.45 留痕）;账面承接不实现、非 V1 验收门,预期 V1 close-out carry-forward 至 post-1.0 里程碑。开工无预造新 deferred,执行期按 14 §4 追加 RD-025+ 并双侧标注
deliverables:
  - id: D-V1-1
    name: V1.1 stabilization report（milestones/v1/STABILIZATION_REPORT.md:stable 面盘点 + 观察期判定三段式 + 已知缺口诚实列举 + evidence/v1.1-stabilization/ 机器事实归档）(G-V1-1)
  - id: D-V1-2
    name: V1.1 FCP-lite 公示（公开 GitHub Issue,label fcp-lite,advisory 语义,通告即推进、保持开放;issue URL 回填 §8）(G-V1-2)
  - id: D-V1-3
    name: V1.2 最小 stable channel 清单（MR-0008 Mini-RFC 前置 + spec/release.md RXS-0185 续号条款 + rurixup channel_manifest.json + Release 层第 8 子门 + CI 步骤 50;stable 快照因条款增长同 PR 重 bless）(G-V1-3)
  - id: D-V1-4
    name: V1.3 v1.0.0 首个 stable 发行（workspace 版号 1.0.0 + annotated tag v1.0.0 + release.yml 机器发布门全绿 + bundle rurix_version 一致）(G-V1-4)
  - id: D-V1-5
    name: V1.3 首个 GitHub Release（gh release create v1.0.0 附产物 + SHA256SUMS,发布说明诚实标注测试证书签名 + 生产签名 pending 人工门 + FCP-lite/report 链接）(G-V1-5)
acceptance_gates:
  - id: G-V1-1
    check: "stabilization report 合入 main:milestones/v1/STABILIZATION_REPORT.md 内容闭合——① stable 面盘点逐项取自 ci/stable_snapshot.py --check 真实输出 + 快照 SHA-256 + 锚定 commit hash;② 观察期判定三段式（定型点 2026-06-30 G2.5 / 后续两里程碑 G2.6+GRX stable 面零修订实证 / 残余弱点与对冲如实陈述,裁决载体 = report §3,契约 §7 ⑩）;③ 已知缺口诚实列举（🔒 禁区未条款化残余 / open RD / 生态判据 carve-out / 生产签名 pending）,每项注明属加性/实现缺口不修改既有条款语义;④ 配套 evidence/v1.1-stabilization/ 机器事实归档（evidence 只增不删）。证据等级 measured_local"
  - id: G-V1-2
    check: "FCP-lite 公示:公开 GitHub Issue 已创建（标题含 [FCP-lite] + 语言 1.0 稳定化,label fcp-lite）,正文含 report 链接 + stable 面四元组摘要 + advisory 语义声明（不设截止,agent 按通告即推进口径继续,后续意见按 10 §2.2 处理）+ 已知缺口摘要 + 在途事项预告;真实 issue URL 回填契约 §8（不伪造）;发布不关闭 issue（通告保持开放）"
  - id: G-V1-3
    check: "最小 stable channel 清单:MR-0008 Mini-RFC 合入先于实现（10 §3 失败测试先行:步骤 50 脚本与 channel 单测在基线上不存在→RED,落地后漂移注入/未知 channel/清单缺失即红）;spec/release.md RXS-0185 续号条款体与 ≥1 测试锚定同 PR 落地（commit 序条款在前,trace_matrix --check 维持全锚定,沿用全局 m1.counter.spec_clause_test_anchoring,**不另立 v1 counter**）;rurixup release 产出确定性 channel_manifest.json（channel=stable,同一输入两次逐字节一致,无时间戳）+ Release 层第 8 子门 channel-manifest（既有 7 门相对顺序 0-byte,失败 → allow_upload=false 退出码 2,零新 RX 码）;**stable 快照因条款增长同 PR 重 bless**（tests/stable/bless_log.md 同 diff 追加,RXS-0180 L2 加性演进,步骤 49 复绿）;CI 步骤 50 ci/channel_manifest_smoke.py 真实红绿闭合 + run URL 归档"
  - id: G-V1-4
    check: "v1.0.0 首个 stable 发行:Cargo.toml workspace 版号 1.0.0 合入（16 crate 统一继承 + Cargo.lock 同步）;ci/release_pipeline_smoke.py 版号注入点参数化 workspace_version()（根治 5 处硬编码漂移）;release.yml tag 触发器收窄 v[0-9]+.[0-9]+.[0-9]+*（防 v1-closed 误触发,须先于 v1-closed tag 生效）;annotated tag v1.0.0 推送 → release.yml self-hosted runner 全量 success + run URL 归档——该 run 即 10 §6 / 08 §9 机器发布门兑现（签名验签 RURIXUP_SIGN=1 真实 Authenticode + SBOM 双视图 + NVIDIA 白名单审计 + budget_eval --strict + cargo fmt/clippy/test + release_pipeline_smoke）;run 日志核验 bundle rurix_version=1.0.0 与 tag 一致（RXS-0135 同版号判据）"
  - id: G-V1-5
    check: "首个 GitHub Release:gh release create v1.0.0 --verify-tag 完成（本机人工链路执行,workflow 不授写权限——人工执行即 gh 发布动作的人工门）,附 release.yml run 产物（channel_manifest.json + bundle.json + sbom.spdx/cdx + signing_manifest.json + gate_decision.json）+ SHA256SUMS + 测试签名二进制;Release body **诚实标注**:附件为自签测试证书的真实 Authenticode 签名、of-record Azure 生产签名 pending 人工门（SmartScreen 会告警）、NVIDIA 白名单 pending-human-review 声明;body 含 stabilization report 与 FCP-lite issue 链接;body 定稿存 evidence/v1.3-release/（只增）"
guardrails:
  - "milestones/m0~g2 的 measured_local 既有预算条目 git diff 0-byte（新增 v1 条目允许但本期预期为零）;v1_budget.json 经命名空间强制前缀 v1. + namespace check 单测（14 §3,经既有 *_budget.json glob 自动纳入,零 CI 代码改动);entries/ratio/counter 全期留空 = 不预欠占位、不预造 counter（无 ci/budget_eval.py 新分支）"
  - "milestones/m0~g2 的 *_CONTRACT.md（均 closed）既有内容只追加不修改（check_closed_contracts,glob 已泛化）;本契约 V1_CONTRACT.md 于 V1.4 close-out 翻 closed 后自动纳入字节守卫"
  - "registry/deferred.json 与 registry/spike_gating.json 只追加;RD-007/RD-009 状态翻转仅由 agent 自主签署留痕追加;V1 期 SG 复评（SG-001/002/003/007/008 维持 not_triggered)只追加 decisions"
  - "registry/error_codes.json 错误码语义可加不可改;**V1 预期零新码**（channel 清单失败走工具层退出码 + failed_gates 枚举,spec/release.md §3 口径;RX7021 续号仅备用）"
  - "evidence/ 只增不删不改（M0.3 起）"
  - "00–14 共 15 份规划文档（含 13_DECISION_LOG.md）不被执行 PR 改写（check_planning_docs）;开工裁决记本契约 §7;**11_ROADMAP 发行标注走 00 §6.3 独立勘误 PR**（V1.3 发布后,与执行 PR 分离）"
  - "**stable API 快照变更必经 bless**（check_stable_snapshot_bless,RD-008 已激活）:V1 期唯一预期触发 = V1.2 条款增长（RXS-0185 续号）→ 快照重 bless + tests/stable/bless_log.md 同 diff 追加,与条款/实现同 PR（步骤 49 硬红,不可分 PR）"
  - "tests/ui .stderr / tests/mir .mir / tests/ptx .nvptx / tests/dxil golden 变更必须经审批 bless(既有机制);V1 预期零 golden 变更"
  - "全仓 crate 维持 unsafe_code=deny;V1 预期零新 unsafe(channel 清单为纯 host 确定性 JSON)"
  - "guardrail 回退基准默认 = g2-closed(G2 close-out 已切;PR 路径以 GITHUB_BASE_REF 为准);V1.4 close-out 时切至 v1-closed(agent 自主签署,glob 已泛化仅切基准默认值 + 打 tag)"
  - "仓库 LF byte-exact(* -text):新文件 LF + 尾换行,禁 Python 文本模式写文件;既有 CRLF 例外文件(registry/*.json 等)追加行保持其原行尾风格,既有行 0-byte"
  - "本契约 in_scope/acceptance_gates 等既有条款 0-byte 修改,close-out 只追加 §8;status active→closed 翻转 / 基准切换(g2-closed→v1-closed) / v1-closed tag / RD·SG 处置由 agent 自主签署"
---

# V1 契约 — 语言 1.0 正式发布（稳定化收尾）：stabilization report + FCP-lite 公示 + 最小 stable channel 清单 + v1.0.0 首个 stable 发行 + 首个 GitHub Release

> 所属:[../../11_ROADMAP.md](../../11_ROADMAP.md) §5「语言 1.0」交付线（三要件已于 G2.5 达成,本期为正式发布收尾）/ 契约机制见 [../../14_ENGINEERING_DISCIPLINE.md](../../14_ENGINEERING_DISCIPLINE.md) §1。
> 规范先行延续（AGENTS.md 硬规则第 7 条）:channel 清单语义面 PR 必须引用 RXS-#### 条款号（RXS-0185 续号）;缺条款先补 spec,条款 commit 先于实现 commit。
> 基准 ref:**默认 `g2-closed`**（G2 close-out 已完成切换;`ci/check_guardrails.py` 无参默认 = `g2-closed`,PR 路径以 `GITHUB_BASE_REF` 为准）。
> 粒度:**单 V1 阶段契约**:一份契约覆盖 V1 期,V1.1~V1.4 子里程碑分解见 [V1_PLAN.md](V1_PLAN.md)（对齐 M*/G1/G2「每里程碑一份契约 + 内部子里程碑」范式）。
> **定位口径:V1 不是重造 1.0 机制的里程碑。**11 §5 三要件（spec 全量条款化 / conformance 覆盖 / 首个 edition）与 RD-008 stable 快照激活均已于 G2.5 完成（G2_CONTRACT §8.7/§8.8）;V1 执行 RFC-0008 §6 stabilization 路径的收尾环节（观察期判定 → stabilization report → FCP-lite → 进入 stable = v1.0.0 发行）,并按用户 2026-07-14 裁决补最小 stable channel 清单（MR-0008）。
> **脚手架口径:本契约为 V1 开工结构件,不实现任何语义面、不发通告、不打 tag;§8 close-out 开工时为空。**

---

## 1. 目标

V1 期结束时项目获得:语言 1.0 正式发布——stabilization report 落档 + FCP-lite 公开公示（advisory,保持开放）+ 最小 stable channel 清单（channel=stable 身份锚,MR-0008）+ workspace 版号 1.0.0 + annotated tag `v1.0.0` 经 release.yml 机器发布门全绿 + 仓库史上首个 GitHub Release（测试证书签名产物 + 诚实标注）。1.0 之后 stable 面破坏只能走 edition（10 §6 / RXS-0180）。

## 2. 范围

### 2.1 in-scope

| 项 | 说明 | gating | 对应交付物 |
|---|---|---|---|
| stabilization_report | 稳定化报告:stable 面盘点 + 观察期三段式 + 已知缺口诚实列举（10 §5 / RFC-0008 §6） | Direct（RFC-0008 已 Approved,report 是其明列产物） | D-V1-1 |
| fcp_lite_announcement | FCP-lite 公示:公开 GitHub Issue,advisory,通告即推进、保持开放（10 §2.2） | Direct（治理动作） | D-V1-2 |
| stable_channel_manifest | 最小 stable channel 清单:channel_manifest.json + Release 层第 8 子门 + CI 步骤 50 | **Mini-RFC 前置（MR-0008）**;条款先行（RXS-0185 续号） | D-V1-3 |
| v1_0_0_release | 版号 1.0.0 + tag v1.0.0 + release.yml 机器发布门全绿（10 §6 / 08 §9） | Direct（发布元数据/工程） | D-V1-4 |
| github_release | 首个 GitHub Release:测试签名产物 + 诚实标注 | Direct（本机人工链路 gh release create） | D-V1-5 |

### 2.2 out-of-scope（显式排除）

- **azure_production_signing**:of-record Azure 生产签名维持 secret+人工门（spec/release.md §4）,不作 1.0 阻断门（用户裁决:测试证书 + 诚实标注）。
- **rustup_frontend**:rurixup install/update/channel 切换前端（08 §9,MVP 后期）;V1 只落 channel 身份锚。
- **second_edition**:第二 edition / edition-gated 行为差异,RFC-0008 §8 红线维持（首期差异集 = 空集）。
- **registry**（D-312/SG-007 休眠）/ **multi_backend**（D-008/SG-003 红线 3 不解除——1.0 发布 ≠ NVIDIA 纵深完成）/ **grx_merge**（独立轨道,V1 期间不合入）/ **ecosystem_criteria**（时间驱动,维持 carve-out）/ **rd_implementation**（RD-007/RD-009 仅账面承接）。

## 3. 交付物清单

| ID | 交付物 | 形态 | 完成判据 |
|---|---|---|---|
| D-V1-1 | stabilization report | milestones/v1/STABILIZATION_REPORT.md + evidence/v1.1-stabilization/ | 内容闭合 + 机器事实归档（G-V1-1） |
| D-V1-2 | FCP-lite 公示 | 公开 GitHub Issue（label fcp-lite） | issue 创建 + URL 回填 §8 + 保持开放（G-V1-2） |
| D-V1-3 | 最小 stable channel 清单 | MR-0008 + spec/release.md RXS-0185 续号 + src/rurixup channel 模块 + ci/channel_manifest_smoke.py | Mini-RFC 先行 + trace 全锚定 + 快照重 bless + 步骤 50 红绿（G-V1-3） |
| D-V1-4 | v1.0.0 发行 | workspace 1.0.0 + tag v1.0.0 + release.yml run | 机器发布门全绿 + 版号一致核验（G-V1-4） |
| D-V1-5 | 首个 GitHub Release | gh release v1.0.0 + 产物 + SHA256SUMS | 诚实标注 + 链接闭环 + body 留痕（G-V1-5） |

## 4. 验收门（完整版,YAML 头为可提取摘要）

见 YAML 头 `acceptance_gates` 字段 G-V1-1 ~ G-V1-5。要点:
- **G-V1-1/2（V1.1,先行）**:report 数字全部取自机器真实输出（快照 --check / git 实证）,观察期弱点如实陈述不夸大;FCP-lite issue 真实 URL 回填,不伪造。
- **G-V1-3（V1.2,唯一新机制面）**:MR-0008 先行 → 条款+实现+快照重 bless+步骤 50 同 PR（commit 序条款在前）;零新 RX 码、零新 unsafe、不另立 v1 counter。
- **G-V1-4/5（V1.3,发行）**:release.yml 全量 success 即机器发布门兑现;GitHub Release 诚实标注签名状态;版号一致三点核验（tag / workspace / bundle rurix_version）。
- close-out `budget_eval --strict` 全局零 estimated 残留（14 §3;v1_budget 全期留空则自然满足）。

## 5. Guardrails（字节级,机器核对）

见 YAML 头 `guardrails` 字段。核对方式:`py -3 ci/check_guardrails.py`（无参默认基准 = `g2-closed`;PR 路径以 `GITHUB_BASE_REF` 为准）。要点:00–14（含 13）+ deep-research 0-byte;registry/预算/已关闭契约/evidence 只追加;error_codes 含义冻结（V1 零新码）;spec 档位标记 + 修订数据行避「版本」子串（用「版号」）;stable 快照 bless（V1.2 预期触发一次）;golden bless;基准 `g2-closed`（close-out 切 `v1-closed`）;LF byte-exact。

## 6. Deferred 引用

| 编号 | 内容摘要 | 承接 |
|---|---|---|
| RD-007 | const 泛型值运行期单态化（turbofish const 实参 → 实例值代入 + codegen） | inherited,owner_milestone G2→V1 顺延;V1 无 device codegen 工作,账面承接不实现,非 V1 验收门,close-out carry-forward |
| RD-009 | `#[export(c)]` C ABI 导出属性 + 编译器内建头文件生成 codegen | open,owner_milestone G2→V1 顺延;1.0 的 C ABI stable 面 = 手写 `extern "C"`（RXS-0125,10 §6 口径）,`#[export(c)]` 为加性未来项;账面承接不实现,非 V1 验收门,close-out carry-forward |

详情以 [../../registry/deferred.json](../../registry/deferred.json) 为唯一事实源,本表仅引用。其余 open RD（RD-011/012/014/015/018/019/020/022/023/024）维持 G2.x 归属不进 deferred_refs,统一在 STABILIZATION_REPORT §4「已知缺口」如实列举。V1 开工无预造新 deferred;执行期按 14 §4 追加 `RD-025+` 并双侧标注。

## 7. 修订记录 / 开工裁决留痕

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-07-14 | 初版契约固化（V1 开工脚手架）。**开工裁决**（用户 2026-07-14 经 AskUserQuestion 四项裁决 + agent 完全自主判档 AGENTS v3.0 硬规则 1,记于本节;13_DECISION_LOG 执行 PR 字节冻结,不改决策日志）:① **新里程碑 = milestones/v1/**,namespace `v1.`,收口 tag `v1-closed`（用户裁决;post-G2 里程碑路线图未定义 G2 §8.8.3,V1 即其「后续阶段」;命名直指语言 1.0 发布,与 5 年愿景期不混淆）。② **子里程碑分解 = V1.1 stabilization report + FCP-lite → V1.2 最小 stable channel 清单 → V1.3 v1.0.0 发行 + GitHub Release → V1.4 close-out**,严格串行（V1_PLAN.md）。③ **FCP-lite = 通告即推进**（用户裁决;10 §2.2 开源后 FCP-lite 为 advisory、AI agent 可自主推进;通告保持开放承接追溯意见,发布不关闭 issue）。④ **rurixup 范围 = 最小 stable channel 清单,判档 Mini-RFC（MR-0008）**（用户裁决;工具行为/发布产物清单形态量级,对齐 MR-0002/MR-0005 先例;不实现 install/update/channel 切换,rustup 式前端 08 §9 留后续按档处置）。⑤ **GitHub Release = 测试证书签名产物 + 诚实标注**（用户裁决;RURIXUP_SIGN=1 自签测试证书真实 Authenticode;of-record Azure 生产签名维持 secret+人工门,显式不作 1.0 阻断门;gh release create 走本机人工链路,workflow 不授写权限）。⑥ **版号 0.1.0→1.0.0 归属 V1.3 发行 PR,判档 Direct**（发布元数据非语义面;RXS-0135 同版号判据为参数化判据,无条款修改;ci/release_pipeline_smoke.py 版号注入点同 PR 参数化为 workspace_version()）。⑦ **撞号规避留痕**:新 RXS 条款自 **RXS-0185** 起（RXS-0181~0184 已被 GRX showcase 分支占用,未合 main）/ Mini-RFC = **MR-0008**（MR-0006/0007 GRX 占用）/ 新错误码自 RX7021 起（本期预期零新码）/ 新 CI 步骤 = **50**;编号永不复用（10 §9.5);**V1 期间 GRX 不合入 main**（快照面串行化,若例外合入则 V1.2 rebase 后按合并面重 bless）。⑧ **deferred 承接**:RD-007 inherited / RD-009 open,owner_milestone G2→V1 顺延（deferred.json v1.44「待后续阶段顺延」兑现,v1.45 留痕）;账面承接不实现、非 V1 验收门,close-out carry-forward;开工无预造新 deferred。⑨ **红线/SG 复评:1.0 发布不构成任何触发**——D-008 红线 3 不解除（NVIDIA 纵深口径未达,解除属独立 one-at-a-time 决策 10 §9.2）/ SG-001/002/003/007/008 维持 not_triggered / D-312 registry 维持休眠 / 第二 edition 不引入（RFC-0008 §8）。⑩ **观察期判定（10 §5「两个里程碑无重大修订」）裁决载体 = STABILIZATION_REPORT.md §3**,三段式诚实落笔（定型点 G2.5 / 后续两里程碑 G2.6+GRX stable 面零修订实证 / 残余弱点与对冲如实陈述,含「main 自 g2-closed 零提交」佐证的必要非充分性）;判档争议向上取严作为自我约束建议,本裁决按 10 §2.2 advisory 治理口径 agent 自主推进。⑪ **release.yml tag 触发器收窄**（`v*` → `v[0-9]+.[0-9]+.[0-9]+*`）归属 V1.3 PR,**必须先于 v1-closed tag 生效**（防 milestone tag 误触发 release workflow;历史 m*/g*-closed tag 均无 v 前缀,v1-closed 是首次撞上）。⑫ **判档:脚手架取 rfc_required: none**（结构件,对齐 M4~G2 先例;stabilization/edition Full RFC 面已由 RFC-0008 Agent Approved 承载,V1 为其 §6 路径执行收尾,不新开 Full RFC;唯一新机制面 channel 清单经 MR-0008 前置）。**V1 close-out 关闭判定 / 基准切换（g2-closed→v1-closed）/ v1-closed tag / RD-007·RD-009 处置 / SG 复评由 agent 自主签署** |

---

## 8. Close-out（只追加区 — 开工时为空）

<!-- 验收记录、guardrail 核对输出、V1.1~V1.3 端到端留痕（report / FCP issue URL / MR-0008 + 步骤 50 run URL / v1.0.0 tag + release.yml run URL / GitHub Release URL）、RD-007/RD-009 处置留痕、SG 复评结论追加于此;上方条款 0-byte 修改。V1 close-out 关闭判定 / 基准切换（g2-closed→v1-closed）/ v1-closed tag / RD·SG 处置由 agent 自主签署兑现。 -->

### 8.1 V1.1 验收留痕（2026-07-14,G-V1-1 / G-V1-2）

agent 完全自主签署（AGENTS v3.0 硬规则 1),记录机器事实:

- **G-V1-1 stabilization report**:[STABILIZATION_REPORT.md](STABILIZATION_REPORT.md) 经 PR [#120](https://github.com/qwasg/Rurix/pull/120) 合入 main（pr-smoke 全量 success [run 29324745826](https://github.com/qwasg/Rurix/actions/runs/29324745826)）;stable 面盘点数字逐项取自本机 `ci/stable_snapshot.py --check` 真实输出（180/88/["2026"]/8,快照 SHA-256 `08e2e264…e91e0` 锚定 `0ceca0d9`）;观察期三段式判定落 §3（G2.6 `f659f57a` + GRX `95d5af43` 两里程碑零 stable 面修订实证 + 残余弱点如实陈述）;机器事实原文归档 [../../evidence/v1.1-stabilization/](../../evidence/v1.1-stabilization/)。证据等级 measured_local。
- **G-V1-2 FCP-lite 公示**:公开 GitHub Issue **[#121](https://github.com/qwasg/Rurix/issues/121)**（标题 `[FCP-lite] Rurix 语言 1.0 稳定化(edition 2026)`,label `fcp-lite`,2026-07-14 创建）——含 report 链接 + stable 面四元组 + advisory 语义声明（不设截止,通告即推进,§7 ③）+ 已知缺口摘要 + 在途事项预告（MR-0008 / v1.0.0 发行）;**issue 保持开放**,发布不关闭,后续 v1.0.0 tag / GitHub Release 链接以评论回填。
- 判定:D-V1-1 / D-V1-2 交付闭环,G-V1-1 / G-V1-2 达成;**V1 契约仍 active**,不执行基准切换 / v1-closed tag(归 V1.4 close-out)。

### 8.2 V1.2 验收留痕（2026-07-14,G-V1-3）

agent 完全自主签署(AGENTS v3.0 硬规则 1),记录机器事实:

- **Mini-RFC 前置**:MR-0008(agent Approved 2026-07-14)经 PR [#122](https://github.com/qwasg/Rurix/pull/122) 先行合入 main(pr-smoke success [run 29325620618](https://github.com/qwasg/Rurix/actions/runs/29325620618));失败测试先行兑现(步骤 50 脚本与 channel 模块在提案时点 main 上不存在 = RED)。
- **条款先行 + 实现**:PR [#123](https://github.com/qwasg/Rurix/pull/123)(5 commit,条款 commit `e82e2923` 先于实现 commit `3b1a46aa`)合入 main——spec/release.md v1.3 RXS-0185/0186 条款体 + `src/rurixup/src/channel.rs`(VALID_CHANNELS=["stable"] / generate / consistent / to_json 确定性)+ gate 第 8 子门 `channel-manifest`(既有 7 门相对顺序 0-byte)+ main CLI `--channel`(缺省 stable)/`--simulate-channel-drift`。零新 RX 码 / 零新 unsafe / 零外部依赖 / 不另立 v1 counter。
- **stable 快照重 bless**:spec_clauses 180→182(其余三段 0 变化),`tests/stable/bless_log.md` 同 diff 追加(commit `42ed318c`,check_stable_snapshot_bless 守卫过;RXS-0180 L2 加性演进);步骤 49 `ci/edition_smoke.py` 篡改红绿闭合复绿。
- **CI 步骤 50 真实红绿 + run URL**:`ci/channel_manifest_smoke.py` 接线 pr-smoke.yml + release.yml;PR #123 pr-smoke 全量 success **[run 29326570278](https://github.com/qwasg/Rurix/actions/runs/29326570278)**(步骤 50 在 runner 真跑:green + 确定性两次逐字节一致 + 漂移注入红 exit 2 + 未知 channel 红 exit 1 + 复原绿);trace 182/182(454 测试文件);`cargo test --workspace` 全绿(rurixup 14 passed)。
- 判定:D-V1-3 交付闭环,G-V1-3 达成;**V1 契约仍 active**。
