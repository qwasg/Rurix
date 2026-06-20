---
contract: G1
title: G1 期——CUDA–D3D12 interop / 实时呈现 / 流序分配 AsyncBuffer / 首个引擎集成 / 开源社区基建（MVP 后图形路线第二阶段）
status: active            # active → closed（close-out 只追加,既有条款 0-byte 修改;G1 close-out 关闭判定 / 基准切换 / g1-closed tag 由 owner 人工签署,AI 不代签）
version: v1.0
date: 2026-06-18
timebox: "MVP+约 12 个月（两级结构 G1.1~G1.4 见 G1_PLAN.md;月份为相对刻度,非日历承诺）"
rfc_required: none        # G1 期内容（CUDA–D3D12 interop / 实时呈现 / 流序分配 AsyncBuffer / 引擎集成 / 开源社区基建 / 生态包第二梯队 / cubin-fatbin 真分发）是对已锁定决策（D-002 图形分期 / D-122 流序分配推迟 G1 / D-130 G1 interop=D3D12 external memory/semaphore / D-207 fatbin G1 起 / D-401 开源后 FCP-lite）的条款化与工程实现:纯追加。开工脚手架取 rfc_required: none（对齐 M4~M8 先例,高层方向已锁 00–14）。G1 执行期新决策面（AsyncBuffer API 具体形态 / Graph API 是否立项 / G2 DXIL 路径 D-131 / 多后端 D-008 / 引擎宿主选型 / 外部 RFC 流程）按 10 §3 升档,AI 不自判 Direct,判档争议向上取严;触及死亡路线红线 / UB / 内存模型映射 / FFI ABI / 安全包络须人工经 Full RFC（AGENTS 硬规则 5/8）
upstream_docs:
  - "11 §4 (G1 期定义:CUDA–D3D12 interop / 流序分配 AsyncBuffer + Graph API 评估 / 首个引擎集成 / 开源社区基建 + 生态包第二梯队 / 持续:cubin-fatbin 真分发·LSP 中期·编译性能)"
  - "01 §6 (使命与第二层生态成功判据;G1 为图形愿景反哺动力的第二阶段)"
  - "06 §6 (三阶段图形路线 G0→G1→G2) / §8.1 (G1 interop:D3D12 external memory/semaphore,ExternalBuffer/ExternalSemaphore affine 类型化,D-130) / §5.4 (流序分配 AsyncBuffer<'stream,T> 类型契约 G1 设计预留,D-122) / §8.3 (引擎级工作流 U5 服务承诺,UC-05 前奏)"
  - "07 §7 (device codegen 分发:M8 维持 PTX-only;生产分发『按架构预编 cubin + 保守 PTX fallback』= G1 任务,D-207)"
  - "08 §2.2 (内存分配策略:G1 = stream-ordered allocator cuMemAllocAsync + CUmemoryPool;VMM G2 评估,D-232)"
  - "02 §U5 (图形引擎开发者画像 + UC-05 最小 RHI + render graph;C ABI FFI 成熟 + 增量 check <5s 为采纳判据)"
  - "09 §5 (生态包第二梯队:geometry G0 后 / cuDNN Phase 2+ 明确延后) / §7.2 (GPU 元数据 manifest/lockfile:sm 预编 cubin 覆盖 G1 起 + [[artifact]] digest,D-311)"
  - "10 §2/§6 (开源后三人组实体化 + FCP-lite + 6 周 train,D-401/D-405;G1-4 开源社区基建) / §3 (变更三档门)"
  - "14 §1 §3 §4 §8 (契约 / 预算 / deferred / CI 三层门禁)"
in_scope:
  - d3d12_interop           # G1.1 CUDA–D3D12 interop:ExternalBuffer/ExternalSemaphore affine 类型化(import 句柄生命周期 + 信号时序类型化,D3D12 侧薄 C FFI 不进语言),06 §8.1 / D-130
  - realtime_present        # G1.1 软光栅 demo 升级实时窗口呈现:G0 软光栅 kernel 语义不变,新增 interop 呈现通路(backbuffer 等价纹理 → kernel 写入 → 信号量同步 present),11 §4 / spec/softraster.md:153
  - async_buffer           # G1.2 流序分配 AsyncBuffer<'stream,T> 类型契约:分配未完成访问被 stream 序排除 / 释放后访问 = 编译期生命周期错误 / 跨 stream 经 share_with(other,event) 显式时序边;Compute Sanitizer 锁定 CUDA.jl #780 事故类回归,06 §5.4 / 08 §2.2 / D-122
  - graph_api_eval         # G1.2 Graph API 评估(spike report,与流序分配交互;CUDA Graph 文档/CUB-Thrust 实现对标),08 §2.2 / D-232;立项与否执行期裁决留痕,触发新 gating 则按需登记 SG-###
  - engine_integration     # G1.3 首个引擎集成:Rurix DLL(C ABI)嵌入现存 C++/D3D12 渲染框架承担 compute pass(UC-05 前奏),06 §8.3 / 02 §U5
  - oss_community          # G1.4 开源社区基建:贡献指南实体化 + FCP-lite + 首批外部 RFC 通道(仓库 2026-06-17 已 public,D-003/D-007),10 §2/§6 / D-401/D-405
  - ecosystem_tier2       # G1.4 生态包第二梯队:geometry 库评估/落地(09 §5,G0 后);cuDNN 留 Phase 2+(明确延后,非本期)
  - cubin_fatbin_dist     # 持续:生产分发 fatbin(按架构预编 cubin + 保守 PTX fallback);manifest/lockfile [[artifact]](ptx/cubin/fatbin)digest 记录;rurixup 发布链路覆盖,07 §7 / 09 §7.2 / D-207/D-311
  - spec_g1_clauses       # spec 互操作呈现 / 流序分配 / 分发产物语义面条款(RXS-0140 续号,FLS 体例);**条款 PR 先于实现 PR**(AGENTS 硬规则 7)
out_of_scope:
  - g2_native_d3d12        # G2 原生 D3D12 + DXIL 图形管线(vertex/fragment/mesh/task/RT 着色阶段 + DXIL codegen 第二后端)→ G2(06 §8.2,D-131 待决;G2 启动重评估)
  - multi_backend          # 多后端(AMD/Intel/Metal/Vulkan/SPIR-V)→ G2 完成 + 所有者解除红线 3(D-008,registry/spike_gating.json SG-003 维持 not_triggered)
  - registry_sumdb         # 包 registry(sparse index + sumdb 透明日志 + OIDC/Sigstore)→ D-312(社区规模驱动) / G2(09 §7.3;SG-007 维持 not_triggered,MVP+G1 = lockfile+vendor+checksum)
  - advanced_gpu_intrinsics # Tensor Core/WGMMA/TMA / cluster / 动态并行 / cooperative groups 永久裁剪(11 §2 红线,SG-001/SG-002 维持 not_triggered)
  - declarative_macros     # 声明宏:触发条件 = G1 后真实样板痛点 ≥3 类且 derive 不可覆盖(SG-006);**G1 期满后复评,非本期触发**
  - vmm_multi_gpu          # VMM(cuMemAddressReserve 族)/ 多 GPU / NVLink / MIG → G2 评估(08 §2.2;A-06 单机单 GPU 是 MVP 语义边界,G1 多 context 基础设施就位但不正式接触多 GPU)
  - autodiff_fusion        # autodiff / 可微渲染 / kernel fusion / 稀疏结构:永久 gating(SG-004/SG-005),生态包层面探索不动语言核心
  - python_native_embed    # Python 原生嵌入永久裁剪(死亡路线红线 1,仅 C ABI/PYD 通道,SG-008 维持 not_triggered)
deferred_refs: [RD-007, RD-008]   # RD-007(const 泛型值运行期单态化,owner_milestone=G1,inherited;G1 device codegen / 运行期数组 aggregate codegen 扩展评估接通,RXS-0064 语义不变)/ RD-008(stable API 快照冻结机制激活,owner_milestone=G1,open;首个 stable 发布时定义 stable 面并激活快照+bless 守卫)。G1 开工无预造新 deferred,执行期做不完的事按 14 §4 追加 RD-009+ 并双侧标注
deliverables:
  - id: D-G1-1
    name: G1.1 CUDA–D3D12 interop:ExternalBuffer/ExternalSemaphore affine 类型化 + 软光栅 demo 升级实时窗口呈现 + spec 互操作呈现条款先行(RXS-0140 续号)(G-G1-1)
  - id: D-G1-2
    name: G1.2 流序分配 AsyncBuffer<'stream,T> 类型契约(生命周期错误编译期拦截 + Compute Sanitizer 锁定 CUDA.jl #780 事故类回归)+ Graph API 评估 spike report(G-G1-2)
  - id: D-G1-3
    name: G1.3 首个引擎集成:Rurix DLL(C ABI)嵌入现存 C++/D3D12 框架承担 compute pass 端到端(UC-05 前奏)(G-G1-3)
  - id: D-G1-4
    name: G1.4 开源社区基建(贡献指南实体化 + FCP-lite + 首批外部 RFC 通道)+ 生态包第二梯队(geometry 评估/落地;cuDNN 留 Phase 2+)(G-G1-4)
  - id: D-G1-5
    name: 持续:生产分发 fatbin(按架构预编 cubin + 保守 PTX fallback)+ manifest/lockfile [[artifact]] digest + rurixup 发布链路覆盖(G-G1-5)
  - id: D-G1-6
    name: spec G1 条款(互操作呈现 / 流序分配 / 分发产物语义面,RXS-0140 续号,FLS 体例,条款 PR 先于实现 PR)(G-G1-6)
acceptance_gates:
  - id: G-G1-1
    check: "CUDA–D3D12 interop 端到端:ExternalBuffer/ExternalSemaphore import D3D12 共享堆/信号量 → Rurix kernel 写 backbuffer 等价纹理 → 信号量同步 present,实时窗口呈现真跑;句柄生命周期 + 跨 context 误用 + 信号时序由类型系统编译期拦截(预设错误类别全拦截);覆盖计数 g1.counter.d3d12_interop ≥1 + g1.counter.realtime_present ≥1。激活经真实红绿验证(篡改 interop 同步时序 / 放行跨 context 误用 → 红 → 复原绿,run URL 归档,反 YAML-only);软光栅 kernel 语义面 0-byte（G0 RXS-0118~0121 不变,仅新增呈现通路）"
  - id: G-G1-2
    check: "流序分配 AsyncBuffer<'stream,T> 类型契约:分配未完成访问 / 释放后访问 / 跨 stream 未经 share_with 同步三类生命周期错误 100% 编译期拦截(conformance reject 类别全拦截 + UI golden);三 stream 流序分配端到端真跑,覆盖计数 g1.counter.async_buffer_pipeline ≥1;device 路径纳入 Compute Sanitizer racecheck+memcheck nightly 全绿(CUDA.jl #780 事故类永久回归项)。Graph API 评估产 spike report(立项与否裁决留痕,触发新 gating 则登记 SG-###)。真实红绿(放行混用违例 → 红 → 复原绿,run URL 归档)"
  - id: G-G1-3
    check: "首个引擎集成端到端:Rurix DLL(#[export(c)] C ABI + 内建头文件)嵌入现存 C++/D3D12 渲染框架承担 ≥1 个 compute pass,宿主框架调用 Rurix 编译产物真跑(数值/呈现对照);覆盖计数 g1.counter.engine_integration ≥1;采纳判据对照(02 §U5:C ABI FFI 成熟 + 增量 check <5s 可控)。激活经真实红绿验证(篡改 compute pass 结果 → 红 → 复原绿,run URL 归档)"
  - id: G-G1-4
    check: "开源社区基建实体化:贡献流程(三档门自助判定 + RFC 模板 + provenance/验证强制/条款号引用 CI 阻断,10 §7)落地;FCP-lite 机制(D-401/D-405)文档化 + 首批外部 RFC 通道开放;生态包第二梯队 geometry 评估/落地证据(cuDNN 留 Phase 2+ 留痕)。机制类交付:以可核对的流程文件 + 至少一条走通的样例 RFC/贡献为证据"
  - id: G-G1-5
    check: "生产分发 fatbin:按架构预编 cubin + 保守 PTX fallback 真分发(脱离 M8 PTX-only 开发期形态);manifest/lockfile [[artifact]](ptx/cubin/fatbin 变体)+ digest 记录(D-311);rurixup 发布链路覆盖 fatbin 产物 + 既有 Release 层签名/SBOM/NVIDIA 白名单审计延续;cubin/fatbin codegen 形态纳入既有 PTX/IR golden 核对。性能判据(若有)measured_local 回填,close-out budget_eval --strict 零 estimated"
  - id: G-G1-6
    check: "traceability 延续:G1 新增 RXS 条款(RXS-0140 续号:互操作呈现 / 流序分配 / 分发产物语义面)每条 ≥1 测试锚定;ci/trace_matrix.py 全局口径核对(m1.counter.spec_clause_test_anchoring 全局断言,无需另立 g1 计数器);**条款 PR 先于实现 PR**(AGENTS 硬规则 7)"
guardrails:
  - "milestones/m0~m8 的 measured_local 既有预算条目 git diff 0-byte(新增 g1 条目允许);g1_budget.json 经命名空间强制前缀 + namespace check 单测(14 §3)"
  - "milestones/m0~m8 的 M*_CONTRACT.md(均 closed)既有内容只追加不修改(check_closed_contracts);本契约 G1_CONTRACT.md close-out 守卫随 G1 close-out 接入 check_closed_contracts 的 *_CONTRACT.md 口径(开工记录于 CI_GATES §5,G1 close-out 动作)"
  - "registry/deferred.json 与 registry/spike_gating.json 只追加(既有条目不可变字段修改触发人工审查);RD-007 仅允许 inherited→closed、RD-008 仅允许 open→inherited→closed 的状态留痕追加;G1 期 SG 复评(SG-006 G1 期满 / SG-007 维持 / Graph API 若立项新 SG)只追加 decisions"
  - "registry/error_codes.json 错误码语义可加不可改(M1.1 已激活);G1 新段位(interop 呈现/流序分配/引擎集成/分发诊断)首批分配随 G1.x 诊断 PR 留痕,段位分配制递增、含义冻结;**开工脚手架不预造错误码**"
  - "evidence/ 只增不删不改(M0.3 起)"
  - "00–14 共 15 份规划文档不被执行 PR 改写(勘误走 00 §6.3 追加式修订)"
  - "tests/ui/ .stderr / tests/mir/ .mir / tests/ptx/ .nvptx golden 变更必须经审批 bless(M1.4/M3.3/M4.2 已激活);G1 interop/流序分配 device codegen 形态变更纳入既有 PTX/IR golden 核对;cubin/fatbin 真分发产物形态纳入 golden + 白名单审计"
  - "全仓 crate 维持 unsafe_code=deny;src/rurix-rt 维持 undocumented_unsafe_blocks=deny(M4.3);G1 interop(D3D12 external memory/semaphore + DXGI)/ 引擎集成 C ABI 边界 / fatbin 装载凡落 unsafe 须每 unsafe 块 // SAFETY: + unsafe-audit 注册条目(AGENTS 硬规则 9),新 crate 默认 unsafe_code=deny"
  - "NVIDIA 再分发白名单审计维持(M5.4 check_redistribution);G1 cubin/fatbin 真分发产物经 Attachment A 白名单最小集审计,完整 Toolkit/驱动/Nsight 永不捆绑(许可红线 r6);D3D12/DXGI 系 Windows SDK 系统组件不受 NVIDIA 再分发约束"
  - "Compute Sanitizer racecheck+memcheck nightly 维持全绿(M5.4);G1.2 流序分配 device 路径 + G1.1 interop device 写路径落地后纳入既有 nightly 全跑(AsyncBuffer 为 CUDA.jl #780 事故类永久回归项)"
  - "guardrail 回退基准默认 = m8-closed(M8 close-out 已切;ci/check_guardrails.py 无参默认 m8-closed,PR 路径仍以 GITHUB_BASE_REF 为准);G1 close-out 时按 check_* 守卫风格 + 双基准核对切至 g1-closed(owner 人工签署兑现)"
  - "stable API 快照冻结机制(RD-008)维持 not_frozen/未激活至首个 stable 发布;激活时机与 stable 面定义经 owner 裁决留痕,激活后 stable API 快照变更须经审批 bless"
  - "本契约 in_scope/acceptance_gates 等既有条款 0-byte 修改,close-out 只追加 §8;契约 status active→closed 翻转 / 基准切换 / g1-closed tag 由 owner 人工签署,AI 不代签"
---

# G1 契约 — CUDA–D3D12 interop / 实时呈现 / 流序分配 AsyncBuffer / 首个引擎集成 / 开源社区基建（MVP 后图形路线第二阶段）

> 所属:[../../11_ROADMAP.md](../../11_ROADMAP.md) §4 G1 期 / 契约机制见 [../../14_ENGINEERING_DISCIPLINE.md](../../14_ENGINEERING_DISCIPLINE.md) §1
> 规范先行延续(AGENTS.md 硬规则第 7 条):互操作呈现 / 流序分配 / 分发产物的语义面 PR 必须引用 RXS-#### 条款号(RXS-0140 续号);缺条款先补 spec,**条款 PR 先于实现 PR**。
> 基准 ref:**默认 `m8-closed`**(M8 close-out 已完成 `m7-closed → m8-closed` 切换;`ci/check_guardrails.py` 无参默认 = `m8-closed`,PR 路径仍以 `GITHUB_BASE_REF` 为准)。
> 粒度:**单 G1 阶段契约**(owner 裁定,见 §7 v1.0):一份契约覆盖整个 G1 期,G1.1~G1.4 子里程碑分解见 [G1_PLAN.md](G1_PLAN.md)(对齐「每里程碑一份契约 + 内部子里程碑」范式)。

---

## 1. 目标

把 Rurix 从 MVP 收口(M8:互操作 / 加固 / 三旗舰用例端到端 / 零 estimated)推进到 **图形路线第二阶段(G1 期,11 §4)**:落下 **CUDA–D3D12 interop**(`ExternalBuffer`/`ExternalSemaphore` affine 类型化,把 import 句柄生命周期与信号时序做成编译期约束,D3D12 侧以薄 C FFI 驱动不进语言,D-130 / 06 §8.1),并据此把 **G0 软光栅 demo 从离屏出图升级为实时窗口呈现**(11 §4 / spec/softraster.md:153);引入 **流序分配 `AsyncBuffer<'stream,T>` 类型契约**(分配/释放/跨 stream 三类生命周期错误编译期拦截 + Compute Sanitizer 锁定 CUDA.jl #780 事故类回归,D-122 / 06 §5.4 / 08 §2.2)并对 **Graph API** 做评估(spike report);完成 **首个引擎集成里程碑**(Rurix DLL 经 C ABI 嵌入现存 C++/D3D12 渲染框架承担 compute pass,UC-05 前奏,06 §8.3 / 02 §U5);建成 **开源社区基建**(贡献指南实体化 + FCP-lite + 首批外部 RFC 通道,仓库已 public,D-401/D-405)并评估 **生态包第二梯队**(geometry;cuDNN 留 Phase 2+);**持续**项推进 **生产分发 fatbin**(按架构预编 cubin + 保守 PTX fallback,脱离 M8 PTX-only 开发期形态,D-207 / D-311)。G1 期延续 MVP 工程纪律:每子里程碑至少一项真实硬件证据、零 estimated 占位、规范↔测试↔PR 三角、真实红绿、字节级 guardrails。

## 2. 范围

### 2.1 in-scope

| 项 | 说明 | 对应交付物 |
|---|---|---|
| CUDA–D3D12 interop | `ExternalBuffer`/`ExternalSemaphore` affine 类型化(import 句柄生命周期 + 信号时序类型化;D3D12 薄 C FFI 不进语言),06 §8.1 / D-130 | D-G1-1 |
| 实时窗口呈现 | 软光栅 demo 升级:G0 kernel 语义不变,新增 interop 呈现通路(backbuffer → kernel 写入 → 信号量 present),11 §4 / spec/softraster.md:153 | D-G1-1 |
| 流序分配 AsyncBuffer | `AsyncBuffer<'stream,T>` 类型契约(分配/释放/跨 stream 三规则编译期拦截);Compute Sanitizer 锁定 CUDA.jl #780,06 §5.4 / 08 §2.2 / D-122 | D-G1-2 |
| Graph API 评估 | spike report(与流序分配交互;立项与否裁决留痕,触发新 gating 则登记 SG-###),08 §2.2 / D-232 | D-G1-2 |
| 首个引擎集成 | Rurix DLL(C ABI)嵌入现存 C++/D3D12 框架承担 compute pass(UC-05 前奏),06 §8.3 / 02 §U5 | D-G1-3 |
| 开源社区基建 | 贡献指南实体化 + FCP-lite + 首批外部 RFC 通道(仓库已 public,D-003/D-007),10 §2/§6 / D-401/D-405 | D-G1-4 |
| 生态包第二梯队 | geometry 评估/落地(09 §5,G0 后);cuDNN 留 Phase 2+(明确延后) | D-G1-4 |
| 生产分发 fatbin | 按架构预编 cubin + 保守 PTX fallback;manifest/lockfile [[artifact]] digest;rurixup 覆盖,07 §7 / 09 §7.2 / D-207/D-311 | D-G1-5 |
| spec G1 条款 | 互操作呈现 / 流序分配 / 分发产物语义面(RXS-0140 续号,FLS 体例);**条款 PR 先于实现 PR** | D-G1-6 |

### 2.2 out-of-scope（显式排除）

- G2 原生 D3D12 + DXIL 图形管线(着色阶段进语言 + DXIL codegen 第二后端)——→ G2(06 §8.2,**D-131 待决**,G2 启动按 LLVM DirectX 后端成熟度重评估)。
- 多后端(AMD/Intel/Metal/Vulkan/SPIR-V)——→ G2 完成 + 所有者解除红线 3(**D-008 待决**,[../../registry/spike_gating.json](../../registry/spike_gating.json) SG-003 维持 not_triggered)。
- 包 registry(sparse index + sumdb 透明日志 + OIDC/Sigstore)——→ **D-312 待决**(社区规模触发) / G2(09 §7.3;SG-007 维持 not_triggered,MVP+G1 = lockfile+vendor+checksum)。
- 声明宏——触发条件 = **G1 后真实样板痛点清单 ≥3 类且 derive 不可覆盖**(SG-006);**G1 期满后复评,非本期触发**。
- VMM(cuMemAddressReserve 族)/ 多 GPU / NVLink / MIG——→ G2 评估(08 §2.2;A-06 单机单 GPU 是语义边界,G1 多 context 基础设施就位但不正式接触多 GPU)。
- Tensor Core/WGMMA/TMA intrinsics、cluster、动态并行、cooperative groups——永久裁剪(11 §2 红线,SG-001/SG-002 维持 not_triggered)。
- autodiff / 可微渲染 / kernel fusion / 稀疏结构——永久 gating(SG-004/SG-005),生态包层面探索不动语言核心。
- Python 原生嵌入——永久裁剪(死亡路线红线 1,仅 C ABI/PYD 通道,SG-008 维持 not_triggered)。
- const 泛型值运行期单态化(RD-007)随 device codegen / 运行期数组 aggregate codegen 扩展评估接通——**非本契约验收门**;G1 interop/引擎集成若触发数组长度类 const 泛型则按需接通或继续留痕(执行期处置,RXS-0064 语义不变)。

## 3. 交付物清单

| ID | 交付物 | 形态 | 完成判据 |
|---|---|---|---|
| D-G1-1 | CUDA–D3D12 interop + 实时呈现 | `ExternalBuffer`/`ExternalSemaphore` affine 类型 + 软光栅 demo 实时窗口呈现 + spec 互操作呈现条款(RXS-0140 续号) | G-G1-1 + G-G1-6 |
| D-G1-2 | 流序分配 AsyncBuffer + Graph API 评估 | `AsyncBuffer<'stream,T>` 类型契约(三规则编译期拦截)+ Sanitizer 回归 + Graph API spike report | G-G1-2 |
| D-G1-3 | 首个引擎集成 | Rurix DLL(C ABI)嵌入 C++/D3D12 框架承担 compute pass 端到端 | G-G1-3 |
| D-G1-4 | 开源社区基建 + 生态包第二梯队 | 贡献流程/FCP-lite/外部 RFC 通道 + geometry 评估/落地 | G-G1-4 |
| D-G1-5 | 生产分发 fatbin | 按架构预编 cubin + PTX fallback + manifest/lockfile [[artifact]] digest + rurixup 覆盖 | G-G1-5 |
| D-G1-6 | spec G1 条款 | 互操作呈现 / 流序分配 / 分发产物语义面(RXS-0140 续号,条款 PR 先于实现 PR) | G-G1-6 |

## 4. 验收门（完整版，YAML 头为可提取摘要）

1. **G-G1-1（CUDA–D3D12 interop + 实时呈现）**:`ExternalBuffer`/`ExternalSemaphore` import D3D12 共享堆/信号量 → Rurix kernel 写 backbuffer 等价纹理 → 信号量同步 present,实时窗口呈现端到端真跑;句柄生命周期 / 跨 context 误用 / 信号时序由类型系统编译期拦截(预设错误类别全拦截);覆盖计数 `g1.counter.d3d12_interop ≥1` + `g1.counter.realtime_present ≥1`。**真实红绿**(篡改 interop 同步时序 / 放行跨 context 误用 → 红 → 复原绿,run URL 归档)。软光栅 kernel 语义面 0-byte(G0 RXS-0118~0121 不变)。
2. **G-G1-2（流序分配 AsyncBuffer + Graph API 评估）**:分配未完成访问 / 释放后访问 / 跨 stream 未经 `share_with(other,event)` 同步三类生命周期错误 100% 编译期拦截(conformance reject 类别全拦截 + UI golden);三 stream 流序分配端到端真跑,`g1.counter.async_buffer_pipeline ≥1`;device 路径纳入 Compute Sanitizer racecheck+memcheck nightly 全绿(CUDA.jl #780 事故类永久回归项)。Graph API 评估产 spike report(立项与否裁决留痕)。**真实红绿**(放行混用违例 → 红 → 复原绿)。
3. **G-G1-3（首个引擎集成）**:Rurix DLL(`#[export(c)]` C ABI + 内建头文件)嵌入现存 C++/D3D12 渲染框架承担 ≥1 个 compute pass,宿主框架调用真跑(数值/呈现对照);`g1.counter.engine_integration ≥1`;采纳判据对照(02 §U5:C ABI FFI 成熟 + 增量 check <5s 可控)。**真实红绿**(篡改 compute pass 结果 → 红 → 复原绿)。
4. **G-G1-4（开源社区基建 + 生态包第二梯队）**:贡献流程(三档门自助判定 + RFC 模板 + provenance/验证强制/条款号引用 CI 阻断,10 §7)落地;FCP-lite(D-401/D-405)文档化 + 首批外部 RFC 通道开放;geometry 评估/落地证据(cuDNN 留 Phase 2+ 留痕)。机制类交付以可核对流程文件 + 至少一条走通样例为证据。
5. **G-G1-5（生产分发 fatbin）**:按架构预编 cubin + 保守 PTX fallback 真分发(脱离 PTX-only);manifest/lockfile `[[artifact]]`(ptx/cubin/fatbin 变体)+ digest(D-311);rurixup 发布链路覆盖 fatbin + 既有 Release 层签名/SBOM/NVIDIA 白名单审计延续;cubin/fatbin codegen 形态纳入既有 PTX/IR golden。性能判据(若有)`measured_local` 回填,close-out `budget_eval --strict` 零 estimated。
6. **G-G1-6（traceability 延续）**:G1 新增 RXS 条款(RXS-0140 续号)每条 ≥1 测试锚定;`ci/trace_matrix.py` 全局口径核对(`m1.counter.spec_clause_test_anchoring` 全局断言);**条款 PR 先于实现 PR**。

> 验收门为子里程碑级累计;各 g1.x 子里程碑出口判据与排程见 [G1_PLAN.md](G1_PLAN.md)。性能门(如有)注明证据等级 `measured_local` 与 BENCH_PROTOCOL §3 采样协议。

## 5. Guardrails（字节级，机器核对）

见 YAML 头 `guardrails` 字段。核对方式:`ci/check_guardrails.py [基准ref]`(**默认基准 = `m8-closed`**;PR 路径以 `GITHUB_BASE_REF` 为准)。G1 期计划动作:**(1)新段位错误码首批分配**(interop 呈现/流序分配/引擎集成/分发诊断,随 G1.x 诊断 PR 留痕,分配制递增、含义冻结);**(2)interop/引擎/分发 unsafe-audit**(D3D12 external memory/semaphore + DXGI / C ABI 引擎边界 / fatbin 装载,凡落 unsafe 须 `// SAFETY:` + 注册条目);**(3)NVIDIA 再分发白名单审计**(cubin/fatbin 真分发产物经 Attachment A 白名单,check_redistribution 延续);**(4)Compute Sanitizer nightly**(AsyncBuffer + interop device 路径纳入既有全跑);**(5)G1 close-out 守卫切换**(`check_guardrails.py` 回退基准 `m8-closed → g1-closed` + `check_closed_contracts` 的 `M*_CONTRACT.md` 口径泛化为 `*_CONTRACT.md` 以纳入 G1_CONTRACT.md,均为 G1 close-out 动作,owner 人工签署兑现)。M0~M8 历史预算/契约/registry/error_codes/bless/spec guardrail 走既有机制,无需新代码。

## 6. Deferred 引用

| 编号 | 内容摘要 | 承接 |
|---|---|---|
| RD-007 | const 泛型值运行期单态化(turbofish const 实参 → 实例值代入 + codegen) | G1(M8 close-out owner M8→G1 顺延,inherited;随 device codegen / 运行期数组 aggregate codegen 扩展评估接通,RXS-0064 语义不变,回填仅补实现侧。**非本契约验收门**,接通与否执行期处置留痕) |
| RD-008 | stable API 快照冻结机制激活(stable 面定义 + 快照比对 + bless 审批守卫) | G1(M8.6 close-out 登记,open;首个 stable 发布时定义 stable 面并激活快照机制 + bless 守卫,激活与否随该子里程碑裁决留痕。**非本契约开工验收门**) |

详情以 [../../registry/deferred.json](../../registry/deferred.json) 为唯一事实源,本表仅引用。RD-002/RD-003/RD-004/RD-005 已 closed;RD-001/RD-006 已于 M8 close-out closed。G1 开工无预造新 deferred;执行期做不完的事按 14 §4 追加 `RD-009+` 并双侧标注。

## 7. 修订记录 / 开工裁决留痕

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-06-18 | 初版契约固化(G1 开工脚手架)。**开工 owner 裁决**(经 AskQuestion 确认,落 D-005「MVP 验收后的 G1 优先级与协作者引入」):① 粒度 = **单 G1 阶段契约**(milestones/g1/,G1.1~G1.4 作为 G1_PLAN.md 内子里程碑,对齐既有「每里程碑一份契约 + 内部子里程碑」范式);② 首子里程碑 = **G1-1 CUDA–D3D12 interop**(ExternalBuffer/ExternalSemaphore 类型化 + 软光栅 demo 升级实时窗口呈现,D-130 / 06 §8.1;延续「每阶段必有出图」动力,11 §4 / H06 §6);③ 协作者引入策略 = 随 G1-4 开源社区基建(仓库已 public)逐步开放外部贡献,先 interop 出图后社区(D-005 默认)。判档:脚手架取 `rfc_required: none`(对齐 M4~M8 先例,高层方向 D-002/D-122/D-130/D-207/D-401 已锁 00–14)。承接:RD-007(inherited)/ RD-008(open)owner_milestone 已于 M8 close-out 顺延至 G1,本契约 deferred_refs 引用;开工无预造新 deferred。基准 ref 默认 m8-closed(M8 已切,无需再切)。RXS-0140 续号预留,条款体随 G1.x 与测试同 PR(条款先于实现);新段位错误码首批分配随 G1.x 诊断 PR。**G1 执行期新决策面**(AsyncBuffer API 形态 / Graph API 立项 / G2 DXIL D-131 / 多后端 D-008 / 引擎宿主选型 / 外部 RFC 流程)在对应 g1.x 子里程碑带档位标记落笔,**AI 不自判 Direct,判档争议向上取严**;触及红线/UB/内存模型/FFI ABI/安全包络须人工经 Full RFC。**G1 close-out 关闭判定 / 基准切换(m8-closed→g1-closed) / g1-closed tag / RD-007·RD-008 状态翻转由 owner 人工签署,AI 不代签** |

---

## 8. Close-out（只追加区 — 开工时为空）

<!-- 验收记录、guardrail 核对输出、deferred 继承/关闭记录、G1.1~G1.5 子里程碑端到端红绿留痕、interop/AsyncBuffer/引擎集成证据、fatbin 分发签名/白名单审计、Graph API spike report 结论、性能 measured_local 回填、RD-007/RD-008 处置留痕追加于此;上方条款 0-byte 修改。G1 close-out 关闭判定 / 基准切换 / g1-closed tag 由 owner 人工签署兑现,AI 不代签。 -->

### 8.1 G1.2 子里程碑验收留痕（2026-06-19）

owner 于本工作会话授权完成 G1.2 人工收尾；以下由 Codex 代录机器事实与裁决，**不构成 AI 代签 G1 整体 close-out**：

- `AsyncBuffer<'stream,T>`（MR-0001 / RXS-0144~0148）三类生命周期错误 3/3 编译期拦截；RTX 4070 Ti（driver 591.86 / CUDA Toolkit 13.3）三 stream 流序分配 + 两条 `share_with` 时序边 + 往返数值对照真跑，`pipeline_ok=true`。证据：[async_buffer_smoke.json](../../evidence/async_buffer_smoke.json)，`g1.counter.async_buffer_pipeline=1` PASS。
- Compute Sanitizer 专项 racecheck 0 hazards / 0 errors、memcheck 0 errors，证据：[racecheck](../../evidence/compute_sanitizer_racecheck_async_buffer_20260619.json) / [memcheck](../../evidence/compute_sanitizer_memcheck_async_buffer_20260619.json)。
- 真实红绿：baseline green [27833847240](https://github.com/qwasg/Rurix/actions/runs/27833847240) → 临时放行 alloc-incomplete 违例后步骤 42 red [27834392530](https://github.com/qwasg/Rurix/actions/runs/27834392530) → 恢复拦截 restored green [27834580448](https://github.com/qwasg/Rurix/actions/runs/27834580448)。
- Graph API 裁决：**G1.2 不立项**；defer 至 G1.3 出现实测 launch-overhead 瓶颈时或 G2 重评估。当前不登记 SG-010、不起 Full RFC，详见 [graph_api_spike.md §7](graph_api_spike.md#7-裁决留痕owner-人工裁决)。

判定：D-G1-2 / G-G1-2 子里程碑验收要件闭环；G1 契约仍为 `active`，不执行 `g1-closed` tag / 基准切换 / RD-007·RD-008 翻转。

### 8.2 G1.3 子里程碑验收留痕（2026-06-20）

owner 白栀于本工作会话授权完成 G1.3 人工收尾；以下由 Codex 代录机器事实，**不构成 AI 代签 G1 整体 close-out**：

- MR-0002 / RXS-0149 条款先行 PR #70 经全量 smoke green [27864248795](https://github.com/qwasg/Rurix/actions/runs/27864248795) 后合入 `main`；实现 PR #71 随后重定向 `main`，保持条款先于实现的栈序。
- 自托管 `rurix-dev-4070ti`（RTX 4070 Ti、driver 591.86、CUDA Toolkit 13.3、VS 2022 MSVC 14.44 + Windows SDK）执行步骤 43：`rurix_engine.dll` + import lib 经随附头文件嵌入自建最小 C++/D3D12 render-graph harness，LUID 匹配 adapter 后调用 SAXPY compute pass，设备数值对照通过（`n=4096`，checksum `85d1316b4d754b25`）。证据：[engine_integration_smoke.json](../../evidence/engine_integration_smoke.json)，`integration_ok=true`，`g1.counter.engine_integration=1` PASS。
- 真实红绿：baseline green → 临时将 compute pass 改为 `a+1.0` 后 4095 mismatch、步骤 43 red → 功能源码 0-byte 复原后 restored green；最终 GitHub Actions 全量 PR smoke green [27865635269](https://github.com/qwasg/Rurix/actions/runs/27865635269)，步骤 43 输出 `ENGINE_INTEGRATION: ok pass=saxpy numeric=ok n=4096 checksum=85d1316b4d754b25 present=false`。
- 采纳判据中的增量 check `<5s` 当前无 `rx bench` incremental-check 基准入口；本记录不伪造 `measured_local`，按 CI_GATES v1.4/v1.5 留 G1 整体 close-out 在基准入口具备后统一回填。

判定：D-G1-3 / G-G1-3 子里程碑要件闭环；G1 契约仍为 `active`，不执行 `g1-closed` tag / 基准切换 / RD-007·RD-008 翻转。
