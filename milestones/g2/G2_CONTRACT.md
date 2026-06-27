---
contract: G2
title: G2 期——原生 D3D12 + DXIL 第二后端 / 着色阶段进语言 / 绑定布局推导 / UC-04 deferred 渲染器 / 语言 1.0 + edition（MVP 后图形路线第三阶段）
status: active            # active → closed（G2 close-out 时 owner 人工签署翻转；基准 g1-closed→g2-closed 切换 + g2-closed tag 同属 close-out 动作，本期不做）
version: v1.0
date: 2026-06-23
timebox: "MVP+约 18–24 个月（两级结构 G2.1~G2.n 见 G2_PLAN.md；月份为相对刻度，非日历承诺）"
rfc_required: none        # 开工脚手架取 rfc_required: none（结构件，对齐 M4~G1 先例）：G2 高层方向（原生 D3D12 + DXIL 第二后端）由 D-002（已批准：图形分期 MVP→G0→G1→G2）锁定，本脚手架仅落契约骨架 + 蒸馏导航，不预造条款/错误码/counter/SG。**但 G2 各 in_scope 实体项不可由脚手架预锁**：着色阶段进语言（新语法/类型系统）/ DXIL 第二后端（codegen 面，D-131 路径待决）/ 绑定布局推导（codegen）/ registry（D-312）/ VMM·多 GPU（A-06 边界）——每项触 AGENTS 硬规则 5 禁区或 10 §3 Full RFC 面，须在其子里程碑经 owner 人工 Full RFC 前置，脚手架**只登记 + 标 gating + 留 owner 裁决位**，不实现、不解红线、不预判档为 Direct/Mini。**AI 不自判 Direct，判档争议向上取严（硬规则 8）；触及死亡路线红线 / UB / 内存模型映射（06 §4.2）/ FFI ABI / 安全包络须人工经 Full RFC（硬规则 5）**
upstream_docs:
  - "11 §5 (G2 期定义:原生 D3D12 + DXIL 第二后端 / vertex·fragment·mesh·task·RT 着色阶段进语言 / 绑定布局编译器推导 / UC-04 deferred 渲染器 demo / 语言 1.0 = spec 全量条款化 + conformance 覆盖 + 首个 edition / 生态 ≥3 非作者维护项目 / registry 决策点 D-312)"
  - "11 §2 (死亡路线红线 1 无 Python 原生嵌入 / 红线 2·3 无多后端;G2 in_scope 不触红线)"
  - "06 §8.2 (G2 原生 D3D12 + DXIL:着色阶段 = kernel 着色扩展 vertex/fragment/compute/mesh/task/RT fn / MIR→DXIL codegen 第二后端 / descriptor·root signature 编译器推导 P-11 / PSO·资源状态·纹理采样器类型化 / 纹理路径内存模型条款扩展点) / §4.2 (内存模型禁区:纹理路径 G2 引入时再扩展映射条款,仅人类 Full RFC)"
  - "07 §7 (device codegen 分发:MVP/G1 维持 NVPTX→PTX/cubin/fatbin;DXIL 第二后端无 MVP 期 PTX↔DXIL 对应信息,完全于 G2 重评估,D-131) / §7.1 (MLIR kernel island 后置,SG-001)"
  - "05 §1 (device ⊂ host 单向可达子集) / §2.2 (trait 单态化子集 D-104:无 dyn/特化/HKT/async,stable 前重评估)"
  - "04 (设计公理 P-01 strict-only / P-13 防 AI 幻觉治理三角,均准永久条款 10 §9)"
  - "01 §6 (使命与生态成功判据;G2 = 语言 1.0 + 生态 ≥3 非作者维护真实项目)"
  - "13 D-002 (图形分期,已批准) / D-131 (DXIL 生成路径,待决,G2 启动重评估) / D-008 (多后端红线解除,待决,G2 完成后) / D-312 (registry 启动,待决,社区规模驱动) / D-104 (trait 子集 stable 前重评估)"
  - "14 §1 §3 §4 §5 §7 (契约 / 预算零占位 / deferred / 证据分级 / spike gating)"
  - "10 §3 (变更三档) / §7 (AI 八条) / §9.2 (红线解除一次一条) / agents/AGENTS.md §2 (十条硬规则)"
in_scope:
  - shader_stages_in_lang   # G2.1 着色阶段进语言:vertex/fragment/mesh/task/RT 着色阶段作为 kernel 着色扩展进语言的类型面（新语法 + 类型系统）→ **Full RFC 前置**（新语法/类型系统,硬规则 5 / 10 §3）;**首子里程碑 = 类型面条款先行（spec-first,RXS-0153 续号,条款 PR 先于实现 PR）**,06 §8.2
  - dxil_backend            # G2.2 DXIL 第二后端:MIR→DXIL codegen → **Full RFC 前置**（codegen 面 + FFI/ABI 风险,硬规则 5）;**D-131 生成路径（LLVM DirectX 后端 vs SPIR-V→DXIL 转译）owner 裁,本期 defer 至该子里程碑 Full RFC 按当时后端成熟度评估**,06 §8.2 / 07 §7
  - binding_layout_inference # G2.3 绑定布局推导:descriptor / root signature 由编译器推导生成 → **Full RFC 前置**（codegen 推导,P-11）,06 §8.2
  - uc04_deferred_renderer  # G2.4 UC-04 deferred 渲染器 demo:**依赖着色阶段语言面（G2.1）+ DXIL 后端（G2.2）+ 绑定推导（G2.3）就位后落地**,11 §5
  - lang_1_0_edition        # G2.5 语言 1.0:spec 全量条款化 + conformance 覆盖 + **首个 edition 机制** → **edition/stabilization 触 Full RFC 面（10 §3）**,11 §5 / 01 §6
  - registry                # 包 registry（sparse index + sumdb 透明日志 + OIDC/Sigstore）:**D-312 owner 触发（社区规模 >50 包 / 强需求驱动）,本期 not_triggered（SG-007 维持）**,11 §5 / 09 §7.3
  - spec_g2_clauses         # spec 着色阶段 / DXIL 分发 / 绑定布局 / edition 语义面条款（RXS-0153 续号,FLS 体例）;**条款 PR 先于实现 PR（AGENTS 硬规则 7）**,各实体面条款随其子里程碑 Full RFC 前置后落笔
out_of_scope:
  - multi_backend           # 多后端（AMD/Intel/Metal/Vulkan/SPIR-V）→ 死亡路线红线 3,D-008 维持不解除（默认直至 NVIDIA 纵深完成,解除一次一条 10 §9.2）;registry/spike_gating.json SG-003 维持 not_triggered
  - python_native_embed     # Python 原生嵌入永久裁剪（死亡路线红线 1,仅 C ABI/PYD 通道,SG-008 维持 not_triggered）
  - advanced_gpu_intrinsics # Tensor Core/WGMMA/TMA / cluster / 动态并行 / cooperative groups 永久裁剪（11 §2 红线,SG-001 MLIR / SG-002 Tensor Core 维持 not_triggered）
  - vmm_multi_gpu           # VMM（cuMemAddressReserve 族）/ 多 GPU / NVLink / MIG:A-06 单机单 GPU 是 MVP 语义边界,G2 碰多 GPU 须人工 Full RFC（08 §2.2;脚手架不接触）
  - autodiff_fusion         # autodiff / 可微渲染 / kernel fusion / 稀疏结构:永久 gating（SG-004/SG-005）,生态包层面探索不动语言核心
deferred_refs: [RD-007, RD-008, RD-009]   # RD-007（const 泛型值运行期单态化,inherited,owner_milestone=G2;G1.6 顺延,随 device codegen / 运行期数组 aggregate codegen 扩展评估接通,RXS-0064 语义不变）/ RD-008（stable API 快照冻结机制激活,open,owner_milestone=G2;首个 stable 发布时定义 stable 面并激活快照+bless 守卫——G2.5 语言 1.0 为候选触发点）/ RD-009（`#[export(c)]` C ABI 导出属性 + 编译器内建头文件生成 codegen,open,owner_milestone=G2;G1.3 复用 `extern "C"` RXS-0125 兑现,`#[export(c)]` codegen 触 FFI ABI 面后续判档）。G2 开工无预造新 deferred,执行期做不完的事按 14 §4 追加 RD-010+ 并双侧标注
deliverables:
  - id: D-G2-1
    name: G2.1 着色阶段进语言类型面条款先行（vertex/fragment/mesh/task/RT 着色阶段作为 kernel 着色扩展的语法 + 类型系统语义面,RXS-0153 续号,FLS 体例,条款 PR 先于实现 PR;实体面经人工 Full RFC 前置）(G-G2-1) — **首子里程碑**
  - id: D-G2-2
    name: G2.2 DXIL 第二后端（MIR→DXIL codegen;D-131 生成路径 owner 裁,本期 defer 至该子里程碑 Full RFC 按当时 LLVM DirectX 后端成熟度评估;codegen 面 Full RFC 前置）(G-G2-2)
  - id: D-G2-3
    name: G2.3 绑定布局推导（descriptor / root signature 编译器推导生成,P-11;codegen 推导面 Full RFC 前置）(G-G2-3)
  - id: D-G2-4
    name: G2.4 UC-04 deferred 渲染器 demo（依赖 G2.1~G2.3 就位;端到端原生 D3D12 + DXIL 出图）(G-G2-4)
  - id: D-G2-5
    name: G2.5 语言 1.0 + 首个 edition（spec 全量条款化 + conformance 覆盖 + edition 机制;edition/stabilization Full RFC 面;stable API 快照冻结 RD-008 候选触发点）(G-G2-5)
  - id: D-G2-6
    name: spec G2 条款（着色阶段 / DXIL 分发 / 绑定布局 / edition 语义面,RXS-0153 续号,FLS 体例,条款 PR 先于实现 PR）(G-G2-6)
acceptance_gates:
  - id: G-G2-1
    check: "着色阶段进语言类型面条款先行（首子里程碑,spec-first）:新建 spec 着色阶段语义面条款（vertex/fragment/mesh/task/RT 作为 kernel 着色扩展的语法 + 类型系统,RXS-0153 续号,FLS 体例),**经人工 Full RFC 前置后落笔（新语法/类型系统,硬规则 5 / 10 §3）**;每条款 ≥1 测试锚定（conformance/UI golden）随实现 PR 同落,ci/trace_matrix.py 全局口径维持全锚定（沿用既有 m1.counter.spec_clause_test_anchoring 全局断言,**不另立 g2 counter**,对齐 G-G1-6 范式);**条款 PR 先于实现 PR**;真实红绿（放行类型违例 → 红 → 复原绿,run URL 归档,反 YAML-only）"
  - id: G-G2-2
    check: "DXIL 第二后端:MIR→DXIL codegen 端到端,**经人工 Full RFC 前置（codegen 面,硬规则 5）+ D-131 生成路径裁决（LLVM DirectX 后端 vs SPIR-V→DXIL,按当时后端成熟度,13 §D-131）**;DXIL codegen 形态纳入 golden 核对（新增 DXIL golden + bless 机制随实现);device 真跑数值/呈现对照;真实红绿（篡改 codegen 输出 → 红 → 复原绿,run URL 归档)。**脚手架不实现,仅登记 gating**"
  - id: G-G2-3
    check: "绑定布局推导:descriptor / root signature 由编译器推导生成（P-11）,**经人工 Full RFC 前置（codegen 推导面）**;推导正确性 conformance + golden;真实红绿。**脚手架不实现,仅登记 gating**"
  - id: G-G2-4
    check: "UC-04 deferred 渲染器 demo:依赖 G2.1~G2.3 就位,原生 D3D12 + DXIL 端到端出图（多 pass deferred 管线）真跑;呈现对照 + 真实红绿。**脚手架不实现,仅登记 gating**"
  - id: G-G2-5
    check: "语言 1.0 + 首个 edition:spec 全量条款化 + conformance 覆盖达标 + edition 机制落地,**edition/stabilization 经人工 Full RFC（10 §3）**;stable API 快照冻结机制（RD-008）在首个 stable 发布时按 owner 裁决激活（stable 面定义 + 快照比对 + bless 守卫);close-out budget_eval --strict 零 estimated。**脚手架不实现,仅登记 gating**"
  - id: G-G2-6
    check: "traceability 延续:G2 新增 RXS 条款（RXS-0153 续号:着色阶段 / DXIL 分发 / 绑定布局 / edition 语义面）每条 ≥1 测试锚定;ci/trace_matrix.py 全局口径核对（m1.counter.spec_clause_test_anchoring 全局断言,无需另立 g2 计数器）;**条款 PR 先于实现 PR**（AGENTS 硬规则 7）"
guardrails:
  - "milestones/m0~g1 的 measured_local 既有预算条目 git diff 0-byte（新增 g2 条目允许）;g2_budget.json 经命名空间强制前缀 g2. + namespace check 单测（14 §3,经现有 *_budget.json glob 自动纳入）"
  - "milestones/m0~g1 的 *_CONTRACT.md（均 closed）既有内容只追加不修改（check_closed_contracts,glob 已于 G1 close-out 泛化为 *_CONTRACT.md）;本契约 G2_CONTRACT.md close-out 守卫随 G2 close-out 接入 check_closed_contracts 的 *_CONTRACT.md 口径（status: closed 后字节守卫;开工记录于 CI_GATES §5,G2 close-out 动作）"
  - "registry/deferred.json 与 registry/spike_gating.json 只追加（既有条目不可变字段修改触发人工审查）;RD-007/RD-008/RD-009 状态翻转仅由 owner 人工签署留痕追加;G2 期 SG 复评（SG-001 MLIR / SG-002 Tensor Core / SG-003 多后端 / SG-007 registry 维持 not_triggered;新触发方向登记 SG-010+）只追加 decisions"
  - "registry/error_codes.json 错误码语义可加不可改（M1.1 已激活）;G2 新段位（着色阶段 / DXIL codegen / 绑定布局 / edition 诊断）首批分配随 G2.x 诊断 PR 留痕,段位分配制递增、含义冻结;**开工脚手架不预造错误码（RX7020 续号预留）**"
  - "evidence/ 只增不删不改（M0.3 起）"
  - "00–14 共 15 份规划文档（**含 13_DECISION_LOG.md**）不被执行 PR 改写（check_planning_docs 字节守卫;勘误/新 D-### 走 00 §6.3 独立规划文档 PR,与本脚手架 PR 分离）"
  - "tests/ui/ .stderr / tests/mir/ .mir / tests/ptx/ .nvptx golden 变更必须经审批 bless（M1.4/M3.3/M4.2 已激活）;G2 DXIL codegen 形态变更纳入既有 golden 核对 + 新增 DXIL 文本 golden + bless 机制随 DXIL 子里程碑实现"
  - "全仓 crate 维持 unsafe_code=deny;src/rurix-rt 维持 undocumented_unsafe_blocks=deny（M4.3）;G2 DXIL/D3D12 原生管线 / 绑定推导 / 着色阶段边界凡落 unsafe 须每 unsafe 块 // SAFETY: + unsafe-audit 注册条目（AGENTS 硬规则 9,U23 续号）,新 crate 默认 unsafe_code=deny"
  - "NVIDIA 再分发白名单审计维持（M5.4 check_redistribution）;G2 原生 D3D12 + DXIL 系 Windows SDK / DirectX 系统组件,不受 NVIDIA 再分发约束;CUDA 侧 cubin/fatbin 产物（若 G2 保留 compute 互操作）延续 Attachment A 白名单最小集审计（许可红线 r6）"
  - "Compute Sanitizer racecheck+memcheck nightly 维持全绿（M5.4）;G2 device 路径（若涉 CUDA compute 互操作）落地后纳入既有 nightly 全跑"
  - "guardrail 回退基准默认 = g1-closed（G1 close-out 已切;ci/check_guardrails.py 无参默认 g1-closed,PR 路径仍以 GITHUB_BASE_REF 为准）;G2 close-out 时按 check_* 守卫风格 + 双基准核对切至 g2-closed（owner 人工签署兑现,glob 已泛化无需再改）"
  - "stable API 快照冻结机制（RD-008）维持 not_frozen/未激活至首个 stable 发布（G2.5 语言 1.0 为候选触发点）;激活时机与 stable 面定义经 owner 裁决留痕,激活后 stable API 快照变更须经审批 bless"
  - "本契约 in_scope/acceptance_gates 等既有条款 0-byte 修改,close-out 只追加 §8;契约 status active→closed 翻转 / 基准切换（g1-closed→g2-closed）/ g2-closed tag / RD·SG 状态翻转 / 红线解除由 owner 人工签署,AI 不代签"
---

# G2 契约 — 原生 D3D12 + DXIL 第二后端 / 着色阶段进语言 / 绑定布局推导 / UC-04 deferred 渲染器 / 语言 1.0 + edition（MVP 后图形路线第三阶段）

> 所属:[../../11_ROADMAP.md](../../11_ROADMAP.md) §5 G2 期 / 契约机制见 [../../14_ENGINEERING_DISCIPLINE.md](../../14_ENGINEERING_DISCIPLINE.md) §1
> 规范先行延续（AGENTS.md 硬规则第 7 条）:着色阶段 / DXIL 分发 / 绑定布局 / edition 的语义面 PR 必须引用 RXS-#### 条款号（RXS-0153 续号）;缺条款先补 spec,**条款 PR 先于实现 PR**。
> 基准 ref:**默认 `g1-closed`**（G1 close-out 已完成 `m8-closed → g1-closed` 切换;`ci/check_guardrails.py` 无参默认 = `g1-closed`,PR 路径仍以 `GITHUB_BASE_REF` 为准）。
> 粒度:**单 G2 阶段契约**（owner 裁定,见 §7 v1.0）:一份契约覆盖整个 G2 期,G2.1~G2.n 子里程碑分解见 [G2_PLAN.md](G2_PLAN.md)（对齐 M*/G1「每里程碑一份契约 + 内部子里程碑」范式）。
> 上下文蒸馏:G2 执行 agent 起步上下文 = [G2_CONTEXT.md](G2_CONTEXT.md)（导航 + 决策面摘要,非规范源）+ 本四件套;深挖规范正文派 Explore 子 agent 读 00–14 / spec / registry 原文（原文不动）。
> **脚手架口径:本契约为 G2 开工结构件,不实现任何 G2 语义面、不解红线、不立 Full RFC 项目、不预造条款/错误码/counter/SG;§8 close-out 开工时为空。**

---

## 1. 目标

G2 期结束时项目获得:原生 D3D12 + DXIL 第二后端图形管线（着色阶段 vertex/fragment/mesh/task/RT 进语言 + MIR→DXIL codegen + 绑定布局编译器推导）;UC-04 deferred 渲染器端到端出图;语言 1.0（spec 全量条款化 + conformance 覆盖 + 首个 edition 机制）;生态成功判据 ≥3 个非作者维护的真实项目（11 §5 / 01 §6）。**本开工脚手架只建契约骨架 + 蒸馏导航,各实体面经 owner 人工 Full RFC 前置后在其子里程碑落地。**

## 2. 范围

### 2.1 in-scope

| 项 | 说明 | gating | 对应交付物 |
|---|---|---|---|
| shader_stages_in_lang | 着色阶段（vertex/fragment/mesh/task/RT）作为 kernel 着色扩展进语言的语法 + 类型系统（06 §8.2） | **Full RFC 前置**（新语法/类型系统）;首子里程碑条款先行 | D-G2-1 |
| dxil_backend | MIR→DXIL codegen 第二后端（06 §8.2 / 07 §7） | **Full RFC 前置**（codegen 面）;D-131 路径 owner 裁,本期 defer | D-G2-2 |
| binding_layout_inference | descriptor / root signature 编译器推导（P-11,06 §8.2） | **Full RFC 前置**（codegen 推导） | D-G2-3 |
| uc04_deferred_renderer | UC-04 deferred 渲染器 demo（11 §5） | 依赖 G2.1~G2.3 就位 | D-G2-4 |
| lang_1_0_edition | 语言 1.0 = spec 全量条款化 + conformance + 首个 edition（11 §5 / 01 §6） | **Full RFC 面**（edition/stabilization） | D-G2-5 |
| registry | 包 registry（sparse index + sumdb + OIDC/Sigstore,09 §7.3） | **D-312 owner 触发**;本期 not_triggered（SG-007） | （触发后另立交付物） |
| spec_g2_clauses | spec 着色阶段 / DXIL 分发 / 绑定布局 / edition 语义面条款（RXS-0153 续号） | 条款 PR 先于实现 PR | D-G2-6 |

### 2.2 out-of-scope（显式排除）

- **multi_backend**（多后端 AMD/Intel/Metal/Vulkan/SPIR-V）:死亡路线红线 3,D-008 维持不解除（默认直至 NVIDIA 纵深完成,解除一次一条 10 §9.2);SG-003 维持 not_triggered。
- **python_native_embed**（Python 原生嵌入）:死亡路线红线 1,永久裁剪,仅 C ABI/PYD 通道;SG-008 维持 not_triggered。
- **advanced_gpu_intrinsics**（Tensor Core/WGMMA/TMA / cluster / 动态并行 / cooperative groups）:11 §2 红线;SG-001（MLIR）/ SG-002（Tensor Core）维持 not_triggered。
- **vmm_multi_gpu**（VMM / 多 GPU / NVLink / MIG）:A-06 单机单 GPU 语义边界;G2 碰多 GPU 须人工 Full RFC（08 §2.2）。
- **autodiff_fusion**（autodiff / 可微渲染 / kernel fusion / 稀疏结构）:永久 gating（SG-004/SG-005），生态包层面探索不动语言核心。

## 3. 交付物清单

| ID | 交付物 | 形态 | 完成判据 |
|---|---|---|---|
| D-G2-1 | G2.1 着色阶段类型面条款先行（首子里程碑） | spec 条款（RXS-0153 续号）+ conformance/UI 锚定 | Full RFC 前置后落条款 + trace 全锚定 + 真实红绿（G-G2-1） |
| D-G2-2 | G2.2 DXIL 第二后端 | rurixc/rurix-rt codegen + golden | Full RFC + D-131 裁决 + device 真跑 + golden（G-G2-2） |
| D-G2-3 | G2.3 绑定布局推导 | 编译器推导 + conformance | Full RFC + 推导正确性 + 真实红绿（G-G2-3） |
| D-G2-4 | G2.4 UC-04 deferred 渲染器 demo | demo crate + 端到端出图 | 原生 D3D12+DXIL 出图 + 呈现对照（G-G2-4） |
| D-G2-5 | G2.5 语言 1.0 + 首个 edition | spec 全量 + conformance + edition 机制 | Full RFC + conformance 达标 + RD-008 激活裁决（G-G2-5） |
| D-G2-6 | spec G2 条款 | spec 文件（RXS-0153 续号,FLS 体例） | 每条款 ≥1 测试锚定 + 条款先于实现（G-G2-6） |

## 4. 验收门（完整版，YAML 头为可提取摘要）

见 YAML 头 `acceptance_gates` 字段 G-G2-1 ~ G-G2-6。要点:
- **首子里程碑 G-G2-1（着色阶段条款先行,spec-first）**:验收 = RXS 条款 trace 锚定（沿用既有 `m1.counter.spec_clause_test_anchoring` 全局断言,**不另立 g2 counter**,对齐 G-G1-6 范式）+ 条款 PR 先于实现 PR + 真实红绿。
- **G-G2-2~G-G2-5 均标 Full RFC 前置 gating**:脚手架不实现,各实体面经 owner 人工 Full RFC（DXIL codegen / 着色语法 / 绑定推导 / edition）+（DXIL）D-131 路径裁决后,在其子里程碑落地;device 真跑 + golden + 真实红绿（反 YAML-only）。
- 性能门（若有）`g2.bench.*` / `g2.ratio.*` 随各 g2.x 实测 measured_local 回填;close-out `budget_eval --strict` 全局零 estimated 残留（14 §3）。

## 5. Guardrails（字节级，机器核对）

见 YAML 头 `guardrails` 字段。核对方式:`py -3 ci/check_guardrails.py`（无参默认基准 = `g1-closed`;PR 路径以 `GITHUB_BASE_REF` 为准）。要点:00–14（含 13_DECISION_LOG.md）+ deep-research 0-byte;registry/预算/已关闭契约/evidence 只追加;error_codes 含义冻结;spec 档位标记;golden bless;baseline `g1-closed`（G2 close-out 时 owner 切 `g2-closed`,glob 已泛化）。

## 6. Deferred 引用

| 编号 | 内容摘要 | 承接 |
|---|---|---|
| RD-007 | const 泛型值运行期单态化（turbofish const 实参 → 实例值代入 + codegen） | inherited,owner_milestone=G2;随 device codegen / 运行期数组 aggregate codegen 扩展评估接通,RXS-0064 语义不变 |
| RD-008 | stable API 快照冻结机制激活（stable 面定义 + 快照比对 + bless 守卫） | open,owner_milestone=G2;首个 stable 发布（G2.5 语言 1.0 候选触发点）时定义 stable 面并激活 |
| RD-009 | `#[export(c)]` C ABI 导出属性 + 编译器内建头文件生成 codegen | open,owner_milestone=G2;G1.3 复用 `extern "C"`（RXS-0125）兑现,`#[export(c)]` codegen 触 FFI ABI 面后续判档 |

详情以 [../../registry/deferred.json](../../registry/deferred.json) 为唯一事实源,本表仅引用。RD-001~RD-006 已 closed。G2 开工无预造新 deferred;执行期做不完的事按 14 §4 追加 `RD-010+` 并双侧标注。

## 7. 修订记录 / 开工裁决留痕

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-06-23 | 初版契约固化（G2 开工脚手架）。**开工 owner 裁决**（经 AskUserQuestion 确认,记于本节,引既有 `D-002` 图形分期 MVP→G0→G1→G2 **已批准**[../../13_DECISION_LOG.md](../../13_DECISION_LOG.md)）:① 粒度 = **单 G2 阶段契约**（milestones/g2/,G2.1~G2.n 作为 G2_PLAN.md 内子里程碑,对齐 M*/G1「每里程碑一份契约 + 内部子里程碑」范式）;② 首子里程碑 = **G2.1 着色阶段类型面条款先行**（spec-first,RXS-0153 续号,对齐规范先行硬规则 7;着色阶段语言面是 DXIL codegen / 绑定推导 / UC-04 的共同依赖基座,先立条款再实现）;③ **D-131 DXIL 生成路径 = 本期 defer**（脚手架不锁路径,D-131 维持**待决**,留至 DXIL 子里程碑按当时 LLVM DirectX 后端成熟度经 Full RFC 评估,13 §D-131）;④ **红线 3（多后端,D-008）维持不解除**（默认直至 NVIDIA 纵深完成,解除一次一条 10 §9.2;SG-003 维持 not_triggered）;⑤ **registry（D-312）本期维持休眠**（not_triggered,留社区规模 >50 包 / 强需求触发;SG-007 维持）;⑥ **延续 G1.4 RFC 流程**（FCP-lite + 贡献门 ci/check_contribution.py 已在 main;G2 新 Full RFC 续号 + Mini-RFC 续 mini-0006+）。判档:脚手架取 `rfc_required: none`（对齐 M4~G1 先例,高层方向 D-002 已锁 00–14）。承接:RD-007（inherited）/ RD-008（open）/ RD-009（open）owner_milestone 已于 G1 close-out 顺延至 G2,本契约 deferred_refs 引用;开工无预造新 deferred。基准 ref 默认 g1-closed（G1 已切,无需再切）。RXS-0153 续号预留,条款体随 G2.x 与测试同 PR（条款先于实现）;新段位错误码（RX7020 续号）随 G2.x 诊断 PR。**13_DECISION_LOG.md 在执行 PR 中字节冻结（check_planning_docs）,本裁决留痕记于本契约 §7（对齐 G1 §7 引 D-005 先例,不改决策日志）;若需在规范决策日志落正式新 D-###（如 G2 开工/范围）,须 owner 经 00 §6.3 独立规划文档勘误 PR 兑现,与本脚手架 PR 分离。** **G2 执行期新决策面**（着色阶段语法/类型形态 / DXIL D-131 路径 / 绑定推导面 / edition 机制 / registry 触发 / stable 面定义）在对应 g2.x 子里程碑带档位标记落笔,**AI 不自判 Direct,判档争议向上取严**;触及红线/UB/内存模型映射（06 §4.2）/FFI ABI/安全包络须人工经 Full RFC。**G2 close-out 关闭判定 / 基准切换（g1-closed→g2-closed）/ g2-closed tag / RD-007·RD-008·RD-009 状态翻转 / 红线解除由 owner 人工签署,AI 不代签** |

---

## 8. Close-out（只追加区 — 开工时为空）

<!-- 验收记录、guardrail 核对输出、deferred 继承/关闭记录、G2.x 子里程碑端到端红绿留痕、着色阶段/DXIL/绑定推导/UC-04 证据、语言 1.0 conformance 终审、edition 机制留痕、性能 measured_local 回填、RD-007/RD-008/RD-009 处置留痕、SG 复评结论追加于此;上方条款 0-byte 修改。G2 close-out 关闭判定 / 基准切换（g1-closed→g2-closed）/ g2-closed tag / RD·SG 状态翻转由 owner 人工签署兑现,AI 不代签。 -->

### 8.1 G2.1 子里程碑验收留痕（2026-06-23）

owner 白栀于本工作会话授权完成 G2.1 人工收尾与签字；以下由 AI 代录机器事实与验收核对，**不构成 AI 代签 G2 整体 close-out**：

- **Full RFC 前置**：RFC-0002（着色阶段进语言的类型面，新语法 + 类型系统，硬规则 5 / 10 §3）经 owner 2026-06-23 工作会话明确裁决 §9 Q1~Q6 与 §4.5 🔒 禁区边界处置（代录非代签，硬规则 1），状态 Owner Approved，PR [#79](https://github.com/qwasg/Rurix/pull/79) 合入 `main`（merge `516a855`）。
- **条款先行（spec-first）**：PR-B1 spec 脚手架 [#80](https://github.com/qwasg/Rurix/pull/80) 仅登记 `spec/shader_stages.md` + RXS-0153~0156 预留区间（不落裸条款头）合入 `main`（merge `76a179c`）；PR-B2 [#81](https://github.com/qwasg/Rurix/pull/81) 内条款体 commit `f80dd2e` 先于前端 commit `4c099e4`，保持条款先于实现序，合入 `main`（merge `4c760f9`）。
- **条款体 + 前端落地**：RXS-0153（着色阶段函数着色规则，扩展 RXS-0066，**着色阶段误用复用既有 RX3001 无新码**）/ RXS-0154（阶段专属 I/O 语义类型，违例 RX3011）/ RXS-0155（阶段间接口类型契约，违例 RX3012）/ RXS-0156（资源句柄·纹理采样器参数化类型面 `Texture2D<F>`+`Sampler`，违例 RX3013）带编号条款体 + 测试锚定；`src/rurixc/src/shader_stages.rs`（623 行 AST 层 typeck，feature `shader-stages`，纯前端**零新 unsafe**，无需 U23）+ parser/ast/hir/query/mir_build 接线。新段位错误码 RX3011~RX3013（**3xxx 着色/地址空间段续号，非 7xxx**，真实可达只追加 + 双语 en/zh message-key，`ci/bilingual_coverage.py` 71/71 对齐）。
- **traceability**：`ci/trace_matrix.py --check` PASS，trace 152→**156/156**（RXS-0153~0156 各 ≥1 锚定：conformance accept/reject `conformance/shader/**` + UI golden `tests/ui/shader/*.stderr`，bless 留痕 `tests/ui/bless_log.md`）；沿用全局 `m1.counter.spec_clause_test_anchoring`，**不另立 g2 counter**（对齐 G-G1-6 范式）。
- **真实红绿（反 YAML-only）**：CI 步骤 45 `ci/shader_stages_smoke.py` **本机 PASS**——green（合法着色阶段声明 0 诊断）+ red（RX3001 / RX3011 / RX3012 / RX3013 四类编译期拦截）+ red 自检（green 注入无标注 I/O 字段 → 翻红 RX3011）。**着色阶段类型面为编译期 / 纯 host，无需 device。**
- **host 门全绿（本机，AI 独立复核非轻信「备绿」报告）**：`cargo test -p rurixc` lib 314/0 + shader_corpus 4/0 + ui_golden 4/0 + 全 corpus 0 fail；`cargo clippy --all-targets -D warnings` / `cargo fmt --check` 干净；`cargo build -p rurixc --no-default-features`（feature 门关）编译通过；`check_guardrails g1-closed` / `check_contribution` / `budget_eval --strict 69-0` / `check_schemas` / `bilingual_coverage` PASS；改动文件 LF 字节级核对无 CRLF 回归。
- **CI run 说明（不伪造）**：[run 28019732761](https://github.com/qwasg/Rurix/actions/runs/28019732761) 步骤 1–42 全部 host 门 + 可降级 smoke 绿；唯一红为步骤 43 `engine_integration_smoke`（G1.3 device 段，CI runner 缺 MSVC/CUDA、workflow 强制 `RURIX_REQUIRE_REAL=1` 未降级 SKIP），与本里程碑无关、自 #78 起即 main 长期环境红；job 遇首红即止，纯 host 的步骤 45 在 CI 未及执行，**已本机真实红绿验证**（不伪造 CI 步骤 45 green URL）。未按红线动 engine_integration。device 段 CI 绿待 owner `rurix-dev-4070ti` runner 配齐 MSVC+CUDA。

判定：D-G2-1 / G-G2-1 子里程碑验收要件闭环（着色阶段类型面条款先行 + trace 全锚定 156/156 + 真实红绿）；**G2 契约仍为 `active`**，不执行 `g2-closed` tag / 基准切换（g1-closed→g2-closed）/ RD-007·RD-008·RD-009 翻转（均属 G2 整体 close-out，owner 另动）。

### 8.2 G2.2 子里程碑验收留痕（2026-06-27）

owner 白栀于本工作会话监督确认 G2.2 人工收尾、DXIL golden bless 与 G-G2-2 子里程碑签字；以下由 AI 代录机器事实并执行机械落档，**不构成 AI 代签 G2 整体 close-out**：

- **Full RFC / D-131 路径**：DXIL 第二后端 codegen 面经 RFC-0003 / RFC-0004 承载；D-131 已按图形=B 路（MIR→SPIR-V→SPIRV-Cross→HLSL→dxc→DXIL）落地到 PR-D2。RXS-0157（DXIL target 分发）/ RXS-0158（阶段→shader type）/ RXS-0159（阶段 I/O 签名校验门）/ RXS-0161（MIR→SPIR-V）/ RXS-0162（B 转译链确定性 + validator gate + golden）已入 spec 并有测试锚定；RXS-0160 仍为后续多阶段链接核对计划项。
- **DXIL golden bless**：`tests/dxil/graphics/gfx_vs_min.dxil-disasm` 已在 owner pin 环境重 bless。环境为 `RURIX_DXC_DIR=H:\dxc-round7\extracted\bin\x64`（含 `dxc.exe` / `dxv.exe` / `dxil.dll`，DXC 1.9.2602.24）与 `RURIX_SPIRV_CROSS=C:\ti-localappdata\ti-build-cache\vulkan-1.3.296.0\Bin\spirv-cross.exe`；命令 `RURIX_BLESS=1 cargo test -p rurixc --features dxil-backend --test dxil_golden dxil_b_disasm_golden_matches_when_toolchain_present -- --exact --nocapture` PASS，入 golden 前 `dxv.exe` validator 接受。审批留痕：[tests/dxil/bless_log.md](../../tests/dxil/bless_log.md)。当前 `gfx_vs_min` 语料仍为平凡 passthrough，锁定的是已登记 RD-013 / RD-017 缺口下的 `TEXCOORD` baseline，**不声称 output varying 用户语义保真已兑现**。
- **device 真跑 / run URL**：PR #107 最新 `pr-smoke` [run 28284960733](https://github.com/qwasg/Rurix/actions/runs/28284960733) 全量 success（3m19s，head `06ca54e`）。步骤 46 同时执行 `ci/dxil_codegen_smoke.py` 与 `ci/dxil_device_smoke.py`，日志含 `DXIL_DEVICE: ok adapter="NVIDIA GeForce RTX 4070 Ti" pixel=64,127,255,255 draw=ok`；`dxil_device_smoke` 写入 `target\dxil_device_smoke\result.json` 并记录该 run URL。
- **真实红绿（反 YAML-only）**：host 段 `ci/dxil_codegen_smoke.py` 覆盖转译链可达、确定性 ×N、validator gate、系统值/顶点输入语义保真、SPIR-V 字流篡改红绿、译后签名篡改红绿与供应链 pin 核对；device 段 `ci/dxil_device_smoke.py` 覆盖 signed DXC 编译 VS/PS、`dxv.exe` validator 接受、篡改 DXIL 被 `dxv` 拒绝、MSVC C++ D3D12 hardware PSO/offscreen draw/readback 像素对照。CI run 同时证明步骤 22 Windows GPU toolchain 初始化、步骤 43 engine integration、步骤 44 fatbin distribution 均已真跑转绿。
- **验证补充（本机会话）**：`RURIX_DXC_DIR=H:\dxc-round7\extracted\bin\x64 py -3 ci/dxil_codegen_smoke.py` PASS；`RURIX_DXC_DIR=H:\dxc-round7\extracted\bin\x64 RURIX_REQUIRE_REAL=1 py -3 ci/dxil_device_smoke.py` PASS，输出同一 RTX 4070 Ti offscreen draw/readback；`cargo test -p rurixc --features dxil-backend --lib dxil_codegen` 与 `cargo test -p rurixc --features dxil-backend --test dxil_corpus --test dxil_golden` PASS。

判定：D-G2-2 / G-G2-2 子里程碑验收要件闭环（Full RFC + D-131 图形=B路径 + DXIL golden owner bless + validator gate + device 真跑 + run URL + 内建篡改红绿）；**G2 契约仍为 `active`**，不执行 `g2-closed` tag / 基准切换（g1-closed→g2-closed）/ RD-007·RD-008·RD-009 翻转；RD-013 / RD-017 继续按 deferred 机制承接，未在本签字中关闭。
