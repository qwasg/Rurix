---
contract: M6
title: 工具链与包管理——rx CLI / 包管理 / LSP MVP
status: active            # active → closed(close-out 只追加,既有条款 0-byte 修改)
version: v1.0
date: 2026-06-15
timebox: "M+11 ~ M+13(约 8 周,两级结构见 M6_PLAN.md)"
rfc_required: none        # rx CLI 子命令面 / 包管理 manifest·lock·vendor 格式 / LSP 能力面是对 07/08/09/13 已锁定决策(D-203/D-210/D-230·D-239·D-240/D-241/D-308~D-312)的条款化与工程实现:纯追加、尚无 stable 面;任何偏离已锁定决策的语义动作(尤其 registry sumdb D-312 / build.rs 逃生舱)按 10 §3 升档,判档争议向上取严
upstream_docs:
  - "11 §3 (M6 定义,工具链与包管理)"
  - "07 §2 §6 §9 (查询化与增量编译 D-203 / 编译性能预算 / LSP 与工具模式 D-210——单一前端,常驻 query 层服务 LSP)"
  - "08 §4 §6 §7 §8 (rx bench harness 工具化 / 热重载 rx watch / 开发者工具集 rx CLI D-239 / IDE 集成 VS Code 优先 D-240)"
  - "09 §7 (包管理与供应链 D-308~D-312:rurix.toml 意图 + rurix.lock 解析图 + vendor + SHA-256;path/git/archive 三来源;无 build.rs 声明式;workspace 单根锁)"
  - "14 (契约/预算/deferred/证据分级/测试纪律/基准协议)"
in_scope:
  - rx_cli_core             # rx CLI 总入口 + 核心子命令(build/run/check/test/bench/fmt/doc/fix/watch/vendor 的 MVP 核心集,08 §7 D-239);单一前端经 rurixc query 层
  - pkg_manifest_lock_vendor # 包管理:rurix.toml(意图)+ rurix.lock(精确解析图 + 内容树 SHA-256)+ 可提交 vendor/ + 默认离线可重建;依赖三来源 path/git/archive;workspace 单根锁(09 §7.1/7.2)
  - rx_test_gpu_isolation   # rx test 内建 #[test] + GPU 测试自动子进程隔离选项(H03 §6 纪律工具化,14 §6;08 §7)
  - lsp_mvp_vscode          # LSP MVP(publishDiagnostics/completion/definition+references/highlight/rename,07 §9)经 rurixc --tooling-server 常驻 query 层 + VS Code 扩展(LSP 客户端 + 语法高亮,08 §8 D-240)
  - natvis_first            # Natvis 首批(标准库 Buffer/View/Vec/Mat 可视化,08 §5)
  - spec_m6_clauses         # spec rx CLI 子命令语义面 / 包管理清单·lock 格式 / LSP 能力面条款(spec/toolchain.md,RXS-0083 续号,规范先行;条款 PR 先于实现 PR)
out_of_scope:
  - cubin_fatbin_dist       # libdevice 真分发 / 生产分发 fatbin(按架构 cubin + PTX fallback)→ G1(07 §7 / RD-001 系);M6 维持 PTX-only 开发期产物
  - scoped_atomics_mapping  # scoped atomics + PTX atom.{order}.{scope} 映射层(D-406 禁区,人工落笔)已于 M5 条款化,M6 不扩
  - stdlib_math_full        # core 数学库定型(Vec/Mat/swizzle/几何原语)→ M7(11 §3 M7)
  - release_chain           # 发布链路(rurixup/MSI/winget + 签名/SBOM/许可审计 + artifact 上传)→ RD-001/M8(08 §9 / 11 §3 M8)
  - registry_sumdb          # registry(sparse index + sumdb 透明日志 + OIDC/Sigstore)→ 所有者决策点 D-312(09 §7.3 阶段三;SG registry 触发条件未满足)
  - build_rs_escape_hatch   # 任意构建脚本 / 受限 runner 逃生舱 → 后置评估(09 §7.1:MVP 无 build.rs,build.model="declarative")
  - const_generic_value_mono # const 泛型值运行期单态化(RD-007)随 device codegen 进一步扩展评估接通,非本契约验收门,执行期处置留痕
  - advanced_gpu_intrinsics # Tensor Core/WGMMA/TMA / cluster / 动态并行 / cooperative groups 永久裁剪(11 §2 红线,SG-001~SG-009 维持 not_triggered)
deferred_refs: [RD-003, RD-004, RD-005, RD-007]   # RD-003(rx bench 工具化,owner M6,M6 开工承接)+ RD-005(rx fmt 完整工具化,owner M6,M6 开工承接)+ RD-004(无损语法树/LSP 增量通道,owner M6,M6 开工承接评估)+ RD-007(const 泛型值运行期单态化,owner M5→M6 顺延,inherited;非本契约验收门);M6 不预造新 deferred,执行期按需登记 RD-###(14 §4)
deliverables:
  - id: D-M6-1
    name: rx CLI 总入口 + 核心子命令骨架(build/run/check)+ rx fmt 收编(RD-005)+ rx bench 收编(RD-003);单一前端经 rurixc query 层 + spec 条款先行(rx CLI 语义面)
  - id: D-M6-2
    name: 包管理 rurix.toml + rurix.lock + vendor/ + SHA-256 内容树校验,path/git/archive 三来源,workspace 单根锁 + spec 条款(manifest/lock 格式)
  - id: D-M6-3
    name: rx test 内建 #[test] + GPU 子进程隔离选项(14 §6)+ workspace 多包 + 三包离线重建逐字节可复现门(G-M6-1)
  - id: D-M6-4
    name: LSP MVP(publishDiagnostics/completion/definition+references/highlight/rename,07 §9)经常驻 query 层 + 无损语法树通道评估(RD-004)+ spec 条款(LSP 能力面)
  - id: D-M6-5
    name: VS Code 扩展(LSP 客户端 + 语法高亮)+ Natvis 首批 + LSP 10k 行交互延迟预算实测(G-M6-2,measured_local)
  - id: D-M6-6
    name: 工具链 conformance / 诊断面延续(rx CLI 子命令端到端 + 包管理离线复现 + LSP 能力面测试)+ traceability 延续(G-M6-5)
acceptance_gates:
  - id: G-M6-1
    check: "三包 workspace 离线重建逐字节可复现:path + git + archive 三来源各 ≥1 包的 workspace,经 rx build --locked --offline 在干净环境(清 vendor 缓存外部源)重建,两次重建产物逐字节一致(content SHA-256 比对),rurix.lock 解析图稳定;CI 批跑断言计数 m6.counter.offline_rebuild_reproducible ≥1 份可复现证据(11 §3 M6 验收门 / 09 §7.1)。激活经真实红绿验证(篡改一个内容树 digest → 重建失败/校验红 → 复原转绿,run URL 归档,反 YAML-only)"
  - id: G-M6-2
    check: "LSP 在 10k 行样例工程交互延迟达标(预算项实测):rurixc --tooling-server 常驻 query 层,10k 行样例工程上 completion / publishDiagnostics(保存后) / definition 的交互延迟,采样按 BENCH_PROTOCOL.md 协议化(墙钟交互延迟为主指标,instructions:u 趋势参考);预算断言 m6.bench.lsp_interaction_latency_ms direction=max,evidence_level=measured_local,close-out 跑 budget_eval --strict 通过(本占位在 M6 内生灭,不跨里程碑欠债,14 §3)。阈值 estimated → measured_local 于 M6.5 回填"
  - id: G-M6-3
    check: "rx CLI 核心子命令端到端:build/run/check/test/fmt/bench 在样例工程上端到端真跑成功(rx build 产 EXE、rx run 执行、rx check 仅前端、rx test 含 GPU 子进程隔离、rx fmt 幂等、rx bench 经 BENCH_PROTOCOL harness);核心子命令覆盖计数 m6.counter.rx_cli_core_subcommands ≥ 预设核心集数。CI 批跑(子命令冒烟步骤),失败即红"
  - id: G-M6-4
    check: "rx fmt 幂等承诺(RD-005 收编)+ rx bench 工具化(RD-003 收编):rx fmt 对全 conformance/tests 语料二次格式化 0 diff(check_fmt_idempotent 既有机制延续到 rx fmt 收编后);rx bench 复用 BENCH_PROTOCOL §3 协议(L0 锁频前置 / 三次进程级独立运行 / trimmed mean),M5 bench harness 脚本经 rx bench 收编后退役并在 RD-003/RD-005 close 时留痕"
  - id: G-M6-5
    check: "traceability 延续:M6 新增 RXS 条款(rx CLI 子命令语义面 / 包管理 manifest·lock 格式 / LSP 能力面,spec/toolchain.md RXS-0083 续号)每条 ≥1 测试锚定(ci/trace_matrix.py 全局口径,沿用 m1.counter.spec_clause_test_anchoring)"
guardrails:
  - "milestones/m0~m5 的 measured_local 既有预算条目 git diff 0-byte(新增条目允许)"
  - "milestones/m0~m5 的 M*_CONTRACT.md(均 closed)既有内容只追加不修改"
  - "registry/deferred.json 与 registry/spike_gating.json 只追加(既有条目修改触发人工审查);RD-003/RD-004/RD-005 仅允许 open→inherited/closed、RD-007 仅允许 inherited→closed 的状态留痕追加;SG 复评只追加 decisions"
  - "registry/error_codes.json 错误码语义可加不可改(M1.1 已激活);M6 新段位(rx CLI/包管理/LSP 工具链诊断)首批分配随 M6.1+ 诊断 PR 留痕,段位分配制递增、含义冻结"
  - "evidence/ 只增不删不改"
  - "00–14 共 15 份规划文档不被执行 PR 改写(勘误走 00 §6.3 追加式修订)"
  - "tests/ui/ 的 .stderr snapshot 变更必须经审批 bless(M1.4 已激活,check_ui_bless)"
  - "tests/mir/ 的 .mir golden 变更必须经审批 bless(M3.3 WP6 已激活,check_mir_bless)"
  - "tests/ptx/ 的 IR golden 变更必须经审批 bless(M4.2 已激活,check_ptx_bless)"
  - "spec/ 变更必须携带变更档位标记(M1.2 已激活);spec/toolchain.md 新增条款 PR 先于实现 PR,每条 ≥1 测试锚定(G-M6-5)"
  - "src/rurix-rt 的 unsafe 边界维持 undocumented_unsafe_blocks=deny(M4.3 已激活,每 unsafe 块 // SAFETY:);全仓其余 crate 维持 unsafe_code=deny;rx CLI/包管理/LSP 新 crate 默认 unsafe_code=deny"
  - "guardrail 核对基准维持 m5-closed(M5 close-out 已完成 m4-closed→m5-closed 切换,M6 开工无需再切;PR 路径仍以 GITHUB_BASE_REF 为准);若 M6 期需再切按 check_* 守卫风格 + 双基准核对"
  - "Compute Sanitizer racecheck+memcheck nightly 维持全绿(M5.4 已激活);rx test GPU 子进程隔离落地后 device 测试纳入既有 nightly 全跑"
  - "本契约 in_scope/acceptance_gates 等既有条款 0-byte 修改,close-out 只追加"
---

# M6 契约 — 工具链与包管理(rx CLI / 包管理 / LSP MVP)

> 所属:[../../11_ROADMAP.md](../../11_ROADMAP.md) §3 M6 / 契约机制见 [../../14_ENGINEERING_DISCIPLINE.md](../../14_ENGINEERING_DISCIPLINE.md) §1
> 规范先行延续(AGENTS.md 硬规则第 7 条):rx CLI / 包管理 / LSP 的语义面 PR 必须引用 RXS-#### 条款号(`spec/toolchain.md`,RXS-0083 续号);缺条款先补 spec,**条款 PR 先于实现 PR**。
> 基准 ref:**维持 `m5-closed`**(M5 close-out 已完成 `m4-closed → m5-closed` 切换,M6 开工**无需再切基准**;`ci/check_guardrails.py` 无参默认 = `m5-closed`,PR 路径仍以 `GITHUB_BASE_REF` 为准)。

---

## 1. 目标

把 Rurix 从 M5 的"安全并行核心交付"(views 不相交 / shared+barrier / scoped atomics / libdevice 链接 + gpu 并行基元自研 kernel ≥ 手写 CUDA C++ 90%)推进到 **可用工具链与包管理**:交付 **`rx` CLI 总入口 + 核心子命令**(build/run/check/test/bench/fmt/doc/fix/watch/vendor 的 MVP 核心集,08 §7),把 M0~M5 散落的 harness 脚本(`rx bench`,RD-003)与雏形格式器(`rx fmt`,RD-005)**收编进统一工具链**;落下 **声明式包管理**(`rurix.toml` + `rurix.lock` + `vendor/` + SHA-256 内容树,path/git/archive 三来源,workspace 单根锁,无 build.rs,09 §7);接通 **LSP MVP**(publishDiagnostics/completion/definition+references/highlight/rename)经 `rurixc --tooling-server` 常驻 query 层(单一前端,07 §9)+ **VS Code 扩展** + **Natvis 首批**。M6 结束时兑现两条硬证据:**三包 workspace(path/git/archive 三来源)离线重建逐字节可复现**(供应链可信根的工程兑现),以及 **LSP 在 10k 行样例工程交互延迟达标**(预算项 measured_local 实测)——这是"Rurix 从'能编译能上 GPU'走向'可日常开发'"的里程碑。

## 2. 范围

### 2.1 in-scope

| 项 | 说明 | 对应交付物 |
|---|---|---|
| rx CLI 核心子命令 | `rx` 总入口 + build/run/check/test/bench/fmt/doc/fix/watch/vendor 的 MVP 核心集(08 §7 D-239);单一前端经 rurixc query 层,不另起引擎 | D-M6-1 |
| 包管理 | `rurix.toml`(意图)+ `rurix.lock`(精确解析图 + 内容树 SHA-256)+ 可提交 `vendor/` + 默认离线可重建(`--locked/--offline`);依赖三来源 path/git/archive;workspace 单根锁;无 build.rs(声明式,09 §7.1/7.2) | D-M6-2 |
| rx test | 内建 `#[test]` + GPU 测试自动子进程隔离选项(H03 §6 纪律工具化,14 §6) | D-M6-3 |
| LSP MVP + VS Code | LSP(publishDiagnostics 直接消费 §5 JSON / completion / definition+references / highlight / rename,07 §9)经 `rurixc --tooling-server` 常驻 query 层 + VS Code 扩展(LSP 客户端 + 语法高亮起步) | D-M6-4 |
| Natvis 首批 | 标准库 Buffer/View/Vec/Mat 的 Natvis 可视化(08 §5,PDB 路线天然兼容) | D-M6-5 |
| spec M6 条款 | rx CLI 子命令语义面 / 包管理 manifest·lock 格式 / LSP 能力面 spec 条款(`spec/toolchain.md`,RXS-0083 续号,FLS 体例);**条款 PR 先于实现 PR** | D-M6-1 ~ D-M6-4 |

### 2.2 out-of-scope(显式排除)

- libdevice 真分发 / 生产分发 fatbin(按架构预编 cubin + 保守 PTX fallback)——→ G1(07 §7 / RD-001 系);M6 维持 **PTX-only 开发期产物**,不打包再分发 cubin/fatbin。
- scoped atomics + PTX `atom.{order}.{scope}` 映射层(D-406 禁区,人工落笔)——已于 M5 条款化(RXS-0080),M6 不扩。
- core 数学库定型(Vec/Mat/swizzle/几何原语)——→ M7(11 §3 M7)。
- 发布链路(`rurixup` 引导 + MSI + winget + 签名/SBOM/许可审计 + artifact 上传)——→ RD-001/M8(08 §9 / 11 §3 M8)。
- registry(sparse index + sumdb 式透明日志 + scopes/OIDC trusted publishing/Sigstore)——→ 所有者决策点 **D-312**(09 §7.3 阶段三;[../../registry/spike_gating.json](../../registry/spike_gating.json) registry 方向触发条件未满足)。
- 任意构建脚本 / 受限 runner 逃生舱——→ 后置评估(09 §7.1:MVP 无 build.rs,`build.model="declarative"`)。
- const 泛型值运行期单态化(RD-007)随 device codegen 进一步扩展评估接通——**非本契约验收门**,接通与否执行期处置留痕。
- 11 §2 MVP 红线清单全部不触碰:Tensor Core/WGMMA/TMA intrinsics、cluster、动态并行、cooperative groups([../../registry/spike_gating.json](../../registry/spike_gating.json) SG-001 ~ SG-009 维持 not_triggered)。

## 3. 交付物清单

| ID | 交付物 | 形态 | 完成判据 |
|---|---|---|---|
| D-M6-1 | rx CLI 核心子命令 | `rx` 总入口 + build/run/check + rx fmt 收编(RD-005)+ rx bench 收编(RD-003)+ spec 条款(RXS-0083 续号) | G-M6-3 + G-M6-4 + G-M6-5;host 回归网持续绿 |
| D-M6-2 | 包管理 | `rurix.toml` + `rurix.lock` + `vendor/` + SHA-256 内容树,path/git/archive 三来源,workspace 单根锁 + spec 条款 | G-M6-1 |
| D-M6-3 | rx test + workspace | 内建 `#[test]` + GPU 子进程隔离 + workspace 多包 + 离线重建复现门 | G-M6-1 子集;CI 子进程隔离 |
| D-M6-4 | LSP MVP | 常驻 query 层 LSP(诊断/补全/跳转/引用/高亮/重命名)+ 无损语法树通道评估(RD-004)+ spec 条款 | G-M6-2 + G-M6-5 |
| D-M6-5 | VS Code 扩展 + Natvis | VS Code 扩展(LSP 客户端 + 高亮)+ Natvis 首批 + 10k 行 LSP 交互延迟实测 | G-M6-2 |
| D-M6-6 | 工具链 conformance + traceability | rx CLI 端到端 + 包管理离线复现 + LSP 能力面测试 + 矩阵延续 | G-M6-3 + G-M6-5 |

## 4. 验收门(完整版,YAML 头为可提取摘要)

1. **G-M6-1(三包 workspace 离线重建逐字节可复现)**:path + git + archive 三来源各 ≥1 包的 workspace,经 `rx build --locked --offline` 在干净环境重建,两次重建产物**逐字节一致**(content SHA-256 比对)、`rurix.lock` 解析图稳定;CI 批跑断言 `m6.counter.offline_rebuild_reproducible ≥1`(11 §3 M6 验收门 / 09 §7.1)。激活经**真实红绿验证**(篡改一个内容树 digest → 重建/校验红 → 复原转绿,run URL 归档,反 YAML-only)。
2. **G-M6-2(LSP 10k 行交互延迟达标 — measured_local)**:`rurixc --tooling-server` 常驻 query 层,10k 行样例工程上 completion / publishDiagnostics(保存后) / definition 交互延迟,采样按 [../m0/BENCH_PROTOCOL.md](../m0/BENCH_PROTOCOL.md) 协议化(墙钟交互延迟主指标,instructions:u 趋势参考);预算断言 `m6.bench.lsp_interaction_latency_ms` `direction=max`,`evidence_level=measured_local`,close-out 跑 `budget_eval --strict` 通过(本占位在 M6 内生灭,不跨里程碑欠债,14 §3)。阈值 `estimated → measured_local` 于 M6.5 回填(目标参照 07 §6 增量 check < 5s 行业线裁定具体阈值)。
3. **G-M6-3(rx CLI 核心子命令端到端)**:build/run/check/test/fmt/bench 在样例工程端到端真跑成功;核心子命令覆盖计数 `m6.counter.rx_cli_core_subcommands ≥` 预设核心集数(数量为 estimated 工程选择,增删经 Direct PR 留痕,对齐 G-M3-1/G-M4-2/G-M5-2 计数器先例)。CI 批跑(子命令冒烟步骤)。
4. **G-M6-4(rx fmt 幂等 RD-005 + rx bench 工具化 RD-003)**:`rx fmt` 对全 `conformance/`+`tests/` 语料二次格式化 0 diff(`ci/check_fmt_idempotent.py` 既有机制延续到 rx fmt 收编后);`rx bench` 复用 BENCH_PROTOCOL §3 协议(L0 锁频前置 / 三次进程级独立运行 / trimmed mean);M5 bench harness 脚本经 `rx bench` 收编后退役,RD-003/RD-005 close 时留痕。
5. **G-M6-5(traceability 延续)**:M6 新增 RXS 条款(`spec/toolchain.md` RXS-0083 续号:rx CLI 子命令语义面 / 包管理 manifest·lock 格式 / LSP 能力面)每条 ≥1 测试锚定;`ci/trace_matrix.py` 全局口径核对(`m1.counter.spec_clause_test_anchoring` 全局断言,无需另立 m6 计数器)。

## 5. Guardrails(字节级,机器核对)

见 YAML 头 `guardrails` 字段。核对方式:`ci/check_guardrails.py [基准ref]`(**默认基准维持 `m5-closed`**,M5 close-out 已完成 `m4-closed → m5-closed` 切换,M6 开工无需再切;PR 路径仍以 `GITHUB_BASE_REF` 为准)。M6 期计划动作:**(1)新段位错误码首批分配**(rx CLI/包管理/LSP 工具链诊断,随 M6.1+ 诊断 PR 留痕,分配制递增、含义冻结);**(2)rx fmt 收编后** `ci/check_fmt_idempotent.py` 既有幂等门延续(G-M6-4);**(3)rx test GPU 子进程隔离** device 测试纳入既有 nightly。M0~M5 历史预算的回填/冻结与既有 bless/spec/error_codes guardrail 走既有机制,无需新代码。若 M6 期需再切基准按 `check_*` 守卫风格 + 双基准核对。

## 6. Deferred 引用

| 编号 | 内容摘要 | 承接 |
|---|---|---|
| RD-003 | rx bench 工具化(协议从独立 harness 脚本收编进工具链组件,08 §4) | M6(本契约 D-M6-1/G-M6-4 承接:`rx bench` 收编 BENCH_PROTOCOL §3 协议,M5 harness 脚本退役;**M6 开工 open→inherited**,owner M6 不变,close 留痕待收编完结) |
| RD-004 | 无损语法树(rowan 式)完整通道(parser 事件流 → 无损语法树) | M6(本契约 D-M6-4 评估接通:LSP MVP 开工时评估;若按 07 §9 容忍"保存时全量 body 重查询"则可继续推迟,处置留痕。代码侧 `// STUB(RD-004)` 双侧标注;**M6 开工 open→inherited**,owner M6 不变) |
| RD-005 | rx fmt 完整工具化(配置面 + 稳定性承诺 + rx CLI 收编) | M6(本契约 D-M6-1/G-M6-4 承接:`rx fmt` 收编进 rx CLI,雏形形态退役;**M6 开工 open→inherited**,owner M6 不变,close 留痕待收编完结) |
| RD-007 | const 泛型值运行期单态化(turbofish const 实参 → 实例值代入 + codegen)+ 运行期数组 aggregate codegen | M6(M5 close-out owner M5→M6 顺延,inherited;随 device codegen 进一步扩展评估接通,spec/consteval.md RXS-0064 已条款化,回填仅补实现侧。**非本契约验收门**,接通与否执行期处置留痕) |

详情以 [../../registry/deferred.json](../../registry/deferred.json) 为唯一事实源,本表仅引用。RD-001(M8)/RD-002(M5 已 closed)/RD-006(M8)不属 M6 范围,维持原承接。M6 开工无预造新 deferred;执行期做不完的事按 14 §4 追加 `RD-###` 并双侧标注。

## 7. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-06-15 | 初版契约固化(M6 开工脚手架;基准 ref 维持 m5-closed 无需再切;deferred RD-003/RD-004/RD-005 open→inherited 承接、RD-007 维持 inherited;spec/toolchain.md RXS-0083 续号预留,条款体随 M6.1+ 与测试同 PR;新段位错误码首批分配随 M6.1+ 诊断 PR) |

---

## 8. Close-out(只追加区 — 开工时为空)

<!-- 验收记录、guardrail 核对输出、deferred 继承/关闭记录、离线重建复现红绿留痕、LSP 交互延迟 measured_local 证据追加于此;上方条款 0-byte 修改。 -->

### 8.1 G-M6-2 LSP 10k 行交互延迟 measured_local 回填(M6.5,2026-06-15)

- 采样:`rurixc --tooling-server` 常驻 query 层,在 `bench/gen_lsp_workspace.py` 确定性生成的 ~10k 行样例工程(前向调用链,来源留痕入证据)上,经客户端 JSON-RPC 墙钟计时 completion / definition / publishDiagnostics(`didChange` 保存后全文重同步,07 §9)三类交互;按 BENCH_PROTOCOL §3 三次进程级独立运行 + trimmed mean(`bench/lsp_bench.py` + `bench/lsp_latency_triple.py`,`rx bench lsp` 经 RD-003 泛分发编排)。CPU 路径 `clock_control=not_applicable_cpu`,沿用与 GPU 基准互斥队列纪律。
- 证据:[`evidence/lsp_latency_20260615_agg.json`](../../evidence/lsp_latency_20260615_agg.json)(measured_local;+ `lsp_latency_20260615_1/2/3.json` 三次单 run)。实测(ms,trimmed mean):completion 35.9975 / definition 29.9585 / publishDiagnostics 72.6226。
- 预算:[`m6_budget.json`](m6_budget.json) `m6.bench.lsp_interaction_latency_ms` estimated → measured_local(revision_log v1.1);逐交互阈值 = 实测 × 1.5(max 方向上界)= 54.0 / 44.94 / 108.93,publishDiagnostics 远在 07 §6 增量 check < 5s 行业线天花板内;阈值为 agent 提案,待人工终审批准(硬规则 1)。
- 判定:`py -3 ci/budget_eval.py --strict` = PASS(全局零 estimated 残留,本占位在 M6 内生灭);`m6.bench.lsp_interaction_latency_ms` 经 `ci/budget_eval.py` 特例分支逐交互对阈。门接线 nightly 趋势归档(参考)+ pr-smoke `budget evaluator` / close-out `--strict`(达标判定)。真实 PR 红绿(#36):绿基线 [27537555626](https://github.com/qwasg/Rurix/actions/runs/27537555626) / 红 [27537714804](https://github.com/qwasg/Rurix/actions/runs/27537714804)(`thresholds.completion` 改到实测之下 → `budget evaluator load` 红)/ 绿 revert [27537807841](https://github.com/qwasg/Rurix/actions/runs/27537807841),详见 CI_GATES §6 v1.8。
- 余项:VS Code 扩展 / Natvis 首批(D-M6-5 人工部分)与 M6 整里程碑 close-out 终审(`m6-closed` tag / 基准切换 / RD-003·RD-005 formal close / registry 汇总)留 M6 收官终审步(白栀/owner 人工签署),本契约 status 维持 `active`。
