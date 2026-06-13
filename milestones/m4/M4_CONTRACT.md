---
contract: M4
title: device codegen 与运行时——第一个 Rurix kernel 上 GPU
status: active            # active → closed(close-out 只追加,既有条款 0-byte 修改)
version: v1.0
date: 2026-06-13
timebox: "M+7 ~ M+9(约 8 周,两级结构见 M4_PLAN.md)"
rfc_required: none        # device codegen/着色/地址空间/launch 语义条款是对 06/07/08/13 已锁定决策(D-120/D-121/D-123/D-205/D-207)的条款化:纯追加、尚无 stable 面;任何偏离已锁定决策的语义动作按 10 §3 升档,判档争议向上取严
upstream_docs:
  - "11 §3 (M4 定义,MVP 中点硬证据)"
  - "06 §1 §2 §3 §4 §5 (执行模型 / kernel 抽象 D-120 / 内存空间 D-121 / 同步与 PTX 内存模型 D-123 / 运行时对象与装载协商)"
  - "07 §3 §5 §7 (着色与地址空间检查在 HIR 层 / 四条黄金路径之第四条 / device codegen MIR→LLVM(NVPTX)→PTX D-205·D-207 与 ptxas 干验证关卡)"
  - "08 §2 (运行时 Context/Stream/Buffer/launch / 启动协商 / poisoned 状态机)"
  - "14 (契约/预算/deferred/证据分级/测试纪律/基准协议)"
in_scope:
  - coloring_addrspace_check    # host/device/kernel 着色 + 地址空间检查(HIR 层,07 §3;3xxx 错误码首批)
  - nvptx_codegen              # MIR→LLVM(NVPTX 约束子集)→PTX(ptx_kernel 调用约定 / addrspace 0·1·3·4·5 / sreg intrinsics,r2 第一阶段范围;LLVM pin 22.1.x)
  - ptxas_dry_gate             # 生成 PTX 过 ptxas -arch=sm_89 干验证关卡(strict-only:ptxas 拒绝=RX6xxx 编译错误,07 §7)
  - runtime_context_launch     # 运行时 Context/Stream/Buffer/launch 经典内存路径 + 装载协商(PTX .version 比对) + poisoned 状态机(06 §5 / 08 §2)
  - embedded_ptx_artifact      # PTX 嵌入 host 产物 data 段的单可执行产物(cuModuleLoadDataEx JIT 装载)
  - ui_golden_path4            # 黄金路径 4 = 目标后端错误(3xxx 着色/地址空间 + 6xxx codegen/ptxas;07 §5 第四条黄金路径)
  - spec_device_clauses        # spec device codegen/着色/地址空间/launch 类型契约条款(RXS-#### 续号,规范先行)
out_of_scope:
  - views_disjoint_barrier     # views 不相交证明 / shared let + barrier 一致性 = MIR 借用检查的 device 扩展 pass → M5(07 §4 / 11 §3 M5),本里程碑只保证 host 借用检查 pass 结构可扩展
  - scoped_atomics_mapping     # scoped atomics + PTX 映射层(D-406 禁区由人工落笔)→ M5(06 §4 / 11 §3 M5)
  - libdevice_link             # libdevice 链接(保留外部符号→链 bc→internalize→DCE→NVVMReflect)→ M5 按需(06 §7 / 07 §7)
  - gpu_parallel_primitives    # reduce/scan/transpose/tiled GEMM 自研 kernel + L1/L2 全量基准 → M5(RD-002)
  - cubin_fatbin_dist          # 生产分发 fatbin(按架构 cubin + PTX fallback)→ G1(07 §7 / RD-001 系)
  - async_buffer_streamorder   # 流序分配 AsyncBuffer<'stream> 类型契约 → G1(D-122,06 §3 §5.4)
  - const_generic_value_mono   # const 泛型值运行期单态化(RD-007,M3.4 遗留)+ 运行期数组 aggregate codegen 按需接通,非本契约验收门
  - advanced_gpu_intrinsics    # Tensor Core/WGMMA/TMA / cluster / 动态并行 / cooperative groups 永久裁剪(11 §2 红线,SG-001~SG-009 维持 not_triggered)
deferred_refs: [RD-007]        # M3.4 WP3 登记、owner_milestone=M4(const 泛型值单态化随 device codegen / 运行期数组 aggregate codegen 评估接通);M4 不预造新 deferred,执行期按需登记 RD-###(14 §4)
deliverables:
  - id: D-M4-1
    name: 着色 + 地址空间检查(host/device/kernel 边界 + addrspace 类型一致性,HIR 层;3xxx 错误码首批)
  - id: D-M4-2
    name: spec device 语义条款(着色/地址空间/NVPTX codegen/launch 类型契约,RXS-#### 续号,规范先行)
  - id: D-M4-3
    name: device codegen MIR→LLVM(NVPTX 子集)→PTX(ptx_kernel/addrspace/sreg) + ptxas 干验证关卡(RX6xxx) + NVPTX 雷区回归集起步
  - id: D-M4-4
    name: 运行时 Context/Stream/Buffer/launch + 装载协商 + poisoned 状态机 + PTX 嵌入单可执行产物
  - id: D-M4-5
    name: Rurix SAXPY 端到端上 GPU + measured_local ≥ M0 手写基线 95% 回填(中点硬证据)
  - id: D-M4-6
    name: launch 类型契约 conformance + 黄金路径 4(目标后端错误)
acceptance_gates:
  - id: G-M4-1
    check: "MVP 中点硬证据:Rurix 写的 SAXPY 经全管线产 PTX+EXE,RTX 4070 Ti 上 measured_local 有效带宽 ≥ M0 手写 PTX 基线(m0.bench.saxpy.effective_bandwidth_gbps)的 95%(三次进程级独立运行 trimmed mean,BENCH_PROTOCOL.md §3 协议 + bench/stats.py;evidence_level=measured_local,锁频降级证据不得回填);比值断言 m4.ratio.saxpy_vs_m0_baseline ≥ 0.95 在 close-out 跑 budget_eval --strict 通过"
  - id: G-M4-2
    check: "launch 类型契约 conformance:conformance/launch/reject/<category>/*.rx 反例全拦截(维度不匹配/参数类型不符/context-brand 不一致/对非 kernel 着色函数 launch 等,3xxx/6xxx 诊断)+ accept/ 正例 0 诊断,CI 批跑(m4.counter.launch_conformance_categories ≥4)"
  - id: G-M4-3
    check: "黄金路径 4(目标后端错误)snapshot ≥10(m4.counter.ui_golden_path4_snapshots;3xxx 着色/地址空间 + 6xxx codegen/ptxas 拒绝),走 M1.4 已激活的 bless 审批 guardrail"
  - id: G-M4-4
    check: "ptxas 干验证关卡真跑:合法 kernel 产出的 PTX 过 ptxas -arch=sm_89 通过;构造 ptxas 拒绝场景 → 编译期 RX6xxx 诊断(CI 自动核对,对齐真跑铁律)"
  - id: G-M4-5
    check: "traceability 延续:M4 新增 RXS 条款(着色/地址空间/codegen/launch)每条 ≥1 测试锚定(ci/trace_matrix.py 全局口径,沿用 m1.counter.spec_clause_test_anchoring)"
guardrails:
  - "milestones/m0/m0_budget.json、milestones/m1/m1_budget.json、milestones/m2/m2_budget.json 的 measured_local 既有条目 git diff 0-byte(新增条目允许)"
  - "milestones/m3/m3_budget.json 既有条目 0-byte(新增条目允许)"
  - "milestones/m0~m3 的 M*_CONTRACT.md(均 closed)既有内容只追加不修改"
  - "registry/deferred.json 与 registry/spike_gating.json 只追加(既有条目修改触发人工审查);RD-007 仅允许 open→inherited/closed 的状态留痕追加"
  - "registry/error_codes.json 错误码语义可加不可改(M1.1 已激活);3xxx/6xxx/7xxx 段位分配制递增、含义冻结"
  - "evidence/ 只增不删不改"
  - "00–14 共 15 份规划文档不被执行 PR 改写(勘误走 00 §6.3 追加式修订)"
  - "tests/ui/ 的 .stderr snapshot 变更必须经审批 bless(M1.4 已激活,check_ui_bless)"
  - "tests/mir/ 的 .mir golden 变更必须经审批 bless(M3.3 WP6 已激活,check_mir_bless)"
  - "spec/ 变更必须携带变更档位标记(M1.2 已激活)"
  - "guardrail 核对基准自 M4 开工切换 m2-closed → m3-closed(切换留痕 CI_GATES.md 修订表;PR 路径仍以 GITHUB_BASE_REF 为准)"
  - "NVIDIA 再分发白名单审计:device 路径开工时评估激活(14 §2 常驻集,M0~M3 标注 device 路径 M4 起评估的到期时点;结论入 CI_GATES.md §4)"
  - "PTX 文本 golden / NVPTX 雷区回归集:随 device codegen 定型评估挂 IR golden 机制(14 §2 / 07 §11;激活时点与红绿验证见 CI_GATES.md §4)"
  - "本契约 in_scope/acceptance_gates 等既有条款 0-byte 修改,close-out 只追加"
---

# M4 契约 — device codegen 与运行时(第一个 Rurix kernel 上 GPU)

> 所属:[../../11_ROADMAP.md](../../11_ROADMAP.md) §3 M4 / 契约机制见 [../../14_ENGINEERING_DISCIPLINE.md](../../14_ENGINEERING_DISCIPLINE.md) §1
> 规范先行延续(AGENTS.md 硬规则第 7 条):着色/地址空间/codegen/launch 语义 PR 必须引用 RXS-#### 条款号;缺条款先补 spec。
> 基准 ref:`m3-closed` tag 已随 M3 终审打出(2026-06-13);guardrail 核对基准自 M4 开工切换 `m2-closed → m3-closed`,切换留痕 [CI_GATES.md](CI_GATES.md) 修订表。

---

## 1. 目标

把 rurixc 从"host 全量安全检查"(M3 收口)推进到 **device 编译闭环 + 运行时装载**:在 HIR 层落下 host/device/kernel 着色与地址空间检查(07 §3),建成 device codegen 链路 MIR→LLVM(NVPTX 约束子集)→PTX(`ptx_kernel`/addrspace/sreg intrinsics,r2 第一阶段范围,D-205/D-207),设 ptxas 干验证关卡(生成的 PTX 过 `ptxas -arch=sm_89`,拒绝即 RX6xxx 编译错误),并交付运行时 Context/Stream/Buffer/launch 经典内存路径 + 装载协商(PTX `.version` 与驱动能力比对)+ poisoned 状态机(06 §5/08 §2),把 PTX 嵌入 host 产物 data 段产出单可执行文件。M4 结束时兑现 **MVP 中点的硬证据**:Rurix 写的 SAXPY 在 RTX 4070 Ti 上 `measured_local` 达到 M0 手写 PTX 基线 ≥95%——这是"Rurix 第一次自己写出能跑且不慢的 GPU 程序"的里程碑。M4 为 M5(views 不相交证明 / shared+barrier / scoped atomics)备好 device 借用检查扩展 pass 的接入点与基准对照通道。

## 2. 范围

### 2.1 in-scope

| 项 | 说明 | 对应交付物 |
|---|---|---|
| 着色 + 地址空间检查 | host/device/kernel 函数着色(符号属性)+ 地址空间(类型参数,addrspace 0/1/3/4/5)一致性,在 HIR 层完成(无需数据流,07 §3);barrier 可达性的保守 uniform 检查骨架(06 §2.2,违例须 unsafe);3xxx 错误码首批 | D-M4-1 |
| spec device 条款 | 着色规则 / 地址空间映射 / NVPTX codegen 约束 / launch 类型契约的 spec 条款(RXS-#### 续号,FLS 体例);**条款 PR 先于实现 PR** | D-M4-2 |
| NVPTX codegen | MIR→LLVM IR(NVPTX 约束子集)→PTX 文本:`ptx_kernel` 调用约定、launch bounds → `nvvm.maxntid/reqntid`、addrspace 显式建模、`llvm.nvvm.read.ptx.sreg.*` 索引 intrinsics;目标基线 `compute_89`,产物 **PTX-only**(开发期);LLVM pin 22.1.x(r2/07 §7) | D-M4-3 |
| ptxas 干验证关卡 | 生成的 PTX 过 `ptxas -arch=sm_89` 干验证(strict-only:ptxas 拒绝 = 编译错误带 RX6xxx 码);防御非 ASCII 路径(ptxas 崩溃先例);NVPTX 雷区回归集起步(shfl 选择失败/sqrt 近似约束类,遇雷登记 pin 绕行) | D-M4-3 |
| 运行时 + 装载 | Context(affine 根)/Stream/Buffer(`cuMemAlloc`/`cuMemAllocHost`)/launch(`cuLaunchKernel`)经典内存路径(显式 H2D/D2H + pinned staging,D-121);Module 装载 `cuModuleLoadDataEx`;装载前 PTX `.version` 与驱动能力协商(不匹配给结构化诊断,07 §7/08 §2.4);`CudaError` 结构化映射 + poisoned context 状态机(确定性错误而非 UB 级联,06 §5) | D-M4-4 |
| 嵌入单产物 | PTX 嵌入 host EXE data 段,运行时 `ctx.load_module(embedded::MODULE)?` 装载,`module.kernel::<f>()` 取强类型句柄(06 §5.2) | D-M4-4 |
| SAXPY 上 GPU + 回填 | Rurix `kernel fn saxpy` 全管线(着色检查→codegen→ptxas→嵌入→装载→launch→拷回核对)产 EXE 真跑;基准采样 measured_local 回填 m4_budget.json 比值断言(G-M4-1) | D-M4-5 |
| launch conformance + 黄金路径 4 | `conformance/launch/`(launch 类型契约反例/正例)+ `tests/ui/`(目标后端错误 snapshot,3xxx/6xxx) | D-M4-6 |

### 2.2 out-of-scope(显式排除)

- views 不相交证明 / `shared let` + barrier 一致性——实现为 MIR 借用检查的 device 扩展 pass(07 §4),→ M5(11 §3 M5);本里程碑只保证 host 借用检查 pass 结构可扩展、着色检查提供 device 边界信息。
- scoped atomics(`Atomic<T, Scope>`)+ PTX `atom.{order}.{scope}` 映射层——D-406 禁区由人工落笔,→ M5(06 §4)。
- libdevice 链接(SAXPY 级不需要)——→ M5 按需(06 §7/07 §7)。
- gpu 库并行基元(reduce/scan/transpose/tiled GEMM 自研 kernel)与 L1/L2 全量微基准——→ M5(RD-002 承接;M4 只交付 SAXPY 一条 measured_local 锚点对照)。
- 生产分发 fatbin(按架构预编 cubin + 保守 PTX fallback)——→ G1(07 §7);M4 产物 PTX-only。
- 流序分配 `AsyncBuffer<'stream, T>` 类型契约——→ G1(D-122,先把经典路径做对,06 §3/§5.4);ManagedBuffer/MappedBuffer 的 opt-in 形态本里程碑不交付。
- const 泛型值运行期单态化(RD-007)+ 运行期数组 aggregate codegen——按需在 device codegen 落地时评估接通(spec/consteval.md RXS-0064 已条款化),**非本契约验收门**;接通与否在执行期处置留痕。
- 11 §2 MVP 红线清单全部不触碰:Tensor Core/WGMMA/TMA intrinsics、cluster、动态并行、cooperative groups([../../registry/spike_gating.json](../../registry/spike_gating.json) SG-001 ~ SG-009 维持 not_triggered);MLIR kernel-island(SG-001)、dyn/特化/HKT/async(D-104)、宏(D-111/SG-006)永久裁剪。

## 3. 交付物清单

| ID | 交付物 | 形态 | 完成判据 |
|---|---|---|---|
| D-M4-1 | 着色 + 地址空间检查 | `src/rurixc/` 着色/addrspace 检查(HIR 层)+ 3xxx 错误码首批 | 单测 + conformance 正例 0 诊断 + 条款锚定(G-M4-5);host 回归网(hello-world 冒烟,CI 步骤 12/13/14)持续绿 |
| D-M4-2 | spec device 语义条款 | `spec/` device 着色/addrspace/codegen/launch 条款(RXS-#### 续号) | G-M4-5 |
| D-M4-3 | NVPTX codegen + ptxas 关卡 | MIR→LLVM(NVPTX)→PTX + ptxas 干验证(RX6xxx)+ PTX golden 起步 | G-M4-4 + IR golden(PTX 层)接入 |
| D-M4-4 | 运行时 + 装载 + 嵌入单产物 | Context/Stream/Buffer/launch + 装载协商 + poisoned 状态机 + PTX 嵌入 EXE | SAXPY 全管线真跑(G-M4-1 通道) |
| D-M4-5 | SAXPY 上 GPU + 回填 | Rurix kernel 真跑 EXE + 证据 JSON + [m4_budget.json](m4_budget.json) 比值回填(revision 留痕) | G-M4-1 |
| D-M4-6 | launch conformance + 黄金路径 4 | `conformance/launch/` + `tests/ui/` snapshot(3xxx/6xxx) | G-M4-2 + G-M4-3 |

## 4. 验收门(完整版,YAML 头为可提取摘要)

1. **G-M4-1(MVP 中点硬证据 — Rurix SAXPY ≥ M0 基线 95%)**:Rurix 写的 `kernel fn saxpy` 经 rurixc 全管线(着色检查 → NVPTX codegen → ptxas 干验证 → PTX 嵌入 → 运行时装载 → launch → 拷回逐元素核对)产出单可执行文件并真跑成功。基准采样按 [../m0/BENCH_PROTOCOL.md](../m0/BENCH_PROTOCOL.md) §3 协议(warmup/稳态判定/L2 清理/50×3 trials/trimmed mean),**三次进程级独立运行**取再次 trimmed mean,`evidence_level=measured_local`(锁频降级 `unlocked` 证据不得用于回填,§2.1)。证据 JSON 入 `evidence/`;比值断言 `m4.ratio.saxpy_vs_m0_baseline`(numerator = M4 Rurix SAXPY 有效带宽,denominator = `m0.bench.saxpy.effective_bandwidth_gbps` = 412.87 GB/s 的 measured_local 锚点)`direction=min, threshold=0.95`,close-out 跑 `py -3 ci/budget_eval.py --strict` 通过(本占位在 M4 内生灭,不跨里程碑欠债)。正确性:host 参考实现 f32 精确相等比对(SAXPY 无重排)。
2. **G-M4-2(launch 类型契约 conformance)**:`conformance/launch/` 按类别组织(`reject/<category>/*.rx` + `accept/*.rx`),预设类别全拦截、正例 0 诊断,CI 批跑。预设类别(`m4.counter.launch_conformance_categories` 计数对象,≥4,类别即 reject/ 下子目录,数量为 estimated 工程选择,增删经 Direct PR 留痕):
   1. `dim_mismatch` — launch 维度(grid/block 维数)与 `ThreadCtx<DIM>` 不匹配;
   2. `arg_type_mismatch` — kernel 形参与 launch 实参类型不符(含 View 地址空间不符);
   3. `context_brand_mismatch` — Buffer/Stream 与 launch 所在 Context 的 brand 不一致(context-brand 资源跨 context 误用);
   4. `launch_non_kernel` — 对非 `kernel` 着色函数发起 launch。
3. **G-M4-3(黄金路径 4 — 目标后端错误)**:目标后端错误 snapshot ≥10 条(`m4.counter.ui_golden_path4_snapshots`),覆盖 3xxx(着色违例:host 代码调 device-only、device 代码触 host-only、barrier 非 uniform 可达;地址空间不匹配)与 6xxx(codegen 不支持构造、ptxas 拒绝);走 M1.4 已激活的 bless 审批 guardrail;诊断措辞允许保守粗糙(先正确性后诊断打磨),snapshot 锁行为底线。
4. **G-M4-4(ptxas 干验证关卡真跑)**:合法 kernel 产出的 PTX 过 `ptxas -arch=sm_89` 通过(干验证,不产 cubin);构造 ptxas 拒绝场景(或注入非法 PTX)→ rurixc 给 RX6xxx 编译期诊断,CI 自动核对(对齐 G-M2-1/G-M3-4 真跑铁律)。NVPTX 雷区(shfl 选择失败/sqrt 近似约束类)遇到即登记雷区回归集并 pin 绕行。
5. **G-M4-5(traceability 延续)**:M4 新增 RXS 条款(着色/地址空间/codegen/launch)每条 ≥1 测试锚定;`ci/trace_matrix.py` 全局口径核对(`m1.counter.spec_clause_test_anchoring` 全局断言,无需另立 m4 计数器)。

## 5. Guardrails(字节级,机器核对)

见 YAML 头 `guardrails` 字段。核对方式:`ci/check_guardrails.py <基准ref>`(默认基准自 M4 开工切换 `m2-closed → m3-closed`,切换留痕 [CI_GATES.md](CI_GATES.md) 修订表;PR 路径仍以 GITHUB_BASE_REF 为准)。M4 期计划动作三项:**(1)基准 ref 切换**(M4.1 任务 1,切换前双基准核对 `py -3 ci/check_guardrails.py m3-closed` PASS);**(2)NVIDIA 再分发白名单审计**激活评估(14 §2 常驻集,M0~M3 标注"device 路径 M4 起评估"的到期时点——M4 产物 PTX-only、不打包 NVIDIA 再分发二进制,评估结论入 [CI_GATES.md](CI_GATES.md) §4;formal 激活随 libdevice/cubin 引入 M5/G1);**(3)PTX 文本 golden / NVPTX 雷区回归集**挂 IR golden 机制(14 §2 / 07 §11),随 device codegen 形态定型评估激活,激活必须经真实红绿验证(反 YAML-only 铁律)。M0~M3 历史预算的回填/冻结走 `check_guardrails.py` 既有机制,无需新代码。

## 6. Deferred 引用

| 编号 | 内容摘要 | 承接 |
|---|---|---|
| RD-007 | const 泛型值运行期单态化(turbofish const 实参 → 实例值代入 + codegen)+ 运行期数组 aggregate codegen | M4(device codegen / 运行期数组 aggregate codegen 开工时评估接通;spec/consteval.md RXS-0064 已条款化,回填仅补实现侧。**非本契约验收门**,接通与否执行期处置留痕) |

详情以 [../../registry/deferred.json](../../registry/deferred.json) 为唯一事实源,本表仅引用。M0/M1 遗留 RD-001(M8)/RD-002(M5)/RD-003(M6)/RD-004(M6)/RD-005(M6)/RD-006(M8)不属 M4 范围,维持原承接。M4 开工无预造新 deferred;执行期做不完的事按 14 §4 追加 `RD-###` 并双侧标注。

## 7. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-06-13 | 初版契约固化(M4 开工脚手架;基准 ref 切换 m2-closed → m3-closed 为 M4.1 任务) |

---

## 8. Close-out(只追加区 — 开工时为空)

<!-- 验收记录、guardrail 核对输出、deferred 继承/关闭记录追加于此;上方条款 0-byte 修改。 -->
