# RFC-0018 — G7 生产帧闭环期伞形:compute RayQuery 语言面 / SPIR-V 1.4 per-entry 模块政策 / AS descriptor 与资源生命周期 / renderer W3 使用约束与对拍纪律

| 字段 | 值 |
|---|---|
| RFC 编号 | RFC-0018(4 位制,编号永不复用,10 §9.5) |
| 标题 | G7 生产帧闭环期伞形:compute RayQuery 语言面 / SPIR-V 1.4 per-entry 模块政策 / AS descriptor 与资源生命周期 / renderer W3 使用约束与对拍纪律 |
| 档位 | **Full RFC**(伞形,G3 RFC-0013 / G4 RFC-0015 / G5 RFC-0016 / G6 RFC-0017 单伞形先例:一份 RFC 承载全期各面,一次对抗性评审、一次 Approved 合入即满足各面「RFC Approved 前置」;触及**新 device 不透明类型 + 运行时语义 + codegen 内存模型相邻面**(RayQuery 类型面/状态机/动态语义、SPIR-V 1.4 capability 声明、AS descriptor 运行时),AGENTS 硬规则 5;判档争议向上取严 = Full,硬规则 8) |
| 状态 | **Agent Approved**(2026-08-01;§9.1 对抗性评审〔评审 provenance `trae-ide:kimi-k3 reviewer-session` 独立 subagent 会话 B ≠ 起草 provenance `trae-ide:kimi-k3` session A,六镜头 S/V/C/R/A/Q,D-409〕完成,10 findings 逐条 disposition〔0 blocker + 5 major + 5 minor,全部采纳并修、无驳回、无未决〕,先于任何实现 PR,G-G7-2) |
| 承接里程碑 | G7.1(验收门 G-G7-2;下游 G7.2~G7.5 各面「RFC Approved 前置」由本伞形一次承载,[G7_CONTRACT](../milestones/g7/G7_CONTRACT.md) `rfc_required: RFC-0018` 单号伞形) |
| 关联条款 | 拟落 spec **RXS-0297 起按需**(registry/number_ledger.json `reserved_in_flight[G7]`;预计扩 [spec/shader_stages.md](../spec/shader_stages.md)(章 A 类型面)与 [spec/vulkan_backend.md](../spec/vulkan_backend.md)(章 B 编码面);实际条款数随 spec diff materialize,未消费不占号;G4.5 C ABI v2 earmark RXS-0295~0296 不动);关联 RD-038(registry/deferred.json,G7 唯一主线)、[RFC-0016](0016-native-renderer.md) §4.E3/§9.1 R-3(RD-038 来源) |
| 依据决策 | D-406 v2.0(agent 完全自主)· D-409(对抗性评审,评审 provenance ≠ 起草)· D-402(三档门)/ D-403(spec 领导实现)/ D-404(特性生命周期)· 04 P-01(strict-only)/ P-09(证据压过进度)/ P-12(克制压过完整性)· 06 §8.3(库不进语言)· [RFC-0016](0016-native-renderer.md) §4.E3/§9.1 R-3(device 腿条件臂)· [G7_CONTRACT](../milestones/g7/G7_CONTRACT.md)/[G7_PLAN](../milestones/g7/G7_PLAN.md)/[CI_GATES](../milestones/g7/CI_GATES.md)· RD-034(DXIL RT blocked 维持)· [渲染器调研/rurix 渲染器设备化调研报告.md](../渲染器调研/rurix%20渲染器设备化调研报告.md)(shader supply wave 与 W3a/W3b/W3c 波次裁决) |
| Provenance | `Assisted-by: trae-ide:kimi-k3`(起草 provenance A)。agent 自主决策;批准前置 = §9.1 对抗性评审完成 |
| Agent 批准 | **Agent Approved 2026-08-01**——§9.1 对抗性评审(评审 provenance `trae-ide:kimi-k3 reviewer-session` 独立 subagent 会话 B ≠ 起草 `trae-ide:kimi-k3` session A,六镜头,D-409)完成,10 findings(0 blocker + 5 major + 5 minor)全部采纳并修、正文实改逐条 disposition(§9.1),先于任何实现 PR(G-G7-2) |
| 对抗性评审 | **已完成 第 1 轮 2026-08-01**——见 §9.1;评审 provenance `trae-ide:kimi-k3 reviewer-session`(独立 subagent 会话,独立进程/零共享上下文)≠ 起草 `trae-ide:kimi-k3`(硬规则 2 可机验,`ci/check_contribution.py` advisory);首选跨工具/跨模型评审不可得,本轮为同工具同模型族独立会话评审,偏差如实登记 §9.1 环境留痕(RFC-0015/RFC-0017 §9.1 先例) |

---

## 1. 背景与问题陈述

### 1.1 摘要

本 RFC 是 G7 生产帧闭环期的**单伞形 Full RFC**(G3~G6 单伞形先例:一次对抗性评审、一次 Approved 合入即满足各面「RFC Approved 前置」),唯一主线 = 收口 **RD-038**(渲染器效果 kernel device 化)。上游事实源 = [G7_CONTRACT](../milestones/g7/G7_CONTRACT.md) v1.0 + [G7_PLAN](../milestones/g7/G7_PLAN.md) v1.0 + registry/deferred.json RD-038 + [RFC-0016](0016-native-renderer.md) §4.E3/§9.1 R-3 + 渲染器设备化调研报告(2026-07-29)。四章:

- **章 A(§3)compute RayQuery 语言面**——`RayQuery` 不透明类型、initialize→proceed→terminate 状态机、builtins 清单(逐个签名与类型约束)、动态语义与非法状态诊断(编译期结构约束,严禁 UB)。
- **章 B(§4)SPIR-V 1.4 per-entry module policy**——使用 RayQuery 或接 `AccelStruct` 形参的 compute entry 升 SPIR-V 1.4 + `RayQueryKHR` capability + `SPV_KHR_ray_query` extension 按需声明(per-entry 并集判定,§9.1 V-1);W1/W2 entry 维持 1.0 与既有 golden 字节零漂移;spirv-val 双口径与反汇编 golden 锚定。
- **章 C(§5)AS descriptor 与资源生命周期**——复用既有 as_manager/BLAS/TLAS/BDA 唯一所有权;compute pipeline AS binding 类型、同步/barrier 要求、过期 TLAS fail-closed、KernelWave::W3 七项能力链缺一确定性拒绝。
- **章 D(§6)renderer W3 使用约束**——gi_probe/rtao/hard_shadow 三 kernel 共用同一真实 TLAS;host oracle 仅作 oracle 不参与成功路径;容差 measured 后冻结;禁止 mock/host substitution/isolated nonzero;RD-038 字面余项(VSM depth/TSR/HW raster diff)处置口径。

```text
.rx compute RayQuery (章 A 类型面)
  └─ rurixc MIR → SPIR-V 1.4 + RayQueryKHR/SPV_KHR_ray_query (章 B per-entry 升版)
       └─ rurix-rt 复用 as_manager/BLAS/TLAS → compute AS descriptor (章 C 生命周期/协商)
            ├─ gi_probe.rx      (章 D:共用真实 TLAS)
            ├─ rtao.rx          (章 D:host oracle 仅对拍)
            └─ hard_shadow.rx   (章 D:禁止降级/余项处置)
```

**与既有 RT 条款的关系**:本 RFC 对 RXS-0242~0248(mesh-task-RT 类型面/编码/运行时)**只增量不矛盾**——RT pipeline 六执行模型(RXS-0247/0248)维持既有面不动;compute inline ray query 是**新增**的供给面,沿 G7_PLAN §4「RT pipeline 沿既有 RXS-0242~0248」分界。

### 1.2 RD-038 与 W3 缺口(问题陈述)

RD-038(registry/deferred.json,status=open)源自 [RFC-0016](0016-native-renderer.md) §9.1 **R-3**(major,implementability):章 E/F 的 device 腿隐含假设 `rayQueryEXT` compute 编码通道已在工具链就位——**rurixc vulkan_codegen RT 面现为 `emit_*_min` 见证形态**,GI/RTAO 的 `.rx→SPIR-V` ray query 编码未经核实,device 全量承诺有伪造风险;经 §4.E3 修定为「device 腿条件臂」:通则全量,不通则降档「G3.6 RT 底座最小见证 + host 参考器全量对拍」+ RD 存续,**不伪造 device 绿**。2026-07-30 W1+W2 分波部分兑现后(RD-038 history 第二条:五效果内核 device 真跑对拍全绿,RTX 4070 Ti;**「validation 零报错」句经 G-G7-3 字面审计未在 evidence 锚定,如实标未证实**——既有 `validation_clean` 字段仅为环境开关记录,validation 零错误须由 G7 新证据以 validation 开启真跑锚定,见 [RD038_LITERAL_MATRIX](../milestones/g7/RD038_LITERAL_MATRIX.md) §4.2-③),剩余 W3 缺口三件套:

1. **rurixc 无 RayQuery 供给面**。[`vulkan_codegen`](../src/rurixc/src/vulkan_codegen.rs) RT 面为 `emit_raygen_min`/`emit_miss_min`/`emit_closesthit_min`/`emit_anyhit_min`/`emit_intersection_min`/`emit_callable_min` 固定最小合规模块(库级见证形态,RXS-0247);[`lower_compute`](../src/rurixc/src/vulkan_codegen.rs) 形参分类仅 buffer/scalar/ThreadCtx/image 四类,**无 RayQuery 类型、无 AS 形参类别、无 compute 路 SPIR-V 1.4 分叉**(`assemble` 恒 [`SPIRV_VERSION_1_0`]=0x0001_0000;1.4 仅 mesh/RT min 模块经 `ExtBuilder` 与 `assemble_mesh` 走 [`SPIRV_VERSION_1_4`]=0x0001_0400,RXS-0247 per-entry 分叉)。调研报告 §1.3:使用 ray query 的 SPIR-V 模块版本不得低于 1.4(Vulkan API 有效性规则),版本升级应作独立波次先回归。
2. **compute 无 AS descriptor 通道**。`AccelStruct` 类型面(RXS-0245)现**仅可作 RT 阶段签名形参**(非 RT 阶段签名 → RX3013);[`render_exec`](../src/rurix-rt/src/render_exec.rs) `ResourceDesc` 现仅 Buffer/Texture 类别,无 AS 资源类别;[`vk.rs`](../src/rurix-rt/src/vk.rs) `run_ray_tracing_offscreen`/`run_rt_inner` 的 BLAS/TLAS 两段构建与 `dsl_tlas`(DESCRIPTOR_TYPE_ACCELERATION_STRUCTURE_KHR=1000150000,写描述符 sType=1000150007)仅服务 RT pipeline 见证,compute dispatch 无消费路径。
3. **效果 kernel 无 `.rx` 表达通道**。[`apps/uc06-renderer/kernels`](../apps/uc06-renderer/kernels) 现仅 5 个 `.rx`(cull/classify_resolve/vsm_page_mark/taa=W1,visbuffer_sw_u64=W2);`gi_probe`/`rtao`/`hard_shadow` 无 `.rx` 源,CI 步骤 84~86 device 段 W3 为 blocked-honest 探针(`missing_toolchain_caps` 逐项留痕,RD-038 history);效果本体现仅 host 参考实现([`rurix-render`](../src/rurix-render) rt/gi/shadow 模块,239 单测金标准)。

**本期定位**(G7_CONTRACT 定位口径):G7 不新增渲染效果,而是把 G5/G6 已有 host/reference 与孤立 device 证明收敛成连续、可测量、不可静默降级的真实设备帧;本 RFC 只冻结语义,spec diff(RXS-0297 起)与 RED 语料由后续任务落地(G7.1 spec-first,硬规则 7)。

### 1.3 为何需要 Full RFC(而非 Direct/Mini)+ 为何伞形单 RFC

触及**新 device 不透明类型与运行时语义**(RayQuery 类型面/状态机/动态语义)、**codegen 内存模型相邻面**(SPIR-V 1.4 capability/extension 声明、AS descriptor 编码)、**unsafe 边界可能面**(compute AS descriptor 写入若新增 FFI unsafe 自 U44 顺位)——AGENTS 硬规则 5 Full RFC 触发面;判档争议向上取严(硬规则 8)。**为何伞形**:四章共享一套跨章一致性约定(RD-038 收口主线、per-entry 版本轴、唯一 AS 所有权、fail-closed 协商、host oracle 纪律),一次对抗性评审覆盖全文(D-409),各面失败测试先行判据不变(RFC 合入时点各面 CI 脚本与 `.rx` kernel 在 main 不存在 = RED);G7_CONTRACT `rfc_required: RFC-0018` 单号伞形登记在先。

## 2. 范围与红线

### 2.1 in-scope(本 RFC 冻结面)

| 面 | 内容 | 下游波次 | 验收门 |
|---|---|---|---|
| 章 A 语言面 | RayQuery 类型/状态机/builtins/动态语义/诊断 | G7.2(W3a) | G-G7-4 |
| 章 B 编码面 | SPIR-V 1.4 per-entry、capability/extension 按需、golden/val 锚定 | G7.2(W3a) | G-G7-4 |
| 章 C 运行时面 | AS descriptor/binding、生命周期、同步、W3 fail-closed 协商 | G7.3(W3b) | G-G7-5 |
| 章 D 使用约束 | 三 kernel 共用 TLAS、host oracle 对拍、容差纪律、余项处置 | G7.4(W3c)/G7.5 | G-G7-6/G-G7-7 |

### 2.2 out-of-scope(红线,本 RFC 明确不做)

- **RT pipeline/SBT/raygen 六执行模型扩张**——既有 RXS-0247/0248 面维持不动;命中点着色仍同质(调研报告 §1.5 判据未触发),SER/OMM/ShaderRecordBufferKHR 不进本期(RD-040 backfill_condition 逐字维持)。
- **DXIL RT 腿**——RD-034 upstream blocked 维持 open,本 RFC 零 DXIL 承诺;步骤 69 blocked 探针恒跑不动。
- **新渲染效果**——ReSTIR/MegaLights/世界辐射缓存/SMRT/降噪器接入等 RD-039~RD-041 P3+ 项不因「顺手」进入(G7_CONTRACT out_of_scope 逐字)。
- **RD-037(`.rx` gfx submit 真派发)不并入**——G7 只记录接口依赖;若 W3 闭环事实证明缺它不可运行,登记 RD-045+ 单独重立项,禁止偷偷并入(G7_PLAN §3)。
- **RD-044 物理 P3+、Safe GPU Operator Platform(G8 候选)、Tile/Neural、AD/fusion、WebGPU/多 GPU**——G7_CONTRACT out_of_scope 逐字维持;SG-002/004/005 not-triggered 维持,SG-010 软保留维持。
- **spec 条款全文与 RED 语料本体**——本 RFC 只冻结语义与 RXS 号段规划(章 A6/章 B4);spec diff、conformance/UI RED 语料由 G7.1 后续任务 spec-first 落地(硬规则 7:条款 PR 先于实现 PR)。

## 3. 章 A — compute RayQuery 语言面(类型、状态机、builtins、动态语义)

### A0 现状基线(2026-08-01 只读核实)

- `AccelStruct` 不是 [`ty.rs`](../src/rurixc/src/ty.rs) 的 device 类型,而是 [`shader_stages`](../src/rurixc/src/shader_stages.rs) `is_accel_struct` 的**头名匹配**(`ty_head_name(ty) == Some("AccelStruct")`),位置纪律 = 仅 RT 阶段签名形参(返回/结构体字段/非 RT 阶段签名 → RX3013,RXS-0245)。
- device builtin 注册面三处:[`hir.rs`](../src/rurixc/src/hir.rs) `Builtin` 枚举(现仅 `Println`)与 `DeviceIntrinsic` 枚举(`ThreadCtx` 方法族,`from_method` 注册,`min_dim` 维数约束);[`typeck.rs`](../src/rurixc/src/typeck.rs) `builtin_sig` 签名核对;[`shader_stages`](../src/rurixc/src/shader_stages.rs) `KNOWN_BUILTINS` 常量(#[builtin(..)] 标注名集,含 RT builtins:launch_id/launch_size/world_ray_origin/world_ray_direction/ray_t_min/hit_t/hit_kind/primitive_index,RXS-0245)。
- `trace_ray` 为 RXS-0245 已知签名自由函数(raygen 可达域限定,递归恒 1 编译期结构约束)——**spec 级先例**;其调用点签名核对/可达域检查在代码中**现状未接线**(RXS-0245 IR 自注「归后续 mir_build/coloring 接线」,评审核实:rurixc 全仓仅注释提及),`ray_query_initialize`(§A4)沿用该先例时同为 spec 级,接线点随 spec diff 冻结;**compute 阶段无任何 ray 遍历供给面**。
- 错误码现状:3xxx typeck 段已用 RX3011~3017(spec/shader_stages.md §3);6xxx codegen 段已用至 RX6033(registry/error_codes.json;RX6026=vulkan 编码子集外/spirv-val 拒,RXS-0246 类别扩充先例);7xxx 工具段下一可用 RX7023(number_ledger)。

### A1 `RayQuery` 不透明类型与位置纪律(冻结)

- **`RayQuery` = 新增 device 不透明类型**(lang-item;首期无泛型参数化,沿 `AccelStruct` 头名匹配先例或 lang-item 注册,具体接线点实现期核实并随 spec diff 冻结)。语义 = 一次 ray 遍历的**遍历器对象**,承载 SPIR-V `OpTypeRayQueryKHR` 的 Function-storage 变量。
- **位置纪律**(冻结):`RayQuery` **仅可作 `compute fn`(device kernel/其可达 device fn)体内的 function-local 变量**;**值形态**签名形参/返回位置/结构体字段/host 函数体/非 compute 着色阶段 → **编译期拒**(资源/遍历器句柄位置违例,优先复用 **RX3013 扩类别**,沿 AccelStruct/RXS-0245 与 `[Texture2D<F>]`/RXS-0231 扩类别先例;确需新类别自 RX3018 顺位,§7.4)。device fn 间 `&mut RayQuery` 借用形参为唯一豁免(见下「禁止逃逸」,位置纪律不覆盖借用形态)。SPIR-V 侧自觉收窄声明:规范允许 `OpTypeRayQueryKHR` 指针存储类 Private **或** Function(SPV_KHR_ray_query OpTypePointer 修订条款),首期收窄为 **Function-only**(Private 全局遍历器 = 跨 launch 持久化等价物,与禁止逃逸同源),收窄为自觉首期选择、非规范强制。规范另明列 `OpStore`/`OpLoad`/`OpCopyMemory`/`OpCopyMemorySized` 对 RayQuery 类型指针禁用——非 Copy 纪律在编码面 by-construction 成立(评审核实,SPV_KHR_ray_query rev 17)。
- **所有权形态**(冻结):`RayQuery` 非 Copy/非 Clone;只能经 `ray_query_initialize`(§A4)产出;无用户可及缺省构造/零值路径——「未初始化即使用」在类型面 **by-construction 不可达**(无 UB 措辞,沿 RXS-0245「递归深度 = 编译期结构约束」先例)。
- **禁止逃逸**(冻结):`RayQuery` 不得存入 buffer/跨 launch 持久化/跨函数以形参传递(device fn 间以 `&mut RayQuery` 借用传递为**唯一豁免**,借用纪律沿 device 既有借用面;豁免面在 spec diff 中逐字冻结)。

### A2 `AccelStruct` 进 compute 签名(RXS-0245 加性扩展,冻结)

- 对 RXS-0245 位置纪律作**加性修订**(修订行经新 RXS 条款登记,体例沿 RXS-0242→RXS-0153 / RXS-0244→RXS-0155 先例):`AccelStruct` **亦可作 `compute fn` 签名形参**,绑定轴维持 **SRV**(`OpTypeAccelerationStructureKHR` descriptor,UniformConstant,承 RXS-0163/0164 推导;既有形态锚 = [`emit_raygen_min`](../src/rurixc/src/vulkan_codegen.rs) set0/binding0 注释钉死 = vk.rs `run_rt_inner` `set_layouts[0]=dsl_tlas`)。
- **既有语义 0-byte**:RT 阶段 AccelStruct 面、`trace_ray` 已知签名、RT builtins 阶段矩阵全部不动;返回/字段/非着色签名位置维持 RX3013。
- 首期 compute 签名中 `AccelStruct` **至多一个**(单 TLAS 纪律,与章 D「三 kernel 共用同一真实 TLAS」同源);多 AS 形参 = 扩展方向,首期编译期拒。

### A3 状态机(冻结)

```text
                 ray_query_initialize(...)                ray_query_terminate(&mut rq)
  [无值] ────────────────────────────────► Initialized ─────────────────────► Terminated
                                              │  ▲                                   │
                                              └──┘ ray_query_proceed(&mut rq)        │
                                                  (true=本次 proceed 提交了一个        │ 任何后续操作
                                                   committed 交点或遇候选;              │ = 非法状态
                                                   false=遍历穷尽)                      │ (诊断)
```

- **三态**:`Initialized`(initialize 产出即入)/ 遍历推进(`proceed` 自环)/ `Terminated`(terminate 消费)。proceed 返回 false 后对象仍处 Initialized,committed 查询族可读;`terminate` 为**可选早退**(SPIR-V 语义不要求终结;未 terminate 的 `RayQuery` 随 function 作用域结束自然消亡,Function-storage 变量无析构语义)。
- **candidate/committed 二分**(语义模型冻结):`proceed` 遇**候选交点**(candidate,需着色器确认)与**提交交点**(committed,遍历采纳)二态。**首期 ray flags 恒 `Opaque` + cull mask 恒 `0xFF`**(沿 RXS-0245 `trace_ray` 恒 opaque/恒 0xFF 纪律):三角形几何全 opaque,候选路径在首期**不可达**(proceed 内直接提交;以 SPV_KHR_ray_query 规范语义为准,spirv-val 实现期核实),阴影早退由着色器 `terminate` 手动表达。**健全性前提钉死**(评审修订):「`proceed()==true` ⇒ committed 已存在」仅在首期 flags 恒 Opaque + 三角形几何前提下成立(SPV_KHR_ray_query:committed = 「closest recorded hit so far」,其存在判据 = `OpRayQueryGetIntersectionTypeKHR(Committed) != RayQueryCommittedIntersectionNoneKHR`);§A5-S3 的支配规则以此前提为健全性条件,**candidate/confirm 面开放时该支配规则必须经 spec 修订行同步重审**,不得静默沿用。candidate 查询族/`confirm_intersection`/ray flags 参数化(TerminateOnFirstHit/CullBackFace 等)= **已登记扩展方向**(§A4 下表下半),首期不开放、调用即编译期拒。
- **遍历自由度**(冻结,如实登记):`proceed` 的候选/提交产生**顺序与次数为实现定义但有界**(Vulkan/DXR 双规范一致遍历自由度,沿 RXS-0245 anyhit 先例),**不写成「未定义」**;committed 最终集合的语义 = 最近三角形交点(closest)唯一确定。
- **非法状态 = 编译期结构约束**(冻结,严禁 UB 节):terminate 后任何操作、committed 不存在时的 committed_* 查询、跨函数非豁免传递——**编译期可判部分一律编译期拒**(结构化诊断,§A5);不可静态判定的 committed 存在性经**支配域约束**(§A5-S3)前移到编译期。**不存在非法状态运行期路径**(无 UB 措辞)。

### A4 builtins 清单(逐个签名与类型约束,冻结)

形态裁决(§9 Q1):**方法族 intrinsic 于 lang-item `RayQuery`**(沿 `ThreadCtx` DeviceIntrinsic/`tex.sample` 方法先例)+ 构造经**已知自由函数 `ray_query_initialize`**(沿 `trace_ray` 已知签名先例)。查询族**不入** `KNOWN_BUILTINS` 标注集(非阶段 I/O 标注;`#[builtin]` 集 0-byte)。**首期开放面**:

| builtin(拟名) | 签名 | 类型约束 | 合法状态 | SPIR-V 映射(实现期核实) |
|---|---|---|---|---|
| `ray_query_initialize` | `fn ray_query_initialize(tlas: AccelStruct, origin: vec3<f32>, t_min: f32, dir: vec3<f32>, t_max: f32) -> RayQuery` | `tlas` 为 compute 签名 AS 形参或经豁免传递的引用;`t_min >= 0`、`t_max > t_min` 为值域契约(运行期值,不静态检,违约为实现定义但有界——不产生遍历结果保证);`dir` 不要求归一化(t 随 dir 缩放,与 host `rt::ref_tracer` 口径一致性实现期核实) | 任意(产 Initialized) | `OpTypeRayQueryKHR` Function 变量 + `OpRayQueryInitializeKHR`(flags 恒 Opaque,mask 恒 0xFF) |
| `rq.proceed` | `fn proceed(self: &mut RayQuery) -> bool` | 仅 Initialized;返回语义见 §A3 | Initialized | `OpRayQueryProceedKHR` |
| `rq.terminate` | `fn terminate(self: &mut RayQuery)` | 仅 Initialized;二次调用 = 非法状态(§A5-S2) | Initialized→Terminated | `OpRayQueryTerminateKHR` |
| `rq.has_committed` | `fn has_committed(self: &RayQuery) -> bool` | 仅 Initialized;遍历结束后亦合法 | Initialized | `OpRayQueryGetIntersectionTypeKHR`(Committed)或等价(实现期核实) |
| `rq.committed_t` | `fn committed_t(self: &RayQuery) -> f32` | 须 committed 存在(§A5-S3 支配域约束) | Initialized + committed | `OpRayQueryGetIntersectionTKHR`(Committed) |
| `rq.committed_barycentric` | `fn committed_barycentric(self: &RayQuery) -> vec2<f32>` | 同上 | 同上 | `OpRayQueryGetIntersectionBarycentricsKHR`(Committed) |
| `rq.committed_instance_index` | `fn committed_instance_index(self: &RayQuery) -> u32` | 同上 | 同上 | `OpRayQueryGetIntersectionInstanceIdKHR`(Committed) |
| `rq.committed_primitive_index` | `fn committed_primitive_index(self: &RayQuery) -> u32` | 同上 | 同上 | `OpRayQueryGetIntersectionPrimitiveIndexKHR`(Committed) |
| `rq.committed_geometry_index` | `fn committed_geometry_index(self: &RayQuery) -> u32` | 同上 | 同上 | `OpRayQueryGetIntersectionGeometryIndexKHR`(Committed) |

**已登记扩展方向(首期不开放,签名拟定不冻结,开放时经 spec 修订行)**:`rq.confirm_intersection(&mut self)`(candidate→committed)、`candidate_*` 查询族(t/barycentric/instance/primitive/geometry/front_face 镜像)、ray flags 参数化(TerminateOnFirstHit/CullBackFace/CullOpaque 等)、object-space 查询与 4×3 instance 矩阵获取、`front_face` 查询。冻结纪律:首期三 kernel(GI/RTAO/硬阴影)几何语义需求 = hit/miss、t、instance/primitive/geometry index、barycentric(G-G7-6 逐字),全部由 committed 族承载;**不为首期假想需求预开面**(P-12)。

### A5 动态语义与非法状态诊断(冻结)

- **S1 未初始化使用**:by-construction 不可达(§A1,无独立诊断)。
- **S2 terminate 后使用 / 二次 terminate**:MIR 数据流编译期检查(mir_build/coloring 层新建,现状未证实、实现期核实);违例 → **结构化诊断**(类型面方向,优先 RX3013 扩类别或新类别 RX3018 起顺位;codegen 兜底防御性拒自 **RX6034** 顺位,§7.4)。
- **S3 committed 不存在时的 committed_* 查询**:`committed_*` 调用点须被 `proceed()==true` 或 `has_committed()==true` 的 true 分支**支配**(支配域数据流检查,沿 RXS-0232「不做推断、保守全标」精神前移到编译期);无法满足支配关系 → **编译期拒**。**守卫形态枚举钉死**(评审修订):spec diff 须枚举识别的守卫形态——① `if rq.proceed()` true 分支体内;② `if rq.has_committed()` true 分支体内;③ `while rq.proceed()` 循环体内;其余形态(经布尔变量中转、跨函数守卫、循环后无 `has_committed` 守卫等)**一律保守拒**(误判方向恒为拒、恒不为放,strict-only)。健全性前提见 §A3(candidate 不可达首期;`proceed()==true` ⇒ committed 存在仅此前提下成立)。支配域检查器为新建面(现状未证实:[`dataflow`](../src/rurixc/src/dataflow.rs) 有 fixpoint 框架、无 dominator 计算,实现期核实);**备选设计**见 §8.2-E(聚合 `try_committed(self: &RayQuery) -> Option<CommittedHit>` 单次取全,by-construction 消除非法查询;`Option` 为既有 plain generic enum,typeck 先例在),评审可在 §9.1 裁决切换。
- **S4 initialize 重入**:by-construction 不可达(initialize 产新值,无 `&mut` 重入形态)。
- **S5 `RayQuery` 逃逸**(形参/返回/字段/持久化):§A1 位置纪律,RX3013 扩类别方向。
- **动态语义总则**:遍历顺序/次数 = 实现定义但有界(§A3);几何数值语义(t/barycentric/index)以 SPIR-V 规范与 host oracle 对拍为双重锚(章 D);**严禁 UB 节**——非法状态全部以编译期结构约束定义,无运行期未定义路径(spec 体例,沿 RXS-0245/0243 先例)。
- **诊断风格**:strict-only(P-01),无运行期回退;message-key en/zh 成对(`ci/bilingual_coverage.py` 覆盖门);错误码策略见 §7.4(不预造)。

### A6 章 A 消费的 RXS 号段规划(指明,不落条款全文)

拟扩 [spec/shader_stages.md](../spec/shader_stages.md),自 **RXS-0297** 起(合入时 number_ledger 实际 next_free 顺位校准;未消费不占号):

| 条款(拟) | 标题(拟) | 锚定计划(每条 ≥1 `//@ spec`) |
|---|---|---|
| RXS-0297 | `RayQuery` 不透明类型与位置纪律 + `AccelStruct` compute 签名加性扩展(RXS-0245 修订行) | shader_stages accept(compute 签名 AccelStruct + 体内 RayQuery)+ reject(返回/字段/非 compute/多 AS 形参 → RX3013 扩) |
| RXS-0298 | RayQuery 状态机与 builtins 方法族类型面(§A3/§A4 首期面) | typeck/conformance accept(initialize→proceed→terminate 全流程)+ reject(扩展方向调用/签名错配) |
| RXS-0299 | RayQuery 动态语义与非法状态诊断(S2/S3 编译期结构约束,严禁 UB) | reject 语料(terminate 后用/未支配 committed 查询)+ UI golden + 诊断码锚定 |

## 4. 章 B — SPIR-V 1.4 per-entry module policy(编码面)

### B0 现状基线(2026-08-01 只读核实)

[`vulkan_codegen`](../src/rurixc/src/vulkan_codegen.rs):`SPIRV_VERSION_1_0 = 0x0001_0000`(line 43)/ `SPIRV_VERSION_1_4 = 0x0001_0400`(line 50);[`assemble`](../src/rurixc/src/vulkan_codegen.rs)(compute)恒 1.0(line 1932);[`assemble_mesh`](../src/rurixc/src/vulkan_codegen.rs) 与 RT min 模块(`ExtBuilder`)恒 1.4(line 1995/2314);**per-entry 版本轴分叉落发射函数级**(RXS-0247 Dynamic Semantics 逐字:「mesh/RT 入口 emit 1.4 + interface 全量;既有 compute/vertex/fragment 入口维持 1.0 emit,产物字节零漂移」)。W1/W2 既有供给(RD-038 history):32 位 SSBO 原子 + `TextureRw2D` 存储图像写读(format-qualified,SPIR-V 1.0 零新能力)+ Int64/Int64Atomics(capability 按需声明,SPIR-V 维持 1.0)——「capability 只按真实使用声明」已有先例。`build_and_emit_vulkan` 入口:`stage=Some` 图形 → `dxil_spirv::emit_spirv_body_vulkan`;`stage=None` compute → `lower_compute`;失败 → `E_VULKAN_UNSUPPORTED` 结构化诊断(`codegen.vulkan_unsupported`)。

### B1 版本轴与 capability/extension 声明纪律(冻结)

- **使用 RayQuery 或接 `AccelStruct` 形参的 compute entry → 该入口模块升 SPIR-V 1.4**(`SPIRV_VERSION_1_4` header + `OpEntryPoint` interface **全量枚举**全部被引用全局变量,与 mesh/RT 同律,RXS-0247)。判定 = **per-entry 按 MIR 体是否真实消费 RayQuery(存在 `RayQuery` local 类型/ray query intrinsic)或 compute 签名是否含 `AccelStruct` 形参**(二者任一即触发;具体检测点实现期核实并随 spec diff 冻结)。**判定点并集钉死**(评审修订):`AccelStruct` 形参 → descriptor 变量(UniformConstant)→ `OpTypeAccelerationStructureKHR`,其 capability 承载 = `RayQueryKHR`(见下条)——仅看 RayQuery local 会把「AS 形参在、RayQuery 不在」的 kernel 留在 1.0 且不声明 capability,致 `OpTypeAccelerationStructureKHR` 无 capability 承载、spirv-val 必拒;并集判定封闭该情形(W1/W2 五 kernel 无 AS 形参,零漂移面不受并集影响)。同一 kernel 的判定结果必须确定性(同 MIR 同版本轴)。
- **capability/extension 按需声明**(冻结):`RayQueryKHR` capability + `OpExtension "SPV_KHR_ray_query"` **当且仅当模块含 `OpTypeRayQueryKHR` 或 `OpTypeAccelerationStructureKHR` 时声明**(与判定点并集同源);均不含的 compute entry 维持 1.0 且**零新 capability**。`OpTypeAccelerationStructureKHR` 在 compute(非 RT 阶段)模块中的 capability 承载 = **`RayQueryKHR`**(评审核实,SPV_KHR_ray_query rev 17:该指令列于 RayQueryKHR capability 下;RT 阶段模块另有 RayTracingKHR/RayTracingPipelineKHR 承载路径,compute 面唯一);shader 面**不引入** `SPV_KHR_physical_storage_buffer`(BDA 为 host/运行时面,首期 kernel 不使用物理地址缓冲指令)。
- **升版依据**(评审精确化):SPV_KHR_ray_query 扩展自身 **requires SPIR-V 1.0**(评审核实,rev 17 Dependencies)——1.4 并非 SPV 扩展强制;升版依据 = ① Vulkan 侧依赖链(VK_KHR_ray_query 依赖 VK_KHR_spirv_1_4 **或** Vulkan 1.2 核心,+ VK_KHR_acceleration_structure,Vulkan 附录评审核实)+ 调研报告 §1.3(使用 ray query 的 SPIR-V 模块版本不得低于 1.4,Vulkan API 有效性规则);② RXS-0247 同源口径(RT 腿硬性 1.4)与 interface 全量枚举同律——rurix **自觉沿 1.4 per-entry 口径冻结**(非 SPV 扩展强制,如实归因)。升版与回归分波(W3a 独立波次,调研报告 §1.3「版本升级单独一波先回归」与 G7_PLAN G7.2 一致)。

### B2 W1/W2 零漂移门(冻结)

- 既有 compute/vertex/fragment 入口维持 1.0 emit,**产物字节零漂移**(RXS-0247 零回归门延伸):五 W1/W2 kernel golden(cull/classify_resolve/vsm_page_mark/taa/visbuffer_sw_u64)+ 全部既有 vulkan golden **字节 diff 空,不重 bless**;DXIL B 路消费的 SPIR-V 字节不变;dxil 套件恒定。
- 既有 `assemble`(1.0)**0-byte 不动**;RayQuery compute 走**新增发射路径**(新增 assemble 变体或 `lower_compute` 内分叉,形态实现期定),分叉落发射函数级(RXS-0247 既有机制)。
- 校验轴(冻结,承 RXS-0247):合规判定以 `spirv-val` **退出码**为准,**不以驱动宽容度为准**;`--target-env vulkan1.2` 与 `--target-env spv1.4` **双口径皆 accept**(承 RXS-0212 三态 gate)。

### B3 反汇编 golden 锚定(冻结)

- 新增 golden 锚定指令面(最小集,G-G7-4 逐字):`OpTypeRayQueryKHR`、`OpRayQueryInitializeKHR`、`OpRayQueryProceedKHR`、`OpRayQueryTerminateKHR`、committed 查询族 `OpRayQueryGetIntersection{TKHR,BarycentricsKHR,InstanceIdKHR,PrimitiveIndexKHR,GeometryIndexKHR}`(按真实使用)+ 1.4 header + interface 全量 + `RayQueryKHR`/`SPV_KHR_ray_query` 声明行。golden 随实现 PR 落 SPIR-V golden 套件,**W1/W2 golden 0-byte**。
- device 段:最小 hit/miss/属性查询 kernel 真跑(CI 步骤 93 device 段,`RURIX_REQUIRE_REAL=1`;缺 provisioning SKIP = dev-env degrade,不充绿)。

### B4 章 B 消费的 RXS 号段规划(指明,不落条款全文)

拟扩 [spec/vulkan_backend.md](../spec/vulkan_backend.md),与章 A 共享 **RXS-0297 起**顺位(先合先得、后合校准):

| 条款(拟) | 标题(拟) | 锚定计划(每条 ≥1 `//@ spec`) |
|---|---|---|
| RXS-0300 | MIR→SPIR-V compute RayQuery 编码 + SPIR-V 1.4 per-entry 升版(RayQueryKHR/SPV_KHR_ray_query 按需声明;W1/W2 1.0 零漂移;子集外/emit 失败/spirv-val 拒 → RX6026 类别扩充或 RX6034 起新类别) | vulkan_codegen 单测(1.4 分叉 + compute 1.0 零漂移锚点)+ golden 反汇编 + `spirv-val` vulkan1.2/spv1.4 accept 集成测试 |

## 5. 章 C — AS descriptor、资源生命周期、同步与 fail-closed 能力协商

### C0 现状基线(2026-08-01 只读核实)

- **host 策略单源**:[`rurix-render/src/rt/as_manager.rs`](../src/rurix-render/src/rt/as_manager.rs)——`BlasKey`(网格内容 FNV-1a 64 位哈希,位级精确)、`BlasId(u32)`(evict 后失效、槽位复用)、`BlasCache::get_or_build/rebuild/evict`、`DynamicPolicy::{Static, Deformable{refit_budget_frames}, FullRebuild}`(变形超阈 = 任一叶 AABB 膨胀比 > 2 → rebuild)、`TlasBuilder`(每帧全量重建 + `rebuild_if_dirty` 干净帧零成本)、`AsStats{blas_builds, refits, tlas_rebuilds}` evidence 埋点;**device 压缩/scratch 归 device 腿,现状未实现**(as_manager.rs 模块文档逐字「device 压缩/scratch 归下一波 device 腿」)。
- **device AS 底座**:[`vk.rs`](../src/rurix-rt/src/vk.rs) `run_ray_tracing_offscreen`/`run_rt_inner`——BLAS→内存屏障→TLAS 两段构建(单 queue 全序)、`vkGetAccelerationStructureDeviceAddressKHR` 取 BLAS 地址填 TLAS instance、AS handle 先于其 storage buffer 逆序销毁(RXS-0248 IR2)、`dsl_tlas` descriptor(DESCRIPTOR_TYPE_ACCELERATION_STRUCTURE_KHR=1000150000;`VkWriteDescriptorSetAccelerationStructureKHR` sType=1000150007;`set_layouts[0]=dsl_tlas`)、device address 裸 GPU 指针面切 **U30** 审计(RXS-0248,仅作 build/instance 引用不解引用)。
- **能力协商**:[`render_exec`](../src/rurix-rt/src/render_exec.rs) `probe_device_caps`(instance 级只读探测,不建 device;features2 链 descriptor_indexing→buffer_device_address→acceleration_structure→ray_query→int64→sync2 逐节读回,不存在扩展的节读回 0;deferred_host_operations 仅看扩展存在性)+ `require_wave` fail-closed(`RenderExecError::MissingCapabilities{wave, missing}`,缺失名按声明序稳定输出)。
- **缺口**:render_exec `ResourceDesc` 无 AS 类别;compute dispatch 无 AS binding 路径;as_manager(host 模型)与 vk.rs device AS 对象之间**无接线**(现状未证实处已逐条标注)。

### C1 唯一所有权与复用纪律(冻结)

- **复用既有 as_manager + vk.rs AS/BDA 面,不建第二所有者**(G7_PLAN G7.3 逐字「复用 G3/G5 已有 BLAS/TLAS/BDA/AsManager,不建第二所有者」):BLAS 缓存键/refit 决策树/TLAS 重建时机以 as_manager 为**策略单源**;device AS 对象(VkAccelerationStructureKHR + storage buffer + scratch)由 **rurix-rt 执行层唯一持有**;renderer 侧只握 as_manager 句柄(BlasId/实例集),**不持原生 AS 指针/句柄**。
- 若复用受阻(as_manager 策略与 device 对象生命周期失配)→ **先修所有权图,不推进效果核**(G7_PLAN §4 止损行,本 RFC 冻结为止损触发器)。
- `rurix-render` 维持 `#![forbid(unsafe_code)]` 0-byte;新增 unsafe(compute AS descriptor 写入/FFI 扩展)**优先复用 U30(AS/SBT/device-address)与 U32(render_exec)既有审计边界**,确属新边界才自 **U44** 顺位登记(G7_CONTRACT guardrails;`// SAFETY:` 注释强制)。

### C2 compute pipeline AS binding 类型(冻结)

- `.rx` compute 签名 `AccelStruct` 形参(章 A2)→ `OpTypeAccelerationStructureKHR`(UniformConstant)descriptor;**descriptor type = `VK_DESCRIPTOR_TYPE_ACCELERATION_STRUCTURE_KHR`**,写入经 `VkWriteDescriptorSetAccelerationStructureKHR`(vk.rs 既有常量/先例)。
- **绑定布局**:AS 归 **SRV 轴**类别,沿 Vk-native set-per-class 形态(RXS-0248 descriptor 布局;TLAS SRV / storage image UAV 分属各自类别 set)与 binding_layout 推导单一事实源(RXS-0163/0164 承继);**具体 set/binding 号由推导产出,不手写钉死**(`emit_raygen_min` set0/binding0 仅为见证形态锚)。
- render_exec 侧 AS 资源类别(`ResourceDesc` 扩面)与 ComputePass 装配形态 = 实现面,本 RFC 冻结语义不动形态:**一次 dispatch 至多引用一个 TLAS;同一帧内 gi_probe/rtao/hard_shadow 三 dispatch 引用同一 TLAS 句柄**(章 D1),resource-id/句柄 identity 入 evidence(G-G7-8 provenance 机验同源)。

### C3 同步/barrier 要求(冻结方向,数值实现期冻结)

- **构建序**:BLAS→内存屏障→TLAS 两段构建,单 queue 全序(承 RXS-0248,0-byte);AS build(ACCELERATION_STRUCTURE_WRITE)→ compute dispatch(ACCELERATION_STRUCTURE_READ,COMPUTE_SHADER stage)之间**显式 barrier**,走 render_exec 既有 `VK_KHR_synchronization2` 脊柱(execute_frame 硬依赖);**具体阶段/访问位掩码以 validation 零错误为准在实现 PR 冻结**(本 RFC 不预造位值)。
- **帧内纪律**:TLAS rebuild(实例变换更新)与上帧在途 dispatch 的读写序经 synchronization2 显式时序边;同帧三 kernel 只读同一 TLAS,读-读无冲突,无需互斥 barrier(如实登记)。
- **validation fail-closed**:`VK_LAYER_KHRONOS_validation` ERROR 级消息翻 `Err`(承 RXS-0210/0248,`RURIX_VK_VALIDATION=1`);设备丢失/缺扩展/过期 TLAS/错误 barrier 有 RED 自检(G-G7-5 逐字)。

### C4 生命周期与过期 TLAS(冻结)

- **销毁序**:AS handle 先于其 storage buffer 逆序销毁(承 RXS-0248 IR2,0-byte)。
- **在途保护**:dispatch 在途帧(in-flight)完成前,**不得** destroy/refit/rebuild 同一 TLAS;as_manager `evict`/槽位复用后旧 `BlasId` 失效(host 模型既有语义),device 腿对应面 = **过期 TLAS/句柄再提交 dispatch → 确定性 `Err`**(fail-closed,非 panic、非 UB)。**代际/句柄校验机制现状未证实**(render_exec 无 AS 代际面),实现期核实并随实现 PR 冻结;若需新增 unsafe 句柄表,自 U44 顺位。
- **统计面**:`AsStats` 埋点延续;device 腿 compaction/scratch 池化为**扩展方向**(as_manager.rs 自述「归下一波 device 腿」),首期允许 TLAS 每帧全量重建(调研报告 §1.7 W3c 注:「可先用最小 TLAS 每帧全量重建联调,再替换正式 BLAS 治理」),不冒充完成 BLAS 治理。

### C5 KernelWave::W3 七项 fail-closed 协商(冻结,现状已就位面如实标注)

- **W3 累积能力链七项**([`W3_REQUIRED_CAPABILITIES`](../src/rurix-rt/src/render_exec.rs) 现状):`synchronization2`、`shader_buffer_int64_atomics`、`ray_query`、`acceleration_structure`、`buffer_device_address`、`descriptor_indexing`、`deferred_host_operations`(后五项 = RD-038 history 所称「ray query 五件链」)。**缺一 → `require_wave` 确定性拒绝**(`MissingCapabilities`,缺失名稳定序,现状已实现);`KERNEL_WAVE_ROUTES` 现状 gi_probe/rtao/hard_shadow → W3(路由表 0-byte 不扩)。
- **禁止隐式降级**:缺任一能力不得回退 host/低波次执行充绿;完整链 `RURIX_REQUIRE_REAL=1` 真跑(G-G7-5);capability snapshot 入 evidence(G7_PLAN G7.3)。
- probe 为 instance 级只读(现状);**device 创建时 feature 链逐项 enable 面现状未证实**(render_exec 现状以 probe + panic-on-missing 为主,[`device_kernels`](../apps/uc06-renderer/src/device_kernels.rs) `device_gate` 先例),实现期核实并随 W3b PR 冻结。
- **依赖链钉死**(评审修订):七项 probe 面之外,**enable 面**须覆盖 ray query 的 Vulkan 依赖链——VK_KHR_ray_query 依赖 **VK_KHR_spirv_1_4 或 Vulkan 1.2 核心**,另加 VK_KHR_acceleration_structure(后者自身依赖 VK_EXT_descriptor_indexing + VK_KHR_buffer_device_address + VK_KHR_deferred_host_operations,已被五件链覆盖;Vulkan 附录评审核实);VK_KHR_spirv_1_4 自身依赖 VK_KHR_shader_float_controls。device api ≥ 1.2 路径:spirv_1_4/shader_float_controls 面由核心覆盖,enable 仅须七项对应扩展;device api < 1.2 扩展路径:须逐项 enable `VK_KHR_spirv_1_4` + `VK_KHR_shader_float_controls`(VUID 依赖完整性,缺一 vkCreateDevice validation ERROR,fail-closed 可捕)。probe 面不单列此二项:规范强制 advertise VK_KHR_ray_query 的实现同时满足其依赖链,传递性由规范承载,如实登记。

## 6. 章 D — renderer W3 使用约束、host oracle 对拍与禁止降级

### D0 现状基线(2026-08-01 只读核实)

- 效果 host 参考实现:[`rurix-render`](../src/rurix-render) `rt::ref_tracer`/`rt::effects`(`rtao_pass`/`hard_shadow_pass`)/`gi::pipeline`/`shadow::vsm`/`temporal::{taa,tsr}`/`geometry::{cull,visbuffer}`(RD-038 reason/backfill_condition 逐字:host 参考已全量锚定,239 单测含逐位对拍金标准);uc06 pipeline 现状经 `rtao_pass`/`hard_shadow_pass` host 计算 + `graph_setup` async compute pass 占位(gi_probe_trace/rtao/hard_shadow 为 AsyncCompute 图节点)。
- device 腿现状:W1/W2 五 kernel 已 device 真跑对拍全绿(RD-038 history:cull 72/120 簇集合一致 / VisBuffer 9216 词 u64 逐位一致容差 0 / classify-resolve 一致 / VSM 页位图一致 / TAA 最大误差 1.2e-7;RTX 4070 Ti;「validation 零报错」句 G-G7-3 审计标未证实,§1.2 同源限定);W3 blocked-honest;**VSM depth/sample 是否真实进入 device = 现状未证实**;**TSR 仅 host reference**(G7_PLAN G7.5 逐字)。

### D1 三 kernel 共用同一真实 TLAS(冻结)

- `gi_probe.rx`、`rtao.rx`、`hard_shadow.rx` **共用同一真实 TLAS**(同一帧同一 AS 句柄,resource-id 证据);「GI 一套 RT、阴影一套 RT」**分裂否决**([RFC-0016](0016-native-renderer.md) §4.F2 逐字);禁止第二套伪 BVH/host 回填(G-G7-5)。
- 三 kernel 经 `KERNEL_WAVE_ROUTES` W3 路由 `require_wave` fail-closed;**不得绕过波次门禁**;`RURIX_REQUIRE_REAL=1` 真跑,缺 provisioning SKIP = dev-env degrade 不充绿(G7 CI_GATES 通用纪律)。
- 供给路径纪律(调研报告 §1.4 裁决):三 kernel 走**路径 a(rurixc MIR lowering)唯一主线**;路径 b(程序化 SPIR-V 直写)仅作能力探针/紧急通道且附强制反汇编 golden diff,事后可被 a 无损替换;路径 c(外部交叉编译)不用于本面。

### D2 host oracle 纪律(冻结)

- host 参考器 = **唯一金标准且仅作 oracle**:对拍比对用,**不参与成功路径**(G-G7-6 逐字「host 仅为 oracle 不参与成功路径」);device 成功判据不得含任何 host 计算结果回填。
- **host oracle 数值语义 0-byte**(G7_CONTRACT guardrails「G5 冻结面 0-byte:…host oracle 数值语义不得为迁就 device 实现而漂移」);冻结数值/感知容差前**先 measured**,不允许为过门修改 host oracle(G7_PLAN G7.4 逐字)。

### D3 对拍面与容差冻结纪律(冻结)

- **几何语义对拍面**(G-G7-6 逐字):hit/miss、t、instance/primitive(/geometry)index、barycentric,同 TLAS 同几何与 host reference 对拍(几何错误与采样错误不互相甩锅,[RFC-0016](0016-native-renderer.md) §4.F3 同结构对拍先例)。
- **效果输出对拍面**(RD-038 backfill_condition 逐字):GI 方向一致性对拍 / RTAO 同 TLAS 对拍 / 硬阴影可见性;固定场景数值或感知门。
- **容差纪律**(冻结):索引与 hit/miss 类 = **集合或逐位一致,零容差**;浮点类(t/barycentric/AO 辐射度/GI 能量)= **先 measured 后冻结**——阈值数字只来自 G7.1/G7.4 真实 GPU baseline 命令输出,**本 RFC 不预造容差数字**(P-09;G7_PLAN G7.1 预算口径同源)。

### D4 禁止事项(冻结,G-G7-5/6/8 同源)

mock 充绿;host substitution(host 结果回填 device 路径);isolated nonzero 拼装(孤立模块非零输出冒充真实帧);第二套伪 BVH/第二所有者;隐式降级(缺能力静默回退);绕过 `require_wave`;为迁就 device 漂移 host oracle/G5 冻结面(MaterialClosure 32B、VisBuffer 位格式、Barrier EB 三轴、PageRequest 字段布局)。

### D5 RD-038 字面余项处置(冻结处置口径;实现归 G7.5)

- **VisBuffer HW raster diff**:真实 graphics raster 对真实 W2 software raster,同场景同投影同 VisBuffer ABI,**整数域逐像素 diff=0**;若 Vulkan top-left/edge coverage 与 software raster 规则存在规范差异 → **先经 RFC 修订裁定,不扩大容差**(G7_PLAN G7.5 逐字;本 RFC §8.1 风险表同源)。
- **VSM depth**:vsm_page_mark(W1)已 device 真跑;VSM 深度采样是否真实进入 device **现状未证实**——G7.5 逐项补真实 device 证据,缺项则 RD-038 保持 open(G-G7-7「未覆盖任一项则 RD-038 保持 open,禁止局部完成冒充全关」)。
- **TAA-TSR**:TAA 已 device 真跑(max_err 1.2e-7,维持零回归);**TSR 仍只有 host reference**(G7_PLAN G7.5 逐字)——TSR device 腿补证据或如实保持 RD-038 open,二选一,不得静默略过。
- RD-038 close 判据 = 按 title/backfill_condition/history **逐字审计全部兑现**(G-G7-9);本 RFC 不宣告任何分项已关闭。

## 7. 证据矩阵与 CI 步骤 93~96 映射

### 7.1 RD-038 字面矩阵(框架,G7.1 D-G7-2 交付物)

矩阵列(冻结,G7_PLAN G7.1 逐字):`分项 / host oracle / 当前 device / 缺口 / 目标 smoke / evidence schema / close 判据`。本 RFC 冻结列结构;行内容(逐项现状)随 G7.1 审计任务填充,事实源 = registry/deferred.json RD-038 逐字 + §6.D0 现状基线。**G-G7-3 基线审计件已落**:[RD038_LITERAL_MATRIX](../milestones/g7/RD038_LITERAL_MATRIX.md)(2026-08-01,8 行分项全列 + 未证实项汇总,评审修订:指向已存在审计件),其列结构与本冻结列结构逐列一致(评审核对);§7.2 步骤 93~96 对 8 行分项的覆盖映射经评审核对完整(GI/RTAO 硬阴影 → 93→94;HW 光栅/VSM 深度/TSR → 95;cull/classify/VisBuffer SW/TAA 帧链并入 → 96)。

### 7.2 CI 步骤映射(冻结;步骤号随真实脚本 materialize 时回填 ledger)

| 步骤(拟) | 脚本(拟) | host/compile 段(恒跑) | device 段(gate real) | 对应门 | evidence schema(拟) |
|---|---|---|---|---|---|
| 93 | `ci/ray_query_codegen_smoke.py` | RED/accept 语料、SPIR-V 1.4/capability/extension/golden(§B3)、`spirv-val` 双口径退出码、W1/W2 最低版本零回归(§B2) | 最小 hit/miss/属性查询 kernel 真跑 | G-G7-4 | `ray_query_codegen_evidence_schema.json` |
| 94 | `ci/renderer_w3_smoke.py` | host BVH/reference 与三效果 oracle;AS/lifetime 审计(§C1/C4) | 同一真实 TLAS 驱动 GI/RTAO/硬阴影 `.rx` kernel,对拍(§D3)与 validation 零错误 | G-G7-5/6 | `renderer_w3_evidence_schema.json` |
| 95 | `ci/renderer_raster_diff_smoke.py` | 固定场景、覆盖规则与 RD-038 字面矩阵完整性 | VisBuffer SW/HW 整数域 diff=0;VSM depth/TSR 等余项 device 见证(§D5) | G-G7-7 | `renderer_raster_diff_evidence_schema.json` |
| 96 | `ci/renderer_device_frame_smoke.py` | graph/resource provenance、禁止 host substitution/isolated 拼装审计(§D4) | 连续真实设备帧、readback、capability snapshot、GPU timestamps;soak ≥30min 且 ≥10000 帧(close-out 专用取证不占 smoke 号) | G-G7-8 | `renderer_device_frame_evidence_schema.json` + `renderer_soak_evidence_schema.json` |

schema 与 `ci/check_schemas.py` 路由必须和对应 smoke **同 PR 落**,避免先有壳后无真实执行(G7 CI_GATES §4 逐字)。

### 7.3 feature gate / tracking / 实现序(10 §3 要件)

- **feature gate**:RayQuery codegen/运行时面 gate 名**实现期定、不预造**(承 `vulkan-backend` gate 先例,默认构建零依赖绿承诺不变);`RURIX_REQUIRE_REAL=1` 贯穿 device 段。
- **实现序**(G7_PLAN PR 栈,本 RFC Approved 前置):PR-1 spec diff + RED 语料(spec-first,硬规则 7)→ PR-2 W3a codegen(章 A/B)→ PR-3 W3b runtime(章 C)→ PR-4 W3c 三 kernel(章 D)→ PR-5 raster diff + 余项(§D5)。
- **真实红绿**(反 YAML-only):RED 语料先行(hit/miss/非法状态/缺能力),构造缺陷 → 红 → 复原 → 绿,run URL 归档(G-G7-2/G-G7-4)。

### 7.4 错误码策略与编号消费汇总(冻结)

- **错误码**:codegen 诊断自 **RX6034** 起(registry/error_codes.json 合入时顺位;6xxx 段,RX6026 类别扩充先例在);类型面诊断**优先复用 RX3012/RX3013 扩类别**(只加类别不改语义,07 §5),确需新类别自 **RX3018** 起(3xxx typeck 段续接 RX3017);工具类自 **RX7023**;en/zh message-key 成对;**只追加、不预造、不预留**(spec/shader_stages.md §3 体例)。
- **编号消费**(number_ledger `reserved_in_flight[G7]` 逐字):RXS-0297 起按需(未消费不占号)/ CI 步骤 93 起(随脚本回填不预占)/ RD-045 起按需(新阻塞才登记)/ U44 起按需(优先复用 U30/U32)/ MR-0012 起按需(预计零)/ SG 零消费(SG-010 软保留维持)/ 共享 D 段零消费(D-408 earmark 不动,D-410 自由池不占)。

## 8. 风险与备选

### 8.1 风险与止损(冻结,G7_PLAN §4 同源)

| 风险 | 预警 | 止损(冻结) |
|---|---|---|
| RayQuery 语义面扩张 | RFC/实现出现完整 RT pipeline/新 shader stage 需求 | **退回最小 compute inline ray-query 子集**(§A4 首期面);RT pipeline 沿既有 RXS-0242~0248,不在本期扩 |
| AS 所有权重复 | renderer、runtime 各持独立 TLAS 生命周期 | 强制复用 as_manager(§C1);无法复用则**先修所有权图,不推进效果核** |
| HW/SW edge rule 不一致 | VisBuffer diff 仅边界像素稳定非零 | **RFC 修订裁定覆盖规则;不放宽整数域容差**掩盖歧义(§D5) |
| 支配域检查工程超限(§A5-S3) | 数据流检查误报/漏报无法收敛 | 切备选 §8.2-E(`try_committed` 聚合查询,by-construction 消非法);经 §9.1 评审裁决并 RFC 修订行留痕 |
| 「全帧」仍是模块拼装 | pass output 不被后继资源消费 | resource-id/provenance 机验,失败即 G-G7-8 红(§7.2 步骤 96) |

### 8.2 备选方案与否决理由(冻结)

- **A. RT pipeline + SBT 承载三效果**——否决:命中后处理同质(取属性→累加/置位),ray query 严格更优(少一条管线、少一套 SBT 内存管理、遍历控制流本地;调研报告 §1.5 判据未触发);RD-040 backfill_condition 逐字维持。
- **B. 程序化 SPIR-V 直写为主线**——否决为唯一主线:构件游离于类型系统外(调研报告 §1.4),仅保留为能力探针/紧急通道 + 强制反汇编 golden diff(§D1)。
- **C. 外引 Slang/DXC 交叉编译**——否决:审计黑盒 + 上游缺陷史/版本漂移(调研报告 §1.4);WGSL/naga 无 ray query/64 位整数,明确排除(同 §1.4)。
- **D. DXIL RT 腿**——不做:RD-034 upstream blocked 维持 open;步骤 69 blocked 探针 0-byte。
- **E. `RayQuery` 聚合查询形态(`try_committed(self: &RayQuery) -> Option<CommittedHit>`,`CommittedHit{t, barycentric, instance_index, primitive_index, geometry_index}`)**——**保留为 §A5-S3 的正式备选**(非否决):by-construction 消除未守卫查询(`Option` 为既有 plain generic enum);代价 = 单次取全对「只取 t」场景过取 + 与 SPIR-V 分指令映射需分支化 lowering。与主设计(分立查询族 + 支配域约束)一并呈 §9.1 评审裁决。
- **F. host BVH 直接喂 compute(无 device AS)**——否决:伪造 device 绿,违 blocked-honest 与 §D4(RFC-0016 §9.1 R-3 原始风险)。

## 9. 未决问题 / 关键裁决

- **Q1 builtins 形态**:裁决 = 方法族 intrinsic(`rq.proceed()` 等,沿 `ThreadCtx`/`tex.sample` 先例)+ 构造自由函数 `ray_query_initialize`(沿 `trace_ray` 先例);lang-item 注册接线点(resolve/typeck/mir_build)现状未证实,实现期核实。**冻结**:形态本身;否决备选 = 纯自由函数族(与方法族先例不一致,弃)。
- **Q2 AccelStruct compute 扩展载体**:裁决 = 经新 RXS 条款登记的 **RXS-0245 加性修订行**(不占旧号、既有 RT 语义 0-byte;RXS-0242/0244 修订行体例先例)。
- **Q3 ray flags/cull mask 首期**:裁决 = 恒 Opaque + 恒 0xFF(沿 RXS-0245 `trace_ray` 纪律),扩展参数越出首期编译期拒;candidate/confirm 面登记为扩展方向(§A4)。
- **Q4 容差数字**:裁决 = **不预造**;索引/hit-miss 零容差,浮点容差经 G7.1/G7.4 measured baseline 追加式冻结(§D3)。
- **Q5 SPIR-V 1.4 升版面**:裁决 = 真实消费 RayQuery **或**签名含 `AccelStruct` 形参的 entry(per-entry 并集判定,§B1,§9.1 V-1 评审修订);W1/W2 与全部不触 RayQuery/AS 的 entry 维持 1.0 字节零漂移(§B2)。
- **Q6 多 TLAS/多 AS 形参**:裁决 = 首期单 TLAS 纪律(§A2/§C2),多 AS = 扩展方向。

### 9.1 对抗性评审记录(D-409 / 硬规则 2)

> 评审镜头(六角度逐章攻击,非走过场):**S** = 与既有条款一致性(对照 spec/shader_stages.md RXS-0242~0245、spec/vulkan_backend.md RXS-0246~0248、spec/binding_layout.md、spec/types.md、spec/device.md)/ **V** = SPIR-V·Vulkan 规范正确性(SPV_KHR_ray_query rev 17 与 Vulkan 附录经 WebFetch 公开事实核实)/ **C** = 编译器落地可行性(对照 ty.rs/typeck.rs/hir.rs/mir.rs/vulkan_codegen.rs 现状)/ **R** = 运行时落地可行性(对照 vk.rs/render_exec.rs/as_manager.rs/uc06 现状)/ **A** = 可验收性(冻结决策机验面、对拍/容差 loophole、证据矩阵与 CI 93~96 映射)/ **Q** = 开放问题裁决。环境留痕:首选跨工具/跨模型评审不可得,本轮为**同工具同模型族、独立 subagent 会话**评审(独立进程、零共享上下文;评审会话与起草 session A 隔离),偏差如实登记(RFC-0015/RFC-0017 §9.1 先例);provenance 字符串与起草者相异,`ci/check_contribution.py` advisory 可机验。

| 字段 | 值 |
|---|---|
| 评审者 provenance | `Assisted-by: trae-ide:kimi-k3 reviewer-session`(独立 subagent 会话 B;**≠** 起草 `trae-ide:kimi-k3` session A;同模型族偏差如实登记,见上环境留痕) |
| 评审轮次 | 第 1 轮,2026-08-01 |
| 结论 | **0 blocker / 5 major / 5 minor,共 10 findings;全部采纳并修、正文实改完成,无驳回、无未决** |

**Findings 与 disposition**(每条一行;disposition:**采纳并修** §X / **驳回** + 理由):

| # | Finding(评审者提出) | 严重度 | Disposition |
|---|---|---|---|
| S-1 | §A1 位置纪律只写语言面:「仅 function-local 变量 / 禁止逃逸」缺 SPIR-V 编码面规范锚——SPV_KHR_ray_query 允许 `OpTypeRayQueryKHR` 指针存储类 Private **或** Function,Private 全局遍历器 = 跨 launch 持久化等价物,不写约束则实现期可放 Private 绕过禁止逃逸;规范另禁 `OpStore`/`OpLoad`/`OpCopyMemory(/Sized)` 于 RayQuery 指针,「非 Copy by-construction」未引证 | major | **采纳并修 §A1**:SPIR-V 侧自觉收窄 Function-only(收窄为自觉首期选择、非规范强制,如实归因)+ 禁用指令引证(评审核实,SPV_KHR_ray_query rev 17 OpTypePointer/内存指令修订条款) |
| S-2 | §A0 引 `trace_ray` 为「spec 级先例」时未标注其调用点签名核对/可达域检查在代码中**现状未接线**(RXS-0245 IR 自注「归后续 mir_build/coloring 接线」,评审核实 rurixc 全仓仅注释提及)——读者易误以为有代码先例可循,`ray_query_initialize` 沿用该先例的落地深度被高估 | minor | **采纳并修 §A0**:如实标注 spec 级先例与代码未接线现状;接线点随 spec diff 冻结 |
| S-3 | §A3「`proceed()==true` ⇒ committed 已存在」被 §A5-S3 支配规则当作健全性前提使用却未钉死——该蕴含仅在首期 flags 恒 Opaque + 三角形几何前提成立(SPV_KHR_ray_query:committed = 「closest recorded hit so far」,存在判据 = `OpRayQueryGetIntersectionTypeKHR(Committed) != None`);candidate/confirm 面(§A4 已登记扩展方向)开放时前提失效,支配规则若静默沿用即产生非法查询放行 | major | **采纳并修 §A3**:健全性前提钉死——支配规则以此前提为健全性条件,candidate/confirm 面开放时必须经 spec 修订行同步重审,不得静默沿用 |
| C-1 | §A5-S3 支配域检查无守卫形态定义:「被 `proceed()`/`has_committed()` true 分支支配」无机验面,spec diff 写不出 accept/reject 语料(可验收性不成立);且支配域检查器为新建面(评审核实:[`dataflow`](../src/rurixc/src/dataflow.rs) 有 fixpoint 框架、无 dominator 计算) | major | **采纳并修 §A5-S3**:守卫形态枚举钉死(① `if rq.proceed()` true 分支;② `if rq.has_committed()` true 分支;③ `while rq.proceed()` 循环体;其余形态一律保守拒,误判方向恒为拒 strict-only)+ 新建面现状如实标注 + §8.2-E 备选止损路径保留 |
| V-1 | §B1 per-entry 判定点只看「MIR 体消费 RayQuery」——**漏「compute 签名含 `AccelStruct` 形参但无 RayQuery 消费」情形**:AS 形参 → descriptor(UniformConstant)→ `OpTypeAccelerationStructureKHR`,其 capability 承载 = `RayQueryKHR`(V-2 核实);判定点不覆盖则该 kernel 留 1.0 且不声明 capability → `OpTypeAccelerationStructureKHR` 无 capability 承载,spirv-val 必拒。判定点与声明规则不自洽(首期三 kernel 同现二者不触发,但冻结规则实现期必踩) | major | **采纳并修 §B1**:判定点并集钉死(消费 RayQuery **或**签名含 `AccelStruct` 形参,任一即升 1.4 + 声明 RayQueryKHR/SPV_KHR_ray_query);W1/W2 五 kernel 无 AS 形参,零漂移面不受并集影响(评审核对五 kernel 签名面) |
| V-2 | §B1 两处规范归因不精确:① `OpTypeAccelerationStructureKHR` capability 承载标「现状未证实」——评审核实 SPV_KHR_ray_query rev 17 已明列(该指令 capability = **RayQueryKHR**;RT 阶段另有 RayTracingKHR 承载路径,compute 面唯一),可升级为规范事实;②「模块版本不得低于 1.4」归因——SPV 扩展自身 **requires SPIR-V 1.0**(rev 17 Dependencies),1.4 要求源自 Vulkan 侧依赖链(VK_KHR_ray_query → VK_KHR_spirv_1_4 或 Vulkan 1.2 核心,+ VK_KHR_acceleration_structure;Vulkan 附录评审核实)+ rurix 自觉沿 RXS-0247 per-entry 口径 | minor | **采纳并修 §B1**:capability 承载升级为规范事实(rev 17);升版依据精确化(如实归因:非 SPV 扩展强制,= Vulkan 依赖链 + 自觉沿 1.4 同律双重依据) |
| V-3 | §C5 W3 七项 probe 链未覆盖 ray query 的 Vulkan 依赖链 **VK_KHR_spirv_1_4 / VK_KHR_shader_float_controls**(VK_KHR_ray_query 依赖前者或 Vulkan 1.2;VK_KHR_spirv_1_4 依赖后者):enable 面漏依赖扩展 → vkCreateDevice VUID 依赖完整性违例;原稿仅标「enable 面现状未证实」未钉依赖链构成 | major | **采纳并修 §C5**:依赖链钉死——probe 面不单列(advertise ray_query 的实现规范强制满足依赖链,传递性由规范承载);enable 面按 api 版本路径分档(≥1.2 核心覆盖 / <1.2 逐项 enable `VK_KHR_spirv_1_4` + `VK_KHR_shader_float_controls`,fail-closed 可捕) |
| V-4 | §A1 禁用指令列漏 `OpCopyMemorySized`(rev 17 同条修订禁 `OpCopyMemory` **与** `OpCopyMemorySized`) | minor | **采纳并修 §A1**:补 `OpCopyMemorySized` |
| A-1 | §1.2/§D0 转述 RD-038 history「validation 零报错」未加 G-G7-3 审计限定——[RD038_LITERAL_MATRIX](../milestones/g7/RD038_LITERAL_MATRIX.md) §4.2-③ 已判该句**未在 evidence 锚定**(既有 `validation_clean` 字段仅为 `RURIX_VK_VALIDATION` 开关记录,非「已开启且零错误」测量),不加剧透限定会误导为已锚定事实 | minor | **采纳并修 §1.2/§D0**:两处转述补 G-G7-3 审计限定(标未证实;validation 零错误须由 G7 新证据以 validation 开启真跑锚定,与 §C3 validation fail-closed 自洽) |
| A-2 | §7.1 称矩阵「行内容随 G7.1 审计任务填充」——G-G7-3 基线审计件 [RD038_LITERAL_MATRIX](../milestones/g7/RD038_LITERAL_MATRIX.md)(2026-08-01,8 行分项)**已存在**且列结构与本冻结列逐列一致(评审核对),未指向则冻结框架与已落审计件脱节 | minor | **采纳并修 §7.1**:指向已存在审计件;并评审核对 §7.2 步骤 93~96 对 8 行分项覆盖完整(GI/RTAO 硬阴影→93→94;HW 光栅/VSM 深度/TSR→95;cull/classify/VisBuffer SW/TAA 帧链并入→96) |

**开放问题逐条裁决**(§9 Q1~Q6 + 首要裁决点):

| # | 裁决 | 理由 |
|---|---|---|
| Q1 builtins 形态 | **接受主设计**(方法族 intrinsic + 构造自由函数 `ray_query_initialize`) | `ThreadCtx` DeviceIntrinsic/`tex.sample` 方法与 `trace_ray` 已知签名先例均在仓,形态一致性最优;接线点现状未证实已如实标注 |
| Q2 AccelStruct 扩展载体 | **接受**(新 RXS 条款登记的 RXS-0245 加性修订行) | RXS-0242→RXS-0153 / RXS-0244→RXS-0155 修订行体例先例在;既有 RT 语义 0-byte 可机验(golden 字节 diff 空);不占旧号 |
| Q3 ray flags/cull mask 首期 | **接受**(恒 Opaque + 恒 0xFF) | 沿 RXS-0245 `trace_ray` 纪律;flags 参数化/candidate 面已登记扩展方向,P-12 克制压过完整性;与 S-3 健全性前提修订自洽 |
| Q4 容差数字 | **接受不预造** | 索引/hit-miss 零容差 + 浮点先 measured 后冻结(P-09);堵住「为过门放容差」loophole;阈值数字只来自真实 GPU baseline |
| Q5 SPIR-V 1.4 升版面 | **接受,经 V-1 修订后判定点并集封闭** | per-entry 判定沿 RXS-0247 版本轴;V-1 修订后 AS-only kernel 情形封闭,判定点与声明规则自洽 |
| Q6 多 TLAS/多 AS 形参 | **接受首期单 TLAS 纪律** | 与章 D1「三 kernel 共用同一真实 TLAS」同源;多 AS 登记扩展方向,首期编译期拒 |
| **首要裁决点:§A5-S3 支配域检查器 vs §8.2-E `try_committed` 聚合备选** | **维持主设计(分立查询族 + 支配域约束);备选 E 保留为 §8.1 止损触发器,不切换** | ① 主设计 committed_* 族与 SPIR-V `OpRayQueryGetIntersection*` 分指令一一对应,§B3 golden 锚定面简单,备选 E 需分支化 lowering(`Option<CommittedHit>` 拆分指令 + 判别式);② 硬阴影早退等「只取 t / 只问 hit-miss」场景在主设计下成本最小,备选 E 单次取全为过取;③ C-1 修订后守卫形态枚举已把检查器复杂度压到「支配关系 + 形态白名单」最小面,且误判方向恒为拒(strict-only)——误报仅要求用户改写为白名单形态,无正确性风险;④ 类型面与 `ThreadCtx`/`tex.sample` 方法族先例一致。**切换条件**(止损触发,§8.1 风险表同源):实现期支配域检查误报/漏报无法收敛 → 经 RFC 修订行切备选 E 并留痕 |

**最终结论**:10 findings(0 blocker / 5 major / 5 minor)全部采纳并修、正文实改完成,无驳回、无未决;§9 Q1~Q6 与首要裁决点全部给出明确裁决。六镜头攻击面核实留痕:S(spec 条款逐条对照,RXS-0245/0247 提法成立、加性修订不矛盾)/ V(SPV_KHR_ray_query rev 17 与 Vulkan 附录公开事实核实,三处归因精确化)/ C(注册点/MIR 表达/per-entry 判定点落地路径成立,「现状未证实」标注诚实且充分)/ R(as_manager/vk.rs/render_exec 复用主张逐项属实——W3 七项、require_wave、dsl_tlas 常量、BLAS/TLAS 两段构建均代码核实,无第二所有者风险,C3 同步主张与 synchronization2 脊柱一致)/ A(冻结决策机验面成立,D4 禁止事项堵 mock/host substitution/isolated nonzero;§7.2 映射覆盖 RD-038 8 行分项)/ Q(开放问题逐条裁决如上)。**全部 blocker/major 关闭 → 状态翻 Agent Approved**(批准链回填见头部字段),先于任何实现 PR(G-G7-2)。

**决策日志处置说明**:本欲按 13_DECISION_LOG.md 体例追加 D 条目记录本次评审裁决,但查 [registry/number_ledger.json](../registry/number_ledger.json) `reserved_in_flight[G7]` 与本 RFC §7.4 均钉死「共享 D 段零消费,D-410 自由池不占;开工裁决记契约 §7」——占用 D-410 会违 G7 编号冻结面,且 registry D 段校准权在编排者。**处置**:不改 13_DECISION_LOG.md;本次评审裁决完整记录于本段,建议编排者按 G7 体例(记 G7_CONTRACT §7 或统一校准 D 段载体)登记。

## 10. 规范与实现依据

- Khronos,SPV_KHR_ray_query 扩展规范(SPIRV-Registry);Khronos「Vulkan Ray Tracing Final Specification Release」(ray query/RT pipeline 模块 SPIR-V ≥ 1.4 有效性规则,调研报告 §1.3 转引);Vulkan `VK_KHR_ray_query`/`VK_KHR_acceleration_structure`/`VK_KHR_deferred_host_operations`/`VK_KHR_buffer_device_address`/`VK_EXT_descriptor_indexing` 扩展与 `VkWriteDescriptorSetAccelerationStructureKHR`。
- 仓内依据:[G7_CONTRACT](../milestones/g7/G7_CONTRACT.md)/[G7_PLAN](../milestones/g7/G7_PLAN.md)/[CI_GATES](../milestones/g7/CI_GATES.md);registry/deferred.json RD-038;registry/number_ledger.json `reserved_in_flight[G7]`;[RFC-0016](0016-native-renderer.md) §4.E3/§4.F2/§9.1 R-3;[RFC-0017](0017-engine-physics.md)(伞形体例先例);[spec/shader_stages.md](../spec/shader_stages.md) RXS-0242~0245;[spec/vulkan_backend.md](../spec/vulkan_backend.md) RXS-0246~0248;[渲染器调研/rurix 渲染器设备化调研报告.md](../渲染器调研/rurix%20渲染器设备化调研报告.md) §1.3/§1.4/§1.5/§1.7;[13_DECISION_LOG.md](../13_DECISION_LOG.md) D-406/D-409。
- 代码现状锚:[`vulkan_codegen`](../src/rurixc/src/vulkan_codegen.rs)(SPIRV_VERSION_1_0/1_4、`build_and_emit_vulkan`/`lower_compute`/`assemble`/`assemble_mesh`/`emit_*_min`);[`shader_stages`](../src/rurixc/src/shader_stages.rs)(`is_accel_struct`/`KNOWN_BUILTINS`);[`hir.rs`](../src/rurixc/src/hir.rs)(`Builtin`/`DeviceIntrinsic`);[`render_exec`](../src/rurix-rt/src/render_exec.rs)(`DeviceCaps`/`KernelWave`/`W3_REQUIRED_CAPABILITIES`/`require_wave`/`KERNEL_WAVE_ROUTES`/`probe_device_caps`);[`vk.rs`](../src/rurix-rt/src/vk.rs)(`run_ray_tracing_offscreen`/`run_rt_inner`/`dsl_tlas`/AS 常量);[`as_manager`](../src/rurix-render/src/rt/as_manager.rs)(`BlasKey`/`BlasCache`/`DynamicPolicy`/`TlasBuilder`/`AsStats`);[`device_kernels`](../apps/uc06-renderer/src/device_kernels.rs)(`device_gate`)。

---

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| Draft v0.1 | 2026-08-01 | AI 起草初版(G7.1 波次;起草 provenance A = `trae-ide:kimi-k3`,**只起草不批准**;四章冻结 + §7 证据矩阵 + §8 风险备选 + §9.1 对抗性评审记录留空待独立 provenance 填写) | Full RFC |
| v1.0 | 2026-08-01 | **对抗性评审第 1 轮完成,状态 Draft → Agent Approved**(评审 provenance B = `trae-ide:kimi-k3 reviewer-session` ≠ 起草 session A,D-409/G-G7-2):10 findings(0 blocker + 5 major + 5 minor)全部采纳并修——§A1(S-1 SPIR-V 存储类收窄 + 禁用指令引证;V-4 补 `OpCopyMemorySized`)/ §A0(S-2 `trace_ray` 未接线如实标注)/ §A3(S-3 健全性前提钉死)/ §A5-S3(C-1 守卫形态枚举钉死)/ §B1(V-1 判定点并集钉死封闭 AS-only 情形;V-2 capability 承载 = RayQueryKHR 升级规范事实 + 升版依据精确化)/ §C5(V-3 依赖链 VK_KHR_spirv_1_4/VK_KHR_shader_float_controls 钉死)/ §1.2·§D0(A-1 validation 零报错 G-G7-3 审计限定)/ §7.1(A-2 指向已落 RD038_LITERAL_MATRIX 审计件);§9 Q1~Q6 与首要裁决点(§A5-S3 主设计维持、§8.2-E 备选保留止损)逐条裁决;13_DECISION_LOG 不改(D-410 自由池 G7 冻结不占,处置说明见 §9.1 末) | Full RFC |
