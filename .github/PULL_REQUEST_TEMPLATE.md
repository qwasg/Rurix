<!-- 贡献指南见 CONTRIBUTING.md;三档门自助判定见 CONTRIBUTING.md「变更分档(三档门)」。
     CI 自动阻断缺 provenance / 验证输出 / 条款号的 PR(ci/check_contribution.py)。 -->

## 变更分档(三档门 — 勾选其一)

- [ ] **Direct** — 文档措辞 / 纯重构 / 补测试覆盖 / 不改语义的 bugfix
- [ ] **Mini-RFC** — 规范内 bugfix / 诊断措辞 / 内部开关 / 工具行为 / 规则文件级修改(先合 `rfcs/mini-NNNN-*.md`)
- [ ] **Full RFC** — 新语法 / 类型系统 / 运行时语义 / `unsafe` / FFI ABI / 内存模型映射 / 稳定化 / edition(先合 `rfcs/NNNN-*.md` + feature gate)

> 判档不清 → **向上取严**,取更严档位(自我约束建议);agent 可自判 Direct 并记录依据。

## 摘要

<!-- 这个 PR 做什么、为什么。语义改动请引用条款号 RXS-#### / RFC / deferred 编号。 -->

## Provenance / 验证 / 条款号(CI 阻断项)

- [ ] **Provenance**:每个 commit 含 `Assisted-by: <tool>:<model>` 或 `Co-Authored-By:` trailer(D-406)
- [ ] **条款号**:语义改动(`src/**/*.rs` / `spec/**/*.md`)引用 `RXS-####`(commit body / `//@ spec:` 注释 / 关联 `rfcs/*.md`),纯文档/纯测试可豁免
- [ ] **验证强制**:完成声明附 conformance / 单测命令的**真实输出**;数字来自命令输出,不凭记忆(硬规则 3/10)

## 提交前自检(贴真实输出)

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
py -3 ci/trace_matrix.py --check
py -3 ci/check_guardrails.py && py -3 ci/check_schemas.py && py -3 ci/check_structure.py
py -3 ci/check_contribution.py
```

<!-- 性能数据须遵循 milestones/m0/BENCH_PROTOCOL.md,证据落 evidence/(只增不删不改)。
     agent 完全自主:PR 备好绿由 agent 按栈序自主合并(无人工批准门)。 -->
