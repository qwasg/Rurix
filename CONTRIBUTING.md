# 为 Rurix 贡献

感谢你对 Rurix 的兴趣。Rurix 是一门把*资源所有权、地址空间、并行执行层级*做成类型系统一等公民的 GPU 系统编程语言;它从第一天就把**可测试规范 + conformance 唯一验收边界 + provenance 强制**内建为治理骨架(见 [`10_GOVERNANCE.md`](10_GOVERNANCE.md))。本指南是这些规则对外贡献者的落地说明。

> 治理总览见 [`10_GOVERNANCE.md`](10_GOVERNANCE.md) §7–§9;工程纪律机制见 [`14_ENGINEERING_DISCIPLINE.md`](14_ENGINEERING_DISCIPLINE.md);所有 AI 会话的强制上下文见 [`agents/AGENTS.md`](agents/AGENTS.md)。

## 核心原则:规范 ↔ 测试 ↔ PR 三角

Rurix 的唯一验收边界是 `conformance/`,不是 PR 描述。

- **规范先行**:改 `src/` 前必读相关 `spec/*.md` 条款。语义 PR **必须引用条款号 `RXS-####`**;缺条款的语义改动须先补 spec(走对应变更档位 + 修订行),**条款 PR 先于实现 PR**。
- **每条规范条款 ≥1 测试锚定**(`ci/trace_matrix.py` 全局核对)。
- **验证强制**:完成声明必须附带 conformance / UI / 单测命令的**真实输出**;**数字必须来自命令输出**,禁止凭记忆或推断填写。

## 变更分档(三档门)

按语义影响**自助判定**档位(详见 10 §3)。先对号入座,再按「承办」列动作:

| 你的变更 | 档位 | 需要 | 承办 |
|---|---|---|---|
| 文档措辞 / 纯重构 / 补测试覆盖 / 不改语义的 bugfix | **Direct** | 评审 + CI 绿 | PR 直接走,不进 `rfcs/` |
| 规范内 bugfix / 诊断措辞策略 / 内部开关 / 工具行为变更 / 规则文件(`agents/AGENTS.md`)级修改 | **Mini-RFC** | **失败测试先行** + 单页提案 + 语言或实现负责人批准 | 先合 [`rfcs/mini-NNNN-*.md`](rfcs/TEMPLATE-MINI-RFC.md) |
| 新语法 / 类型系统变更 / 运行时语义 / `unsafe` 边界 / FFI ABI / 内存模型映射 / 稳定化 / edition / 设计原则修改 / 死亡路线触碰 | **Full RFC** | RFC 合入后才可实现 + feature gate + tracking issue + spec diff + conformance 测试 + stabilization report + FCP-lite | 先合 [`rfcs/NNNN-*.md`](rfcs/TEMPLATE-RFC.md),再 feature gate |
| **判档不清** | → **向上取严**(取更严档位) | — | **不自判 Direct**(AI agent 无此权,10 §7 / AGENTS 硬规则 8) |

模板与提案 intake 通道见 [`rfcs/README.md`](rfcs/README.md);FCP-lite 评审窗(≥2/3 同意含语言负责人 + 5–7 天公开等待窗、6 周 train、晋升路径)见 [`rfcs/README.md`](rfcs/README.md) §3。

## AI 贡献政策(D-406,从第一天生效,对所有人含所有者本人)

1. **Human-in-the-loop**:AI 产出必经人类批准合入;AI 不得代签任何署名承诺。
2. **Provenance**:实质性 AI 内容标注 `Assisted-by: <tool>:<model>`;提交说明含影响范围与验证方式。
3. **反 extractive contribution**:不得以"提交了再说"把验证成本转嫁给评审。
4. **禁区**:AI 不得定义/修改 UB 条款、内存模型映射、FFI ABI、安全包络边界——这些只能由人类经 Full RFC 落笔。

> 开源后 CI 自动阻断缺 provenance / 验证输出 / 条款号的 PR——由 [`ci/check_contribution.py`](ci/check_contribution.py) 在 PR Smoke 守卫步骤兑现(10 §7 第一年路线落地)。

### 贡献 PR 自检(`ci/check_contribution.py` 阻断项)

`ci/check_contribution.py` 扫描 PR 范围(`base..HEAD`)的每个非 merge commit,三类缺项即红——提交前自查:

1. **Provenance**:每个 commit 含 `Assisted-by: <tool>:<model>` 或 `Co-Authored-By:` trailer(D-406 / 硬规则 2)。
2. **条款号**:触 `src/**/*.rs` 或 `spec/**/*.md` 的 commit,在 commit body / 新增 `//@ spec: RXS-####` 注释行 / 关联 `rfcs/*.md` 之一引用条款号(或 deferred/RFC 编号;纯文档/纯测试豁免,硬规则 7)。
3. **验证强制**:触 `src/` 功能改动的 commit body 含验证标记(`Validation:` / `验证:` / 引用 `ci/*.py` / `cargo test` 命令;数字必须来自命令输出,硬规则 3/10)。

本机自查:`py -3 ci/check_contribution.py`(PASS=0 / 阻断=非零退出)。

## `unsafe` 纪律

- 每个 `unsafe` 块附 `// SAFETY:` 注释,引用 [`unsafe-audit/`](unsafe-audit/) 注册表条目;单块单操作。
- **无注册条目的 `unsafe` 是 CI 错误。** 全仓默认 `unsafe_code = deny`;FFI 边界(PYD / C ABI / DLPack / cublas)凡落 `unsafe` 须经裁决最小开 + 注册。

## 提交前自检

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
py -3 ci/trace_matrix.py --check        # 规范↔测试锚定全绿
py -3 ci/budget_eval.py --strict        # 性能/诊断预算 measured_local(零 estimated)
py -3 ci/check_guardrails.py && py -3 ci/check_schemas.py && py -3 ci/check_structure.py
```

性能数据须遵循 [`milestones/m0/BENCH_PROTOCOL.md`](milestones/m0/BENCH_PROTOCOL.md)(L0 锁频 + 三次进程级独立运行 + trimmed mean),证据落 `evidence/`(只增不删不改)。

## 上游政策

对 LLVM 的修补优先 upstream;pin 的 fork 补丁必须带 upstream issue 链接(防 fork 漂移)。

## 行为准则

参与本项目即同意遵守 [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md)。

## 许可

提交贡献即表示你同意你的贡献按 **MIT OR Apache-2.0** 双许可授权(见 [`LICENSE-MIT`](LICENSE-MIT) / [`LICENSE-APACHE`](LICENSE-APACHE)),与本项目一致。
