---
contract: M5
title: views、shared、同步——安全并行的核心交付
status: active            # active → closed(close-out 只追加,既有条款 0-byte 修改)
version: v1.0
date: 2026-06-14
timebox: "M+9 ~ M+11(约 8 周,两级结构见 M5_PLAN.md)"
rfc_required: none        # views 不相交证明/shared+barrier 数据流/scoped atomics 映射/libdevice 链接条款是对 05/06/07/08/13 已锁定决策(D-108/D-123/D-406/D-205~D-207)的条款化:纯追加、尚无 stable 面;任何偏离已锁定决策的语义动作(尤其 D-406 scoped atomics 禁区)按 10 §3 升档,判档争议向上取严
upstream_docs:
  - "11 §3 (M5 定义,安全并行核心交付)"
  - "05 §4 §5 §6 (affine 资源 brand / shared let / launch 类型契约——M4 可检形态的借用证明完整化)"
  - "06 §2 §4 §7 (同步与 PTX 内存模型 D-123 / scoped atomics D-406 禁区 / libdevice 数学函数)"
  - "07 §3 §4 §7 (着色与地址空间检查 M4 已落 / MIR 借用检查的 device 扩展 pass(views 不相交)/ device codegen 与 libdevice 链接 D-205·D-207)"
  - "08 §2 §5 (运行时对象 M4 已落 / Compute Sanitizer racecheck+memcheck 纳入 CI nightly)"
  - "14 (契约/预算/deferred/证据分级/测试纪律/基准协议)"
in_scope:
  - views_disjoint_proof        # views 算子集(split_at/chunks 等)+ 不相交证明:MIR 借用检查的 device 扩展 pass(07 §4),消费 M4 着色/地址空间边界信息;3xxx 段位续接
  - shared_barrier_consistency  # shared let + barrier 一致性数据流检查(写后 barrier 前不可读邻 lane 等保守规则,06 §2.2 完整化 M4 的 uniform 骨架)
  - scoped_atomics_ptx          # scoped atomics(Atomic<T, Scope>)+ PTX atom.{order}.{scope} 映射层(spec 条款先行;D-406 禁区由人工落笔,AI 不擅自实现映射语义)
  - libdevice_link              # libdevice 链接(保留外部符号 → 链 bc → internalize → DCE → NVVMReflect,07 §7);gpu 数学基元按需(06 §7)
  - gpu_parallel_primitives     # gpu 库并行基元自研 kernel:reduce / scan / transpose / tiled GEMM + L1/L2 全量微基准(承接 RD-002)
  - spec_m5_clauses             # spec views 不相交/shared+barrier/scoped atomics/libdevice 类型契约与 codegen 条款(RXS-0078 续号,规范先行)
out_of_scope:
  - cubin_fatbin_dist           # 生产分发 fatbin(按架构 cubin + PTX fallback)→ G1(07 §7 / RD-001 系);M5 仍 PTX-only + libdevice bc 链接,不打包再分发 cubin
  - async_buffer_streamorder    # 流序分配 AsyncBuffer<'stream> 类型契约 → G1(D-122,06 §3 §5.4)
  - lsp_tooling                 # rx CLI / LSP / 包管理 → M6(11 §3 M6);M5 不交付工具链面
  - stdlib_math_full            # core 数学库定型(Vec/Mat/swizzle/几何原语)→ M7(11 §3 M7);M5 libdevice 链接只覆盖 gpu 基元 kernel 所需数学函数
  - advanced_gpu_intrinsics     # Tensor Core/WGMMA/TMA / cluster / 动态并行 / cooperative groups 永久裁剪(11 §2 红线,SG-001~SG-009 维持 not_triggered;tiled GEMM 自研 kernel 不触 Tensor Core intrinsics,SG-002 复评留痕仍 not_triggered)
deferred_refs: [RD-002, RD-007]   # RD-002(L1 全量微基准,owner M5,M5 开工承接)+ RD-007(const 泛型值运行期单态化,M4 closed 时仍 open → M5 inherited,随 device codegen 扩展评估接通);M5 不预造新 deferred,执行期按需登记 RD-###(14 §4)
deliverables:
  - id: D-M5-1
    name: views 算子集 + 不相交证明(MIR 借用检查 device 扩展 pass,07 §4;3xxx 段位续接) + spec 条款先行
  - id: D-M5-2
    name: shared let + barrier 一致性数据流检查(M4 uniform 骨架完整化,06 §2.2) + spec 条款
  - id: D-M5-3
    name: scoped atomics 类型契约 + PTX atom.{order}.{scope} 映射层(spec 先行,D-406 禁区人工落笔)
  - id: D-M5-4
    name: libdevice 链接(保留符号 → 链 bc → internalize → DCE → NVVMReflect,07 §7)
  - id: D-M5-5
    name: gpu 库并行基元自研 kernel(reduce/scan/transpose/tiled GEMM)+ L1/L2 全量微基准(RD-002 承接)
  - id: D-M5-6
    name: 并行安全 conformance(views 不相交反例)+ 黄金路径 5(并行安全错误)+ Compute Sanitizer nightly 接入
acceptance_gates:
  - id: G-M5-1
    check: "L1+L2 基准全量 measured_local(UC-01 判据):自研 reduce / scan / tiled GEMM kernel 经全管线产 EXE 真跑,RTX 4070 Ti 上 measured_local 有效带宽/吞吐 ≥ 手写 CUDA C++ 对照的 90%(三次进程级独立运行 trimmed mean,BENCH_PROTOCOL.md §3 协议 + bench/stats.py;evidence_level=measured_local,锁频降级证据不得回填);比值断言 m5.ratio.{reduce,scan,gemm_tile}_vs_cuda ≥ 0.90 在 close-out 跑 budget_eval --strict 通过"
  - id: G-M5-2
    check: "views 不相交证明 conformance:conformance/views/reject/<category>/*.rx 反例全拦截(重叠 split / 别名可变 view / shared view 越界等,3xxx 诊断)+ accept/ 正例(合法 split_at/chunks 不相交)0 诊断,CI 批跑(m5.counter.views_conformance_categories ≥ 预设类别数)"
  - id: G-M5-3
    check: "黄金路径 5(并行安全错误)snapshot ≥10(m5.counter.ui_golden_path5_snapshots;views 重叠/别名 + shared+barrier 一致性违例 + scoped atomics scope 误用),走 M1.4 已激活的 bless 审批 guardrail"
  - id: G-M5-4
    check: "Compute Sanitizer 全绿纳入 CI nightly:racecheck(数据竞争)+ memcheck(越界/未初始化)对 M5 全部自研 kernel(reduce/scan/transpose/GEMM)+ M4 SAXPY 回归全绿(m5.counter.compute_sanitizer_clean;08 §5,GPU 队列子进程隔离)"
  - id: G-M5-5
    check: "traceability 延续:M5 新增 RXS 条款(views/shared+barrier/scoped atomics/libdevice)每条 ≥1 测试锚定(ci/trace_matrix.py 全局口径,沿用 m1.counter.spec_clause_test_anchoring)"
guardrails:
  - "milestones/m0~m4 的 measured_local 既有预算条目 git diff 0-byte(新增条目允许)"
  - "milestones/m0~m4 的 M*_CONTRACT.md(均 closed)既有内容只追加不修改"
  - "registry/deferred.json 与 registry/spike_gating.json 只追加(既有条目修改触发人工审查);RD-002 仅允许 open→inherited/closed、RD-007 仅允许 open→inherited/closed 的状态留痕追加;SG-002 复评只追加 decisions"
  - "registry/error_codes.json 错误码语义可加不可改(M1.1 已激活);3xxx/6xxx/7xxx 段位分配制递增、含义冻结"
  - "evidence/ 只增不删不改"
  - "00–14 共 15 份规划文档不被执行 PR 改写(勘误走 00 §6.3 追加式修订)"
  - "tests/ui/ 的 .stderr snapshot 变更必须经审批 bless(M1.4 已激活,check_ui_bless)"
  - "tests/mir/ 的 .mir golden 变更必须经审批 bless(M3.3 WP6 已激活,check_mir_bless)"
  - "tests/ptx/ 的 IR golden 变更必须经审批 bless(M4.2 已激活,check_ptx_bless)"
  - "spec/ 变更必须携带变更档位标记(M1.2 已激活)"
  - "src/rurix-rt 的 unsafe 边界维持 undocumented_unsafe_blocks=deny(M4.3 已激活,每 unsafe 块 // SAFETY:);全仓其余 crate 维持 unsafe_code=deny"
  - "guardrail 核对基准自 M5 开工切换 m3-closed → m4-closed(切换前打 m4-closed tag + 双基准核对,留痕 CI_GATES.md 修订表;PR 路径仍以 GITHUB_BASE_REF 为准)"
  - "NVIDIA 再分发白名单审计:libdevice 链接引入时 formal 激活(M4 §8.2/CI_GATES §4 标注的到期时点;结论入 CI_GATES.md §4)"
  - "Compute Sanitizer racecheck+memcheck:M5 device 并行 kernel 落地时纳入 nightly,激活经真实红绿验证"
  - "本契约 in_scope/acceptance_gates 等既有条款 0-byte 修改,close-out 只追加"
---

# M5 契约 — views、shared、同步(安全并行的核心交付)

> 所属:[../../11_ROADMAP.md](../../11_ROADMAP.md) §3 M5 / 契约机制见 [../../14_ENGINEERING_DISCIPLINE.md](../../14_ENGINEERING_DISCIPLINE.md) §1
> 规范先行延续(AGENTS.md 硬规则第 7 条):views 不相交/shared+barrier/scoped atomics/libdevice 语义 PR 必须引用 RXS-#### 条款号;缺条款先补 spec。scoped atomics 映射(D-406)为禁区,由人工落笔,AI 仅条款化与挂测试,不擅自实现 PTX 映射语义。
> 基准 ref:M5 开工切换 `m3-closed → m4-closed`(切换前打 `m4-closed` tag 并双基准核对,切换留痕 [CI_GATES.md](CI_GATES.md) 修订表;PR 路径仍以 GITHUB_BASE_REF 为准)。

---

## 1. 目标

把 rurixc 从 M4 的"第一个 Rurix kernel 上 GPU(SAXPY 闭环)"推进到 **安全并行的核心交付**:在 M4 着色/地址空间检查与运行时装载的基础上,落下 **views 算子集 + 不相交证明**(MIR 借用检查的 device 扩展 pass,07 §4——把 M4 只保证"结构可扩展"的借用检查真正扩展到 device 并行场景),完整化 **`shared let` + barrier 一致性数据流检查**(把 M4 的保守 uniform 骨架升级为数据流判定,06 §2.2),条款化并由人工实现 **scoped atomics + PTX `atom.{order}.{scope}` 映射层**(D-406 禁区),接通 **libdevice 链接**(保留外部符号 → 链 bc → internalize → DCE → NVVMReflect,07 §7),并交付一组 **gpu 库并行基元自研 kernel**(reduce / scan / transpose / tiled GEMM)+ L1/L2 全量微基准(承接 RD-002)。M5 结束时兑现 **安全并行的硬证据**:自研 reduce/scan/GEMM-tile kernel 在 RTX 4070 Ti 上 `measured_local` 达到手写 CUDA C++ 对照 ≥90%(UC-01 判据),并把 **Compute Sanitizer(racecheck + memcheck)全绿纳入 CI nightly**——这是"Rurix 的安全抽象不以性能为代价、且并行正确性有运行期工具背书"的里程碑。

## 2. 范围

### 2.1 in-scope

| 项 | 说明 | 对应交付物 |
|---|---|---|
| views 不相交证明 | views 算子集(`split_at`/`chunks`/`windows` 等)产出的子 view 不相交性,作为 MIR 借用检查的 device 扩展 pass(07 §4);重叠/别名可变 view → 3xxx 诊断;消费 M4 着色/地址空间边界信息 | D-M5-1 |
| shared+barrier 一致性 | `shared let` 声明(addrspace 3)的读写与 `block.sync()` barrier 的一致性数据流检查(写后未过 barrier 不可读他 lane 写入等保守规则),把 M4 的保守 uniform 骨架(RXS-0068)完整化为数据流判定(06 §2.2) | D-M5-2 |
| scoped atomics + PTX 映射 | `Atomic<T, Scope>` 类型契约 + PTX `atom.{order}.{scope}` 映射层;**spec 条款先行,D-406 禁区由人工落笔**——AI 条款化语义与挂测试,映射实现由人工完成 | D-M5-3 |
| libdevice 链接 | device codegen 链路接通 libdevice:保留外部数学符号 → 链 libdevice bc → internalize → DCE → NVVMReflect(07 §7);gpu 基元 kernel 所需数学函数按需 | D-M5-4 |
| gpu 并行基元 + 基准 | 自研 reduce / scan / transpose / tiled GEMM kernel(Rurix 源,全 safe 代码目标)+ L1/L2 全量微基准(RD-002 承接,harness 复用 BENCH_PROTOCOL) | D-M5-5 |
| 并行安全 conformance + 黄金路径 5 + Sanitizer | `conformance/views/`(不相交反例/正例)+ `tests/ui/`(并行安全错误 snapshot)+ Compute Sanitizer nightly | D-M5-6 |
| spec M5 条款 | views 不相交 / shared+barrier / scoped atomics / libdevice 类型契约与 codegen 的 spec 条款(RXS-0078 续号,FLS 体例);**条款 PR 先于实现 PR** | D-M5-1 ~ D-M5-4 |

### 2.2 out-of-scope(显式排除)

- 生产分发 fatbin(按架构预编 cubin + 保守 PTX fallback)——→ G1(07 §7 / RD-001 系);M5 仍 **PTX-only + libdevice bc 链接**,不打包再分发 cubin/fatbin。
- 流序分配 `AsyncBuffer<'stream, T>` 类型契约——→ G1(D-122,06 §3/§5.4)。
- rx CLI / LSP / 包管理——→ M6(11 §3 M6);M5 不交付工具链面。
- core 数学库定型(Vec/Mat/swizzle/几何原语)——→ M7(11 §3 M7);M5 libdevice 链接只覆盖 gpu 基元 kernel 所需数学函数。
- 11 §2 MVP 红线清单全部不触碰:Tensor Core/WGMMA/TMA intrinsics、cluster、动态并行、cooperative groups([../../registry/spike_gating.json](../../registry/spike_gating.json) SG-001 ~ SG-009 维持 not_triggered);**tiled GEMM 自研 kernel 走经典 shared-memory tiling,不触 Tensor Core/WGMMA intrinsics**(SG-002 触发条件"L2 基准证明 GEMM 类负载是真实用户瓶颈 且 中层抽象成熟度复评通过"在 M5 期复评留痕仍 not_triggered,结论入 CI_GATES §4)。

## 3. 交付物清单

| ID | 交付物 | 形态 | 完成判据 |
|---|---|---|---|
| D-M5-1 | views 不相交证明 | `src/rurixc/` MIR 借用检查 device 扩展 pass + 3xxx 段位续接 + spec 条款(RXS-0078 续号) | G-M5-2 + G-M5-5;host 回归网持续绿 |
| D-M5-2 | shared+barrier 一致性 | shared let 读写 + barrier 一致性数据流检查 + spec 条款 | 单测 + conformance + G-M5-3 子集 |
| D-M5-3 | scoped atomics + PTX 映射 | `Atomic<T, Scope>` 类型契约 + PTX `atom.{order}.{scope}` 映射(D-406 人工落笔)+ spec 条款 | spec 锚定 + 人工实现真跑 |
| D-M5-4 | libdevice 链接 | device codegen 接通 libdevice bc 链接(internalize/DCE/NVVMReflect) | gpu 基元 kernel 数学函数真跑 |
| D-M5-5 | gpu 并行基元 + 基准 | 自研 reduce/scan/transpose/tiled GEMM kernel + L1/L2 微基准 + [m5_budget.json](m5_budget.json) 比值回填 | G-M5-1 |
| D-M5-6 | conformance + 黄金路径 5 + Sanitizer | `conformance/views/` + `tests/ui/` snapshot + Compute Sanitizer nightly | G-M5-2 + G-M5-3 + G-M5-4 |

## 4. 验收门(完整版,YAML 头为可提取摘要)

1. **G-M5-1(安全并行硬证据 — 自研 kernel ≥ 手写 CUDA C++ 90%)**:自研 reduce / scan / tiled GEMM kernel(Rurix 源,全 safe 代码目标)经 rurixc 全管线(着色/views 不相交/shared+barrier 检查 → NVPTX codegen → libdevice 链接 → ptxas 干验证 → 嵌入 → 装载 → launch → 拷回核对)产 EXE 真跑成功。基准采样按 [../m0/BENCH_PROTOCOL.md](../m0/BENCH_PROTOCOL.md) §3 协议(warmup/稳态判定/L2 清理/50×3 trials/trimmed mean),**三次进程级独立运行**取再次 trimmed mean,`evidence_level=measured_local`(锁频降级 `unlocked` 证据不得用于回填,§2.1)。证据 JSON 入 `evidence/`;比值断言 `m5.ratio.{reduce,scan,gemm_tile}_vs_cuda`(denominator = 手写 CUDA C++ 对照实现的同协议 measured_local 锚点)`direction=min, threshold=0.90`,close-out 跑 `py -3 ci/budget_eval.py --strict` 通过(本占位在 M5 内生灭,不跨里程碑欠债)。正确性:host 参考实现比对(浮点重排类按 BENCH_PROTOCOL 容差口径)。
2. **G-M5-2(views 不相交证明 conformance)**:`conformance/views/` 按类别组织(`reject/<category>/*.rx` + `accept/*.rx`),预设类别全拦截、正例 0 诊断,CI 批跑。预设类别(`m5.counter.views_conformance_categories` 计数对象,类别即 reject/ 下子目录,数量为 estimated 工程选择,增删经 Direct PR 留痕,对齐 G-M3-1/G-M4-2 先例):重叠 `split_at` 子 view 同时可变借用 / 别名可变 view 跨 view 写冲突 / view 越界(超出父 view 长度)/ shared view 与 barrier 不一致访问 等。
3. **G-M5-3(黄金路径 5 — 并行安全错误)**:并行安全错误 snapshot ≥10(`m5.counter.ui_golden_path5_snapshots`),覆盖 views 重叠/别名(3xxx 续接)、shared+barrier 一致性违例、scoped atomics scope 误用;走 M1.4 已激活的 bless 审批 guardrail;诊断措辞允许保守粗糙(先正确性后诊断打磨),snapshot 锁行为底线。
4. **G-M5-4(Compute Sanitizer 全绿纳入 nightly)**:Compute Sanitizer `racecheck`(数据竞争)+ `memcheck`(越界/未初始化访存)对 M5 全部自研 kernel(reduce/scan/transpose/GEMM)+ M4 SAXPY 回归在 nightly 全绿(`m5.counter.compute_sanitizer_clean`);GPU 队列、子进程隔离(14 §6);Sanitizer 报告归档。激活经真实红绿验证(构造已知竞争 kernel → racecheck 红 → 修复转绿,run URL 归档,反 YAML-only)。
5. **G-M5-5(traceability 延续)**:M5 新增 RXS 条款(views 不相交/shared+barrier/scoped atomics/libdevice)每条 ≥1 测试锚定;`ci/trace_matrix.py` 全局口径核对(`m1.counter.spec_clause_test_anchoring` 全局断言,无需另立 m5 计数器)。

## 5. Guardrails(字节级,机器核对)

见 YAML 头 `guardrails` 字段。核对方式:`ci/check_guardrails.py <基准ref>`(默认基准自 M5 开工切换 `m3-closed → m4-closed`,切换留痕 [CI_GATES.md](CI_GATES.md) 修订表;PR 路径仍以 GITHUB_BASE_REF 为准)。M5 期计划动作三项:**(1)基准 ref 切换**(M5.1 任务 1,先打 `m4-closed` tag,切换前双基准核对 `py -3 ci/check_guardrails.py m4-closed` PASS);**(2)NVIDIA 再分发白名单审计 formal 激活**(M4 §8.2/CI_GATES §4 标注的到期时点——libdevice 链接引入再分发物时逐项核对,结论入 [CI_GATES.md](CI_GATES.md) §4);**(3)Compute Sanitizer racecheck+memcheck** 纳入 nightly(08 §5,M4 §3 标注"全量 racecheck 随 M5"的到期时点;激活经真实红绿验证)。M0~M4 历史预算的回填/冻结走 `check_guardrails.py` 既有机制,无需新代码。

## 6. Deferred 引用

| 编号 | 内容摘要 | 承接 |
|---|---|---|
| RD-002 | L1 全量微基准套件(Reduction / 2D Stencil / Transpose / GEMM,r11 §8 清单) | M5(本契约 D-M5-5/G-M5-1 承接:reduce/scan/transpose/GEMM-tile 自研 kernel + L1/L2 全量 measured_local ≥ 手写 CUDA C++ 90%;harness 复用 BENCH_PROTOCOL) |
| RD-007 | const 泛型值运行期单态化(turbofish const 实参 → 实例值代入 + codegen)+ 运行期数组 aggregate codegen | M5(M4 closed 时仍 open,inherited → M5;随 device codegen 扩展(views 子 view 长度类 const 泛型)评估接通,spec/consteval.md RXS-0064 已条款化,回填仅补实现侧。**非本契约验收门**,接通与否执行期处置留痕) |

详情以 [../../registry/deferred.json](../../registry/deferred.json) 为唯一事实源,本表仅引用。M0/M1 遗留 RD-001(M8)/RD-003(M6)/RD-004(M6)/RD-005(M6)/RD-006(M8)不属 M5 范围,维持原承接。M5 开工无预造新 deferred;执行期做不完的事按 14 §4 追加 `RD-###` 并双侧标注。

## 7. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-06-14 | 初版契约固化(M5 开工脚手架;基准 ref 切换 m3-closed → m4-closed 为 M5.1 任务 1,先打 m4-closed tag) |

---

## 8. Close-out(只追加区 — 开工时为空)

<!-- 验收记录、guardrail 核对输出、deferred 继承/关闭记录、NVIDIA 白名单审计结论、Compute Sanitizer 红绿留痕追加于此;上方条款 0-byte 修改。 -->
