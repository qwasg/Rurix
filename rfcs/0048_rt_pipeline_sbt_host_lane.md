# RFC-0048 — RT pipeline + SBT 宿主车道：hit/miss 着色阶段 kernel 子语言语义面与生产车道形态

| 字段 | 值 |
|---|---|
| RFC 编号 | RFC-0048（4 位制，编号永不复用，10 §9.5；registry/number_ledger.json RFC.next_free=48 落盘前实测顺位领取） |
| 标题 | RT pipeline + SBT 宿主车道：hit/miss 着色阶段 kernel 子语言语义面与生产车道形态 |
| 档位 | **Full RFC**（10 §3：hit/miss 着色阶段**体求值与 codegen 语义**进 kernel 子语言 = 新执行模型运行时语义 + 新 device 执行形态；🔒 触 SBT/record 字节编码与 SPIR-V 物理编码相邻边界（非 stable ABI 面，不冻结）；unsafe 边界不扩（U30 扩注面复用，零新 U）；AGENTS 硬规则 5） |
| 状态 | Agent Approved（2026-08-25）。agent 自主批准后可推进下游 spec-first PR（硬规则 7） |
| 承接里程碑 | G31+ 波 C Task C15（G31_PLUS_COMMERCIAL_RENDERER_TODO §3.2 #31/#32；M52 承接锚 = g30_campaign_handover_registry.json G28 行「RT pipeline/SBT 宿主车道出现（RD-040 分项 reeval_anchor）」；#31 锚 = hit/miss 语义需求成立〔多材质 RT 分派〕+ SER 收益 measured 预估窗） |
| 关联条款 | 拟落 spec **RXS-0408~**（区间随条款数定，见 §5；number_ledger RXS.next_free=408 落盘前实测，条款 PR 先于实现 PR）；扩展 `spec/shader_stages.md` + `spec/vulkan_backend.md`，既有条款 RXS-0242~0245/0297~0300/0322~0327 字面 0-byte |
| 依据决策 | D-402（三档变更门）· D-406（agent 完全自主）· D-409（对抗性评审）· RFC-0019 §4.1（M50 RT pipeline/SBT 冻结面）· RFC-0023 §4.4（SER/HitObject 设计登记面）· RFC-0046 §1/§2（slab 材质公式面与侧表）· RFC-0047 §5.5（G31+ 唯一法定输入面） |
| Provenance | `Assisted-by: TraeCode:Kimi-K3`（起草）。agent 自主决策，批准后推进下游 spec-first PR |
| Agent 批准 | Approved — 2026-08-25；批准范围 = 全文（含 §4.3/§4.5 🔒 禁区子节）；记录方式 = 本行 + §10 修订记录 |
| 对抗性评审 | 评审者 provenance `Assisted-by: TraeCode:Kimi-K3（D-409 独立评审视角实例，与起草逻辑隔离）`（**provenance 字符串 ≠ 起草**；同模型偏差如实登记并效力自限，见 §9.1）；第 1 轮 8 findings 全 disposition；详见 §9.1 |

---

## 1. 摘要

本 RFC 做两件事：

1. **语义面**：把 raygen / closesthit / miss 着色阶段的**体求值与 codegen 正面语义**纳入 .rx kernel 子语言。RXS-0242~0245（G3.6）与 RXS-0322~0324（G8.2 M50）已冻结六 RT 阶段的类型面、payload/record/组形态契约与冻结子集拒绝面；本 RFC 补其后的正面语义——阶段体内 payload 读写、record 只读消费、RT builtins、`trace_ray` 调用点定参与动态语义、以及 MIR→SPIR-V 降级面的语义归属。callable 阶段本波**只预留不开放**（§8）。
2. **车道面**：定义 RT pipeline + SBT **宿主车道**的生产形态——多材质经 SBT record 分派到不同 hit 着色组（首例 = slab 双材质，承 G29/G31 波 B 的 16 槽侧表与 MaterialClosure 生产资产面），与既有 RayQuery compute 生产车道（RXS-0297~0300、RXS-0357 谱系）**并存且 0-byte**；两车道同场景对拍协议与容差结构依据随本 RFC 冻结。SER（Shader Execution Reordering）**只预留不实现**（§4.8），本波兑现 = capability 新鲜实测 + harness 级 measured 预估窗。

```
.rx（raygen ×1 + miss ×1 + closesthit ×N + #[shader_record]）
   │  --emit=check（类型面，既有）  --emit=rt-manifest（装配事实源，既有）
   ▼
rurix.rt-pipeline-manifest.v1（单一事实源：组/record hash/payload hash/capability）
   │  host 装配：plan_sbt_v2 + pack_shader_record（RXS-0326 既有唯一入口）
   │  (instance, geometry) → group_index 装配期静态映射（越界/漏映射 fail-closed）
   ▼
VK_KHR_ray_tracing_pipeline 真跑（U30 扩注面；M50 增量底座 run_rt_pipeline_offscreen 谱系）
   │  对拍：同场景 RayQuery compute 臂（真 .rx 编译，RXS-0297~0300 谱系）
   ▼
像素级对拍（容差结构依据 §4.7）+ 双跑位级 + digest 锚
```

## 2. 动机

- **TODO #31（RT pipeline + SBT 宿主车道）**：承接锚 =「hit/miss 语义需求成立（多材质 RT 分派）+ SER 收益 measured 预估窗」。需求证据本战役已在树：B3 slab 多材质进生产资产面（milestones/g31/g31_slab_side_table_bistro_interior.json 16 槽侧表 + bistro 五材质映射，MaterialClosure 32B ABI 在案）；B2 ReSTIR 高档车道（kernels/g28_restir.rx device 化兑现）提供「命中点需多样化材质着色」的论证素材。RD-040 backfill_condition 的 RT pipeline/SBT 分项字面 =「命中点需多样化材质着色真实出现时（与 GI hit lighting 同步评估）」——多材质生产资产面已出现，分项重判窗随本 RFC 启动。
- **TODO #32（SER workload，M52）**：M52 现态 =「capability 现势 available + workload 零实现 → maintain-defer（单半命中不得改判）」，g31_anchor =「RT pipeline/SBT 宿主车道出现（#31 是前置）」。本 RFC 是 #31 的语义面；SER workload 兑现形态与诚实边界见 §4.8。
- **现状**：生产车道 = RayQuery compute 单 kernel（RXS-0357 谱系）；M50 库面底座在树（多 hit group/SBT user data/stack/pipeline library 真跑，RXS-0322~0327）；`.rx` RT 阶段类型面与 rt-manifest 在树；**`.rx → SPIR-V RT 阶段 codegen 缺位**（实测：`rurixc two_hit_groups.rx --target vulkan` 确定性退出码 2「no compute kernel fn found」；vulkan_codegen 仅有 hand-emitted RT 语料 emit_m50_*，mir_build 的 TraceRay intrinsic 降级在案标注「接线归后续 PR」）。

**为何需要 Full RFC（而非 Direct/Mini）**：着色阶段**体求值与 codegen 语义**= 运行时语义 + 新执行模型进 kernel 子语言（10 §3 新语法语义面/运行时语义）；触 🔒 SBT record 字节编码与 SPIR-V 物理编码相邻边界（沿用 RFC-0019 §4.0-5「非 stable ABI」纪律，本 RFC 不冻结任何物理布局）；SER 预留面触 RXS-0311 capability ID 闭集相邻面（预留位登记，不占闭集——与 RXS-0349 预留位同律，须经该条款加性修订行，本 RFC 不消费）。

## 3. 指导级解释（用户视角）

多材质 RT 分派首例（slab 双材质）的 kernel 形态（本 RFC 的 conformance 锚 `src/rurix-render/kernels/g31_rt_slab_hit.rx`）：

```rust
struct RayPayload { r: f32, g: f32, b: f32 }
struct SlabRec { rc: f32, ab: f32, albedo_r: f32, albedo_g: f32, albedo_b: f32 }

raygen fn rg(tlas: AccelStruct, out_img: TextureRw2D<f32>, #[payload] p: &mut RayPayload) {
    // 逐像素一次 trace_ray；递归恒 1；SBT 寻址装配期静态（调用点无动态 offset）。
    // launch_id/launch_size builtins 逐像素寻址；命中色写 out_img。
}

miss fn ms(#[payload] p: &mut RayPayload) {
    // 背景色写 payload。
}

#[hit_group(slab_a)]
closesthit fn ch_a(#[payload] p: &mut RayPayload, #[shader_record] rec: &SlabRec) {
    // slab 双层闭式反照率（RFC-0046 §1 修法 A 同式）：tc=1−rc; denom=1−rc·ab;
    // R = rc + tc·tc·ab / max(denom, 1e-30)；payload = albedo × R。
}

#[hit_group(slab_b)]
closesthit fn ch_b(#[payload] p: &mut RayPayload, #[shader_record] rec: &SlabRec) {
    // 同形不同组：装配期 (instance, geometry) → group_index 静态映射决定分派。
}
```

宿主车道使用面：编译产 rt-manifest（组/record schema hash/payload hash/required capabilities）→ host 依据 manifest 以 `pack_shader_record`（唯一合法 record 编码入口）铺设 SBT → AS 装配期声明 `(instance, geometry) → group_index` 静态映射 → device 真跑。RayQuery compute 车道**不因此改动一个字节**；两车道关系与选择律见 §4.5。

## 4. 参考级设计

### 4.1 阶段体求值语义（raygen / closesthit / miss）

- **入口与着色**：`raygen fn` / `closesthit fn` / `miss fn` 取 kernel 入口着色（RXS-0153/0242 既有面继承）；直接调用入口 / 跨着色非法调用维持 RX3001；PTX 收集根排除维持 D-207（RT 阶段不进 CUDA 面）。
- **payload**：`#[payload] p: &mut P` 为阶段间唯一可变状态通道。单一 payload schema 全 manifest 域逐字段一致（RXS-0323 既有面）。动态语义：raygen 在 `trace_ray` 调用点把 payload 交由本次追踪；closesthit/miss 看到的是**同一次追踪**的 payload（写后读 = 同 invocation 程序序）；`trace_ray` 返回后 raygen 读回被 hit/miss 写过的值。跨 invocation 无共享 payload；禁经 payload 指针逃逸构造全局通道（`&mut` 借用纪律沿用 MIR 借用检查）。
- **record**：`#[shader_record] rec: &R` 只读（RXS-0322 类型面继承）；动态语义 = 本次命中所分派 hit group 对应 SBT record 的只读 typed payload；record 字节在 pipeline 生命周期内不可变（RFC-0019 §4.1.2 继承），写路径在语言面不存在（by-construction）。
- **builtins**：阶段矩阵沿用 RXS-0245（launch_id/launch_size 仅 raygen；world_ray_origin/direction、t_current 等命中 builtins 仅 hit/miss 可达域；primitive_index/instance_index/geometry_index/hit_kind/hit_t 仅 hit 阶段；miss 无命中 builtins）。builtins 名入 KNOWN_BUILTINS 既有面；阶段外使用 → 编译期拒（RX3013 扩类别谱系）。
- **`trace_ray`**：仅 raygen 可达域；固定签名（tlas, origin, t_min, dir, t_max）+ ray flags 恒 Opaque、cull mask 恒 0xFF、SBT 寻址由装配期静态映射确定（不接受运行期动态 offset/stride/miss index 实参）、递归恒 1（RXS-0245 修订行与 RFC-0019 §4.1.6 继承，字面 0-byte）。同一 raygen 内多次 `trace_ray` 合法（逐次独立 payload 往返）；递归 trace（hit/miss 内 trace）维持不开放。
- **miss 语义**：无命中（含全部候选被 ignore——any-hit 面本波不开放新语义，沿用 RXS-0324 冻结子集）→ 最近一次 miss 调用；多 miss[] 的选取 = `trace_ray` 调用点 miss index 恒 0（首期；多 miss 分派需求出现时经加性修订行评估）。
- **输出面**：raygen 输出经 `TextureRw2D<F>` store（RXS-0223 既有面：TextureRw2D = fragment+raygen 合法阶段矩阵已含 raygen）或 SSBO ViewMut 直写（与 compute 同一分配律）；本 RFC 不新增输出通道。

### 4.2 组形态与装配期映射

- 组形态冻结表（triangles = closesthit 必选 + anyhit 可选 / procedural = intersection+closesthit 必选）沿 RFC-0019 §4.1.1 继承；本波开放面 = **triangles 多组**（多材质分派首例）；procedural/any-hit/intersection 不开放新语义（沿用 RXS-0324 冻结子集既有兑现面，不扩）。
- `(instance, geometry) → group_index` = 装配期静态映射（单一事实源 = rt-manifest）；越界 / 漏映射 / 重复但不一致映射 → 装配期 typed Err（fail-closed，RFC-0019 §4.1.1 继承）；运行期动态改写 SBT 寻址维持不开放。
- **callable 本波只预留不开放**：`callables[]` manifest 域可空维持；`execute_callable` 维持 RXS-0324 冻结子集语义面既有兑现（M50 已兑现 callable 最小面）；本 RFC 不扩 callable 新语义，开放评估归 §6 PR-5 可选窗。

### 4.3 🔒 codegen 语义（MIR→SPIR-V；非 stable ABI 面不冻结）

> 本子节为 codegen 语义归属面；SPIR-V 物理编码（字节布局/ID 分配/指令序）**不冻结为 stable ABI**（RFC-0019 §4.0-5 / RFC-0003 §4.6 纪律继承）。

- **收集根**：RT 阶段入口（raygen/closesthit/miss）纳入 device MIR 收集根（与 compute/graphics 根并列）；每入口一独立 SPIR-V 模块（RXS-0291 继承，无合并无链接器）；driver 发射形态 = 多模块产物（rt-manifest 与模块集同 generation）。
- **storage class 映射**（语义归属，物理编码不冻结）：`#[payload]` ↔ RayPayloadKHR（raygen 侧）/ IncomingRayPayloadKHR（hit/miss 侧）；`#[shader_record]` ↔ ShaderRecordBufferKHR（只读块，布局律 = RXS-0322 编译器 layout 律单一事实源，与 runtime packer 同律）；callable-data ↔ CallableDataKHR/IncomingCallableDataKHR（预留不开放）。
- **intrinsic 降级**：`trace_ray` → OpTraceRayKHR（flags=OpaqueKHR、mask=0xFF、SBT offset/stride/miss index 恒 0——分派由实例 sbt_record_offset 装配期注入，语义沿 RFC-0019 §4.1.6）；`execute_callable` → OpExecuteCallableKHR（预留不开放）；`ignore_intersection`/`report_intersection` 维持 RXS-0324 冻结子集既有兑现面。
- **builtins 降级**：LaunchIdKHR/LaunchSizeKHR（raygen）、WorldRayOriginKHR/WorldRayDirectionKHR、InstanceCustomIndexKHR/PrimitiveId 系、HitTKHR 等按阶段矩阵装饰（语义 = RXS-0245 矩阵）。
- **模块面**：SPIR-V 1.4 per-entry 升版 + RayTracingKHR capability + SPV_KHR_ray_tracing 扩展声明（RXS-0247/0300 机制继承——分叉落发射函数级，既有 compute/graphics 模块字节零漂移）；OpEntryPoint interface 全量枚举（1.4 律）；spirv-val `--target-env vulkan1.2` 门禁继承。
- **确定性**：同一源 + edition + target + profile + 编译参数 → 逐字节相同模块（RXS-0291/0304 谱系继承）。

### 4.4 SBT 布局与 record 编码

- **单一事实源**：`plan_sbt_v2`（四 region 布局）+ `pack_shader_record`（唯一合法 record 编码入口；schema hash 精确匹配 fail-closed；禁 repr(C) memcpy 契约）沿 RXS-0326 继承，字面 0-byte。
- **record schema**：`record_schema_hash = SHA-256("rurix.shader-record.v1\0" || canonical_fields)`（RXS-0322 继承）；每条 record 的实际 schema hash 与目标 group 精确匹配（装配期核验）。
- **对齐律**：region stride = align_up(handle_size + max_record_bytes, handle_alignment)；region base 对齐 base_alignment；多 group 时 hit region stride 取本 region 最大 record 的对齐值（RFC-0019 §4.1.2 继承）。slab 双材质首例 record = `{f32 rc, f32 ab, f32 albedo_r, f32 albedo_g, f32 albedo_b}`（20B；两 group 同 schema——同形不同槽值，分派语义在 group_index 不在 schema 差异）。
- **不可变与回收**：record bytes 在 pipeline 生命周期内不可变；材质参数更新 = 新 record buffer/新 generation + 旧 trace 完成后回收（RFC-0019 §4.1.2 继承）。
- **stack sizing**：逐组 query → 保守公式（版本进 evidence）→ configured ≥ required 核验；人为缩小 RED（RXS-0327 继承）。

### 4.5 🔒 宿主车道执行面与 RayQuery 面关系（capability 门控 / fail-closed）

- **车道定位**：本 RFC 车道 = **独立 witness 车道**（harness 级真跑 + 对拍），不进 g14_3 生产渲染管线（该面 0-byte；生产接线经 G31+ 后续立项程序，与 B3 slab「生产接线窗」同律）。
- **capability 门控**：raygen/miss/closesthit 阶段 → 隐式推导 `rt.pipeline`；`#[shader_record]` → 隐式推导 `rt.sbt_user_data`；`trace_ray` → `rt.pipeline`（RXS-0311 隐式推导映射表既有面，漏推导即实现 bug）。profile 选择律与 runtime snapshot 核验沿 RXS-0312/0313：required capability 缺位 → 编译期 `capability.missing_required`；snapshot 漂移 → 装载期 RED。
- **fail-closed**：device 缺 `rayTracingPipeline`/`accelerationStructure`/`bufferDeviceAddress` feature 或缺扩展 → 装配期确定性 typed Err（U30 面「无静默降级」继承）；**禁止向 RayQuery 车道静默 fallback**——两车道分派语义不等价（SBT 静态分派 vs megakernel 分支），静默换道 = 语义变更；需要降级形态的消费方须走 profile fallback 变体（RXS-0312 选择律，变体独立可寻址）。
- **与 RayQuery compute 面关系**：并存不替代。RayQuery 面覆盖「逐像素程序化查询」需求（生产谱系）；RT pipeline 面覆盖「命中点多样化材质着色分派」需求（本 RFC）。选择律 = 需求驱动（出现多材质 SBT 分派需求方用本车道），非能力驱动；RayQuery 生产面（kernels/g9_m98_hwrt.rx、g28_restir.rx、g29_slab.rx 及 vk.rs run_ray_query_* 谱系）0-byte 不破坏。

### 4.6 确定性协议影响

- RXS-0357 L2 谱系扩展：禁 atomic、逐 invocation 独立（invocation 间零交互）、输出直写、全 f32（device f64 = RX6026 构造性拒绝面）、固定输入双跑位级一致。
- **遍历序**：BLAS 遍历序/命中候选求值序 = 实现定义但有界（不锚定）；**最近命中语义 = 规范确定**（最近未忽略交点唯一），故输出对遍历序无依赖；digest 锚只锚输出缓冲字节，不锚 driver 内部遍历细节。
- **分派确定性**：hit group 分派 = 装配期静态映射（instance sbt_record_offset），同场景同装配 → 分派逐字节确定。
- **digest 协议**：既有生产车道 digest 锚（Stage A 谱系）0-byte；本车道 digest 面 = witness 车道输出缓冲 digest（独立锚，不混入生产锚表）。

### 4.7 对拍协议（RT pipeline 臂 vs RayQuery compute 臂）

- **同场景同材质**：两臂消费同一几何（两三角形 × 两实例）与同一材质参数源（slab 双材质槽值 host 单源生成一次原字节上传；RayQuery 臂经 SSBO、RT 臂经 SBT record——**同一 host 字节源**，禁双源）。
- **同相机同公式**：逐像素 ray 生成公式逐字同源；slab 求值公式 = RFC-0046 §1 修法 A 同式（`R = rc + tc·tc·ab / max(denom, 1e-30)`，分母安全化）；命中色 = albedo × R；miss 色 = 背景常量。
- **容差结构依据**：RT 臂输出 RGBA8 unorm（量子 1/255）；RayQuery 臂 f32 直写。f32 求值差来源 = 除法/FMA 收缩 ≤ 数 ULP ⇒ 相对扰动 ≪1/255；量化后预期**位级一致**，至多 ≤1 LSB 翻转且占比 ≤0.1%（沿 g31.waveB.slab 跨臂对拍结构容差先例：bitexact ∨ (mismatch_ratio ≤ 0.001 ∧ max_lsb_diff ≤ 1)；位级一致 = 更强终态亦合法）。
- **判据⓪**：输出有限性一等断言先于聚合（RFC-0046 §1.4 F3 继承，NaN 吞没假绿路径封死）。
- **双跑位级**：每臂固定输入双跑输出 digest 位级一致（确定性门）。

### 4.8 SER 预留（只预留不实现）

- **设计登记引用不复制**：SER 语言原语（HitObject 类型面 / reorderThread / hitObjectTraceRay / hitObjectInvoke / capability `rt.ser` / 材质 flags coherence hint 位段）= RFC-0023 §4.4 M108 设计登记面既有在案；本 RFC **引用不复制**，不占新条款、不产 codegen、不进 RXS-0311 闭集（预留位加性须走 RXS-0349 加性修订行，本 RFC 不消费）。
- **本波兑现形态**（harness 级 measured 预估窗，不进语言面）：① capability 新鲜实测（vulkaninfo 三 token：VK_NV_ray_tracing_invocation_reorder / VK_EXT_ray_tracing_invocation_reorder / rayTracingInvocationReorderReorderingHint feature；三态闭集——现势优先，SKIP 态取在案态并登记降级口径）；② available 则 workload 兑现 = hand-emitted SER raygen（HitObject + OpReorderThreadWithHintEXT）A/B 双臂真跑（reorder on/off），分歧场景（多材质随机分布）dispatch 时延 measured 对照 + 收益比如实登记（微基准口径标注：合成分歧、单 GPU、单 driver 版本，不外推生产）；absent 则 M52 维持 defer 如实登记（capability 半命中不冒充）。
- **重判链衔接**：M52 改判条件 = 双条件合取（真实集成需求出现 ∧ capability rt.ser 设备面实测可用）；本波 workload 兑现构成「RT pipeline/SBT 宿主车道出现」事实登记面，SER 语言面 go 仍须独立 Full RFC 评估（RFC-0023 冻结面衔接）。

## 5. 下游 spec 条款映射（spec diff，10 §3 要件）

**spec 条款 PR 先于实现 PR**（硬规则 7）；条款号自落盘时 number_ledger 实测 next_free 顺位领取（本 RFC 登记时实测 RXS.next_free=408）。

| 条款（拟） | 标题 | 测试锚定计划（每条 ≥1） |
|---|---|---|
| RXS-0408（拟，spec/shader_stages.md） | RT 阶段体求值正面语义（payload 读写程序序 / record 只读 / builtins 阶段矩阵消费 / 多 trace 逐次独立 / miss 选取恒 0） | `conformance/rt_pipeline/accept/` 扩体求值语料（payload 往返/record 消费/多 trace）+ reject（payload 逃逸/miss 内 trace/阶段外 builtin）；`//@ spec: RXS-0408` 锚定 |
| RXS-0409（拟，spec/vulkan_backend.md） | RT 阶段 MIR→SPIR-V codegen 语义（storage class 映射 / OpTraceRayKHR 定参 / builtins 装饰 / 1.4 + RayTracingKHR 模块面 / 每入口一模块） | spirv-val 全量 + golden 反汇编锚 + 既有 compute/graphics 模块零漂移机核；`//@ spec: RXS-0409` 锚定 |
| RXS-0410（拟，spec/shader_stages.md） | 宿主车道 capability 门控与 RayQuery 面关系（隐式推导引用 / 禁静默 fallback / 需求驱动选择律） | capability profile 选择律语料（required 缺位 → missing_required RED）+ 静态审计（RayQuery 生产面 0-byte）；`//@ spec: RXS-0410` 锚定 |

- **错误码策略**：编译期拦截复用 RX3001/RX3012/RX3013/RX3017 扩类别 + capability 四键既有面；codegen 子集外走 RX6026 既有面；**预期零新 RX 码**（不预留、不预造；确需升档按实现 commit 实测 next_free 顺位，registry/error_codes.json 只追加 + en/zh 成对）。

## 6. feature gate / tracking / 实现序（10 §3 要件）

- **feature gate**：零新增——`shader-stages`（类型面既有）+ `vulkan-backend`（codegen 既有）；RT codegen 腿随 `vulkan-backend` 生效。
- **栈式 PR 拆解**（均门控于本 RFC 合入后）：
  - **PR-1（spec-first）**：RXS-0408~0410 条款体 + conformance accept/reject 扩展（既有语料 0-byte 恒跑）+ 本 RFC 的 conformance 锚 kernel（kernels/g31_rt_slab_hit.rx）纳入 `--emit=check`/`--emit=rt-manifest` 恒跑面。
  - **PR-2**：mir_build RT intrinsic（trace_ray → MIR CallTarget）+ RT 阶段收集根 + 借用/payload MIR 表示。
  - **PR-3**：vulkan_codegen RT 腿（storage class/builtins/OpTraceRayKHR/assemble_rt 多模块）+ spirv-val + golden。
  - **PR-4**：宿主车道生产化（manifest → RtPipelineDesc 装配单一事实源）+ slab 双材质首例 .rx 产 SPV 转正（**替代**本波镜像语料见证面）+ 对拍门转正。
  - **PR-5（可选窗）**：callable 面开放评估（RXS-0324 冻结子集之上）。
- **本波兑现面（Task C15 诚实边界）**：本 RFC + 对抗评审 + conformance 锚 kernel（typecheck/manifest 真绿）+ **镜像语料 device 见证**（hand-emitted corpus 与 kernel 公式面逐字同源——M50 同治理类，**明确标注非 .rx 编译产物，不充 .rx codegen 绿**）+ RayQuery 臂真 .rx 编译对拍 + SER measured 预估窗。**PR-2/3/4 维持 open 如实登记**（.rx→SPIR-V RT codegen = 多 PR 工具链工程量，超出本窗；实测证据 = `--target vulkan` 退出码 2「no compute kernel fn found」）。
- **真实红绿**（反 YAML-only）：CI 门 selftest 判读器构造缺陷 → 红 → 复原 → 绿；device 腿 RURIX_REQUIRE_REAL=1 翻硬红；三态 DEV_ENV_DEGRADE 如实。

## 7. 备选方案

1. **hand-emitted SPIR-V 长期化**：否决——违 P-11 单一事实源（manifest 与 shader 双源分叉，record/组映射无编译期核验）；镜像语料仅过渡见证面，禁充 .rx codegen 绿（§6 PR-4 转正即替换）。
2. **RayQuery megakernel 分支承载多材质分派**：否决——命中点材质分派 = per-instance shader 分派语义，megakernel `if material` 分支把发散成本留在单 warp 内且无 SBT 静态分派核验面；但两臂对拍关系保留（§4.7）。
3. **DXIL RT 腿同开**：否决——RD-034（spirv-cross SPV_KHR_ray_tracing 消费路径或 LLVM A 路）maintain-blocked 在案；Vulkan 单腿先行，DXIL 腿条件不变。
4. **callable/any-hit/intersection 同波开放**：否决——首例需求 = triangles 多组（多材质）；冻结子集既有兑现面已覆盖 callable 最小面，扩面需求未成立（P-12 克制）。

## 8. 不做（范围红线）

- SER 语言面实现（HitObject/reorderThread 进 kernel 子语言）——只预留（§4.8）；SG 面不触。
- any-hit / intersection / procedural 新语义面；递归 trace；`terminate_ray`；运行期动态 SBT 寻址；callable nesting。
- DXIL RT 腿（RD-034 在案）。
- g14_3 生产渲染管线接线（生产管线 0-byte；接线经 G31+ 后续立项）。
- MaterialClosure 32B 布局与 graph/types.rs（RFC-0046 §1.7 冻结面 0-byte；slab record 经 SBT 独立通道，不经 MaterialClosure）。
- SBT/SPIR-V 物理字节布局 stable 化（非 stable ABI 面，§4.3）。

## 9. 未决问题 / 关键裁决

| # | 问题 | 裁决 |
|---|---|---|
| Q1 | 多 payload schema（逐 group 不同 P）是否开放？ | **不开放**——单一 payload schema 全 manifest 域维持（RXS-0323 继承）；需求出现时经加性修订行重审 |
| Q2 | callable 本波开放吗？ | **不开放**——只预留（§4.2 末）；开放评估归 §6 PR-5 可选窗 |
| Q3 | 多 raygen / 多 miss 分派？ | raygen 恰一维持；miss[] 可多条但选取恒 index 0（§4.1）；多 miss 分派需求未成立 |
| Q4 | SBT/SPIR-V 物理布局进 stable？ | **不进**——非 stable ABI 面（§4.3/§8）；可复现面 = reflection/manifest/packer golden |
| Q5 | 对拍容差为何不是零容差？ | RT 臂 RGBA8 unorm 量化（量子 1/255）vs RayQuery 臂 f32 直写——量化域不同，结构容差 = bitexact ∨ (≤1 LSB ∧ ≤0.1%)（§4.7；位级一致为预期常态） |

## 9.1 对抗性评审记录（对抗性评审要求，10 §3 / §7 · [`../13_DECISION_LOG.md`](../13_DECISION_LOG.md) D-409）

| 字段 | 值 |
|---|---|
| 评审者 provenance | `Assisted-by: TraeCode:Kimi-K3（D-409 独立评审视角实例，与起草逻辑隔离）`（**provenance 字符串 ≠ 起草**） |
| 评审轮次 | 第 1 轮，2026-08-25 |
| 评审形态 | 独立评审视角实例——评审者以设计弱点攻击立场逐项复核：ABI 稳定面 / 确定性协议 / 能力降级 / 与 RayQuery 面冲突 / 安全边界 / 编号纪律 / 诚实边界；独立事实核对十项在案（number_ledger RFC/RXS next_free 实测、M50 库面底座 grep、`--target vulkan` 退出码 2 实测、rt-manifest 实跑、G28 M52 重判表、deferred.json RD-040 history、RFC-0019 §4.1 冻结面、RFC-0023 M108 登记面、g31.waveB.slab 对拍先例、pr-smoke.yml g31 零占号实测） |
| provenance 偏差登记 | 评审者与起草者**同模型**（Kimi-K3），独立性 = 评审视角逻辑隔离 + 不复用起草结论，不满足 D-409 首选「跨工具/跨模型」字面。按 RFC-0025（单实例偏差登记+效力自限）/ RFC-0028 §9.1 先例如实登记并效力自限：本评审不替代未来跨工具评审；跨工具评审者可得时建议补一轮；留 G31+ 波 C 收官终审复核锚 |

**Findings 与 disposition**（每条一行；disposition 二选一：采纳并修 §X ／ 驳回 + 理由）：

| # | Finding（评审者提出） | 严重度 | Disposition |
|---|---|---|---|
| F1 | §6 本波兑现面把镜像语料 device 见证与「RT pipeline 车道真跑」并置，命名若不显式区隔，读者会把 hand-emitted 语料误读为 .rx 编译车道——构成冒充 .rx codegen 绿的风险 | **blocker** | **采纳并修**：§6 兑现面逐项标注「hand-emitted 镜像语料（非 .rx 编译产物，不充 .rx codegen 绿）」+ PR-4 转正即替换的语义；CI 门 facts 分两类（语言面 typecheck/manifest 绿 vs device 镜像见证），fact 命名强制携带 `hand_emitted_mirror` 字样 |
| F2 | §4.7 对拍容差（≤1 LSB ∧ ≤0.1%）只有结论没有推导，「容差结构依据」不构成机核判据 | high | **采纳并修**：§4.7 补推导链——unorm8 量子 1/255；f32 求值差 = OpFDiv ≤2.5 ULP + FMA 收缩 ⇒ 相对扰动 ≪ 量子 ⇒ 翻转概率上界 ≈ 扰动/量子；判据⓪有限性一等断言先于聚合（RFC-0046 §1.4 F3 继承） |
| F3 | §4.6「遍历序实现定义但有界」若 driver 改变候选求值序，半透明/candidate 面会破输出确定——digest 锚是否依赖遍历序未说清 | high | **采纳并修**：§4.6 显式二分——最近命中语义 = 规范确定（最近未忽略交点唯一，与求值序无关）；digest 只锚输出缓冲字节不锚遍历细节；candidate/半透明面本波不开放（§8），无遍历序敏感面 |
| F4 | slab record 20B（5×f32）与 M50 record 16B 布局不同，跨 group stride 与 schema hash 配对是否需新条款 | med | **驳回**：RXS-0322 schema hash 精确匹配 + RXS-0326 region stride 取本 region 最大 record 对齐值既有面完整覆盖（§4.4 已引用）；同 schema 不同槽值的分派语义在 group_index 不在 schema 差异，无新条款需求 |
| F5 | §4.5 capability 门控只举 rt.pipeline，`#[shader_record]` 隐式推导 rt.sbt_user_data 漏举——profile 编写者会漏配 | med | **采纳并修**：§4.5 补全隐式推导映射引用（RXS-0311 映射表逐项：stage→rt.pipeline、#[shader_record]→rt.sbt_user_data、trace_ray→rt.pipeline）+ 漏推导即实现 bug 字面 |
| F6 | §4.8 SER 预留与 RFC-0023 M108 设计登记面重叠，边界不清（重复登记 = 双源风险） | med | **采纳并修**：§4.8 首行显式「设计登记引用不复制」——本 RFC 不占新条款、不产 codegen、不进 RXS-0311 闭集（预留位加性须走 RXS-0349 加性修订行，本 RFC 不消费） |
| F7 | §4.5「禁止向 RayQuery 车道静默 fallback」过硬：生产面将来可能需要「无 RT pipeline 能力设备」的降级形态 | med | **驳回**：两车道分派语义不等价（SBT 静态分派 vs megakernel 分支），静默换道 = 语义变更（fail-closed 谱系：能力缺失 → 确定性 Err，无静默降级）；需要降级形态的消费方走 profile fallback 变体（RXS-0312 选择律，变体独立可寻址——显式而非静默） |
| F8 | 镜像语料与 .rx kernel 的「公式面逐字同源」仅靠人工核验，漂移无机器拦截 | low | **采纳并修**：CI 门加静态机核 fact——镜像发射器内常量/公式结构与 kernel 源字面比对（slab 三常量 1e-30/rc·ab 项/albedo 乘法序 + 相机公式常量），漂移即 RED |

## 10. 稳定化与 provenance

- **稳定化**（10 §5）：本 RFC Agent Approved = 语义评审完成；随后 PR-1 spec-first → PR-2/3/4 gated implementation → tracking evidence → 至少两个里程碑无重大语义修订 → stabilization report → FCP-lite。stable 面冻结随 RD-008 届时定义；**明确非 stable**：SBT/SPIR-V 物理布局、stack query 值、driver stack 数值、对拍容差数值（标定程序产）。
- **Provenance**：`Assisted-by: TraeCode:Kimi-K3`（起草）。agent 自主批准并记录（D-406）。

## 11. 规范与实现依据

- Vulkan：`VK_KHR_ray_tracing_pipeline` / `VK_KHR_acceleration_structure` / `VK_EXT_ray_tracing_invocation_reorder`（含 `rayTracingInvocationReorderReorderingHint` feature）。
- SPIR-V：`SPV_KHR_ray_tracing`（RayGenerationKHR/ClosestHitKHR/MissKHR 执行模型、RayPayloadKHR/IncomingRayPayloadKHR/ShaderRecordBufferKHR storage class、OpTraceRayKHR）；`SPV_EXT_ray_tracing_invocation_reorder`（HitObject/OpReorderThreadWithHintEXT，预留面）。
- 仓内：RFC-0019 §4.1（M50 冻结面）/ RFC-0023 §4.4（M108 SER 设计登记）/ RFC-0046 §1/§2（slab 公式面）/ spec RXS-0242~0245、0297~0300、0311~0313、0322~0327 / evidence/g8_m50_rt_pipeline_incremental_* / milestones/g28/g28_m52_rd040_workload_rejudgment.json / milestones/g21/g21_ser_capability_probe_results.json。

---

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| Draft v0.1 | 2026-08-25 | AI 起草初版（G31+ 波 C Task C15；TODO #31/#32 + M52/RD-040 承接锚） | Full RFC（Draft） |
| Draft v0.2 | 2026-08-25 | D-409 第 1 轮对抗评审 8 findings 全 disposition（F1/F2/F3/F5/F6/F8 采纳并修 §4.5~§4.8/§6/§9.1，F4/F7 驳回附理由）回填 | Full RFC（Draft） |
| Agent approval | 2026-08-25 | agent 自主批准全文并记录（D-406；批准范围含 §4.3/§4.5 🔒 子节） | Full RFC（Agent Approved） |
