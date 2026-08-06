# RFC-0019 — G8 渲染平台语义：RT pipeline 增量、单源 gfx submit、shader 变体与反射、时域材质及多队列

| 字段 | 值 |
|---|---|
| RFC 编号 | RFC-0019（4 位制，编号永不复用，10 §9.5） |
| 标题 | G8 渲染平台语义：RT pipeline 增量、单源 gfx submit、shader 变体与反射、时域材质及多队列 |
| 档位 | **Full RFC**（新增 RT pipeline/SBT 运行时与 user-data ABI 相邻面、shader capability/profile 类型规则、时域材质语义、多队列 ownership/timeline/happens-before 语义；触及运行时语义、codegen、FFI/unsafe 相邻边界及内存模型相邻面，10 §3 / AGENTS 硬规则 5） |
| 状态 | **Agent Approved（2026-08-02）**。§9.1 独立 provenance 对抗性评审完成，17 findings 逐条 disposition（1 blocker + 10 major 正文实改，6 minor 留痕/实改） |
| 承接里程碑 | G8.1 governance-only RFC 交付；实施候选分别归 G8.2（M50/M89/M29/M30/M31/M32；M62 仅评估窗）、G8.4（M59 transfer/multi-queue）、G8.5a（M05 WPO/位移实现）、G8.5b（M24）；M28 仅语义先行，实现依决策表当前 **no-go** 留 G8.7 |
| 关联条款 | G8.1 **不领取 RXS 数字号**。拟加性扩展 `spec/shader_stages.md`、`spec/rhi.md`、`spec/vulkan_backend.md`，并在 G8.2 实现门后按 §5 决定是否新建 `spec/rendering_platform.md`；actual RXS 从届时 `number_ledger` 的 `next_free` 领取 |
| 依据决策 | D-402/D-403/D-404/D-406/D-409 · P-01/P-09/P-11/P-12/P-13 · [G8_PLAN](../milestones/g8/G8_PLAN.md) v1.2 §1.0/§2/§3/§5 · [G8_CAPABILITY_MATRIX](../milestones/g8/G8_CAPABILITY_MATRIX.md) M24/M28/M29/M31/M32/M50/M59/M62/M89 · [RFC-0013](0013-industrial-rendering.md) RXS-0242~0248 · [RFC-0015](0015-engine-rendering.md) RXS-0270~0294 · [RFC-0016](0016-native-renderer.md) G5 冻结面 · [RFC-0018](0018-compute-rayquery-device-frame.md) G7/RD-038 互锁事实 |
| Provenance | `Assisted-by: Codex:gpt-5 rfc19-drafter-session` |
| Agent 批准 | **Agent Approved 2026-08-02**；批准只表示语义评审完成，**不构成实现许可**（实现仍由 §2.2 G8.2 互锁与 `ci/check_g8_implementation_interlock.py` 硬门决定） |
| 对抗性评审 | **完成**（D-409）：评审 provenance `Assisted-by: Kiro:claude-opus-5 rfc-review-session` ≠ 起草 provenance `Codex:gpt-5 rfc19-drafter-session`；三镜头 correctness/redline/implementability，findings 与 disposition 见 §9.1 |

---

## 1. 摘要

本 RFC 定义 G8 渲染平台的九个相互依赖语义面：

1. M50：在 RXS-0248 单 raygen/miss/closest-hit 最小见证之上的 RT pipeline 增量——多 hit group、材质记录、SBT user data、stack sizing、pipeline library，以及 any-hit/intersection/callable 的冻结子集；
2. M89/RD-037：由 `.rx` 声明的 gfx graph 到真实 device submit 的单源路径；
3. M29：shader permutation domain、约束裁剪、canonical key 与预算纪律；
4. M31：结构化 reflection schema、interface hash 与 DDC/PSO 键关系；
5. M32：capability requirement 的类型化传播、profile 选择与显式 fallback；
6. M24：TSR 的 history provenance、history resurrection、pixel animation、动态分辨率及 WPO current/previous 双时刻语义；
7. M28：多层 closure 的抽象 IR 与跨路径 lowering 语义；该语义进入 RFC，但代码实现严格取决于候选决策表的 go/strategic_override；
8. M59：graphics/compute/transfer 多队列的 ownership、timeline、跨队列 barrier 与单队列 fallback；
9. M62：task shader RHI 开放前必须消费的可复现实测评估窗。

这些面共享一条总原则：**编译器产出的 reflection/capability/permutation/pipeline manifest 是运行时装配的单一事实源**。运行时只核验并逐字执行，不二次猜测 shader 接口、SBT 布局或资源状态。

```text
.rx source
  ├─ type/capability check ── permutation prune ── codegen artifact
  ├─ canonical reflection ── interface hash ────── DDC/PSO/RT manifest
  └─ declarative gfx graph ── queue plan ───────── device submit/readback
                                      │
                                      ├─ dedicated queues: release → timeline → acquire
                                      └─ fallback: one graphics-capable queue, same dependency order
```

本 RFC 是 **G8.1 governance-only** 交付物。即使本文随后 Agent Approved，也只表示语义评审通过，**不会解锁任何 `src/`、`spec/`、`conformance/` 实现**；§2.2 的 G7/RD-038 互锁与 G8.2 validator 仍是独立硬门。

## 2. 动机、范围与治理门

### 2.1 为什么需要 Full RFC

RXS-0242~0248 已建立 RT 六阶段类型/codegen 与单三件套 Vulkan 见证，但其 SBT 固定为 raygen/miss/hit 各一条、无 user data，`trace_ray` 的 SBT offset/stride/miss index 恒零；这不足以表达真实材质分派、procedural geometry、callable shading 或 Path Tracer 前置。与此同时，M32 会把 device capability 从运行时探测提升为编译期类型规则，M24 会冻结跨帧语义，M59 会把既有单 queue happens-before 扩展为跨 queue 同步图。这些都不是 Direct/Mini 可安全承载的局部实现选择。

本 RFC 触及但不擅自稳定化以下高敏边界：

- SBT record 的 host/device 对齐与 device-address 使用；
- RT pipeline library 的接口兼容与 stack 上界；
- capability/profile 对函数可达图的合法性；
- current/previous 帧材质求值与 history 复用条件；
- queue-family ownership transfer 和跨队列 happens-before；
- 多层 closure 跨 raster/RT/path-tracing 的共同语义。

所有非法构造必须在编译期、装配期或提交前确定性拒绝；本文不设 UB 节，也不允许静默降级。

### 2.2 双门互锁：RFC 批准不等于实现开工

| 门 | 允许动作 | 禁止动作 |
|---|---|---|
| G8.1 governance-only | 起草/评审/批准 RFC；维护契约、决策表、验收映射、symbolic gate 与 measured baseline | 不改 `src/`、`spec/`、`conformance/`；不 materialize 数字 CI 步骤；不领取 RXS/RD/U/RX 共享在途号 |
| G8.2 implementation gate | G7 `status: closed`，且 RD-038 `closed`，或 G7 closed 后六行互锁终态填满并有独立 RD-038 override；validator 读取不可变 refs 后全绿 | 互锁任一红时不得以 RFC Approved、M50 strategic_override 或 owner 权限替代机器事实 |

M50 的 strategic_override 只处理 RD-040 backfill 的逐字条件「RT pipeline/SBT 在『命中点需多样化材质着色』真实出现时**(与 GI hit lighting 同步评估)**」；它**不是** RD-038 override。M28 也必须有自己分项的 go/strategic_override。本文既不登记这两类 override，也不把 Draft/Approved 状态当作其替代品。

### 2.3 in-scope

| 面 | 本 RFC 冻结内容 | 实施波次 |
|---|---|---|
| M50 | RT pipeline manifest、group/record 配对、SBT user data、stack 上界、library link、any-hit/intersection/callable 子集 | G8.2，仅 M50 go/strategic_override 后 |
| M89 | `.rx` gfx graph → artifacts v2 → RHI gfx submit → device readback 的单源语义 | G8.2，承 RD-037 字面 |
| M29 | permutation domain、约束、canonical key、裁剪报告、预算拒绝 | G8.2 |
| M31 | reflection schema、canonical serialization、interface hash、装配核验 | G8.2 |
| M32 | capability requirement 推导、profile compatibility、显式 fallback | G8.2 |
| M24 | WPO 双时刻、motion convention、history provenance/resurrection、动态分辨率与 pixel animation | 语义先行；实现 G8.5b |
| M28 | closure graph、合法性、canonical form、跨路径 lowering 与 loss report | 语义先行；实现仅 go/strategic_override 后 |
| M59 | logical queue、ownership、timeline point、release/acquire、single-queue fallback | G8.4；async-compute 第二腿仍经候选决策 |
| M62 | task-on/task-off 评估语料、测量协议、裁决输出与开放条件 | G8.2 评估；是否实施按评估结果 |

## 3. 指导级解释（用户视角）

### 3.1 一个材质化 RT pipeline

用户从 `.rx` 定义一个 raygen、一个 miss、两组材质 hit group。每个 hit group 绑定自己的 closest-hit，可选 any-hit；每条材质记录携带只读的 typed shader-record data。编译器为全部 entry 生成 reflection，并把 payload、hit attribute、callable data、shader record 与资源接口哈希写入 pipeline manifest。运行时按 manifest 构建 SBT；任何 record schema、group index、对齐、capability 或 library hash 不一致都在 trace 前失败。

最低 device 见证必须让两个几何实例命中两个不同 hit group，并由两份不同 SBT user data 产生可区分像素。重复 RXS-0248 的单三角形双色 hit/miss 不算 M50 绿。

### 3.2 `.rx` 是 gfx submit 的单一来源

用户在 `.rx` 中声明资源、raster/mesh pass、入口函数与 readback。编译器把 SPIR-V artifact、reflection 和图装配 manifest 一起交给 RHI。成功路径不得由 Rust 宿主重新手写同一 pass、重建一份 binding 表或替换 shader 输出；Rust/C ABI 只执行编译器已经冻结的计划。最终 readback 像素断言是 RD-037 的退出证据。

### 3.3 capability 与 permutation 的关系

permutation 描述「同一逻辑 shader 的静态变体」，profile 描述「目标平台保证的能力集合」。编译器先选择 profile，再传播可达调用图的 capability requirements，之后求解合法 permutation。缺少能力时只允许选择 manifest 中显式列出的 fallback variant；不存在“运行时发现不支持后偷偷选另一个 shader”。

### 3.4 多队列并非新 barrier 结构

render graph 仍用 G5 冻结的 EB 三轴 `Barrier { sync_before/after, access_before/after, layout_before/after }` 描述资源状态变化。G8 新增的是计划层 companion metadata：资源所有者、producer timeline point、release/acquire 配对与 consumer wait。设备没有专用 transfer/compute queue 时，同一图按依赖序折叠到一个 graphics-capable queue；输出和资源最终状态必须与多队列计划等价。

## 4. 参考级设计

### 4.0 跨面不变量

1. **P-11 单源**：compiler manifest 是 shader interface、pipeline group、record layout、capability、permutation key 的唯一事实源；runtime 不从 SPIR-V、源码名或 host struct 再推导第二份语义。
2. **strict-only**：缺 entry、缺 capability、hash mismatch、SBT 越界、timeline 环、ownership 不配对、history provenance 不兼容均确定性拒绝或显式 invalidate；不得静默继续。
3. **deterministic**：相同源码、edition、target、profile、permutation 与编译参数得到逐字节相同 canonical reflection、interface hash 与 manifest。
4. **no host substitution**：device 验收路径不得用 host 参考结果回填 device buffer；host oracle 只参与对拍。
5. **非 stable ABI**：SBT 物理字节布局、backend handle、queue-family 数值和 driver stack query 值为实现确定、gate 后、非 stable；reflection schema/version 与语义哈希规则由 spec 冻结。

### 4.1 M50 — RT pipeline 与 SBT 增量

#### 4.1.1 Pipeline manifest 与 group 模型

一个 G8 首期 RT pipeline manifest 由以下有序域组成：

- `raygen`：恰好一个 entry；
- `miss[]`：至少一个，可多条；
- `hit_groups[]`：至少一个，每组有稳定的 manifest-local `group_index`；
- `callables[]`：可为空；
- `libraries[]`：可为空，按 manifest 声明序链接；
- 单一 payload schema，以及按 entry/group 记录的 hit-attribute、callable-data、shader-record schema hash；
- pipeline layout/interface hash、required capabilities、maximum recursion depth、stack summary。

`group_index` 只在该 pipeline manifest 与其 interface hash 下有意义，不能跨 pipeline 缓存复用。runtime 以显式 `(instance, geometry) → hit_group_index/material_record_index` 映射选择 hit group；越界、漏映射、重复但不一致映射均在 AS/SBT 装配期拒绝。

首期 hit group 形态冻结为：

| 形态 | 必选 entry | 可选 entry | 非法组合 |
|---|---|---|---|
| triangles | `closesthit` | `anyhit` | 带 `intersection` |
| procedural | `intersection` + `closesthit` | `anyhit` | 无 `intersection` 的 procedural group |

同一 pipeline 内全部 hit/miss entry 的 payload schema 必须逐字段一致；intersection 产生的 hit attribute 必须与该 group 的 any-hit/closest-hit 消费类型一致；callable entry 与调用点的 callable-data 类型必须一致。既有 RXS-0244 的“单三件套配对域”由新条款加性扩为 manifest 全域配对，不改 RX3012 已冻结含义。

#### 4.1.2 SBT user data 与材质记录

shader-record data 是每条 SBT record 中 shader handle 之后的只读 typed payload：

- 参数形态沿既有标注式 I/O 体例，采用 `#[shader_record] record: &R`；该参数仅对 raygen/miss/closesthit/anyhit/intersection/callable entry 合法；
- `R` 首期只允许固定大小 POD：标量、定长向量、定长数组与由这些字段组成的结构；禁止资源句柄、裸指针、引用字段、runtime array 和递归类型；
- compiler reflection 给出字段序、语义类型、offset、size、alignment 与 record schema hash；host 只能依据 reflection builder 编码，禁止 `repr(C)` host struct 直接 memcpy 充当契约；
- 不同 group 可用不同 `R`，SBT region stride 取满足设备属性和本 region 最大 record 的对齐值；每条 record 的实际 schema hash 与目标 group 必须精确匹配；
- record bytes 在 pipeline 生命周期内不可变；更新材质参数必须生成新 record buffer/新 generation，并在旧 trace 完成后回收。

SBT user data 的物理 offset/stride/device address 不进入 stable API。可复现面是 compiler reflection、packer 输入和输出 golden；device 属性导致的 padding 差异进入 environment/evidence，不进入 interface hash。

#### 4.1.3 冻结的 any-hit / intersection / callable 子集

- **any-hit**：triangles/procedural 均可选。首期开放“接受默认交点”与 `ignore_intersection()`；不开放用户可写 traversal order、递归 trace 或任意 side effect。调用次数与顺序实现定义但有界；最终最近的未忽略交点语义确定。
- **intersection**：仅 procedural group 合法；以既有 `report_intersection(t, attr)` 报告候选，`t` 必须位于当前 ray 的合法区间，attribute 类型与 group reflection 精确一致。无报告表示 miss；非法区间报告在 shader 合法性检查或 validator 路径确定性拒绝，不定义 UB。
- **callable**：只允许 raygen 或 closest-hit 通过 `execute_callable(index, data)` 调用 manifest 中已声明 callable；index 必须可证明落在 manifest callable 域。首期禁止 callable 嵌套调用 callable，也禁止 callable 内 `trace_ray`；callable-data 逐字段匹配。
- **递归**：沿 RXS-0245，`trace_ray` 仍只在 raygen 可达域合法，maximum ray recursion depth 固定为 1。M50 不借“完整 pipeline”静默放宽递归。

any-hit 的 `terminate_ray`、**运行期动态**改变的 ray flags 与 SBT offset/stride/miss index、递归 trace 与 callable nesting 均不在首期冻结子集。此处「动态」= trace 调用点按运行期值改写 SBT 寻址；§4.1.1 的 `(instance, geometry) → hit_group_index` 是**装配期**静态映射，二者不冲突（评审 F6 澄清）。

`ignore_intersection()` 的签名与终结语义：`fn ignore_intersection() -> !`，仅在 `anyhit` 阶段合法（其他阶段调用 → 编译期拒），语义为「丢弃当前候选交点并立即终止本次 any-hit 调用」，不影响后续候选交点的遍历，也不改变 §4.1.3 的「最终最近未忽略交点」确定语义（评审 F17）。

#### 4.1.6 冻结面修订行（RXS-0244 / RXS-0245 / RXS-0248；评审 F6 blocker）

多 hit group、多 miss、材质记录与 SBT user data **不是纯加性扩展**，它们实质修订三条既有冻结条款。按 14 §1 与 07 §5 纪律，此处逐条给出原句与修订后句；G8.2 的 spec PR 必须以本表为唯一修订依据，并附 golden 零漂移证明。

| 条款 | 原冻结句（逐字） | 修订后句 | 零漂移证明计划 |
|---|---|---|---|
| `spec/shader_stages.md` RXS-0244 | 「首期配对域 = **单编译单元 + 单 RT 管线三件套**（raygen×1 + miss×1 + closesthit×1）；多 payload / 多 hit group 的 SBT 序配对越出首期 → 编译期拒」 | 配对域扩为 **单编译单元 + 单 RT pipeline manifest 全域**：单一 payload schema 仍逐字段一致，miss/hit group 可多条，每 group 独立 attribute/record schema 全域静态比对；越出 manifest 域或 schema 不一致仍 → `RX3012` | 既有 `rt_payload_pair_is_clean` / `rt_payload_mismatch_is_rx3012` 判据 0-byte 恒跑；新增多 group 语料只加不改 |
| `spec/shader_stages.md` RXS-0245 | 「ray flags 恒 opaque、cull mask 恒 0xFF、SBT offset/stride/miss index 恒 0（单三件套唯一确定）；扩展参数越出首期 → 编译期拒」 | ray flags 与 cull mask 首期**仍恒定不变**；SBT 寻址由 §4.1.1 的装配期 `(instance, geometry) → group_index` 静态映射确定，不再恒 0；trace 调用点仍**不接受运行期动态** offset/stride/miss index 实参，递归深度仍恒 1 | `trace_ray` 已知签名与递归深度判据 0-byte；新增「运行期动态实参 → 编译期拒」RED 语料 |
| 🔒 `spec/vulkan_backend.md` RXS-0248 | 「按对齐铺三 region（raygen/miss/hit **各单条目**）……SBT 内**不嵌用户数据**」 | region 数仍为三；miss/hit region 允许**多条 record**，record 内允许 §4.1.2 的 typed `#[shader_record]` 只读 POD；raygen size == stride、region baseAlignment 对齐、stride ≥ handleSize 且为 handleAlignment 整数倍等对齐律**全部 0-byte 保留** | `plan_sbt_alignment_invariants` / `plan_sbt_nvidia_typical_exact` / `align_up_*` 判据 0-byte 恒跑；多 record 铺设作为新增纯 host 单测 |

三条修订均**只扩容量与配对域，不放宽任何 fail-closed 判据**；RXS-0248 的 🔒 device-address/U30 审计边界与 validation fail-closed 语义不变。若 G8.2 实现发现上表任一句无法在不改既有 golden 的前提下落地，必须停止实现并先修订本 RFC。

#### 4.1.4 Stack sizing

pipeline builder 必须对最终链接后的每个 shader group 查询 backend stack requirement，并结合 manifest 的可达调用图、callable 上界与 recursion depth=1 计算保守上界。产物记录：

- 每组 query 值；
- 采用的计算规则版本；
- final stack size；
- device/driver identity 与 capability snapshot。

final stack size 不能由样例常量、单 hit group 值或 host 猜测代替。runtime 在 trace 前核验 final size 不小于同一 pipeline generation 的计算值；人为缩小必须 RED，计算值必须 device GREEN。不同 driver query 值不影响 interface hash，但影响 pipeline cache device key。

#### 4.1.5 Pipeline library

library export 单位为完整 shader group/callable entry，不允许导出半个 hit group。每个 library manifest 必须携带：layout/interface hash、payload/attribute/record schema hashes、required capabilities、group export names、stack summary 与 compiler/edition identity。

最终 link 规则：

1. 按主 manifest 的 library 与 export 声明序确定 group index；backend 不得自行排序改变索引；
2. duplicate export、缺 export、layout/hash/profile 不兼容、payload/record 不兼容均在 pipeline create 前拒绝；
3. final pipeline 重新计算 stack 上界并生成新的 generation/hash；library 的局部 stack 值不能直接冒充 final 值；
4. pipeline cache 命中仍须复核 final manifest hash，不能仅以 vendor cache handle 判同。

**分库 ≡ 单体等价语义（评审 F14 补齐）**：对同一组 shader group，「分库创建 + 链接」与「单体一次创建」必须产生**可观察等价**的 pipeline——同一 `(instance, geometry) → group_index` 映射、同一 record schema 配对、同一 payload/attribute 语义，因而在同一场景同一 SBT 下 device 输出逐像素相等（`g8.p0.m50.rt_pipeline_incremental` 的第 ④ 项判据即此等价）。允许不等的只有：final stack size 数值（driver query 差异）、vendor cache blob 与 pipeline handle。等价性由 golden 比对证明，不得以「两条路径都能跑」替代。

### 4.2 M89 / RD-037 — `.rx` gfx submit 单源语义

RD-037 的完成定义逐字承接为：rurixc lowering 将 `.rx` gfx pass 的 vertex/fragment SPIR-V 写入 artifacts v2，C ABI/RHI 具备 VB/IB 与资源绑定，`rxrt_rhi_submit` 的 gfx arm 真派发，最终由 `.rx` 图 readback 像素断言。

规范不变量：

- `.rx` graph 的 pass 序、resource identity、entry reference、VB/IB view、draw parameters 与 readback 是提交计划的单一输入；
- compiler manifest 绑定 artifact digest + interface hash + reflection resource set；RHI `seal` 对声明集和 reflection 做双向精确相等核验；
- gfx pass 必须消费真实 compiler artifact，不接受固定最小 SPIR-V、Rust 内嵌 shader 或 host-computed pixel buffer；
- VB/IB 的 element format、offset、extent 与 index type 在装配期检查，越界 draw 在 submit 前拒绝；
- readback 只发生在图完成点之后，且输出 provenance 指向同一 submit generation；
- device 不满足 profile 时 fail-closed；不得改走 CUDA compute 或软件光栅冒充 gfx submit。

### 4.3 M29 — Shader permutation 语义

`PermutationDomain` 是有限、版本化的静态轴集合。每个 axis 有稳定名称、封闭值域与来源（project/material/pass/profile）；首期值只允许 bool、整数枚举与 identifier 枚举。约束表达式是无副作用的编译期布尔式，只引用同一 domain 的 axis 与已选 profile capability。

编译流程固定为：

1. 规范化 axis 声明并拒绝重名/空值域；
2. 求解约束，删除不可满足组合；
3. 对每个可达组合生成 canonical key；
4. 按 `g8_budget` 的 measured 上限核验 variant count、compile time 与 artifact bytes；超限硬失败并输出 axis contribution report；
5. 只把实际可达且被 shader/PSO manifest 引用的变体写入 DDC。

canonical key 使用 axis 名字节序排序，值采用带类型标签的规范编码；声明顺序、线程数、临时路径与 hash-map 迭代序不得影响 key。两个不同语义组合不得得到同 key；同一组合跨两次 clean build 必须逐字节相等。

运行时选择必须精确命中 `(profile_digest, permutation_key, interface_hash)`。缺 variant 是确定性错误；不允许选择“最接近”变体。profile fallback 必须在构建期生成独立、可寻址 variant。

### 4.4 M31 — Reflection schema 与 interface hash

reflection v1 至少包含：

- compiler schema version、edition、target/backend、entry name 与 stage；
- stage I/O 的 field order/type/interpolation/builtin；
- resource class、set/binding、count、access、format 与 push-constant range；
- payload、hit attribute、callable data、task payload、shader-record 的字段/offset/size/alignment；
- RT group membership 与 library export identity；
- required capabilities、selected profile digest；
- permutation domain digest 与本 variant key。

canonical serialization 使用版本前缀 + length-prefixed binary fields；map/set 一律按规范键排序；不得包含绝对路径、mtime、进程 ID、随机 seed、backend handle 或 driver query 值。

`interface_hash = SHA-256("rurix.shader-interface.v1\0" || canonical_interface_bytes)`。

source/artifact digest 与 interface hash 分离：仅改变函数体而不改变接口时 interface hash 可保持不变，但 artifact digest 必须改变；改变资源、I/O、payload、record、capability 或 profile compatibility 时 interface hash 必须改变。DDC/PSO/RT pipeline key 同时包含 artifact digest、interface hash、profile digest、permutation key、compiler/edition 与 target identity。

runtime 只比较 hash 后仍需在 debug/validation 路径核对 schema version 与关键字段，防止错误归因。hash mismatch 绝不通过重反射或 host layout 猜测修复。

### 4.5 M32 — Capability/profile 类型化检查

#### 4.5.1 Capability requirement

着色入口与 device function 可用 `#[requires("capability.id", ...)]` 声明显式 requirement；编译器还必须从 intrinsic、stage、resource type 和 pipeline feature 推导隐式 requirement。函数 `f` 的有效 requirement 是自身显式/隐式集合与所有可达 callee requirement 的并集。

首期 capability ID 至少覆盖：RT pipeline、SBT user data、any-hit、procedural intersection、callable、task shader、timeline semaphore 与 dedicated transfer/compute queue。具体 backend extension 名不作为语言 capability ID；mapping 归 target profile。

**冻结的 symbolic diagnostic key（评审 F13 blocker 级补齐）**：`g8.p0.m32.capability_profile` 要求「以冻结的 symbolic diagnostic key 确定性拒录」，故本 RFC 在此冻结四个诊断类别的符号名（**符号名，不是 RX 数字号**；实号按 §5 错误码策略在实现期从各段 actual next_free 领取，en/zh message key 成对）：

| symbolic diagnostic key | 触发条件 | 判据形态 |
|---|---|---|
| `capability.missing_required` | entry 有效 requirement 含 selected profile 未提供的 capability | 编译期 RED，消息列出缺失 capability ID 与首个引入它的可达 callee |
| `capability.forbidden_used` | entry 使用 profile 显式 `forbidden` 的 capability | 编译期 RED |
| `capability.fallback_incompatible` | manifest 声明的 fallback variant 与主 variant 对外 interface contract 不兼容 | 编译期 RED，消息给出不兼容字段 |
| `capability.runtime_snapshot_mismatch` | 运行期 device capability snapshot 不满足产物所选 profile | 装载期 RED（fail-closed），禁止临时重编或换 profile |

四个 key 的字面名是 M32 三腿（accept / reject / fallback）判据的一部分：reject 腿必须精确匹配 `capability.missing_required`，fallback 腿必须在低 profile 下不发任何上述 key。

#### 4.5.2 Profile

profile 是版本化闭集：`required`、`optional`、`forbidden` capability，target minima，以及可选的显式 fallback mapping。profile 由项目/构建 manifest 选择，不从当前开发机自动生成。编译期规则：

- entry 有效 requirement ⊆ profile 可提供集合时合法；
- 命中 forbidden 或缺 required capability 时编译 RED；
- fallback 只有在 manifest 指向另一个已编译 entry/permutation 且两者对外 interface contract 兼容时合法；
- runtime device capability snapshot 必须满足产物所选 profile；否则装载 RED，不临时重编、不静默换 profile。

single-queue fallback 是两个显式 queue plan 的选择，不是忽略 `timeline` requirement。多队列 plan 要求 timeline/queue capability；fallback plan 不要求它们，二者共享同一资源依赖图与输出契约。

### 4.6 M24 — TSR、pixel animation 与 WPO 时域语义

#### 4.6.1 Motion convention 与 WPO 双时刻

motion vector 采用 jitter-free 输出像素坐标约定：

`previous_output_pixel = current_output_pixel + motion_vector`。

WPO 顶点的 motion 必须分别求值：

- current：当前 object transform、当前 view/projection、当前 material/WPO parameters 与 current time；
- previous：上一已提交帧对应的 object transform、view/projection、material/WPO parameters 与 previous time。

只用 object transform、把 current WPO 结果复用为 previous、或缺 previous parameters 时写零 motion 均非法。缺少 previous 输入时，该对象覆盖像素必须显式标 history invalid；这是可见降级，不是零速度伪装。

#### 4.6.2 Temporal provenance

history sample 只可在以下 provenance 相容时复用：view identity、resource generation、history epoch、previous/current extent 映射、projection/jitter sequence、exposure domain、material interface hash 与 motion convention version。任一不相容都使相关 sample invalidate。

动态分辨率重投影先在 normalized viewport 中消除 current/previous jitter，再分别映射到两帧输出 extent；不得把 input-resolution texel offset 直接当 output-resolution motion。

#### 4.6.3 Pixel animation、透明 velocity 与 resurrection

- material IR 必须标出 pixel animation 对可见辐射的时变影响。能提供运动的路径写 velocity/confidence；不能提供可靠运动的路径写 reactive/history-reject 信号，禁止静默当静态材质；
- 透明 pass 若参与 TSR history，必须显式声明 velocity/coverage provenance；缺失时该透明贡献走 reactive reject，不继承后方不相关 motion；
- history resurrection 只允许从有界 age 的 retired sample 中恢复，且必须匹配同一 provenance key、通过 depth/normal/material compatibility，并保留 confidence；不能跨 camera cut、resource generation、profile/interface hash 或 history epoch 恢复；
- **thin geometry（评审 F12 补齐）**：单像素或亚像素宽度的几何（细线、栏杆、发丝、薄片）在 input resolution 下可能在相邻帧间完全丢失覆盖。规范要求：① 该类像素的 history 复用必须以 coverage/confidence 而非单一 depth 比较判定，depth 判据在覆盖不连续时不得单独否决 history；② 丢失覆盖的帧必须写显式 low-confidence 而非零 velocity；③ 禁止用邻域 motion 外插填补 thin geometry 的 velocity。对应 `g8.p0.m24.tsr_contract` 的 `thin_geometry` case。
- **tolerance 冻结程序（评审 F12）**：本 RFC 不预造数字，但必须给出冻结路径——五个 TSR case 的 golden digest 与逐 case 误差 tolerance 由 G8.5b 的首批 measured corpus 采样后，以**本 RFC 的加性修订行**冻结（与 `g8_budget.json` 的 measured 条目同 PR），此后 `g8.p0.m24.tsr_contract` 才可判 GREEN。冻结前该门只能是 RED/未实现，不得以「tolerance 待定」为由跳过；最大 age 等 tuning 参数仍属 budget/evidence，不入语言 stable 面。

#### 4.6.4 G8.5b M24 measured→frozen（加性修订行；2026-08-06）

首批 corpus = RTX 4070 Ti local freeze（`tests/tsr_contract/freeze.json`；`resurrection_age_max=6`）。逐 case **max_abs ≤ tolerance**（tolerance = measured×2）；digest 为 device 末帧 SHA-256。与 `g8_budget.json` measured 条目同 PR 提升为 `rfc_budget_frozen`。

| case | golden digest (SHA-256 hex) | tolerance (max_abs) |
|---|---|---:|
| `history_resurrection` | `7dfc2c73598795a373ead4484d4c91488594e7eed246dcb56481d402d42daeea` | 1.667216897 |
| `pixel_animation_velocity` | `c216629b3ca1c4228a722817fd436efa6678bff12d1fb20223ceed6685c8f451` | 0.3430680036 |
| `thin_geometry` | `f713aa759438e93ad72916e733981fb8be2a72f09bfd4b6314a9a6cb484fd64b` | 1.3896520138 |
| `dynamic_resolution` | `f76f35d9ccc6ee87a39f6338ed7a152d3b19f82a9d7066dafc19aa2807af00bd` | 0.02785670758 |
| `transparent_velocity` | `014072e8d0d7b0593f29fa7f190354a3b9fb792b20b9b4a62405199477e7d802` | 1.5806208848 |

异机/驱动分歧须另开 RFC 加性修订行，禁止静默放宽容差。

### 4.7 M28 — 多层 closure 抽象语义（实现条件臂）

RFC 层冻结一个 backend-neutral、acyclic closure graph：

- leaf：既有单层 `MaterialClosure` 的参数化表面；
- `mix(a, b, weight)`：`weight ∈ [0,1]` 的凸组合；
- `layer(top, base, coverage)`：有序 top-over-base，`coverage ∈ [0,1]`；顺序具有语义，不可交换；
- graph 在单态化后必须有限、无环，并受 selected profile 的 node/depth 上限约束；
- canonical form 保留 source layer order，常量折叠必须确定性；不得为优化重排非交换 layer；
- raster、RT hit shading 与 Path Tracer 必须消费同一个 canonical closure graph/interface hash，禁止各自维护材质语义分叉。

G5 冻结的 `MaterialClosure` 32B 单层布局 **0-byte 保持**。多层 graph 是编译/资产 IR，不得把节点偷偷塞入 32B 结构或改其字段含义。backend/profile 不能原生表达多层时，只有两条合法路径：

1. strict profile 在构建期拒绝；
2. profile 显式允许 deterministic flatten，并生成 loss report（被丢弃/合并的节点、能量与参数误差指标）及新的 interface/artifact digest。

本文批准也不授权实现 M28。只有 `G8_CANDIDATE_DECISIONS.md` 的 M28 行为 go 或 strategic_override，且证据/override 完整后，才可落 spec 条款、RED 与实现；no-go 时本节只作为未来语义边界，不产生 gate 绿或实现债务。

### 4.8 M59 — 多队列 ownership、timeline 与 barrier

#### 4.8.1 Logical queue 与物理映射

logical queue class 封闭为 `Graphics`、`Compute`、`Transfer`。pass 声明的是 capability requirement/preference，不是对硬件 family index 的承诺：

- graphics pass 只能映射到 graphics-capable queue；
- compute pass 可映射到 dedicated compute 或 graphics-capable queue；
- transfer pass 可映射到 dedicated transfer、compute-capable 或 graphics-capable queue；
- 物理队列选择与 family index 进入 execution evidence，不进入图的语义 hash。

首期只支持 exclusive-sharing resource。concurrent-sharing、跨 device、多 GPU 与 external queue ownership 不在本 RFC。

#### 4.8.2 Ownership/timeline 状态机

每个 resource subrange 的计划态至少含 `(owner_family, eb_state, last_writer_timeline_point, generation)`。跨不同 family 的 producer→consumer 边必须形成同一个 `QueueTransferId` 的五步序列：

1. producer 完成写入；
2. producer 录制 release barrier，EB before/after 取自现有状态推导；
3. producer 在自己的单调 64-bit timeline 上 signal `TimelinePoint(queue, value)`；
4. consumer submit wait 该精确 point；
5. consumer 录制 acquire barrier 后才可访问，并把 owner 更新为 consumer family。

release/acquire 的 resource、subrange、generation、src/dst family 与 EB transition 必须成对相等。wait 缺失、wait 错 value、双 owner、acquire 无 release、release 无消费或 timeline 回退均在提交前 validator RED。timeline dependency graph 必须无环；同一 queue 只能 signal 严格递增值。

同一 family 内跨 queue 时不发生 family ownership transfer，但仍必须有 timeline wait 与适当 memory barrier。只读并行须经 planner 证明无 writer；首期不开放 queue 间 concurrent write。

#### 4.8.3 单队列 fallback

设备无专用 transfer/compute queue、无 timeline 能力或 G8.4 多队列 gate 未开时，planner 必须生成显式 single-queue plan：

- 全部 pass 按原 dependency DAG 的确定性拓扑序映射到一个 graphics-capable queue；
- 不生成 ownership transfer 或跨队列 timeline wait；
- copy/compute 使用该 queue 合法支持的命令；
- 资源 EB 前后态、最终内容 digest、readback 与多队列计划一致；
- evidence 标明 `single_queue_fallback`，不得把它计作多队列重叠性能绿，但它是 portability correctness 硬门。

#### 4.8.4 G5 Barrier EB 冻结面边界

本 RFC **不修改** G5 `Barrier { sync_before/after, access_before/after, layout_before/after }` 的字段、枚举含义、推导规则或 golden。ownership、timeline、queue mapping 与 release/acquire pairing 作为 companion plan metadata 新增；它们不是第四个 EB 轴，也不是 D3D12 enhanced-barrier split flag。

若 G8.2 的真实 spec/RED 证明现有 EB 枚举无法无损表达某个 release/acquire 的状态或可见性，必须停止实现：先对 RFC-0019 做加性修订并在 **G8.2 先行**落 spec 修订行 + RED + G5 golden 零漂移证明，经评审后才可继续 runtime。不得在 `src/` 私加隐式状态绕过 spec。

### 4.9 M62 — Task shader 评估窗

task stage 的语法与类型面见 RXS-0242~0246，但**在树事实是 mesh-only**：RXS-0275 明记 task payload 属条件臂，条件臂外发 `RX6026`，因此今天无法构造 task-on 产物（评审 F1）。评估窗的第一步就是让 task-on 产物**可构造**——这属条件臂兑现，必须在 G8.2 实现门开放后按 spec-first 落条款，不得以「stage 语义已在」冒充产物已在。M62 归属沿 G8_PLAN §2.7 的 **P2 穷举**（评审 F4）：本节只冻结评估协议，评估本身可在 G8.2 与其他面并行，是否实施仍由 G8.7 判档。评估窗必须在任何 task RHI 实现前完成：

1. 用同一 mesh shader workload 构造 task-off 与 task-on 两条产物；task-on 只做 meshlet group cull/amplification，输出与 task-off 同图像/同可见 primitive 集；
2. 至少覆盖低剔除率、高剔除率、放大压力三个固定 corpus；
3. 同一 device/driver/profile、同一分辨率和资产，warm-up 后独立多 trial，使用 GPU timestamp；同时记录 task invocations、mesh invocations、visible groups、pipeline compile/cache 数据；
4. 正确性先过逐像素/primitive digest，再按 `g8_budget` 的 measured noise floor 与收益阈值裁决；本文不预造百分比阈值；
5. 输出 `go` 或 `no-go/defer-to-G9+` 及 evidence 路径。仅“硬件支持 task shader”不能判 go。

go 后才可扩 RHI task→mesh arm，并继续复用 RXS-0243 task payload 契约；no-go 时 RHI 维持确定性拒绝，M62 在 G8.7 穷举表留痕，不影响其他 M50 功能。

## 5. 下游 spec diff 计划（G8.2 实现门后 materialize）

下表的 `RP-*` 是本 RFC 内部 diff key，**不是 RXS 编号或编号占位**。真实条款号只在 §2.2 implementation gate 全绿后读取 `number_ledger` actual `next_free` 并逐条领取；每条 materialize 时必须至少一个 `//@ spec: RXS-实际号` 锚点，`trace_matrix` 全锚定。

| Diff key | 目标 spec | 规范 diff | 最小测试锚定计划 |
|---|---|---|---|
| RP-RT-GROUPS | `shader_stages.md` | RXS-0244/0245 加性扩展：manifest 全域 payload/attribute/callable 配对、多 hit group 与冻结 stage 子集 | accept 两 triangle groups；reject payload/attribute mismatch、非法 group 组合 |
| RP-SBT-RECORD | `shader_stages.md` + `vulkan_backend.md` | `#[shader_record]` POD 类型规则、reflection layout、SBT pack/alignment/generation | host pack golden；reject handle/pointer/错 schema；device 两材质 user-data 像素差异 |
| RP-RT-STACK-LIB | `vulkan_backend.md` | group stack query、保守 final sizing、library export/link/hash 规则 | undersized RED；computed size GREEN；library layout/hash mismatch RED |
| RP-GFX-SUBMIT | `rhi.md` | `.rx` gfx graph artifacts v2、VB/IB binding、gfx arm 真 submit/readback | zero-Rust-host `.rx` device 像素 GREEN；artifact/reflection/VB range mismatch RED |
| RP-PERMUTATION | `rendering_platform.md` | finite domain、constraint prune、canonical key、budget report | 声明序置换 key 相等；不同组合 key 不同；超预算 RED |
| RP-REFLECTION | `rendering_platform.md` | reflection v1 schema、canonical bytes、SHA-256 interface hash、DDC/PSO key | clean build 双 hash 相等；body-only 改动接口 hash 不变；接口改动 hash 改变 |
| RP-CAP-PROFILE | `shader_stages.md` + `rendering_platform.md` | `#[requires]`、call-graph 传播、profile/fallback、runtime recheck | unsupported capability compile RED；显式 fallback GREEN；runtime snapshot mismatch RED |
| RP-TEMPORAL | `rendering_platform.md` | motion convention、WPO current/previous、history provenance/resurrection、dynamic-resolution mapping | WPO 两帧解析运动 golden；缺 previous→invalidate；camera cut/res change 禁 resurrection |
| RP-CLOSURE | `rendering_platform.md` | closure graph 类型/合法性/canonicalization/cross-path lowering/loss report；仅 M28 go/override 时落 | 条件 GREEN：raster/RT 同 graph hash；cycle/depth/profile reject；flatten loss report golden |
| RP-MULTIQUEUE | `rhi.md` + `rendering_platform.md` | QueueTransfer companion plan、timeline DAG、release/acquire、single-queue fallback；G5 EB 0-byte | ownership/wait 缺失 RED；transfer→graphics device GREEN；fallback output digest 相等 |
| RP-TASK-WINDOW | `rendering_platform.md` | task-on/off corpus、measurement schema、go/no-go 规则；go 后才扩 `rhi.md` | 三 corpus correctness + timestamp evidence；无 evidence 不得 open RHI arm |

**错误码策略**：G8.1 零 RX claim。G8.2 后优先复用已冻结的接口/资源/后端不支持类别；只有实现证明出现新的、用户可行动、可独立到达的诊断类别时，才按当时各段 `next_free` 只追加，并同步 en/zh message key。库装配错误优先用 typed `Err`，不为每个状态预造 RX。

## 6. Feature gate、tracking 与 RED/GREEN 实现序

### 6.1 Gates

本 RFC **不新造第三套 gate 命名空间**（评审 F9/F11 blocker 级实改）。唯一合法 key/脚本事实源是 [`G8_ACCEPTANCE_MAP.md`](../milestones/g8/G8_ACCEPTANCE_MAP.md) §2/§3 与 [`CI_GATES.md`](../milestones/g8/CI_GATES.md) §4/§4.0 的 `g8.p{0,1}.m##.<slug>` + `ci/g8_<slug>_smoke.py`，由 `ci/check_g8_acceptance_map.py` 三向比对强制。本 RFC 覆盖面与既有 key 的对应关系：

| 覆盖面 | canonical gate key（外部事实源，本文不新增） | 额外前置 |
|---|---|---|
| M50 RT 增量 | `g8.p0.m50.rt_pipeline_incremental` | M50 strategic_override 已登记 + G8.2 implementation gate |
| M89/RD-037 | `g8.p0.m89.single_source_gfx_submit` | G8.2 implementation gate |
| M29 permutation | `g8.p0.m29.shader_permutation` | **独立硬门**，不与 M30/M31/M32 合并 |
| M30 PSO cache | `g8.p0.m30.pso_cache` | **独立硬门** |
| M31 reflection hash | `g8.p0.m31.reflection_hash` | **独立硬门** |
| M32 capability/profile | `g8.p0.m32.capability_profile` | **独立硬门** |
| M24 TSR | `g8.p0.m24.tsr_contract` | G8.5b 前置与 RD-038 TSR 接入口径 |
| M28 多层 closure | 当前**无 gate**（决策表 no-go，实现留 G8.7） | 改判 go/strategic_override 后先修 MAP §6 覆盖集合再建 gate |
| M59 多队列 | 归 `g8.p0.m37.streaming_io` 的 `queue_mode=multi` 分支 | **RFC-0019 §4.8 多队列章已 Approved**；否则 G8.4 强制单队列 |
| M62 task 评估窗 | 无 P0/P1 gate（G8.7 穷举项） | 评估窗 measured 报告；go 后另立 gate |

G8.2 互锁前不得在 workflow 放空步骤，也不得为 no-go 项预建 gate。

### 6.2 真实 RED/GREEN

| 面 | RED（必须先可复现） | GREEN（不得以较弱见证替代） |
|---|---|---|
| M50 groups/records | group 越界、record hash mismatch、非法 triangles+intersection | 两 hit group + 两 material record + SBT user data device 像素断言 |
| M50 stage subset | any-hit/attribute/callable mismatch；callable nesting；非法 report range | masked any-hit ignore、procedural intersection、callable data 各一条 device RED→GREEN |
| M50 records（评审 F14 强化） | host 直接 memcpy `repr(C)` struct 充 record；record schema hash 与目标 group 不匹配 | SBT user data 由 host 经 reflection packer 写入，shader readback 与源 bytes **逐字节相等**（不是「像素可区分」即可） |
| M50 stack/library（评审 F14 强化） | 手工缩小 stack；library layout/export mismatch | final-link stack query/计算证据（evidence 记 `configured >= required`）+ 分库链接输出与单体 golden **逐像素相等** |
| M89 | host shader/substitution、artifact hash mismatch、VB/IB 越界 | `.rx` 零 Rust pass 装配真实 gfx submit/readback |
| M29 | 不可满足域、key collision、budget exceed | clean build canonical keys/裁剪报告相同 |
| M31 | path/声明序扰动导致 hash 漂移；接口 mismatch | canonical reflection 双构建相同，接口变更必换 hash |
| M32 | required capability 缺失、runtime snapshot 不符 | profile-compatible variant 与显式 fallback 各自精确命中 |
| M24 | 缺 previous WPO 却零 motion、跨 cut resurrection | WPO/动态分辨率/透明 pixel-animation 序列回归与 provenance 断言 |
| M28 | 仅 go 后：cycle/depth/profile 不支持、跨路径 hash 分叉 | canonical graph + raster/RT 共用 + loss report；no-go 不以“无代码”充绿 |
| M59 | wait/ownership pair 缺失、timeline cycle/value rollback | dedicated transfer→graphics device 见证 + single-queue 相同 digest |
| M62 | 只有 capability probe 或单次 timing | 三 corpus 多 trial correctness-first 报告，明确 go/no-go |

### 6.3 栈式实现序

1. **PR-Gate**：只读 validator 证明 G7 closed 与 RD-038 closed/终态接入 override；重读 ledger actual `next_free`。红即停止。
2. **PR-Spec**：按 §5 materialize 实际 RXS 与 RED 语料；条款 commit 先于实现 commit。若触 G5 EB 缺口，先做 §4.8.4 修订，其他多队列实现等待。
3. **PR-ShaderPlatform**：M29/M31/M32 canonical schema/key/typecheck；纯 host deterministic RED/GREEN 先绿。
4. **PR-RT**：M50 group/record/stack/library 编译与 Vulkan runtime；unsafe 优先复用既有 AS/SBT/device-address 审计边界，确有新边界才领 U。
5. **PR-GfxSubmit**：M89 `.rx` gfx single-source 真派发与 readback。
6. **PR-MQ**：M59 planner/validator、single-queue equivalence，再开 dedicated transfer；async compute 依候选裁决。
7. **PR-Temporal**：M24 序列 corpus、WPO previous-state 与 TSR；与 RD-038 TSR 遗留门逐字对齐。
8. **PR-Conditional**：M28 仅 go/override 时；M62 先评估、go 后才开 task RHI。
9. **PR-Evidence**：`G8_ACCEPTANCE_MAP` 指向真实 CI/evidence schema；RTX 4070 Ti validation-on device run，禁止 YAML-only 与 host substitution。

## 7. 备选方案

| 方案 | 裁决 | 理由 |
|---|---|---|
| 继续用 RXS-0248 单 hit group 作为“完整 RT” | 否决 | 不能承载材质记录/SBT user data/stack/library，直接触 R-G8-1 假绿 |
| 允许 host 按 vendor struct 直接 memcpy SBT data | 否决 | 把物理 ABI 冒充 stable 契约，绕过 reflection/hash 与对齐核验 |
| permutation 运行时按“最近值”回退 | 否决 | 不可复现，掩盖缺 variant 与 capability mismatch |
| 从 backend blob 反射并让 runtime 猜接口 | 否决 | 形成第二事实源，违 P-11；compiler reflection 必须主导 |
| capability 只做 runtime probe | 否决 | 非法调用直到装载/派发才失败，无法裁剪 permutation，也无法证明 fallback 完备 |
| 为多队列直接给 G5 `Barrier` 加 queue 字段 | 否决 | 破坏 EB 三轴冻结面；ownership/timeline 是 companion plan 语义 |
| 无专用 queue 时 SKIP 多队列图 | 否决 | portability 不成立；必须有结果等价的 single-queue fallback |
| 无条件实现多层 closure | 否决 | RD-041 backfill 仍需真实资产瓶颈或 strategic_override；RFC 语义不等于触发证据 |
| 看到 task capability 就开放 RHI arm | 否决 | capability 不证明产品收益；必须消费 M62 measured 评估窗 |

## 8. 不做（范围红线）

- 不修改 G5 `Barrier` EB 三轴、`GpuScene`、`MaterialClosure` 32B（含其**已预留的拓扑字段位**——RD-041 backfill 字面「MaterialClosure 已预留拓扑字段位」，§4.7 的多层 graph 是编译/资产 IR，不消费该预留位）、VisBuffer 位格式、PageRequest 字段布局，以及 RFC-0017 物理五纪律（评审 F5/F10）；
- 不修改 RFC-0018 / G7 契约或把 RD-038 当前 open 解释为 closed；
- 不强攻 DXIL RT 腿（RD-034 继续按上游事实）；M50 首期硬门为 Vulkan 主腿；
- 不开放递归 RT、动态 ray flags、动态 SBT offset/stride/miss index、any-hit `terminate_ray`、callable nesting；
- 不做 SER、OMM、ReSTIR、Work Graphs、multi-GPU、WebGPU、concurrent-sharing resource 或 external queue ownership；
- 不稳定化 SBT bytes、device address、queue-family index、driver stack 数值或 vendor pipeline cache blob；
- 不因本 RFC 批准而自动判 M28/M50 go，不自动登记任何 strategic_override；
- 不在 G8.1 改 `src/spec/conformance`、不创建 workflow 空步骤、不领取共享 RXS/RD/U/RX 号。

## 9. 开放问题与 Draft 裁决

下表是本 Draft 的明确裁决提案；Agent Approved 时逐行冻结。若对抗性评审推翻任一项，必须先改正文和本表，再批准。

| ID | 问题 | Draft 裁决 |
|---|---|---|
| Q1 | SBT user data 是裸 bytes 还是 typed record？ | typed `#[shader_record] &R` + compiler reflection packer；物理 bytes 非 stable |
| Q2 | 多 hit group 如何配 payload/attribute？ | 单 pipeline 单 payload schema；每 group 独立 attribute/record schema，manifest 全域静态比对 |
| Q3 | any-hit/intersection/callable 开多大？ | §4.1.3 冻结最小可验子集；递归、动态 flags/offset 与 nesting 不开 |
| Q4 | stack size 由谁决定？ | final-link builder 读取真实 group query 后保守计算，runtime trace 前复核；禁止常量猜测 |
| Q5 | pipeline library 是否可重排 groups？ | 不可；主 manifest 声明序决定最终 index，hash 绑定该顺序 |
| Q6 | RD-037 是否允许 Rust 宿主等价装配？ | 不允许作为成功路径；`.rx` 图和 compiler manifest 是单源，Rust 只执行 |
| Q7 | permutation 与 capability 谁先？ | 先选 profile并传播 capability，再求解/裁剪 permutation；fallback 是显式独立 variant |
| Q8 | interface hash 算法与边界？ | SHA-256 domain-separated canonical reflection；artifact digest 分离，二者共同入缓存键 |
| Q9 | M28 在 no-go 时怎么办？ | 保留 RFC 语义边界，不落 spec/实现、不充 gate 绿；仅 go/override 后 materialize |
| Q10 | 多队列是否改 G5 Barrier？ | 不改；新增 QueueTransfer/timeline companion plan。确需 spec 修订时 G8.2 先行、RED 先落 |
| Q11 | 无专用队列如何处理？ | 强制 deterministic single-queue fallback，并与多队列结果做 digest 等价门 |
| Q12 | task stage 是否随 RFC 自动开放？ | 不；只建立 M62 评估窗，measured go 后才开放 RHI arm |
| Q13 | RFC Approved 是否解锁实现？ | 不；G7/RD-038 互锁与 G8.2 validator 是独立且不可 override-by-wording 的硬门 |

## 9.1 对抗性评审记录

本节由与起草 provenance `Assisted-by: Codex:gpt-5 rfc19-drafter-session` 不同的独立评审者填写（D-409）。

| 评审者 provenance | 评审轮次 | 日期 | 评审镜头 |
|---|---|---|---|
| `Assisted-by: Kiro:claude-opus-5 rfc-review-session` | R1（独立会话，只读评审后由本会话落改） | 2026-08-02 | ① correctness（在树事实核对：RXS-0242~0248/0275 字面、RD-037/040/041 backfill 字面、G7/RD-038 状态、G5 冻结面）② redline（编号 claim、backfill 静默改写、override 混同、冻结面无修订行、Draft 冒充实现许可）③ implementability（判据能否被 `G8_ACCEPTANCE_MAP` 的机器断言求值） |

**结论**：1 blocker + 10 major + 6 minor；blocker 与全部 major 已在正文实改，minor 逐条 disposition 后翻 **Agent Approved**。

| # | Finding | 严重度 | Disposition |
|---|---|---|---|
| F1 | §4.9 称 task「语法/payload/SPIR-V encoding 已有 RXS-0242~0246」，但 RXS-0275 明记 task payload 为条件臂、条件臂外 `RX6026`，故 task-on 产物今天无法构造 | major | **采纳，正文实改**：§4.9 首段改为「在树事实是 mesh-only」，明记条件臂兑现是评估窗第一步，不得以 stage 语义已在冒充产物已在 |
| F2 | 承接里程碑漏 M05，且 WPO/位移实现应归 G8.5a | major | **采纳，正文实改**：头部承接里程碑补 G8.5a（M05） |
| F3 | 头部把 M28 归 G8.5b，与 `G8_CANDIDATE_DECISIONS.md` 的 no-go/实现留 G8.7 冲突 | major | **采纳，正文实改**：头部改为「语义先行，实现依决策表当前 no-go 留 G8.7」；上游 `G8_PLAN` §2.5b 同步勘误 |
| F4 | M62 归属：本文写 G8.2 评估，`G8_PLAN` §2.7 列为 P2 穷举 | minor | **采纳，正文实改**：§4.9 明记 M62 归 G8.7 穷举，G8.2 只做评估窗，且 §6.1 表登记「无 P0/P1 gate」 |
| F5 | §8 冻结面清单漏 `GpuScene` 与物理五纪律 | minor | **采纳，正文实改**：§8 首条补 `GpuScene` 与 RFC-0017 物理五纪律 |
| F6 | 多 hit group/多 miss/材质 record 实质修订 RXS-0244/0245/0248 三条冻结句却自称「加性扩展」，且 §8「不开放动态 SBT offset/stride/miss index」与 §4.1.1 的 group 选择自相矛盾 | **blocker** | **采纳，正文实改**：新增 §4.1.6 逐条列原句→修订后句 + golden 零漂移证明计划；§8 该项限定为「**运行期动态**」并在 §4.1.3 澄清装配期静态映射不属该禁项 |
| F7 | §4.2 称「逐字承接」RD-037 却加入「（及已经合法的 mesh）」，而 RD-037 与 M89 判据只有 vs/fs | major | **采纳，正文实改**：删除该括注，回到 backfill 字面 |
| F8 | §2.2 引 RD-040 时截断，漏「(与 GI hit lighting 同步评估)」 | major | **采纳，正文实改**：补全逐字引用 |
| F9 | §6.1 把 M29/M31/M32 并为一个 `g8.shader_platform` 门，违 `G8_PLAN` §2.2「各有独立 CI 硬门」 | major | **采纳，正文实改**：§6.1 改为逐面独立 canonical key，M30 单列 |
| F10 | 未引 `MaterialClosure`「已预留拓扑字段位」，多层 graph 与该预留位的关系未表态 | minor | **采纳，正文实改**：§8 首条明记多层 graph 是编译/资产 IR，**不消费**该预留位 |
| F11 | §6.1 引入第三套 gate 命名空间（与 CONTRACT/CI_GATES 的小写 key 及 MAP 的大写 key 均不匹配） | major | **采纳，正文实改**：§6.1 声明本 RFC 不新造命名空间，只引用 `G8_ACCEPTANCE_MAP`/`CI_GATES` 的 `g8.p{0,1}.m##.<slug>`；上游三份文档已统一并由 `ci/check_g8_acceptance_map.py` 三向锁定 |
| F12 | M24 五 case 中 `thin_geometry` 语义完全缺失；且拒绝冻结 tolerance 与 MAP「RFC-0019 冻结 golden 与 tolerance」冲突 | major | **采纳，正文实改**：§4.6.3 补 thin geometry 三条规范约束，并给出「首批 measured corpus → 本 RFC 加性修订行冻结 → 才可判 GREEN」的冻结程序（仍零预造数字） |
| F13 | M32 判据要求「RFC 冻结的 symbolic diagnostic key」，而本文冻结了零个 | major | **采纳，正文实改**：§4.5.1 冻结四个 symbolic diagnostic key（符号名非 RX 数字号），并绑定 M32 三腿判据 |
| F14 | §6.2 的 M50 GREEN 弱于 MAP 断言 ②④（user data 逐字节相等、library 链接输出与单体 golden 相等），§4.1.5 未冻结分库≡单体等价 | major | **采纳，正文实改**：§4.1.5 新增「分库 ≡ 单体等价语义」，§6.2 拆出 records 行并把两项 GREEN 提升到 MAP 字面强度 |
| F15 | §6.1「RP-MULTIQUEUE 条款 Approved」应为「RFC-0019 多队列章 Approved」（条款号在 G8.2 后才领） | minor | **采纳，正文实改**：§6.1 表改为「RFC-0019 §4.8 多队列章已 Approved」 |
| F16 | 头部「是否新建 `spec/rendering_platform.md`」与 §5 直接以该文件为目标自相矛盾 | minor | **留痕不改正文**：§5 是 diff **计划**，目标文件名是默认取向；最终归属仍由 G8.2 spec PR 按 07 §5 边界裁定，头部表述已限定「按 §5 决定」，无实质冲突 |
| F17 | `ignore_intersection()` 缺签名、合法阶段与终结语义 | minor | **采纳，正文实改**：§4.1.3 后补签名 `fn ignore_intersection() -> !`、仅 anyhit 合法、终止本次调用而不影响后续遍历 |

**评审者对跨文档矛盾的移交**（非本 RFC 造成，已在上游修）：gate key/脚本双套（MAP vs CONTRACT/CI_GATES）→ 已统一并加机器锁；`G8_PLAN` §3「G8.1 起 spec-first」与「G8.1 spec 0-byte」→ 已勘误为 G8.2 起；M28 波次 → 已勘误；M24 tolerance 冻结主体 → 由本 RFC §4.6.3 承担；M62 波次 → 已统一为 G8.7 穷举。

## 10. 稳定化与 provenance

- **特性生命周期**：RFC Agent Approved 只是语义评审完成；随后仍需 §2.2 implementation gate → spec-first/RED → gated implementation → tracking evidence → 至少两个里程碑无重大语义修订 → stabilization report → FCP-lite。
- **稳定面候选**：reflection schema/version、interface hash 规则、capability ID 语义、permutation canonical key、motion convention 与 queue-plan happens-before；是否 stable 由未来 stabilization report 裁决。
- **明确非 stable**：SBT bytes/stride/address、backend group handle、driver stack query、physical queue mapping/family index、pipeline cache blob、TSR tuning threshold、closure flatten 实现算法。
- **Provenance**：`Assisted-by: Codex:gpt-5 rfc19-drafter-session`。

## 11. 规范与实现依据

- 仓库内：[RFC-0013](0013-industrial-rendering.md)（RXS-0242~0248 RT 阶段/SBT 最小基线）、[RFC-0015](0015-engine-rendering.md)（RHI gfx 图与单 queue barrier 桥）、[RFC-0016](0016-native-renderer.md)（G5 renderer 冻结面、TAA/TSR/WPO 现状）、[RFC-0018](0018-compute-rayquery-device-frame.md)（G7/RD-038 事实边界）。
- G8 上游：[G8_PLAN](../milestones/g8/G8_PLAN.md)、[G8_CAPABILITY_MATRIX](../milestones/g8/G8_CAPABILITY_MATRIX.md)、[R1 UE5 Renderer Panorama](../milestones/g8/research/R1_UE5_RENDERER_PANORAMA.md)、[R3 GPU API / Asset Pipeline](../milestones/g8/research/R3_GPU_API_ASSET_PIPELINE.md)。
- 外部规范落地时以仓库 pin 的 Vulkan/SPIR-V 工具链为准：`VK_KHR_ray_tracing_pipeline`、`VK_KHR_pipeline_library`、timeline semaphore、queue-family ownership transfer、SPIR-V RT execution models；真实 device/driver capability snapshot 随 evidence 归档。

---

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| Draft v0.1 | 2026-08-02 | 初稿：冻结 M50/M89/M29/M31/M32/M24/M28/M59/M62 语义、spec diff/RED-GREEN 计划、G5 EB 0-byte 边界与 G7/RD-038 双门；§9.1 留给独立 provenance 对抗性评审 | Full RFC（Draft） |
| v1.0 | 2026-08-02 | **Agent Approved**：D-409 独立 provenance（`Kiro:claude-opus-5` ≠ 起草 `Codex:gpt-5`）三镜头评审完成，17 findings 全 disposition。正文实改要点：新增 §4.1.6 RXS-0244/0245/0248 冻结面逐句修订行（F6 blocker）、§4.1.5 分库≡单体等价、§4.5.1 四个 symbolic diagnostic key、§4.6.3 thin geometry 与 tolerance 冻结程序、§6.1 改为引用统一 canonical gate key（不新造命名空间）、§4.2 回到 RD-037 字面、§2.2 补全 RD-040 逐字条件、§4.9 澄清 task 条件臂事实、§8 补 `GpuScene`/物理五纪律/`MaterialClosure` 预留位口径。零 RXS/CI/RD/U/RX 数字 claim；批准不解锁实现，G8.2 互锁仍为独立硬门。 | Full RFC（Agent Approved） |
| v1.1 | 2026-08-06 | **§4.6.4 加性**：G8.5b M24 五 case golden digest + 逐 case tolerance 自 measured local freeze 提升为 RFC/budget 冻结（`rfc_budget_frozen`）；`resurrection_age_max=6`。零语言语义改动。 | Full RFC（Agent Approved；加性修订） |
