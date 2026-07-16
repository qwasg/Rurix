# Rurix 语言 1.0 Stabilization Report

> 所属:[V1_CONTRACT.md](V1_CONTRACT.md) D-V1-1 / G-V1-1(V1.1)
> 版本:v1.0(2026-07-14)
> 依据链:11 §5(语言 1.0 三要件)→ 10 §5(稳定化条件:spec 条款齐 + conformance 齐 + UI 测试齐 + 两个里程碑无重大修订 → **stabilization report** → FCP-lite → 进入 stable)→ 10 §6(D-405:1.0 = 开源后第一个 LTS 质量版本)→ RFC-0008 §6(stabilization 路径锚点,Agent Approved 2026-06-30)。本文即该路径的「stabilization report」环节。
> 锚定 commit:**`0ceca0d9`**(= `g2-closed` tag,G2 整体 close-out merge,2026-06-30)。本报告全部数字取自该基线上的机器真实输出,原文归档 [../../evidence/v1.1-stabilization/](../../evidence/v1.1-stabilization/)。
> 撰写口径:**诚实优先**——观察期论证的残余弱点如实陈述(§3.3),已知缺口不掩饰(§4),不宣称未达成之事。

---

## 1. 结论(先摆结论)

**语言 1.0 stable 面已定型且满足 10 §5 稳定化条件,agent 依 10 §2.2 advisory 治理口径裁定进入 stable,兑现为 v1.0.0 首个 stable 发行。**

- 11 §5 三要件已于 G2.5(2026-06-30)达成:spec 全量条款化(180 条 RXS,零散文/零占位)+ conformance 覆盖全部 stable 特性(trace 180/180 全锚定,453 测试文件)+ 首个 edition 机制就绪(edition `"2026"`,RFC-0008/RXS-0177~0180)。
- RD-008 stable API 快照冻结机制已激活(open→closed,G2.5):快照已 bless、CI 步骤 49 已接线、bless 守卫已激活。
- 观察期判定(「后续两里程碑无重大修订」)成立,论证与残余弱点见 §3。
- 1.0 之后:同 edition 内 stable 面只增不破坏(RXS-0180 L2),破坏性变更只能走未来 edition(10 §6)。

## 2. stable 面盘点(机器真实输出)

`py -3 ci/stable_snapshot.py --check` @ `0ceca0d9`(2026-07-14 本机真跑,原文见 evidence):

```
[stable_snapshot] PASS(stable 面与入库快照一致:spec_clauses=180,error_codes=88,editions=['2026'],subcommands=['bench', 'build', 'check', 'doc', 'fmt', 'run', 'test', 'vendor'])
```

| stable 面构成 | 计数 | 内容与冻结语义 |
|---|---|---|
| spec RXS 条款 ID 全集 | **180**(RXS-0001~RXS-0180) | 23 个 spec 域文件,FLS 体例;条款 ID 存在性 + 含义锚定(快照记录全 ID 列表);分配制递增、编号永不复用(10 §9.5) |
| 错误码 ID + 含义 | **88**(registry/error_codes.json) | `id → message_key` 含义冻结(可加不可改义,M1.1 起已运行,10 §6);双语 message 覆盖 88/88 |
| edition 合法值集 | **["2026"]** + edition_anchor "2026" | 首个 edition = 机制锚点,edition-gated 行为差异集 = 空集(RFC-0008 §9 Q-Scope);未知 edition → RX7020 strict-only 拒 |
| rx CLI 子命令面 | **8**:bench / build / check / doc / fmt / run / test / vendor | 子命令存在性锚定(src/rx USAGE 提取) |

- 快照文件:`tests/stable/stable_api.snapshot`(7527 字节),SHA-256 = `08e2e26423579bc933faae2168a4bfc63810d640dfe46e1110a5c062a97e91e0`;首 bless 2026-06-30(`tests/stable/bless_log.md`),变更须经 bless 守卫(`check_stable_snapshot_bless`)。
- conformance 锚定:`py -3 ci/trace_matrix.py --check` = **PASS(180/180 clauses anchored,453 test files scanned)**——零空锚定、零 placeholder/pending。
- **🔒 边界重申(RXS-0180 L3)**:快照锚定 stable 面的**存在性 + 含义**,**不冻结** register 分配 / mask/packing / 字节布局 / PTX·DXIL 产物形状 / 工具链版本为语言 ABI 保证(对齐 RXS-0162 / RXS-0165 / RXS-0171 先例;10 §6「不承诺稳定」清单)。
- **C ABI stable 面口径**:1.0 的 C ABI 导出约定 = 手写 `extern "C"`(RXS-0125,10 §6 稳定面清单);`#[export(c)]` 属性 + 内建头文件生成为**加性未来项**(RD-009,open),不在 1.0 承诺面内。

## 3. 观察期判定(「后续两里程碑无重大修订」,10 §5;裁决载体 = 本节,V1_CONTRACT §7 ⑩)

### 3.1 定型点

语言 1.0 stable 面 + 首个 edition 于 **2026-06-30(G2.5)** 定型:RFC-0008(edition 机制与 stabilization 流程,Full RFC)Agent Approved;spec/edition.md RXS-0177~0180 落地;RD-008 激活(open→closed);快照首 bless;CI 步骤 49(`ci/edition_smoke.py`,含篡改红绿闭合)接线(G2_CONTRACT §8.7 / deferred.json v1.43)。

### 3.2 后续两个里程碑(主证据)

| 里程碑 | 收口 | stable 面修订 | 实证 |
|---|---|---|---|
| **G2.6**(G2 整体 close-out) | 2026-06-30,commit `f659f57a`,PR #117 | **零** | `git show --stat f659f57a`:全部改动 = `ci/check_guardrails.py`(基准切换)+ `milestones/g2/G2_CONTRACT.md`(§8.8 追加 + status 翻转)+ `registry/deferred.json` + `registry/spike_gating.json` 共 4 个治理文件——**零 spec/、零 error_codes、零 src/** |
| **GRX**(showcase 里程碑,Godot 渲染集成) | 2026-07-13,commit `95d5af43`,status active→closed | **对 main stable 面:零(未合入);对既有条款体:零修订** | GRX 全程在独立分支,**未合入 main**;其分支上 spec 改动 = 新增加性 spike 条款 RXS-0181~0184 + 索引/登记行更新(`git diff g2-closed 95d5af43 -- spec/` 核对:既有 RXS-0001~0180 条款体零删改);error_codes 零触碰 |

**佐证**:`git log g2-closed..origin/main`(全路径与 `-- spec/ registry/error_codes.json` 过滤)均为**空**——main 自 `g2-closed` 起零提交。**如实标注:此佐证是「无修订」的必要非充分证据**(main 零提交同时意味着零活动),证明力主要落在上表两个里程碑的实证:两者均有实质工作发生(G2.6 全量回归冻结终审;GRX 完整 showcase 里程碑并签署 close-out),且均未产生对既有 180 条款/88 错误码的修订需求。

### 3.3 残余弱点与对冲(如实陈述)

**弱点**:① 日历窗口短——定型(06-30)至本报告(07-14)约两周;② 里程碑 1(G2.6)与定型同日,其「无修订」的价值在「全量回归冻结跑绿下零修订需求」而非时间厚度;③ 里程碑 2(GRX)未合入 main,以分支 close commit 为锚引用(unmerged 状态已标注)。本仓库语境下「两个里程碑」为里程碑计数而非日历承诺(11 §7:月份为相对刻度),但按向上取严的自我约束口径,此弱点应留痕而非隐去。

**对冲**:① **FCP-lite 通告保持开放**(V1_CONTRACT §7 ③)——发布前短窗转为发布后持续 advisory 通道,任何追溯意见按 10 §2.2 处理;② **RXS-0180 L2**:同 edition 内 stable 面只增不破坏——即便后续发现缺陷,破坏性修正只能经未来 edition 隔离,1.0 承诺的暴露面有界;③ 错误码含义冻结自 M1.1(2026-06 上旬)已运行逾一个月非新事;④ 快照 bless 守卫 + CI 步骤 49 硬门常驻,任何 stable 面漂移在 PR 层即红。

**判定**:观察期条件成立,依 10 §2.2(开源后 FCP-lite 为 advisory,AI agent 可自主推进)+ AGENTS v3.0 硬规则 1,agent 自主裁定进入 stable。判档争议向上取严作为自我约束建议(硬规则 8):已以本节弱点全量留痕兑现。

## 4. 已知缺口(诚实列举——每项均为加性/实现缺口,不修改既有 180 条款语义,不阻断 1.0;阻断判据 = 10 §6 机器门)

### 4.1 🔒 禁区未条款化残余(须 Full RFC 方可条款化)

| 缺口 | 状态 |
|---|---|
| 纹理路径内存模型:RFC-0007 采样首期收敛子集之外——隐式 LOD / quad 导数 / 派生链一致性 / 可配置 sampler 状态(**RD-022**)、非归一化整型 texel fetch(**RD-023**)、比较采样(shadow)/ gather / UAV 写 + memory-order(**RD-024**) | open,RFC-0007 §8 显式收敛留痕;触 06 §4.2 禁区,须 Full RFC |
| FFI ABI 二进制布局 + `#[export(c)]` codegen(**RD-009**) | open(owner_milestone V1 账面承接不实现);1.0 C ABI = `extern "C"`(§2 口径) |
| 并发/barrier 语义本体(RXS-0169 显式不落) | 系统性留白,须 Full RFC |
| UB 边界系统性条款化 | 现行条款「严禁 UB 节」体例;UB 语义本体经 Full RFC 落笔(10 §7.5) |

### 4.2 实现侧缺口(open RD,条款/类型面已 stable、实现未兑现或 defer)

- **RD-012** mesh / amplification(task)/ RT 着色器类型的 DXIL 合规降级(类型面条款 RXS-0153~0156 在 stable 面;降级请求 → RX6007 显式拒,非静默);**RD-011/014/015** DXIL 工具链偏差与供应链跟踪(A 路 PSV patch / B 路转译链 pin / A-graphics 上游迁回);**RD-018** bindless / unbounded descriptor array;**RD-019** 窗口 swapchain present 路径;**RD-020** 自动资源状态跟踪推导;**RD-007** const 泛型值运行期单态化(inherited,RXS-0064 语义不变,回填仅补实现侧)。
- 全部为**加性能力缺口**:各自 backfill 条件未兑现,依 14 §4 承接,不构成对既有条款语义的修订。

### 4.3 治理/生态面

- **生态成功判据**(≥3 非作者维护真实项目,01 §6 第二层):时间驱动社会判据,维持 G2 §8.8.5 carve-out,**不宣称达成**,非 1.0 技术发布阻断门。
- **生产签名**:of-record Azure Artifact Signing 维持 secret+人工门(spec/release.md §4);v1.0.0 发行产物为自签测试证书的真实 Authenticode 签名,发布页诚实标注(V1_CONTRACT §7 ⑤)。
- **多后端红线**(D-008/SG-003):1.0 发布 ≠ NVIDIA 纵深完成,红线 3 不解除;**registry**(D-312/SG-007)维持休眠。

### 4.4 本里程碑内预告(加性演进声明)

V1.2 将新增**加性工具面条款**(stable channel 清单,RXS-0185 续号,Mini-RFC/MR-0008):spec 条款计数 180→182 属 RXS-0180 L2 加性演进,**不构成对本报告所锚定 stable 面的修订**;快照随之重 bless(bless_log 留痕即审计轨迹)。

## 5. 1.0 稳定性承诺重述(10 §6 / D-405)

1. **SemVer**:1.0 = 开源后第一个 LTS 质量版本;stable 面破坏**只能走 edition**。
2. **稳定面清单(P-10)**:语言语法(stable 特性)/ std·core·gpu stable API / C ABI 导出约定(`extern "C"`)/ `rurix.toml`·`rurix.lock` schema / 诊断 JSON schema / 错误码含义(可加不可改义)。
3. **不承诺稳定**:内部 IR / PTX·DXIL 产物形状 / telemetry 字段集 / nightly·gated 特性 / register·字节布局 / 工具链版本。
4. **发布门(机器门)**:conformance 全绿 + UI golden 全绿 + L1 基准无 Critical 回归 + SBOM/签名齐备(08 §9)——v1.0.0 经 release.yml 全量兑现,run URL 归档于 V1_CONTRACT §8。
5. **弃用政策**:10 §9.4;特性生命周期 10 §5(feature gate → tracking → 稳定化)延续。

## 6. FCP-lite 通告

依 10 §2.2(开源后 FCP-lite = advisory:走 RFC 流程并公开等待窗,不强制人工同意数,AI agent 可自主推进)+ V1_CONTRACT §7 ③(通告即推进,通告保持开放):公开 GitHub Issue 承载,标题 `[FCP-lite] Rurix 语言 1.0 稳定化(edition 2026)`,label `fcp-lite`;发布不关闭 issue,任何追溯意见按 10 §2.2 处理。

- 通告 URL:**(issue 创建后回填于 V1_CONTRACT §8,本行不改写——见该处)**

## 7. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-07-14 | 初版(V1.1 / D-V1-1 / G-V1-1):stable 面盘点(180/88/["2026"]/8,快照 SHA-256 锚定)+ 观察期三段式判定(定型点 G2.5 / G2.6+GRX 两里程碑零 stable 面修订实证 / 残余弱点与对冲如实陈述)+ 已知缺口诚实列举(🔒 禁区残余 / open RD / 生态判据 carve-out / 生产签名 pending)+ 1.0 承诺重述 + FCP-lite 通告锚;全部数字取自 `0ceca0d9` 基线机器真实输出,原文归档 evidence/v1.1-stabilization/ |
