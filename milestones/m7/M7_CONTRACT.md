---
contract: M7
title: 标准库充实与 G0 图形演示——core 数学库 / image-io / 软光栅 / UC-03 demo
status: active            # active → closed(close-out 只追加,既有条款 0-byte 修改;M7 close-out 终审 §8 人工签署)
version: v1.0
date: 2026-06-15
timebox: "M+13 ~ M+15(约 9 周,两级结构见 M7_PLAN.md)"
rfc_required: none        # core 数学库类型面(Vec/Mat/swizzle/几何原语)/ image-io 包接口 / G0 软光栅 kernel / UC-03 demo / rx watch 是对 01/08/11 已锁定决策(UC-03 旗舰用例 / stdlib 充实 / G0 软光栅 demo)的条款化与工程实现:纯追加、尚无 stable 面;任何偏离已锁定决策的语义动作(尤其触及 const 泛型值运行期单态化 RD-007 / 软光栅 unsafe 逃生)按 10 §3 升档,判档争议向上取严
upstream_docs:
  - "11 §3 (M7 定义,标准库充实与 G0 图形演示;G0 软光栅 demo 穿插 M7-M8)"
  - "01 §6 (UC-03 旗舰用例:SPH 仿真 + 软光栅出图)"
  - "08 §4 §5 §6 (rx bench harness / Natvis 可视化 / 热重载 rx watch)"
  - "07 §4 §6 §7 (保守先行 / 编译性能预算 / device codegen 作用面——软光栅 kernel 全 safe 目标)"
  - "14 (契约/预算/deferred/证据分级/测试纪律/基准协议)"
in_scope:
  - core_math_stdlib        # core 数学库定型:Vec/Mat/swizzle/几何原语(点/向量/法线/AABB/射线等),全 safe API,host+device 双路径(11 §3 M7;08 §5)
  - image_io_pkg            # image-io 包:确定性图像序列输出(PPM/PNG 等无损格式优先),供 UC-03 demo 出图与软光栅落盘
  - g0_soft_raster          # G0 compute 软光栅:binning / tile 光栅 / 深度 / tonemap kernel,全 safe 代码目标(unsafe 落点+原因入 safe 覆盖率报告反哺 views 扩展清单,11 §3 M7)
  - uc03_demo_sph           # UC-03 验收 demo:SPH 仿真 + 软光栅出图,单 EXE 分发 + 确定性输出图像序列(01 §6 旗舰用例)
  - kernel_hot_reload       # kernel 热重载 rx watch(08 §6:源变更→重编译→重载,开发期迭代体验;CPU/host 编排,非验收硬门)
  - spec_m7_clauses         # spec core 数学库类型面 / image-io 接口 / 软光栅 kernel 语义面条款(新建 spec/stdlib.md,RXS-0104 续号,规范先行;条款 PR 先于实现 PR)
out_of_scope:
  - release_chain           # 发布链路(rurixup/MSI/winget + 签名/SBOM/许可审计 + artifact 上传)→ RD-001/M8(08 §9 / 11 §3 M8);UC-03 demo 单 EXE 为本地构建产物,不打包再分发
  - cubin_fatbin_dist       # libdevice 真分发 / 生产分发 fatbin(按架构 cubin + PTX fallback)→ G1(07 §7 / RD-001 系);M7 维持 PTX-only 开发期产物
  - registry_sumdb          # registry(sparse index + sumdb 透明日志 + OIDC/Sigstore)→ 所有者决策点 D-312(09 §7.3 阶段三;SG registry 触发条件未满足)
  - realtime_window_present # 软光栅 demo 升级为实时窗口呈现 → G1-1(11 §4);M7 只到离线出图图像序列
  - uc01_uc02_demos         # UC-01(PyTorch 算子替换)/ UC-02 → M8 互操作与收口(11 §3 M8)
  - bilingual_diagnostics   # 诊断消息中英双语全量覆盖 → RD-006/M8(MVP 收口前发布门)
  - const_generic_value_mono # const 泛型值运行期单态化(RD-007)随 device codegen 进一步扩展评估接通——非本契约验收门;M7 几何原语/数组长度类 const 泛型若触发则按需接通或继续留痕
  - advanced_gpu_intrinsics # Tensor Core/WGMMA/TMA / cluster / 动态并行 / cooperative groups 永久裁剪(11 §2 红线,SG-001~SG-009 维持 not_triggered)
deferred_refs: [RD-007]   # RD-007(const 泛型值运行期单态化,owner M6→M7 顺延评估,inherited;非本契约验收门,执行期处置留痕);M7 不预造新 deferred,执行期按需登记 RD-###(14 §4)
deliverables:
  - id: D-M7-1
    name: core 数学库定型(Vec/Mat/swizzle/几何原语,全 safe API,host+device 双路径)+ spec 条款先行(新建 spec/stdlib.md,RXS-0104 续号)
  - id: D-M7-2
    name: image-io 包(确定性图像序列输出,无损格式优先)+ spec 条款(图像 IO 接口语义面)
  - id: D-M7-3
    name: G0 compute 软光栅 kernel(binning / tile 光栅 / 深度 / tonemap),全 safe 代码目标 + spec 条款(软光栅 kernel 语义面)
  - id: D-M7-4
    name: UC-03 验收 demo(SPH 仿真 + 软光栅出图,单 EXE 分发 + 确定性输出图像序列)(G-M7-1)
  - id: D-M7-5
    name: 软光栅 L3 基准入库(measured_local,M7.5 回填;direction/阈值 M7.x 裁定)(G-M7-2)
  - id: D-M7-6
    name: safe 覆盖率报告(软光栅 kernel safe 覆盖 + unsafe 落点·原因反哺 views 扩展清单)+ kernel 热重载 rx watch + 工具链 conformance / traceability 延续(G-M7-3 / G-M7-5)
acceptance_gates:
  - id: G-M7-1
    check: "UC-03 验收 demo 单 EXE 分发 + 确定性输出图像序列:SPH 仿真 + 软光栅出图经 rx build 产单 EXE,运行输出确定性图像序列(image-io 落盘,逐帧 content SHA-256 在固定输入/随机种子下两次运行逐字节一致);CI 批跑断言计数 m7.counter.uc03_demo_image_sequence ≥1 份图像序列证据(11 §3 M7 验收门 / 01 §6 UC-03)。激活经真实红绿验证(篡改一帧像素/破坏 demo 管线 → 图像序列校验红 → 复原转绿,run URL 归档,反 YAML-only)"
  - id: G-M7-2
    check: "软光栅 L3 基准入库(预算项实测):G0 compute 软光栅在 L3 规模场景(大三角形/大分辨率帧)上的帧时间/吞吐,采样按 milestones/m0/BENCH_PROTOCOL.md 协议化(L0 锁频前置 / 三次进程级独立运行 / trimmed mean);预算断言 m7.bench.soft_raster_l3_frame_ms,evidence_level=measured_local,direction 与阈值于 M7.5 裁定回填(estimated → measured_local),close-out 跑 budget_eval --strict 通过(本占位在 M7 内生灭,不跨里程碑欠债,14 §3)"
  - id: G-M7-3
    check: "safe 覆盖率报告:G0 软光栅 kernel(binning/tile 光栅/深度/tonemap)以全 safe 代码为目标,safe 覆盖计数 m7.counter.soft_raster_kernels_safe ≥ 预设软光栅 kernel 数(数量为 estimated 工程选择,增删经 Direct PR 留痕,对齐 G-M3-1/G-M4-2/G-M5-2 计数器先例);凡落 unsafe 的 kernel 须每 unsafe 块 // SAFETY: 并在 safe 覆盖率报告留痕原因(反哺 views 扩展清单,11 §3 M7)。CI 批跑(软光栅 kernel 冒烟步骤),失败即红"
  - id: G-M7-4
    check: "core 数学库 conformance:Vec/Mat/swizzle/几何原语在 host+device 双路径端到端真跑(构造/算术/swizzle/矩阵乘/几何谓词正确性),核心原语覆盖计数 m7.counter.math_primitives ≥ 预设核心集数(数量为 estimated 工程选择,增删经 Direct PR 留痕)。CI 批跑(数学库 conformance 冒烟步骤),失败即红"
  - id: G-M7-5
    check: "traceability 延续:M7 新增 RXS 条款(core 数学库类型面 / image-io 接口 / 软光栅 kernel 语义面,新建 spec/stdlib.md RXS-0104 续号)每条 ≥1 测试锚定(ci/trace_matrix.py 全局口径,沿用 m1.counter.spec_clause_test_anchoring)"
guardrails:
  - "milestones/m0~m6 的 measured_local 既有预算条目 git diff 0-byte(新增条目允许)"
  - "milestones/m0~m6 的 M*_CONTRACT.md(均 closed)既有内容只追加不修改"
  - "registry/deferred.json 与 registry/spike_gating.json 只追加(既有条目修改触发人工审查);RD-007 仅允许 inherited→closed 的状态留痕追加(owner M6→M7 顺延为 inherited 生命周期既定动作);SG 复评只追加 decisions"
  - "registry/error_codes.json 错误码语义可加不可改(M1.1 已激活);M7 新段位(stdlib/数学库/image-io/软光栅工具链诊断)首批分配随 M7.1+ 诊断 PR 留痕,段位分配制递增、含义冻结"
  - "evidence/ 只增不删不改"
  - "00–14 共 15 份规划文档不被执行 PR 改写(勘误走 00 §6.3 追加式修订)"
  - "tests/ui/ 的 .stderr snapshot 变更必须经审批 bless(M1.4 已激活,check_ui_bless)"
  - "tests/mir/ 的 .mir golden 变更必须经审批 bless(M3.3 WP6 已激活,check_mir_bless)"
  - "tests/ptx/ 的 IR golden 变更必须经审批 bless(M4.2 已激活,check_ptx_bless);软光栅 kernel codegen 形态变更纳入既有 PTX/IR golden 核对"
  - "spec/ 变更必须携带变更档位标记(M1.2 已激活);spec/stdlib.md 新建 + RXS-0104 续号,条款 PR 先于实现 PR,每条 ≥1 测试锚定(G-M7-5)"
  - "src/rurix-rt 的 unsafe 边界维持 undocumented_unsafe_blocks=deny(M4.3 已激活,每 unsafe 块 // SAFETY:);全仓其余 crate 维持 unsafe_code=deny;core 数学库/image-io/软光栅/demo 新 crate 默认 unsafe_code=deny;软光栅全 safe 代码目标,凡落 unsafe 须注册条目 + safe 覆盖率报告留痕(G-M7-3)"
  - "guardrail 核对基准维持 m6-closed(M6 close-out 已完成 m5-closed→m6-closed 切换,M7 开工无需再切;PR 路径仍以 GITHUB_BASE_REF 为准);若 M7 期需再切按 check_* 守卫风格 + 双基准核对"
  - "Compute Sanitizer racecheck+memcheck nightly 维持全绿(M5.4 已激活);软光栅 device kernel 落地后纳入既有 nightly 全跑"
  - "本契约 in_scope/acceptance_gates 等既有条款 0-byte 修改,close-out 只追加"
---

# M7 契约 — 标准库充实与 G0 图形演示(core 数学库 / image-io / 软光栅 / UC-03 demo)

> 所属:[../../11_ROADMAP.md](../../11_ROADMAP.md) §3 M7 / 契约机制见 [../../14_ENGINEERING_DISCIPLINE.md](../../14_ENGINEERING_DISCIPLINE.md) §1
> 规范先行延续(AGENTS.md 硬规则第 7 条):core 数学库 / image-io / 软光栅 kernel 的语义面 PR 必须引用 RXS-#### 条款号(新建 `spec/stdlib.md`,RXS-0104 续号);缺条款先补 spec,**条款 PR 先于实现 PR**。
> 基准 ref:**维持 `m6-closed`**(M6 close-out 已完成 `m5-closed → m6-closed` 切换,M7 开工**无需再切基准**;`ci/check_guardrails.py` 无参默认 = `m6-closed`,PR 路径仍以 `GITHUB_BASE_REF` 为准)。

---

## 1. 目标

把 Rurix 从 M6 的"可用工具链与包管理"(rx CLI 总入口 + 核心子命令 + 声明式包管理 + LSP MVP)推进到 **标准库充实与 G0 图形演示**:交付 **core 数学库定型**(Vec/Mat/swizzle/几何原语,全 safe API,host+device 双路径,08 §5);落下 **image-io 包**(确定性图像序列输出);兑现 **G0 compute 软光栅**(binning / tile 光栅 / 深度 / tonemap kernel,**全 safe 代码目标**——凡落 unsafe 须留痕原因反哺 views 扩展清单);接通 **UC-03 验收 demo**(SPH 仿真 + 软光栅出图,**单 EXE 分发 + 确定性输出图像序列**,01 §6 旗舰用例);提供 **kernel 热重载 `rx watch`** 开发期迭代体验。M7 结束时兑现三条硬证据:**UC-03 demo 单 EXE 分发 + 确定性图像序列**、**软光栅 L3 基准入库(measured_local)**、**safe 覆盖率报告**(哪些 kernel 落了 unsafe 及原因)——这是"Rurix 从'可日常开发'走向'能跑出旗舰图形用例'"的里程碑(G0 软光栅 demo 穿插 M7-M8)。

## 2. 范围

### 2.1 in-scope

| 项 | 说明 | 对应交付物 |
|---|---|---|
| core 数学库 | Vec/Mat/swizzle/几何原语(点/向量/法线/AABB/射线等),全 safe API,host+device 双路径(11 §3 M7;08 §5) | D-M7-1 |
| image-io 包 | 确定性图像序列输出(PPM/PNG 等无损格式优先),供 UC-03 出图与软光栅落盘 | D-M7-2 |
| G0 软光栅 | binning / tile 光栅 / 深度 / tonemap kernel,**全 safe 代码目标**(unsafe 落点+原因入 safe 覆盖率报告,11 §3 M7) | D-M7-3 |
| UC-03 demo | SPH 仿真 + 软光栅出图,单 EXE 分发 + 确定性输出图像序列(01 §6 旗舰用例) | D-M7-4 |
| kernel 热重载 | `rx watch`:源变更→重编译→重载(08 §6 开发期迭代体验;host/CPU 编排,非验收硬门) | D-M7-6 |
| spec M7 条款 | core 数学库类型面 / image-io 接口 / 软光栅 kernel 语义面 spec 条款(新建 `spec/stdlib.md`,RXS-0104 续号,FLS 体例);**条款 PR 先于实现 PR** | D-M7-1 ~ D-M7-3 |

### 2.2 out-of-scope(显式排除)

- 发布链路(`rurixup` 引导 + MSI + winget + 签名/SBOM/许可审计 + artifact 上传)——→ RD-001/M8(08 §9 / 11 §3 M8);UC-03 demo 单 EXE 为本地构建产物,**不打包再分发**。
- libdevice 真分发 / 生产分发 fatbin(按架构预编 cubin + 保守 PTX fallback)——→ G1(07 §7 / RD-001 系);M7 维持 **PTX-only 开发期产物**。
- registry(sparse index + sumdb 式透明日志 + scopes/OIDC trusted publishing/Sigstore)——→ 所有者决策点 **D-312**(09 §7.3 阶段三;[../../registry/spike_gating.json](../../registry/spike_gating.json) registry 方向触发条件未满足)。
- 软光栅 demo 升级为**实时窗口呈现**——→ G1-1(11 §4 CUDA–D3D12 interop);M7 只到**离线出图图像序列**。
- UC-01(PyTorch 算子替换)/ UC-02——→ M8 互操作与 MVP 收口(11 §3 M8)。
- 诊断消息中英双语全量覆盖——→ RD-006/M8(MVP 收口前发布门)。
- const 泛型值运行期单态化(RD-007)随 device codegen 进一步扩展评估接通——**非本契约验收门**;M7 几何原语 / 数组长度类 const 泛型若触发则按需接通或继续留痕(执行期处置)。
- 11 §2 MVP 红线清单全部不触碰:Tensor Core/WGMMA/TMA intrinsics、cluster、动态并行、cooperative groups([../../registry/spike_gating.json](../../registry/spike_gating.json) SG-001 ~ SG-009 维持 not_triggered)。

## 3. 交付物清单

| ID | 交付物 | 形态 | 完成判据 |
|---|---|---|---|
| D-M7-1 | core 数学库 | Vec/Mat/swizzle/几何原语(全 safe API,host+device 双路径)+ spec 条款(新建 spec/stdlib.md,RXS-0104 续号) | G-M7-4 + G-M7-5;host 回归网持续绿 |
| D-M7-2 | image-io 包 | 确定性图像序列输出(无损格式优先)+ spec 条款 | G-M7-1 子集(demo 出图落盘) |
| D-M7-3 | G0 软光栅 kernel | binning / tile 光栅 / 深度 / tonemap,全 safe 代码目标 + spec 条款 | G-M7-3 + G-M7-1 |
| D-M7-4 | UC-03 demo | SPH 仿真 + 软光栅出图,单 EXE 分发 + 确定性输出图像序列 | G-M7-1 |
| D-M7-5 | 软光栅 L3 基准 | L3 规模软光栅帧时间/吞吐 measured_local 入库(M7.5 回填) | G-M7-2 |
| D-M7-6 | safe 覆盖率报告 + rx watch + 工具链 conformance | safe 覆盖率报告(unsafe 落点+原因)+ rx watch 热重载 + conformance/traceability 延续 | G-M7-3 + G-M7-5 |

## 4. 验收门(完整版,YAML 头为可提取摘要)

1. **G-M7-1(UC-03 demo 单 EXE 分发 + 确定性输出图像序列)**:SPH 仿真 + 软光栅出图经 `rx build` 产**单 EXE**,运行输出**确定性图像序列**(image-io 落盘,固定输入/随机种子下逐帧 content SHA-256 两次运行逐字节一致);CI 批跑断言 `m7.counter.uc03_demo_image_sequence ≥1`(11 §3 M7 验收门 / 01 §6 UC-03)。激活经**真实红绿验证**(篡改一帧像素 / 破坏 demo 管线 → 图像序列校验红 → 复原转绿,run URL 归档,反 YAML-only)。
2. **G-M7-2(软光栅 L3 基准入库 — measured_local)**:G0 compute 软光栅在 L3 规模场景(大三角形 / 大分辨率帧)上的帧时间 / 吞吐,采样按 [../m0/BENCH_PROTOCOL.md](../m0/BENCH_PROTOCOL.md) 协议化(L0 锁频前置 / 三次进程级独立运行 / trimmed mean);预算断言 `m7.bench.soft_raster_l3_frame_ms`,`evidence_level=measured_local`,`direction` 与阈值于 **M7.5 裁定回填**(`estimated → measured_local`),close-out 跑 `budget_eval --strict` 通过(本占位在 M7 内生灭,不跨里程碑欠债,14 §3)。
3. **G-M7-3(safe 覆盖率报告)**:G0 软光栅 kernel(binning/tile 光栅/深度/tonemap)以**全 safe 代码**为目标,safe 覆盖计数 `m7.counter.soft_raster_kernels_safe ≥` 预设软光栅 kernel 数(数量为 estimated 工程选择,增删经 Direct PR 留痕,对齐 G-M3-1/G-M4-2/G-M5-2 计数器先例);凡落 unsafe 的 kernel 须每 unsafe 块 `// SAFETY:` 并在 safe 覆盖率报告留痕原因(**反哺 views 扩展清单**,11 §3 M7)。CI 批跑(软光栅 kernel 冒烟步骤),失败即红。
4. **G-M7-4(core 数学库 conformance)**:Vec/Mat/swizzle/几何原语在 **host+device 双路径**端到端真跑(构造 / 算术 / swizzle / 矩阵乘 / 几何谓词正确性);核心原语覆盖计数 `m7.counter.math_primitives ≥` 预设核心集数(数量为 estimated 工程选择,增删经 Direct PR 留痕)。CI 批跑(数学库 conformance 冒烟步骤)。
5. **G-M7-5(traceability 延续)**:M7 新增 RXS 条款(新建 `spec/stdlib.md` RXS-0104 续号:core 数学库类型面 / image-io 接口 / 软光栅 kernel 语义面)每条 ≥1 测试锚定;`ci/trace_matrix.py` 全局口径核对(`m1.counter.spec_clause_test_anchoring` 全局断言,无需另立 m7 计数器)。

## 5. Guardrails(字节级,机器核对)

见 YAML 头 `guardrails` 字段。核对方式:`ci/check_guardrails.py [基准ref]`(**默认基准维持 `m6-closed`**,M6 close-out 已完成 `m5-closed → m6-closed` 切换,M7 开工无需再切;PR 路径仍以 `GITHUB_BASE_REF` 为准)。M7 期计划动作:**(1)新段位错误码首批分配**(stdlib/数学库/image-io/软光栅工具链诊断,随 M7.1+ 诊断 PR 留痕,分配制递增、含义冻结);**(2)软光栅 unsafe-audit**(全 safe 代码目标,凡落 unsafe 须注册条目 + safe 覆盖率报告留痕,G-M7-3);**(3)软光栅 device kernel** 纳入既有 Compute Sanitizer nightly 全跑;**(4)软光栅 kernel codegen 形态**纳入既有 PTX/IR golden 核对。M0~M6 历史预算的回填/冻结与既有 bless/spec/error_codes guardrail 走既有机制,无需新代码。若 M7 期需再切基准按 `check_*` 守卫风格 + 双基准核对。

## 6. Deferred 引用

| 编号 | 内容摘要 | 承接 |
|---|---|---|
| RD-007 | const 泛型值运行期单态化(turbofish const 实参 → 实例值代入 + codegen)+ 运行期数组 aggregate codegen | M7(M6 close-out owner M6→M7 顺延,inherited;M7 标准库充实与 G0 图形演示作用面——几何原语 / 数组长度类 const 泛型可能触发运行期单态化,届时按需接通或继续留痕,spec/consteval.md RXS-0064 语义不变,回填仅补实现侧。**非本契约验收门**,接通与否执行期处置留痕) |
| RD-008 | scoped atomics 的 PTX `atom.{order}.{scope}` 映射 codegen 实现(D-406 人工落笔);M5 仅交付类型契约 + RX3010 + `atomics_ptx_mapping.rs` 骨架(`#[ignore]` + panic),映射实现待真实 kernel 需要 scoped atomic codegen 时人工落笔 + Compute Sanitizer 背书 | M7(2026-06-15 M0–M6 执行期审查登记,owner M7,open;**D-406 人工落笔禁区**,AI 不实现 PTX 映射,backfill 时只追加 deferred.json history、不静默改既有字段。**非本契约验收门**) |

详情以 [../../registry/deferred.json](../../registry/deferred.json) 为唯一事实源,本表仅引用。RD-001(M8)/RD-006(M8)不属 M7 范围,维持原承接;RD-002/RD-003/RD-004/RD-005 已 closed。M7 开工无预造新 deferred;RD-008 系 2026-06-15 M0–M6 执行期审查按 14 §4 追加(scoped atomics PTX 映射未交付),双侧标注于 `deferred.json` v1.8;后续执行期做不完的事同样按 14 §4 追加 `RD-###` 并双侧标注。

## 7. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-06-15 | 初版契约固化(M7 开工脚手架;基准 ref 维持 m6-closed 无需再切;deferred RD-007 owner M6→M7 顺延承接、维持 inherited;新建 spec/stdlib.md RXS-0104 续号预留,条款体随 M7.1+ 与测试同 PR;新段位错误码首批分配随 M7.1+ 诊断 PR) |
| v1.1 | 2026-06-15 | §6 只追加 RD-008 引用行(scoped atomics PTX `atom.{order}.{scope}` 映射 codegen,2026-06-15 M0–M6 执行期审查登记,owner M7,open,D-406 人工落笔禁区;事实源 deferred.json v1.8);既有 RD-007 行 0-byte 不动 |

---

## 8. Close-out(只追加区 — 开工时为空)

<!-- 验收记录、guardrail 核对输出、deferred 继承/关闭记录、UC-03 demo 图像序列红绿留痕、软光栅 L3 基准 measured_local 证据追加于此;上方条款 0-byte 修改。 -->
