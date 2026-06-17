# 为 Rurix 贡献

感谢你对 Rurix 的兴趣。Rurix 是一门把*资源所有权、地址空间、并行执行层级*做成类型系统一等公民的 GPU 系统编程语言;它从第一天就把**可测试规范 + conformance 唯一验收边界 + provenance 强制**内建为治理骨架(见 [`10_GOVERNANCE.md`](10_GOVERNANCE.md))。本指南是这些规则对外贡献者的落地说明。

> 治理总览见 [`10_GOVERNANCE.md`](10_GOVERNANCE.md) §7–§9;工程纪律机制见 [`14_ENGINEERING_DISCIPLINE.md`](14_ENGINEERING_DISCIPLINE.md);所有 AI 会话的强制上下文见 [`agents/AGENTS.md`](agents/AGENTS.md)。

## 核心原则:规范 ↔ 测试 ↔ PR 三角

Rurix 的唯一验收边界是 `conformance/`,不是 PR 描述。

- **规范先行**:改 `src/` 前必读相关 `spec/*.md` 条款。语义 PR **必须引用条款号 `RXS-####`**;缺条款的语义改动须先补 spec(走对应变更档位 + 修订行),**条款 PR 先于实现 PR**。
- **每条规范条款 ≥1 测试锚定**(`ci/trace_matrix.py` 全局核对)。
- **验证强制**:完成声明必须附带 conformance / UI / 单测命令的**真实输出**;**数字必须来自命令输出**,禁止凭记忆或推断填写。

## 变更分档(三档门)

按语义影响选择档位(详见 10 §3):

- **Direct** — 不改语义面的工程实现(bugfix / 重构 / 文档 / 补测试覆盖)。无需新条款。
- **Mini-RFC** — 小语义面或规则文件(`agents/AGENTS.md`)级修改。
- **Full RFC** — 新语言特性 / 语义面 / 扩张方向。模板:动机 / 设计 / 备选 / 对 spec 的 diff / 未决问题(见 [`rfcs/`](rfcs/))。

**争议向上取严**:档位不清时取更严档位,不自判 Direct。

## AI 贡献政策(D-406,从第一天生效,对所有人含所有者本人)

1. **Human-in-the-loop**:AI 产出必经人类批准合入;AI 不得代签任何署名承诺。
2. **Provenance**:实质性 AI 内容标注 `Assisted-by: <tool>:<model>`;提交说明含影响范围与验证方式。
3. **反 extractive contribution**:不得以"提交了再说"把验证成本转嫁给评审。
4. **禁区**:AI 不得定义/修改 UB 条款、内存模型映射、FFI ABI、安全包络边界——这些只能由人类经 Full RFC 落笔。

> 开源后 CI 将自动阻断缺 provenance / 验证输出 / 条款号的 PR。

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
