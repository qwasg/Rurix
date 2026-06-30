# G2.5 语言 1.0 + 首个 edition 取证报告（2026-06-30）

> 地位：G2.5 子里程碑（D-G2-5 / G-G2-5）端到端取证。agent 完全自主签署执行（AGENTS v3.0 硬规则 1），自主记录机器事实，所有数字来自真实命令输出（硬规则 3）。Provenance：`Assisted-by: cursor:claude-opus-4.8`。证据等级 measured_local（本机 host/编译期，edition 无 device）。evidence/ 只增不改。

## 0. 范围

语言 1.0 + 首个 edition 闭环：RFC-0008 edition/stabilization Full RFC → spec 全量条款化审计 + edition 条款 RXS-0177~0180 → edition 机制实现（rurix-pkg）→ conformance/edition 语料 + 全量 conformance → RD-008 stable API 快照冻结机制激活（stable 面定义 + 快照比对 + bless 守卫 + 首份 bless）→ CI 步骤 49 真实红绿 → close-out 预算门。**G2 契约整体仍 active**，不做任何 G2.6 动作。

## 1. RFC-0008 + spec 全量条款化 + edition 条款

- **RFC-0008**（[rfcs/0008-edition-stabilization.md](../../rfcs/0008-edition-stabilization.md)）：Full RFC，**Agent Approved 2026-06-30**（agent 完全自主，硬规则 1）。§9 裁决 Q-Name=`"2026"` / Q-Scope=仅机制锚点空差异集 / Q-Decl=`[package].edition` / Q-Default=缺省取首个 edition / Q-Mismatch=strict-only 拒 / Q-ErrCode=RX7020 / Q-File=新建 edition.md / Q-Range=4 条 / Q-RD008=激活 / Q-Stabilize=10 §5/§6/§2.2 FCP-lite。不触红线（D-008/SG-003 / SG-008 / D-312/SG-007 维持 not_triggered）、不触 🔒 禁区。RFC 编号台账 [rfcs/README.md](../../rfcs/README.md) §5 同步（补录 RFC-0007/0008，下一未用 RFC-0009）。
- **spec 全量条款化审计**：见 [spec_clausification_audit_20260630.md](spec_clausification_audit_20260630.md)——176 条款头 == 176 锚定条款（审计时基线），零裸条款头/零未锚定/零幽灵锚定/零重复定义；语言 1.0 既有语义面全量覆盖，**edition 为唯一新增语义面**。
- **edition 条款**：新建 [spec/edition.md](../../spec/edition.md) 落 **RXS-0177~0180**（FLS 体例，Syntax/Legality/Dynamic Semantics/Implementation Requirements，**严禁 UB 节**）：RXS-0177 声明语义（`[package].edition`，缺省 `2026`）/ RXS-0178 解析校验（合法集 `{ "2026" }` 确定性纯函数）/ RXS-0179 未知诊断（RX7020 strict-only，无 fallback）/ RXS-0180 stable 面与 edition 关系。[spec/README.md](../../spec/README.md) §4 加 edition.md 行 + §5 v1.51 修订行（只追加）。
- **错误码 + 双语**：[registry/error_codes.json](../../registry/error_codes.json) 追加 RX7020 `toolchain.edition_unknown`（7xxx 段续号接 RX7019，revision_log v1.27，只追加）+ en/zh message-key（[messages/en.messages](../../src/rurixc/src/messages/en.messages) / [zh.messages](../../src/rurixc/src/messages/zh.messages)）。

## 2. 验证命令逐条真实输出（仓库根，2026-06-30）

### 2.1 close-out 预算门 `py -3 ci/budget_eval.py --strict`（零 estimated）

```
  PASS m1.counter.spec_clause_test_anchoring: PASS — 180 条款全部 ≥1 测试锚定
  ...（69 项全 PASS，含全部 m0~m8/g1 bench/ratio/counter）...
  PASS m8.counter.bilingual_diagnostic_coverage: PASS — 1 份诊断双语全量覆盖证据(要求 ≥1)
[budget_eval] PASS (69 pass, 0 skip, strict mode)
```

- **全局零 estimated 残留**（strict mode，0 skip）；G2.5 不立性能门（edition 编译期/host，无 g2.bench.*/g2.ratio.*）。spec_clause_test_anchoring 由 176→**180**。

### 2.2 traceability `py -3 ci/trace_matrix.py --check`

```
[trace_matrix] PASS (180/180 clauses anchored, 453 test files scanned)
```

- 176 → **180/180 全锚定**（RXS-0177~0180 各 ≥1 `//@ spec` 锚定：`src/rurix-pkg/src/manifest.rs` 单测 + `src/rurix-pkg/tests/edition_corpus.rs`）。

### 2.3 schema / 双语 / 贡献门

```
[check_schemas] PASS
[bilingual] PASS 写 evidence\bilingual_diagnostic_coverage.json(coverage_complete=true,zh/en key 集对齐 88/88)
[check_contribution] ADVISORY(base=origin/main,14 commit 扫描,不阻断)
  — flagged 项为 §8.6 既有提交(701cef/c0e8730/db667/0c86647)缺 provenance/验证 trailer,
    非本任务引入(本任务改动未提交);agent 完全自主模式 advisory 不阻断(10 §7 v2.0)
```

- bilingual 87→**88/88**（新增 `toolchain.edition_unknown`）。

### 2.4 stable API 快照冻结机制（RD-008 激活）

```
# 首份 bless: RURIX_BLESS=1 py -3 ci/stable_snapshot.py
[stable_snapshot] BLESS 写 tests\stable\stable_api.snapshot(spec_clauses=180,error_codes=88,editions=['2026'],subcommands=['bench', 'build', 'check', 'doc', 'fmt', 'run', 'test', 'vendor'])
# 校验: py -3 ci/stable_snapshot.py --check
[stable_snapshot] PASS(stable 面与入库快照一致:spec_clauses=180,error_codes=88,editions=['2026'],subcommands=['bench', 'build', 'check', 'doc', 'fmt', 'run', 'test', 'vendor'])
```

- **stable 面定义**：spec RXS 条款 ID 全集（180）+ 错误码 ID/含义（88，message_key，含义冻结 10 §6）+ edition 合法值集（`["2026"]`）+ edition_anchor（`2026`）+ rx CLI 子命令面（8：bench/build/check/doc/fmt/run/test/vendor）。
- 机制：[ci/stable_snapshot.py](../../ci/stable_snapshot.py)（确定性重算 + 比对 + red 自检）+ [tests/stable/stable_api.snapshot](../../tests/stable/stable_api.snapshot) + [tests/stable/bless_log.md](../../tests/stable/bless_log.md) + `RURIX_BLESS=1` 路径 + [ci/check_guardrails.py](../../ci/check_guardrails.py) `check_stable_snapshot_bless` 守卫分支（镜像 UI/MIR/PTX/DXIL golden bless）。
- agent 自主 bless 首份快照（bless_log 2026-06-30 行）。**RD-008 status open→closed**（[registry/deferred.json](../../registry/deferred.json) revision_log v1.43）。🔒 快照仅锚定 stable 面存在性+含义，不冻结二进制 ABI（RXS-0180 L3）。

### 2.5 CI 步骤 49 真实红绿 `py -3 ci/edition_smoke.py`（host-only，无 device，不 SKIP）

```
[edition] OK edition_corpus (accept 解析 OK + reject RX7020/RX7005 strict-only 拦截)
[edition] OK edition unit tests (RXS-0177~0180)
[edition] OK stable snapshot --check (stable 面与入库快照一致)
[edition] OK red (篡改 stable 快照 → --check 翻红)
[edition] OK green-restored (复原 stable 快照 → --check 复绿,红绿闭合)
[edition] PASS (edition 解析/校验真实红绿:accept 解析 OK + 未知 edition RX7020 / 类型错误 RX7005 strict-only 拦截;stable API 快照 RD-008 激活:匹配 + 篡改红绿闭合)
```

- **green**：合法 `edition = "2026"` 接受 + 缺省兼容；**red**：未知 edition → RX7020 / 类型错误 → RX7005 strict-only 拒；篡改 stable 快照 → `--check` 翻红 → **复原绿（红绿闭合，反 YAML-only）**。
- CI 接线：[.github/workflows/pr-smoke.yml](../../.github/workflows/pr-smoke.yml) 步骤 49 `ci/edition_smoke.py`（参照步骤 45 host-only 形态）。[milestones/g2/CI_GATES.md](../../milestones/g2/CI_GATES.md) §7 v1.5 记录步骤 49 落地。

### 2.6 host 门（cargo，measured_local）

```
cargo fmt --check                                                         → exit 0
cargo clippy --all-targets --features "dxil-backend shader-stages" -D warnings → exit 0
cargo test -p rurix-pkg                          → 34 passed; 0 failed（含 edition 单测）
                                                  + edition_corpus 2 passed; 0 failed
cargo test -p rurixc --features "dxil-backend shader-stages" --lib        → 404 passed; 0 failed
cargo test -p rurixc --features "dxil-backend shader-stages" --test dxil_corpus  → 7 passed; 0 failed
cargo test -p rurixc --features "dxil-backend shader-stages" --test dxil_golden  → 5 passed; 0 failed (+1 ignored)
cargo test --workspace（全量 conformance）        → 全部 test result: ok; exit 0（零 failed）
py -3 ci/check_guardrails.py（base=g1-closed）     → exit 0 ADVISORY（flagged 均为 G2 分支 vs g1-closed
                                                    既有差异 + 自再生 bilingual 证据，本任务 append 项
                                                    RXS-0177~0180/RX7020/RD-008/CI_GATES 均未标红）
```

- **edition 机制实现**：[src/rurix-pkg/src/manifest.rs](../../src/rurix-pkg/src/manifest.rs) 增 `Edition` 枚举 + `Edition::parse` 确定性纯函数 + `Manifest.edition`（缺省 `Edition2026`）+ 未知 → `PkgError::EditionUnknown`（RX7020）+ edition-gated 分发锚点 `behavior_differs`（首期空集）；[src/rurix-pkg/src/error.rs](../../src/rurix-pkg/src/error.rs) 增 `EditionUnknown` 变体（code RX7020）。**纯 host/safe，零新 unsafe**（无需 U25）；全 crate 维持 `unsafe_code=deny`。
- **conformance/edition 语料**：[conformance/edition/accept](../../conformance/edition/)（edition_2026 / edition_default）+ [conformance/edition/reject](../../conformance/edition/)（unknown_2099_rx7020 / unknown_latest_rx7020 / type_error_int_rx7005），由 `src/rurix-pkg/tests/edition_corpus.rs` 消费断言。

## 3. unsafe 纪律（硬规则 9）

edition 机制为编译期/host 工具链声明语义，纯 host/safe，**零新 unsafe**；rurix-pkg 维持 `unsafe_code=deny`，无需 unsafe-audit 续号（U25 不消费）。

## 4. CI run URL（诚实标注，不伪造）

- edition 步骤 49 为**编译期/host 面，无 device**。本机 `ci/edition_smoke.py` PASS（红绿闭合，§2.5）+ 全量 host 门绿（§2.1~§2.6），measured_local。
- **CI run URL**：本会话未触发 self-hosted runner（`rurix-dev-4070ti`）/ GitHub Actions；步骤 49 为纯 host，本机真实红绿已兑现。CI run URL 待 self-hosted/GitHub Actions runner 上线时回填（对齐步骤 45 host-only 先例：CI 未及执行的 host 步骤以本机真实红绿为准，不伪造 run URL、不声称未真跑的 CI green）。

## 5. 判定

D-G2-5 / G-G2-5 验收要件本机闭环（host/编译期 measured_local）：RFC-0008 Full RFC Approved + spec 全量条款化审计 + edition 条款 RXS-0177~0180 带编号条款体 + 每条 ≥1 锚定（trace 180/180）+ 双语对齐（88/88）+ edition 机制实现 + conformance/edition + 全量 conformance 绿 + RD-008 stable API 快照冻结机制激活（stable 面定义 + 快照比对 + bless 守卫 + 首份 bless，open→closed）+ CI 步骤 49 接线 + edition_smoke 真实红绿闭合 + budget_eval --strict 零 estimated。**G2 契约整体仍 `active`**——不执行 `g2-closed` tag / 基准切换（g1-closed→g2-closed）/ 契约 status active→closed / G2 整体 close-out / RD-007·RD-009 翻转（均属 G2.6，本任务范围外；本任务仅 RD-008 翻 closed）。
