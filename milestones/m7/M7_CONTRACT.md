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

详情以 [../../registry/deferred.json](../../registry/deferred.json) 为唯一事实源,本表仅引用。RD-001(M8)/RD-006(M8)不属 M7 范围,维持原承接;RD-002/RD-003/RD-004/RD-005 已 closed。M7 开工无预造新 deferred;执行期做不完的事按 14 §4 追加 `RD-###` 并双侧标注。

## 7. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-06-15 | 初版契约固化(M7 开工脚手架;基准 ref 维持 m6-closed 无需再切;deferred RD-007 owner M6→M7 顺延承接、维持 inherited;新建 spec/stdlib.md RXS-0104 续号预留,条款体随 M7.1+ 与测试同 PR;新段位错误码首批分配随 M7.1+ 诊断 PR) |

---

## 8. Close-out(只追加区 — 开工时为空)

<!-- 验收记录、guardrail 核对输出、deferred 继承/关闭记录、UC-03 demo 图像序列红绿留痕、软光栅 L3 基准 measured_local 证据追加于此;上方条款 0-byte 修改。 -->

> **草拟声明(M7.5,档位 Direct)**:本 §8 由 M7.5 PR 草拟(契约 `rfc_required: none`,基准回填 / 报告 / 收官为已锁定决策的工程动作)。**M7 close-out 关闭判定、`m6-closed → m7-closed` 基准切换、`m7-closed` tag 由白栀 / owner 人工签署兑现**,AI 不代签;下方 §8.10 留人工签署位。本轮**未新增 RXS 条款**(L3 基准为工程编排,复用 RXS-0118~0121 kernel 语义);**未触及禁区**(未改 device kernel 签名 / RXS-0118~0121 PTX golden / spec 既有条款本体 / closed 契约 / plan 文档 / error_codes 既有含义)。

### 8.1 G-M7-2 软光栅 L3 基准 measured_local 回填(D-M7-5)

- **口径**:G0 软光栅四 stage device kernel(binning / tile 光栅 / 深度 / tonemap,RXS-0118~0121)在单个 CUDA Event 计时区内按 L3 规模背靠背 launch,`frame_ms` = 四 stage 串行 GPU 墙钟;**kernel 签名不改**(M7.3 PTX golden 冻结,改签名属禁区)。harness:[../../bench/sr_pipeline_bench.py](../../bench/sr_pipeline_bench.py)(`rx bench sr_pipeline` 经 cmd_bench 收编,RD-003);三次进程级独立运行 + 回填:[../../bench/sr_pipeline_triple.py](../../bench/sr_pipeline_triple.py)。
- **L3 规模**(契约"大三角形 / 大分辨率帧"):帧 1920×1080;binning 4096 图元 / 240×135 tiles cap32;raster 大三角形铺满全帧;depth 2073600 像素 ×4 片元;tonemap 6220800 分量。
- **协议**(BENCH_PROTOCOL §3):L0 锁频前置(SM 2610 / MEM 10501,`nvidia-smi -lgc/-lmc` elevated)→ warmup/稳态(steady CV 0.0107)→ 50×3 timed(CUDA Event,L2 清理)→ 三次进程级独立运行各 trimmed mean 再 trimmed mean;**任一次非 measured_local 整组作废**(unlocked 不得回填)。
- **实测**:三次独立运行 trimmed mean = 1.2107 / 1.2259 / 1.2299 ms;聚合 trimmed mean = **1.2222 ms**(CV 0.0083),证据 [../../evidence/sr_l3_20260616_agg.json](../../evidence/sr_l3_20260616_agg.json)(+ `_1/_2/_3.json`,均 `evidence_level=measured_local`,`bench.level=L3`,`correctness_check=pass`)。
- **阈值裁定(Direct)**:`direction=max`(帧时间越小越好,阈值为上界);`threshold = 实测 × 安全系数 1.5 = 1.8333 ms`(50% 回归裕度,远低于行业线软光栅 L3 帧天花板)。预算条目 [m7_budget.json](m7_budget.json) `m7.bench.soft_raster_l3_frame_ms` 由 `estimated` 占位转 `measured_local`(单向合法转换,guardrail `check_budget`)。**安全系数与阈值口径待 owner 终审确认**。

### 8.2 `budget_eval --strict` 输出(全局零 estimated 残留)

命令:`py -3 ci/budget_eval.py --strict`(2026-06-16,RTX 4070 Ti 开发机锁频采样)。判定:`m7.bench.soft_raster_l3_frame_ms` measured_local 转 PASS,**全局零 estimated 残留**(strict 模式 0 skip),三计数器 G-M7-1/G-M7-3/G-M7-4 维持 PASS。

```
  PASS m7.bench.soft_raster_l3_frame_ms: PASS — 1.222 ms vs max 1.8333
  PASS m7.counter.math_primitives: PASS — 15 个 core 数学库原语端到端(要求 ≥8)
  PASS m7.counter.soft_raster_kernels_safe: PASS — 4 个 safe 软光栅 kernel(要求 ≥4)
  PASS m7.counter.uc03_demo_image_sequence: PASS — 1 份 UC-03 demo 图像序列证据(要求 ≥1)
[budget_eval] PASS (46 pass, 0 skip, strict mode)
```

### 8.3 Guardrail / traceability / schema 核对输出

```
$ py -3 ci/trace_matrix.py --check
[trace_matrix] PASS (121/121 clauses anchored, 379 test files scanned)

$ py -3 ci/check_schemas.py
[check_schemas] PASS

$ py -3 ci/check_guardrails.py            # 默认基准 m6-closed
[check_guardrails] PASS (base=m6-closed, 77 changed paths)
```

- `check_guardrails` 含 `check_ptx_bless`:本轮**未改** `tests/ptx/**/*.nvptx`(软光栅 kernel codegen 形态不变),PTX golden 无须 bless。
- `check_budget`:`m7.bench.soft_raster_l3_frame_ms` estimated→measured_local 为合法单向转换;m0~m6 既有 measured_local 条目 0-byte。
- `check_evidence`:`evidence/sr_l3_*.json` 为新增(只增不删不改)。
- 工具链回归:`cargo fmt --all --check` PASS / `cargo clippy --workspace --all-targets -- -D warnings`(pin 1.93.1)PASS / `cargo test --workspace` 全绿(既有回归网不退化)。

### 8.4 G-M7-1 UC-03 demo 单 EXE + 确定性图像序列

- 判据:`rx build` / `cargo build -p uc03-demo` 产 host 单 EXE,运行输出确定性图像序列(12 帧,逐帧 content SHA-256 两次运行逐字节一致),计数 `m7.counter.uc03_demo_image_sequence ≥1` PASS。证据 [../../evidence/uc03_demo_smoke.json](../../evidence/uc03_demo_smoke.json)(`image_sequence_ok=true`)。
- 真实红绿(反 YAML-only):篡改一帧像素 R/B 通道 → content SHA-256 改变(红);复原 → `ci/uc03_demo_smoke.py` exit 0(绿)。本地红绿已验证;**PR Smoke 步骤 32 self-hosted runner red/green run URL 见 PR #44**。判定:**PASS**(M7.4 落地)。

### 8.5 G-M7-2 软光栅 L3 基准入库 — measured_local

判据见 §8.1 / §8.2:measured_local 1.2222 ms ≤ threshold 1.8333 ms,`budget_eval --strict` PASS,全局零 estimated。占位在 M7 内生灭(14 §3)。harness 真跑(非 YAML-only):四 stage 逐 stage 向量化 host 参考一致(correctness PASS)。判定:**PASS**(待 owner 确认安全系数口径)。

### 8.6 G-M7-3 safe 覆盖率报告

- 判据:G0 软光栅 kernel 全 safe 覆盖 4/4,`m7.counter.soft_raster_kernels_safe ≥4` PASS;unsafe 落点为空(`unsafe_kernels: []`)。报告:[SAFE_COVERAGE_REPORT.md](SAFE_COVERAGE_REPORT.md);机器证据 [../../evidence/soft_raster_smoke.json](../../evidence/soft_raster_smoke.json)。
- 本轮零 unsafe → `unsafe-audit/` 无新增条目、views 扩展清单无反哺项(留痕见报告 §4)。
- 真实红绿(反 YAML-only):篡改帧像素 R/B 通道 → SHA 改变(红);复原 → `ci/soft_raster_smoke.py` exit 0(绿)。本地红绿已验证;**PR Smoke 步骤 31 run URL 见 PR #43**。判定:**PASS**。

### 8.7 G-M7-4 core 数学库 conformance

判据:Vec/Mat/swizzle/几何原语 host+device 双路径端到端真跑,核心原语覆盖 `m7.counter.math_primitives` = 15 ≥ 预设 8,PASS。证据 [../../evidence/stdlib_math_smoke.json](../../evidence/stdlib_math_smoke.json)。判定:**PASS**(M7.1 落地)。

### 8.8 G-M7-5 traceability 延续

判据:M7 新增 RXS 条款(spec/stdlib.md RXS-0104~0117 / spec/softraster.md RXS-0118~0121)每条 ≥1 测试锚定;`ci/trace_matrix.py --check` = 121/121 全锚定(`m1.counter.spec_clause_test_anchoring` PASS)。**M7.5 本轮未新增 RXS 条款**(L3 基准 / 报告 / 收官为工程动作)。判定:**PASS**。

### 8.9 Deferred 继承 / 关闭记录

| RD | 状态 | M7 触发 | 处置 |
|---|---|---|---|
| RD-007 | 维持 `inherited`(owner M7) | M7 未触发 | const 泛型值运行期单态化:M7.1~M7.5(数学库 / image-io / 软光栅 / UC-03 demo / L3 基准)均用固定维度常量实现,未依赖 turbofish const 实参实例值代入,**非 M7 验收门**,M7 未接通;承接随后续 device codegen / 运行期数组 aggregate codegen 扩展评估接通(M8 开工按先例顺延),RXS-0064 语义不变。对齐 M5/M6 close-out 顺延先例([../../registry/deferred.json](../../registry/deferred.json) history 2026-06-16 / revision_log v1.8) |

RD-001 / RD-006 维持 open(M8);RD-002 / RD-003 / RD-004 / RD-005 维持 closed。M7 开工无预造新 deferred,执行期无新增 RD-###。

### 8.10 验收门汇总 + 关闭判定(人工签署位)

| 通道 | 判据 | 现状 | 背书 |
|---|---|---|---|
| G-M7-1 | UC-03 demo 单 EXE + 确定性图像序列 ≥1 | PASS | §8.4 / `m7.counter.uc03_demo_image_sequence` |
| G-M7-2 | 软光栅 L3 measured_local + budget_eval --strict | PASS | §8.1 §8.2 / `m7.bench.soft_raster_l3_frame_ms` |
| G-M7-3 | safe 覆盖 ≥4 + 报告留痕 | PASS | §8.6 / `m7.counter.soft_raster_kernels_safe` / SAFE_COVERAGE_REPORT.md |
| G-M7-4 | 数学库原语 ≥8 双路径 | PASS | §8.7 / `m7.counter.math_primitives` |
| G-M7-5 | RXS 条款 ≥1 测试锚定 | PASS | §8.8 / `m1.counter.spec_clause_test_anchoring` |

stacked PR 链(均待合入 main):#41(M7.1)→ #42(M7.2)→ #43(M7.3)→ #44(M7.4)→ M7.5(本 PR,base=feat/m7.4-uc03-demo)。

**以下由白栀 / owner 人工兑现(AI 不代签)**:

- [ ] M7 close-out 关闭判定(G-M7-1~G-M7-5 全绿确认 + §8.1 安全系数 / 阈值口径终审)。
- [ ] 基准 ref 切换 `m6-closed → m7-closed`(M8 开工基准;按 `check_*` 守卫风格 + 双基准核对)。
- [ ] 打 `m7-closed` tag。
- [ ] RD-007 owner_milestone M7→M8 顺延(M8 开工承接留痕)。
- [ ] 各验收门 PR Smoke self-hosted runner 红绿 run URL 归档(#43 步骤 31 / #44 步骤 32 / 本 PR L3 基准)。
- 签署:____________(白栀 / owner) 日期:__________
