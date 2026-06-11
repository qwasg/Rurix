---
contract: M0
title: 基础设施与证据通道
status: active            # active → closed(close-out 只追加,既有条款 0-byte 修改)
version: v1.0
date: 2026-06-11
timebox: "M+1(约 4 周,两级结构见 M0_PLAN.md)"
rfc_required: none        # 本里程碑无语言语义变更,全部工作为 Direct PR 档(见 10 §3)
upstream_docs:
  - "11 §3 (M0 定义)"
  - "14 (契约制/预算/deferred/CI)"
  - "10 §4 §7 (仓库结构/AI 政策)"
  - "08 §4 (基准协议)"
in_scope:
  - repo_and_ci
  - l0_l1_bench_harness
  - handwritten_ptx_baselines
  - discipline_templates
  - agents_md_v1
out_of_scope:
  - any_compiler_code        # 任何 rurixc 编译器代码(M1 起)
  - full_l1_suite            # SAXPY/bandwidthTest 之外的 L1 基准(RD-002)
  - release_ci_tier          # Release 层门禁(RD-001)
  - rx_bench_toolchain       # rx bench 工具化(RD-003)
deferred_refs: [RD-001, RD-002, RD-003]
deliverables:
  - id: D-M0-1
    name: 仓库 + CI PR Smoke
  - id: D-M0-2
    name: L0 环境验证通道(锁频规程 + NVML 环境画像探测器)
  - id: D-M0-3
    name: 手写 PTX + Driver API 的 SAXPY 与 bandwidthTest 基线
  - id: D-M0-4
    name: 契约/预算 JSON/deferred/gating 注册表模板
  - id: D-M0-5
    name: agents/AGENTS.md v1
acceptance_gates:
  - id: G-M0-1
    check: "m0_budget.json 中 m0.bench.* 全部条目 evidence=measured_local(三次运行 trimmed mean,零 estimated 残留)"
  - id: G-M0-2
    check: "CI PR Smoke 在 ≥1 个真实 PR 上绿过(附 run URL,非 YAML 语法检查)"
  - id: G-M0-3
    check: "NVML 探测器输出通过 evidence_schema.json 校验(全字段非空)"
  - id: G-M0-4
    check: "guardrail 核对脚本首版可执行且在 close-out 时全过"
guardrails:
  - "milestones/m0/m0_budget.json 既有条目 git diff 0-byte(新增条目允许)"
  - "registry/deferred.json 与 registry/spike_gating.json 只追加(既有条目修改触发人工审查)"
  - "00–14 共 15 份规划文档不被执行 PR 改写(勘误走 00 §6.3 追加式修订)"
  - "本契约 in_scope/acceptance_gates 等既有条款 0-byte 修改,close-out 只追加"
---

# M0 契约 — 基础设施与证据通道

> 所属:[../../11_ROADMAP.md](../../11_ROADMAP.md) §3 M0 / 契约机制见 [../../14_ENGINEERING_DISCIPLINE.md](../../14_ENGINEERING_DISCIPLINE.md) §1
> 排序不可调换:这是 P-09(证据先行)的兑现,先于一切编译器代码。

---

## 1. 目标

在写第一行编译器代码之前,建成真实硬件证据通道与工程纪律骨架(11 §3)。M0 结束时,项目具备:可在真实 PR 上拦截问题的 CI、可复现的 GPU 基准协议、后续一切性能阈值的 `measured_local` 锚点、以及对 AI 集群生效的纪律模板。

## 2. 范围

### 2.1 in-scope

| 项 | 说明 | 对应交付物 |
|---|---|---|
| 仓库 + CI | 按 10 §4 目录骨架初始化;PR Smoke 真实跑通(上一项目 D11.8-2 教训:禁止 YAML-only 验证) | D-M0-1 |
| L0/L1 基准 harness | RTX 4070 Ti 上的锁频/环境画像/统计协议,r11 协议实现(数字见 08 §4) | D-M0-2 |
| 手写基线 | 手写 PTX + Driver API 装载的 SAXPY 与 bandwidthTest,`measured_local` 入预算 | D-M0-3 |
| 纪律模板 | 契约 YAML 模板、预算 JSON、deferred.json、spike_gating.json(H05 资产改造) | D-M0-4 |
| AGENTS.md v1 | AI 会话强制上下文(10 §7 / 14 §10) | D-M0-5 |

### 2.2 out-of-scope(显式排除)

- 任何 rurixc 编译器代码(lexer/parser 起步于 M1,见 11 §3)。
- SAXPY/bandwidthTest 之外的 L1 基准(Reduction/Transpose/GEMM)→ **RD-002**。
- CI Release 层(签名/SBOM/许可审计)→ **RD-001**。
- `rx bench` 工具化(M0 期协议以独立 harness 脚本形态存在)→ **RD-003**。
- 11 §2 MVP 红线清单全部不触碰,已逐项登记 [../../registry/spike_gating.json](../../registry/spike_gating.json)。

## 3. 交付物清单

| ID | 交付物 | 形态 | 完成判据 |
|---|---|---|---|
| D-M0-1 | 仓库 + CI PR Smoke | git 仓库 + 自托管 runner 工作流 | G-M0-2 |
| D-M0-2 | L0 环境验证通道 | 锁频规程([BENCH_PROTOCOL.md](BENCH_PROTOCOL.md) §2)+ NVML 探测器 | G-M0-3 |
| D-M0-3 | SAXPY / bandwidthTest 基线 | 手写 `.ptx` + Driver API 装载 harness + 证据 JSON | G-M0-1 |
| D-M0-4 | 纪律模板四件套 | 本契约 + [m0_budget.json](m0_budget.json) + [../../registry/](../../registry/) 两注册表 | 文件存在且 schema 自洽 |
| D-M0-5 | AGENTS.md v1 | [../../agents/AGENTS.md](../../agents/AGENTS.md) | 文件存在且含 10 §7 八条政策的执行映射 |

## 4. 验收门(完整版,YAML 头为可提取摘要)

1. **G-M0-1(证据锚点)**:`m0_budget.json` 中 `m0.bench.saxpy.*` 与 `m0.bench.bandwidth.*` 全部条目回填为 `measured_local`(协议:warmup ≥10 + 稳态 CV<5% → 50×3 → trial 内中位数 → 跨 trial trimmed mean 去头尾 20%,来源 r11 §1.3 / 08 §4);**M0 关闭时本预算文件零 `estimated` 残留**。
2. **G-M0-2(CI 真跑)**:PR Smoke 工作流在 ≥1 个真实 PR 上完整执行并绿过,close-out 附 run URL 与命令输出。
3. **G-M0-3(环境画像)**:NVML 探测器输出 JSON 通过 [evidence_schema.json](evidence_schema.json) 校验,驱动模型/HAGS/TDR/版本字段全部非空(08 §2.3 表)。
4. **G-M0-4(guardrail 可执行)**:guardrail 核对脚本([CI_GATES.md](CI_GATES.md) §4 清单)首版可执行,close-out 时全过。

## 5. Guardrails(字节级,机器核对)

见 YAML 头 `guardrails` 字段。核对方式:`git diff --stat` 字节核对(14 §2),非人工 checklist。

## 6. Deferred 引用

| 编号 | 内容摘要 | 承接 |
|---|---|---|
| RD-001 | CI Release 层门禁 | M8 |
| RD-002 | L1 全量基准套件 | M5 |
| RD-003 | `rx bench` 工具化 | M6 |

详情以 [../../registry/deferred.json](../../registry/deferred.json) 为唯一事实源,本表仅引用。

## 7. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-06-11 | 初版契约固化 |

---

## 8. Close-out(只追加区 — 开工时为空)

<!-- 验收记录、guardrail 核对输出、deferred 继承/关闭记录追加于此;上方条款 0-byte 修改。 -->
