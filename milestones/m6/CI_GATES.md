# M6 CI 门禁增量

> 所属契约:[M6_CONTRACT.md](M6_CONTRACT.md)
> 版本:v1.0(2026-06-15)
> 基线:[../m0/CI_GATES.md](../m0/CI_GATES.md) + [../m1/CI_GATES.md](../m1/CI_GATES.md) + [../m2/CI_GATES.md](../m2/CI_GATES.md) + [../m3/CI_GATES.md](../m3/CI_GATES.md) + [../m4/CI_GATES.md](../m4/CI_GATES.md) + [../m5/CI_GATES.md](../m5/CI_GATES.md)(全部沿用:runner 约定、PR Smoke 1–24 步、guardrail 含 M1.1/M1.2/M1.4/M3.3/M4.2/M4.3/M5.4 激活项、nightly 工作流含 Compute Sanitizer racecheck+memcheck + measured 基准);本文只规定 M6 期的**增量**。
> 铁律不变:任何新增门禁必须在真实 PR 上以真实失败/通过路径验证过(反 YAML-only,H06 D11.8-2)。

---

## 1. Runner

沿用 M0 §1(自托管 RTX 4070 Ti 开发机)+ M4 §1(device 路径门禁:CUDA Toolkit 含 `ptxas` + Driver API 装载环境)+ M5 §1(Compute Sanitizer + libdevice bc 探测)。M6 新增 runner 预置项(随实现落地时本表修订行留痕):

- **离线重建复现**(G-M6-1)需"干净环境"——清 vendor 缓存外部源后 `rx build --locked --offline`;CPU-only,不占 GPU 队列。
- **LSP 交互延迟实测**(G-M6-2)在 RTX 4070 Ti 开发机采(墙钟交互延迟主指标),沿用 BENCH_PROTOCOL §2 环境画像/进程隔离纪律;LSP server 为 CPU 路径,但与 GPU 基准互斥队列纪律沿用避免互扰。
- **VS Code 扩展**联调为人工/半自动,不入 PR Smoke 必经门(扩展打包随发布链路 RD-001/M8)。

## 2. PR Smoke 追加步骤(编号接 M5 §2 的 22–24)

| # | 步骤 | 失败即红 |
|---|---|---|
| 25 | rx CLI 核心子命令冒烟:build/run/check/test/fmt/bench 在样例工程端到端真跑(契约 G-M6-3 通道;M6.1/M6.3 落地接入)。**实测命令(M6.3 回填)**:`py -3 ci/rx_cli_smoke.py`(rx build/run/check/test + fmt 收编 + bench --smoke 在 `conformance/toolchain/*.rx` + `conformance/syntax/hello_world.rx` 样例上端到端真跑,失败即非零退出,写唯一 `evidence/rx_cli_smoke_*.json`)+ `cargo test -p rx`(rx 子命令分发/退出码集成测试);计数核对 `py -3 ci/budget_eval.py`(`m6.counter.rx_cli_core_subcommands ≥6`,计数源 = `evidence/rx_cli_smoke_*.json` 的 `subcommands_passed` 去重基数)。**M6.3 纳入 rx test 后达 6/6 → PASS** | 是 |
| 26 | rx fmt 幂等门延续(契约 G-M6-4,RD-005 收编):`py -3 ci/check_fmt_idempotent.py`(经 rx fmt 收编后)对全 `conformance/syntax` 语料二次格式化 0 diff(复用 M1 既有幂等机制,M6.1 收编后路由到 `rx fmt --check-idempotent`;雏形 `rx_fmt` 二进制退役)。`tests/ui` 含词法错误样例(format 源不洁)不入幂等门语料 | 是 |
| 27a | 包管理 manifest/lock/vendor 离线解析门(契约 D-M6-2;M6.2 落地接入,CPU-only,GPU 队列无关):`conformance/pkg` 样例 workspace 经 `rx vendor --offline` 三来源(path)解析 + 写 `rurix.lock`(内容树 SHA-256)+ `rx vendor --locked --offline` 校验。**实测命令(M6.2 回填)**:`py -3 ci/pkg_resolve_smoke.py`(离线解析 + lock 逐字节确定性 + 内容树 digest 红绿:篡改 vendor → RX7008 红、篡改 lock → RX7007 红、复原转绿、path 源缺失 → RX7009)+ `cargo test -p rurix-pkg`(manifest/sha256/content_tree/resolve/lock/vendor 单测)+ `cargo test -p rx`(rx vendor 红绿集成)。门为 check_* 守卫风格(不写 evidence、不新增 budget counter);失败即红(反 YAML-only) | 是 |
| 27 | 三包 workspace 离线重建逐字节可复现门(契约 G-M6-1;M6.3 落地接入,GPU 队列无关 CPU-only):`rx build --locked --offline` 干净环境两次重建 host EXE SHA-256 逐字节一致 + `rurix.lock` 与 `vendor/` 哈希稳定。**实测命令(M6.3 回填)**:`py -3 ci/offline_rebuild_repro.py`(复制 `conformance/workspace/repro` 到干净临时目录,同一路径清输出后两次 build,比较 EXE/lock/vendor SHA-256;临时篡改 `vendor/pathdep/src/lib.rx` → RX7008 红 → 复原转绿,应红却绿即脚本 FAIL,反 YAML-only)+ `cargo test -p rurix-pkg`(workspace members 注入 + lock/vendor 单测)+ `cargo test -p rx`(rx build/test 集成)。计数核对 `py -3 ci/budget_eval.py`(`m6.counter.offline_rebuild_reproducible ≥1`,计数源 = `evidence/offline_rebuild_*.json`) | 是 |
| 28 | LSP 能力面冒烟(契约 G-M6-2/G-M6-5 通道;M6.4 落地接入,CPU-only):`rurixc --tooling-server` 六项 MVP 能力(诊断/补全/跳转/引用/高亮/重命名)往返冒烟。**实测命令(M6.4 回填)**:`py -3 ci/lsp_smoke.py`(`cargo build -p rurixc` 后先以 `rurixc --tooling-smoke conformance/toolchain/lsp_mvp/sample.rx` 验证六项能力,再真跑 `rurixc --tooling-server` stdio JSON-RPC 往返验证 initialize/completion/definition/documentHighlight/rename 失败 RX7012 诊断;内建 `RURIX_LSP_SMOKE_EXPECT_COMPLETION` 篡改红绿;写 `evidence/lsp_smoke_*.json`)+ `cargo test -p rurixc tooling::lsp::tests`。计数核对 `py -3 ci/budget_eval.py`(`m6.counter.lsp_capabilities ≥5`,计数源 = `evidence/lsp_smoke_*.json` 的 `capabilities_passed` 去重基数) | 是 |

预算 evaluator(M0 步骤 6)自动合并加载 [m6_budget.json](m6_budget.json)(命名空间冲突即红;evaluator 已配 `m6.counter.rx_cli_core_subcommands`/`m6.counter.offline_rebuild_reproducible`/`m6.counter.lsp_capabilities` 分支,目录/证据缺失 → 0 → normal SKIP,对齐 M4/M5 计数器先例)。**M6 期 PR Smoke 跑 normal 模式**:`m6.counter.*` 建设期未达标 SKIP 属预期;`m6.bench.lsp_interaction_latency_ms` estimated 占位在 M6.5 回填前继续 SKIP。**M6 close-out 必须跑 `--strict` 且全局零 estimated 残留**(契约 G-M6-2;本占位在 M6 内生灭,不跨里程碑欠债,14 §3)。

## 3. Nightly 追加

- 既有 nightly 全保留(lexer/parser/SAXPY 冒烟 + budget normal + self-profile 归档 + **Compute Sanitizer racecheck+memcheck**(M5.4 激活)+ **gpu 并行基元 measured 基准**(M5.3/M5.4))。
- **rx test GPU 子进程隔离**(M6.3 落地):device 测试经 rx test 子进程隔离纳入既有 nightly 全跑(崩溃不连坐 harness,14 §6)。
- **rx bench 收编后**(RD-003,M6.1):L1/L2 measured 基准经 `rx bench` 入口跑(协议不变,BENCH_PROTOCOL §3),既有 measured_local 证据口径不变。
- **LSP 交互延迟趋势**(M6.5):10k 行样例工程交互延迟纳入 nightly 趋势归档(门禁判定在 close-out `--strict`,nightly 为趋势参考)。
- self-profile 归档自然覆盖 M6 新增阶段计数器(query 层 server 模式 / 包解析布点随实现扩列,非门禁,趋势参考)。
- Release 层仍不建(RD-001,承接 M8 不变)。

## 4. Guardrail

沿用 M0 五项 + M1 三项(spec 档位 / 错误码冻结 / UI bless)+ M3 一项(MIR golden bless,check_mir_bless)+ M4 一项(PTX/IR golden bless,check_ptx_bless)+ M4 unsafe-audit(rurix-rt `undocumented_unsafe_blocks=deny`)+ M5 一项(NVIDIA 再分发白名单审计 check_redistribution / Compute Sanitizer nightly)。M6 期动作:

1. **基准 ref 维持 `m5-closed`**:M5 close-out 已完成 `m4-closed → m5-closed` 切换(M5 CI_GATES §6 / M5_CONTRACT §8.10),`ci/check_guardrails.py` 无参默认 = `m5-closed`,**M6 开工无需再切**(与 m4-closed 随 M5 开工切换的旧节奏不同);PR 路径仍以 `GITHUB_BASE_REF` 为准。若 M6 期需再切按 `check_*` 守卫风格 + 双基准核对,留痕本表修订行。
2. **新段位错误码首批分配**(rx CLI/包管理/LSP 工具链诊断):随 M6.1+ 诊断 PR 留痕,段位按 07 §5 语义分配(工具链类归 7xxx 续接或经裁决新段位),分配制递增、含义冻结(10 §6,`check_error_codes` 既有冻结机制延续)。**开工脚手架不预造错误码**(无实现即无诊断可锚)。
3. **rx fmt 幂等门延续**(G-M6-4,RD-005 收编):`ci/check_fmt_idempotent.py` 既有幂等机制延续到 rx fmt 收编后,格式行为变更须经审查(防风格漂移)。
4. **rx test GPU 子进程隔离纳入既有 nightly**(M6.3):激活经真实验证(构造崩溃 kernel 测试 → 子进程隔离不连坐 harness)。
5. **新增工具链 crate** 默认 `unsafe_code=deny`(全仓纪律延续,rx CLI/包管理/LSP server 新 crate 纳入 unsafe-audit 扫描;若 LSP/FFI 引入 unsafe 边界按 AGENTS 硬规则 9 注册条目,每 unsafe 块 `// SAFETY:`)。

14 §2 常驻集其余项的 M6 期评估结论:

| 项 | 结论 |
|---|---|
| MIR/PTX/IR 文本 golden | M3.3/M4.2 已激活;M6 工具链不改 codegen 形态,沿用既有 golden 核对 |
| stable API 快照 | M6 无 stable 面(rx CLI/包管理/LSP 均 MVP 演进中);stable 面冻结随 MVP 收口(M8)评估激活 |
| unsafe-audit 完整性 | M4.3 已激活(rurix-rt);M6 新工具链 crate 维持 `unsafe_code=deny`,新增 unsafe 边界按硬规则 9 注册 |
| Compute Sanitizer | M5.4 已激活;M6 rx test 子进程隔离落地后 device 测试纳入既有 nightly 全跑 |
| NVIDIA 再分发白名单审计 | M5.4 formal 激活(事实层,`check_redistribution`);M6 维持 PTX-only 开发期产物,再分发面持续为空;真分发(G1 cubin/fatbin)的逐项法律核对随首个分发产物 formal 签署不变 |
| registry sumdb(D-312) | M6 不做 registry;SG registry 方向触发条件(D-312)未满足,维持 not_triggered(M6 期如评估则追加 spike_gating decisions) |

m0~m5 历史预算的回填/冻结走 `check_guardrails.py` 既有机制(measured_local 条目 0-byte;estimated 只允许回填为 measured_local),不属新增激活项。

## 5. 验证程序(对应契约 G-M6-1/G-M6-2/G-M6-3/G-M6-4 与步骤 25–28)

1. 步骤 27(离线重建复现)落地后,构造**篡改一个内容树 digest** 的 PR → 重建/校验必须红;复原后转绿,run URL 归档(反 YAML-only)。
2. 步骤 25(rx CLI 子命令)落地后,构造子命令端到端失败(篡改样例工程使 rx build/run 失败)→ exit 非零红;复原 → 绿,run URL 归档。
3. 步骤 26(rx fmt 幂等)落地后,构造二次格式化 diff(故意非幂等改动)→ `check_fmt_idempotent` 红;修复 → 绿,run URL 归档。
4. 步骤 28(LSP 能力面)落地后,构造能力往返失败(篡改诊断/补全预期)→ 红;复原 → 绿,run URL 归档。
5. 步骤 27a(包管理 manifest/lock/vendor,M6.2 落地)已具真实红绿:`ci/pkg_resolve_smoke.py` 在 `conformance/pkg` 临时副本上 `rx vendor --offline` 写 lock+vendor 后,篡改一个 vendor 内容树文件 → `rx vendor --locked` 校验红(RX7008 digest 不符)→ 复原转绿;篡改 `rurix.lock` content_sha256 → 红(RX7007 不一致)→ 复原转绿;门自含红绿断言(应红却绿即整体 FAIL),包管理逻辑被破坏时 CI 必红(反 YAML-only,D11.8-2)。PR 上另构造样例清单破坏(如 `build="bad"` / feature 引用不存在)使 27a 红 → 修复转绿,run URL 归档。
6. close-out 附 `budget_eval --strict` 输出原文(契约 G-M6-2 LSP 延迟 measured_local 与全局零 estimated 残留判定)+ 离线重建复现证据路径 + 离线重建红绿 run URL + RD-003/RD-004/RD-005 收编/处置 close 留痕。

## 6. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-06-15 | 初版(M6 契约配套;步骤 25–28 为 M6.1/M6.3/M6.4 计划项,落地时回填实测命令;guardrail 动作:基准 ref 维持 m5-closed 无需再切、新段位错误码首批分配随 M6.1+ 诊断 PR、rx fmt 幂等门延续、rx test 子进程隔离纳入 nightly 均为计划项)。配套 `ci/budget_eval.py` 新增 `m6.counter.rx_cli_core_subcommands`/`m6.counter.offline_rebuild_reproducible`/`m6.counter.lsp_capabilities` evaluator 分支(目录/证据缺失 → 0 → normal SKIP,对齐 M4/M5 计数器先例);`m6_budget.json` 含三计数器 + `m6.bench.lsp_interaction_latency_ms` estimated 占位(M6.5 回填)。`py -3 ci/budget_eval.py`(normal)= PASS(m6.* 占位 SKIP 属预期) |
| v1.1 | 2026-06-15 | M6.1 回填步骤 25/26 实测命令(rx CLI 落地):步骤 25 = `py -3 ci/rx_cli_smoke.py`(build/run/check/fmt/bench 端到端真跑,写 `evidence/rx_cli_smoke_*.json`)+ `cargo test -p rx`;步骤 26 = `py -3 ci/check_fmt_idempotent.py`(收编后路由 `rx fmt --check-idempotent`,108 文件 byte-exact)。`m6.counter.rx_cli_core_subcommands` evaluator 计数源回填(`subcommands_passed` 去重基数;M6.1 = 5/6,rx test 待 M6.3 → normal SKIP 属预期)。新段位错误码首批分配 RX7003/RX7004(7xxx 续接,registry revision_log v1.16 留痕);新增 `milestones/m6/rx_cli_smoke_evidence_schema.json` + `ci/check_schemas.py` 路由(`rx_cli_smoke_` 前缀)。pr-smoke.yml 接入步骤 25/26(真实红绿验证见 §5 第 2/3 条) |
| v1.2 | 2026-06-15 | M6.2 回填步骤 27a 实测命令(包管理 manifest/lock/vendor 落地):新增步骤 27a = `py -3 ci/pkg_resolve_smoke.py`(`conformance/pkg` 样例离线解析 + `rurix.lock` 逐字节确定性 + 内容树 SHA-256 digest 红绿:篡改 vendor → RX7008、篡改 lock → RX7007、复原转绿、path 源缺失 → RX7009)。新 crate `src/rurix-pkg`(零依赖,手写 TOML 子集解析 + 手写 SHA-256,unsafe_code=deny 继承)纳入 `cargo test --workspace` / `cargo clippy --workspace`。新段位错误码续接分配 RX7005~RX7009(7xxx,registry revision_log v1.17 留痕)+ message-key `toolchain.pkg_*`(en.messages)。`ci/trace_matrix.py` 补扫 `src/rurix-pkg/**/*.rs`(条款锚定 94/94)。pr-smoke.yml 接入步骤 27a(真实红绿验证见 §5 第 6 条)。步骤 27(三包逐字节复现门 + `m6.counter.offline_rebuild_reproducible ≥1`)仍归 M6.3(G-M6-1);registry sumdb D-312 不触碰维持 not_triggered |
| v1.3 | 2026-06-15 | M6.3 回填步骤 25/27 实测命令(rx test + G-M6-1 离线重建复现门落地):步骤 25 纳入 `rx test conformance/toolchain/rx_test_basic.rx` 后 `m6.counter.rx_cli_core_subcommands` 达 6/6;步骤 27 = `py -3 ci/offline_rebuild_repro.py`(`conformance/workspace/repro` 三包 workspace path/git/archive 三来源,两次 `rx build --manifest-path ... --locked --offline` host EXE SHA-256 一致,lock/vendor 不改写,并内建 `vendor/pathdep/src/lib.rx` 篡改 → RX7008 红 → 复原绿)。新增 `milestones/m6/offline_rebuild_evidence_schema.json` + `ci/check_schemas.py` 路由(`offline_rebuild_` 前缀);新段位错误码续接分配 RX7010/RX7011(7xxx,registry revision_log v1.18 留痕)+ message-key `toolchain.rx_test_*`。nightly.yml 接入 `rx test ... --gpu` 子进程隔离 smoke,Compute Sanitizer 全量路径保留。真实 PR 红绿 run URL 待本 PR 验证后追加到 PR 描述与 close-out |
| v1.4 | 2026-06-15 | M6.3 G-M6-1 真实 PR 红绿 run URL 回填(#34,base main):红验证 commit `7f21554` 临时篡改 `conformance/workspace/repro/vendor/pathdep/src/lib.rx`,pr-smoke run [27528818548](https://github.com/qwasg/Rurix/actions/runs/27528818548) 在 `offline rebuild reproducibility gate (M6 CI_GATES §2.27,G-M6-1)` 失败,日志含 `RX7008` 内容树 digest mismatch;绿验证 commit `324e8f6` 用普通 revert 复原 fixture,pr-smoke run [27529042970](https://github.com/qwasg/Rurix/actions/runs/27529042970) success,step 24 `offline rebuild reproducibility gate` 通过。#34 已 retarget 到 `main`,普通 merge/revert 历史保留,未使用 reset/rebase/force-push |
| v1.5 | 2026-06-15 | M6.4 回填步骤 28 实测命令(LSP MVP + RD-004 接通):`py -3 ci/lsp_smoke.py`(`rurixc --tooling-smoke` 六项能力 + 内建 `RURIX_LSP_SMOKE_EXPECT_COMPLETION` 篡改红绿;`evidence/lsp_smoke_*.json`)+ `cargo test -p rurixc tooling::lsp::tests`;新增 `milestones/m6/lsp_smoke_evidence_schema.json` + `ci/check_schemas.py` 路由(`lsp_smoke_` 前缀);RD-004 inherited→closed(M6.4 无损语法树通道 `src/rurixc/src/lossless.rs`);RX7012(7xxx LSP rename 诊断)分配;pr-smoke.yml 接入步骤 28。真实 PR 红绿 run URL 待本 PR 验证后追加 |
| v1.6 | 2026-06-15 | M6.4 code-review 收口:步骤 28 从 smoke-only 扩展为 `--tooling-smoke` + `--tooling-server` stdio JSON-RPC 双路径;server 往返覆盖标准嵌套 `textDocument` params、DocumentHighlight 对象形状、completion kind、rename 失败空 WorkspaceEdit + RX7012 publishDiagnostics。pr-smoke 步骤 28 同步执行 `cargo test -p rurixc tooling::lsp::tests`;traceability 补齐 RXS-0098~0103 的 `ci/lsp_smoke.py`/`lsp.rs` 锚点。真实 PR 红绿 run URL 待本 PR 验证后追加 |
| v1.7 | 2026-06-15 | M6.4 步骤 28 真实 PR 红绿 run URL 回填 + clippy/门禁修复(#35,base main):(1) 修复 `src/rurixc/src/tooling/lsp.rs` 4 处 clippy lint(L61 `needless_as_bytes`→`json.len()`、L147/L153 `redundant_closure`→直接传 `definition_at`/`references_at`、L240 `manual_strip`→`strip_prefix('"')`),不用 `#[allow]`;commit `8a9930e` 使 pr-smoke 转绿 run [27533874687](https://github.com/qwasg/Rurix/actions/runs/27533874687)(step 28 `LSP capabilities smoke` 通过)。(2) 红绿验证暴露 step 28 多行 PowerShell `run:` 块只取末条 `cargo test` 退出码、吞掉 `py -3 ci/lsp_smoke.py` 非零退出的 YAML-only 漏洞:commit `92e713c` 在 step 28 每条命令后补 `if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }` 强制透传。(3) 红验证 commit `dc4b867` 临时篡改 `initialize` 通告(移除 `definitionProvider`),叠加门禁修复后 pr-smoke run [27534290155](https://github.com/qwasg/Rurix/actions/runs/27534290155) 在 `LSP capabilities smoke (M6 CI_GATES §2.28,G-M6-2)` 失败,日志含 `[lsp_smoke] FAIL: initialize missing definitionProvider`(前序 clippy/`cargo test` 全绿,红精准落在 step 28)。(4) 绿验证 commit `b794ad3` 用普通 `git revert` 复原通告,pr-smoke run [27534478648](https://github.com/qwasg/Rurix/actions/runs/27534478648) success,step 28 通过。#35 base `main`,普通 commit/revert 历史保留,未使用 reset/rebase/force-push;落实 v1.5/v1.6 "真实 PR 红绿 run URL 待本 PR 验证后追加" |
