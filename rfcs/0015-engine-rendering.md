# RFC-0015 — G4 引擎渲染期伞形：图形 RHI 化 / RD-035 执行面三项 / artifacts v2 + .rx 单源 Vulkan RHI / C ABI v2 条件臂

| 字段 | 值 |
|---|---|
| RFC 编号 | RFC-0015（4 位制，编号永不复用，10 §9.5；rfcs/README §5 自由池首号；G4_CONTRACT §7 ⑤ claim 登记，number_ledger v1.13） |
| 标题 | G4 引擎渲染期单伞形 Full RFC 四章：章 A 图形 RHI 化（.rx RHI 库面扩 raster/mesh 图形 pass + 采样/bindless/present 库化 + 自动 barrier 覆盖图形 pass + engine_host v3 嵌入）· 章 B RD-035 执行面三项（transient 别名复用 + 执行期峰值计数器 / 依赖驱动重排 + 并行调度 / RXS-0262 const 泛型定长容量）· 章 C artifacts v2 + .rx 单源 Vulkan RHI（RD-031 兑现）· 章 D C ABI v2 条件臂（repr(C) struct 按值 + 回调指针，判档成立才落实现） |
| 档位 | **Full RFC**（10 §3：章 D 触 FFI ABI codegen（AGENTS 硬规则 5，RD-009 close 注「超界硬需求自 RD-036+ 判档」）；章 A/C 为运行时语义 + codegen 新面 + 既有 🔒 条款（RXS-0239 happens-before / RXS-0209 artifact 面）的兑现与修订；伞形体例沿 G3_CONTRACT §7 v1.1 / MB1 RFC-0011 单期伞形先例；判档争议向上取严 = Full，硬规则 8） |
| 状态 | **Agent Approved**（2026-07-23；§9.1 对抗性评审〔评审 provenance `kimi-cli:kimi-for-coding` 独立实例 ≠ 起草 `Kimi Code CLI (Kimi)`，三镜头 correctness/redline/implementability，D-409〕完成，18 findings 逐条 disposition 落 §9.1，先于任何实现 PR，G-G4-2；跨模型偏差如实登记见 §9.1 环境留痕） |
| 承接里程碑 | G4（[milestones/g4/G4_CONTRACT.md](../milestones/g4/G4_CONTRACT.md)，验收门 **G-G4-2 ~ G-G4-6**；主线 G4.0→G4.7 串行，[G4_PLAN.md](../milestones/g4/G4_PLAN.md)） |
| 关联条款 | 拟落 spec **RXS-0270 ~ RXS-0299**（G4 claim 区间，materialize 数以实现实际为准，未消费号 close-out 作废声明 burned，见 §5）；既有条款修订（追加式，非改写）：RXS-0239 / RXS-0261（重排执行模型下 happens-before 语义）/ RXS-0262（const 容量诚实收窄段兑现后更新）/ RXS-0209（IR2 defer 兑现）/ RXS-0246（witness 锚定 → MIR lowering 兑现深化）；**spec/rhi.md 扩章**（章 A/B）+ **spec/vulkan_backend.md 追加**（章 C）+ **spec/export_c.md 追加**（章 D 条件臂） |
| 关联 deferred | **RD-035**（UC-05 RHI 执行面三项，G4.3 兑现对象，close / 收窄）· **RD-031**（artifacts v2 @__rx_gpu_spirv 段，G4.4 兑现对象，前置已核在 main：src/rurixc/src/codegen.rs:99/1028）· 执行期新 RD 自 **RD-036** 起（章 D 判档不成立之登记 / RT pass 执行臂未落地之登记，均不预留） |
| 依据决策 | D-113（FFI = `#[export(c)]` + 内建头生成）· D-130（窗口/输入不进语言红线——present 面库化维持 C++ shim/运行时层）· D-131（DXIL 混合 compute=A/图形=B——图形 DXIL 腿走 B 链，RD-034 RT blocked 维持）· D-406 v2.0（agent 完全自主）· D-409（对抗性评审，评审 provenance ≠ 起草）· 06 §8.3 :149-151（render graph/ECS「它们是库」）· RD-035 backfill_condition（承接期伞形 Full RFC 一章预期）· RD-031 backfill_condition（artifacts blob / emit_gpu_artifact_globals 合入后补齐）· RXS-0180 L3（符号面非 stable ABI） |
| Provenance | `Assisted-by: Kimi Code CLI (Kimi)`（起草）。agent 自主决策；批准前置 = §9.1 对抗性评审完成 |
| Agent 批准 | **Agent Approved 2026-07-23**——§9.1 对抗性评审（评审 provenance `kimi-cli:kimi-for-coding` 独立实例 ≠ 起草 `Kimi Code CLI (Kimi)`，三镜头 correctness/redline/implementability，D-409）完成，3 blocker 正文实改 + 12 major 正文实改 + 3 minor 措辞订正逐条 disposition（§9.1），先于任何实现 PR（G-G4-2） |
| 对抗性评审 | **已完成 第 1 轮 2026-07-23**——见 §9.1；由与起草者 Provenance **不同**的工具执行三镜头批判性评审（评审 provenance `kimi-cli:kimi-for-coding` ≠ 起草 `Kimi Code CLI (Kimi)`，硬规则 2 可机验，`ci/check_contribution.py` 规则 4）；**环境留痕**：首选 claude-code 全模型 403 不可用，本轮为跨工具/同模型族评审，偏差如实登记 §9.1 |

---

## 1. 摘要

本 RFC 是 G4 引擎渲染期的**单伞形 Full RFC**（G3 v1.1 单伞形先例：一份 RFC 承载全期各面，一次对抗性评审、一次 Approved 合入即满足各面「RFC Approved 前置」）。四章：

- **章 A — 图形 RHI 化（→ RXS-0270~0279，G4.2，验收门 G-G4-3）**：.rx RHI 库面自 compute-only（RXS-0256~0265）扩为图形面——**raster pass（vertex+fragment）与 mesh pass（mesh+fragment）类型** + 图形资源面（color/depth target、texture2d、sampler、TextureTable）+ 访问声明集（`writes_rt`/`writes_depth`/`reads`/`reads_writes_uav`/present handoff，镜像 RXS-0236 封闭枚举）+ **render graph 自动 barrier 覆盖图形 pass**（推导单源 = G3.5 graph.rs，P-11）+ 声明↔反射相等装配期拒 + present 面库化（终端 handoff + headless readback 判据）+ `#[export(c)]` 图形导出面 + **engine_host v3**（C++/D3D12，engine_host v2 母本升级新增文件）嵌入图形 pass device 真跑**三方数值精确相等**。mesh pass 的 device 腿要求 **MIR→SPIR-V mesh/task 编码自 witness 锚定深化为 MIR lowering**（RXS-0246 兑现深化）。**RT pass 为条件臂**（§9 Q-RTArm：mesh lowering 落地后评估；未落地则类型面不条款化、登记 RD-036+，门 G-G4-3 以 raster+mesh 满足）。
- **章 B — RD-035 执行面三项（→ RXS-0280~0289，G4.3，验收门 G-G4-4）**：① transient 资源**别名复用分配器**（生命期区间不重叠者共享设备分配）+ **执行期峰值计数器** device 采集——I10 自 report_only 升 **measured**（峰值 < 声明容量可 device 见证）；② **依赖驱动重排 + 并行调度**（DAG 拓扑层级 + 同级独立 pass 批级提交），RXS-0239/RXS-0261 追加式修订「声明序定义依赖语义、执行序可置换独立 pass」+ **重排后 happens-before 正确性新增确定性拦截项**（I11 入不变量矩阵，漏拦即红）；③ **RXS-0262 const 泛型定长容量 .rx 接线**（lang-item turbofish const 实参 + 编译期静态槽位记账 + 越界编译期拒 + reject 语料锚定）。
- **章 C — artifacts v2 + .rx 单源 Vulkan RHI（→ RXS-0290~0294，G4.2 前置切片 + G4.4，验收门 G-G4-5）**：兑现 RD-031——`@__rx_gpu_artifacts` blob **版本 bump v1→v2**（v1 48B 前缀兼容 + SPIR-V 入口表追加）+ `@__rx_gpu_spirv` 段发射（多入口：compute kernel 与图形阶段各一模块）+ rxrt 解析填 `DeviceArtifactSet.spirv_fallback`（RXS-0209 IR1 槽位已在 main 空置）→ **.rx 单源 Vulkan RHI 通道**（compute+graphics 双腿经 Vulkan 执行，复用 G3 vk 运行时底座，strict 无回退）。**章 C 的 artifacts v2 codegen 切片是章 A 图形 pass device 出图的工程前置**（G4_CONTRACT §7 ④），实现序 = G4.2 首 PR 先落该切片（§6.3）。
- **章 D — C ABI v2 条件臂（→ RXS-0295~0299，G4.5，验收门 G-G4-6）**：以 engine_host v3 图形嵌入的**真实硬需求**判档（10 §3，争议向上取严）——成立则条款先行兑现 **repr(C) struct 按值**（Windows x64 ABI 布局）+ **回调函数指针**（跨 ABI 调用约定）+ ABI 往返 device 真跑；不成立则登记 **RD-036+** 存续（RD-009 close 注先例）。**两种结局均合法，判档依据必须留痕**（P-12：不以「完整」为名扩面；G-EA1-3 / RXS-0249 条件分支先例）。

**BLACKHOLE 面不占本 RFC**（G4.6）：realtime 修复 = 运行时/应用层修复 + 30fps 测量，present 语义已有条款（RXS-0197/0198/0220~0222），零新语义面；实现 PR 按 10 §3 判档（预期 Direct 或 Mini，执行期定，争议向上取严）。

```
apps/uc05-rhi> rx build src/gfx_demo.rx -o uc05_gfx.exe       # 图形 demo：raster+mesh pass 经 RHI 库面 + 自动 barrier 出图
apps/uc05-rhi> rurixc src/embed.rx --emit=dll -o rurix_rhi     # export(c)：DLL + import lib + rurix_rhi.h（生成,CI 逐字节比对）
engine_host_v3.exe                                             # C++/D3D12 宿主链 DLL 执行图形 pass,三方数值精确相等
```

「引擎/外部采纳」维度显式 carve-out（G4_CONTRACT out_of_scope `production_adoption_claim`）；达成表述 =「引擎级可用的工程闭环落地」，不宣称社会事实。

## 2. 动机

EI1 close-out 已把「.rx 写 compute RHI、export(c) 导出、C++ 工程嵌入真跑」做成 measured 工程事实（engine_host v2 三方数值相等，步骤 74）。但「rurix 渲染器可用于游戏引擎」还差四块（G4_CONTRACT 现状锚，已核实）：

1. **图形着色面未 .rx RHI 化**——RHI 库面仅 compute pass graph（spec/rhi.md RXS-0256~0265）；G3 已把 mesh/RT/采样/bindless/present 做到语言与运行时层 device measured（RXS-0220~0248），但**库面零覆盖**——引擎用户无法以 RHI 形态提交一个三角形。这是 G4 主面。
2. **RD-035 三项未实现**——transient 别名复用 + 执行期峰值计数器（I10 维持 report_only，峰值恒等容量平凡成立）/ 无 pass 重排、无依赖驱动并行调度 / RXS-0262 const 泛型定长容量未接线（现 Vec 承载 runtime-bounded）。引擎级负载（多 transient、大 pass 图）使三项自「诚实标注」升为「真实硬需求」（RD-035 backfill_condition ①②③ 触发口径）。
3. **.rx 单源 Vulkan RHI 未通**——RD-031 open：`.rx` host 产物仅嵌 PTX（RXS-0192），48B v1 blob 无 SPIR-V 槽；`DeviceArtifactSet.spirv_fallback`（RXS-0209 IR1）在 main 空置；vk 运行时底座（G3：descriptor/mesh/RT/graph 执行器）只能从 Rust bin 经 build.rs `include_bytes!` 通道消费 SPIR-V，.rx 应用的 SPIR-V 无 artifact 通道可达。**前置已核成立**：artifacts blob / `emit_gpu_artifact_globals` 在 main（src/rurixc/src/codegen.rs:99/1028，RD-031 history 2026-07-16 已记前提解除）。
4. **C ABI 子集 v1 边界未定 v2**——repr(C) struct 按值 / 回调指针 / 数组按值 / 跨堆所有权未进首期（RXS-0251 strict 拒）；是否需要 v2 由 engine_host v3 真实嵌入面判档（P-12：不以「完整」为名扩面）。

外加 **BLACKHOLE 未验收**（G4.6，本 RFC 外）：realtime 路径 `rxp_create` 返回 Shim(-2147467263) = 0x80004001 E_NOTIMPL（apps/realtime_run.log），缺 30fps measured + REALTIME_OK 判据。

**为何伞形单 RFC**：G3 v1.1 先例——同期多面共享一套跨章一致性约定（§4.0），一次对抗性评审覆盖全文（D-409），各面失败测试先行判据不变（各面 CI 步骤脚本在 RFC 合入时点 main 不存在 = RED）；章 D 触 FFI ABI codegen 硬规则 5，伞形 Full 一并承载（RD-035 backfill_condition 明记「预期 Mini-RFC 或随承接期伞形 Full RFC 一章」——取伞形章，与 owner 立项提示词 §4.1 一致）。

## 3. 指导级解释（用户视角）

### 3.1 章 A — 图形 RHI：从 compute 到出图

打开 `apps/uc05-rhi/src/`，图形 pass 与 compute pass 同一个 `Rhi` 根、同一条声明式建图链，**零新语法**（薄映射 std::gpu lang items + G3 既有条款面）：

```rx
let rhi = Rhi::create(&ctx);
let back = rhi.color_target(256, 256);            // 图形资源:color target(薄映射 RXS-0236 资源类)
let tex  = rhi.texture2d(64, 64);                 // 采样面:texture + sampler(RXS-0223~0225 库化)
let smp  = rhi.sampler(SamplerDesc::linear_clamp());
let mut g = rhi.graph();
g.raster_pass(tri_vs, tri_fs)                     // raster pass:vertex+fragment 着色对
 .writes_rt(&back)                                // 访问声明:color attachment 写(镜像 RXS-0236 封闭枚举)
 .reads(&tex).binds_sampler(&smp);
g.mesh_pass(procedural_ms, shade_fs)              // mesh pass:mesh+fragment(RXS-0243 类型面)
 .writes_rt(&back);                               // 同目标顺序写 → 自动 barrier
let done = g.submit();                            // 1-submit typestate 不变(RXS-0260)
rhi.readback(back, &mut out);                     // headless readback 校验(RXS-0222 纪律)
```

用户不写任何 barrier/同步：图形 pass 的资源状态推导与 compute pass 的同步点推导**同一个推导单源**（graph.rs `derive_barriers`，RXS-0238），执行器逐字回放（P-11）。漏声明访问 / 依赖环 / 写写冲突 / 跨 brand 误用 / 重复 submit 维持**编译期或装配期确定性 strict 拒**——图形面拒法与 compute 面同构（声明↔反射双向相等，库层状态值零新 RX 码）。含图形 pass 的图**仅经 Vulkan 后端执行**（§9 Q-A：compute-only 图的 CUDA 既有路 0-byte；图形图走 CUDA → 装配期确定性拒，strict 无回退）。RT pass 类型面见 §9 Q-RTArm 条件臂。

### 3.2 章 B — 执行面：复用、重排、定长

```rx
let mut g = rhi.graph::<8>();                      // const 泛型定长容量(RXS-0262 兑现):资源槽 ≤8,越界编译期拒
// transient 资源生命期不重叠者自动别名复用同一设备分配;执行期峰值计数器 device 采集
let done = g.submit();                             // 依赖驱动重排:独立 pass 可换序/并行批交,依赖序不变量由装配期证明
```

峰值 < 声明容量由 device 证据见证（I10 升 measured）；重排后 happens-before 正确性由新拦截项保证（任何丢依赖的重排在装配期确定性拒）；容量越界在编译期拒（非装配期）。

### 3.3 章 C — 一个 .rx，两条腿

`rx build` 的 host 产物从「只嵌 PTX」升为「PTX + SPIR-V 多入口」同嵌（artifacts v2）：同一个 `.rx` 源，CUDA 腿走既有 PTX 装载（0-byte），Vulkan 腿走 SPIR-V 装载（本章新通）。RHI 创建时显式选后端（§9 Q-E），无静默回退。

### 3.4 章 D — ABI v2（条件臂）

engine_host v3 的真实嵌入签名若需要 repr(C) struct 按值（如帧参数结构按值传）或回调指针（如引擎侧日志/分配回调），则按本臂条款兑现并 ABI 往返真跑；若 v3 嵌入面以子集 v1（标量+裸指针）即可完整表达，则登记 RD-036+ 存续，判档依据留痕（P-12）。

## 4. 参考级设计

### 4.0 跨章一致性约定（汇装层裁决，四章共同事实源）

1. **推导单源（P-11）**：图形/混合图的资源状态推导**唯一事实源 = G3.5 `src/rurix-rt/src/graph.rs`（RXS-0236~0241，`#![forbid(unsafe_code)]`）**——RHI 图形 pass 的访问声明映射进 graph.rs 的 `AccessKind`/`PassSpec` 模型，推导产物 `PlannedBarrier` 由执行器逐字回放；rhi.rs 既有 `derive_syncs`（compute-on-CUDA 路径）**0-byte 维持**，两推导各自服务单一后端，无第三份推导逻辑（否决另起 RHI 专用图形推导，§7-2）。**桥接形态（评审 C-F3 disposition）**：RHI 运行时本体（rhi.rs `RhiGraph`）与 graph.rs 同属 `rurix-rt` crate——gfx 图的 `seal()` 将自身 pass/资源记录**直接构造 graph.rs 的 `Graph`/`PassSpec`**（同 crate 函数调用，无 cabi marshalling；访问枚举**直接复用 graph.rs::AccessKind 单源**，不复制枚举定义），再调 `derive_barriers()`。**compute pass 在 Vulkan 腿上的同步语义映射（评审 R-F5 disposition）**：混合图中 compute pass 的 `read`/`write` 声明按 RXS-0238 映射表落入 `ShaderRead`/`UavReadWrite`，由 `PlannedBarrier::BufferSync` 承载——其 pass 边界全序 happens-before（RXS-0239）⊇ CUDA 腿 `PlannedSync` 流序同步点语义（同为 pass 粒度全序）；行为等价由 G4.4 交叉对照（同图 CUDA/Vulkan 双腿数值相等，步骤 80）device 见证，并写入 RXS-0272 条款。
2. **后端分工（strict 无回退）**：compute-only 图 = CUDA 既有路（EI1 步骤 72~75 零回归）；**含任一图形 pass 的图 = Vulkan 执行**（经 artifacts v2 SPIR-V 通道），CUDA 后端遇图形 pass → 装配期确定性拒（非运行期炸、非静默换后端）。RT pass 见 Q-RTArm。
3. **RHI 库面零新语法、零新借用码**：图形面全为既有 lang items 的薄映射扩面（`Rhi`/`Graph`/`Res` 族 + 着色阶段类型面 RXS-0242~0245 + 采样 RXS-0223~0225 + TextureTable RXS-0235 + present typestate RXS-0197）；affine/brand/typestate 复用 RXS-0054/0189/0197/0260 既有裁决；图合法性违例走**库层状态值零新 RX 码**（rhi.md §3/§5.1 先例，镜像 RX6029/6030 口径）。
4. **反射喂入**：图形 pass 的声明↔反射相等与 compute 面（I4，EI1.4）同机制——编译器在 typeck 自着色函数签名/资源标注提取反射集喂入 `with_reflection`，装配期双向相等核验（计入语言/编译器面，仍零新 RX 码；镜像 EI1-IMPL-03 裁决）。
5. **artifacts v2 单一事实源**：SPIR-V 与 PTX 由**同一次编译、同一份 MIR** 产出（`build_gpu_artifacts` 扩展），非两趟独立编译（防双源漂移）；v2 blob 布局 v1 前缀兼容，v1 解析路径 0-byte（§4.C1）。
6. **合并序敏感号软化**：新 RX 码 / RD-036+ / trace 条数正文一律相对措辞或引 §5 预测表，以各 PR 合入时 registry/ledger/trace 再生实号为准；RXS 严格用 0270~0299 claim 段，未消费号 close-out 作废声明 burned（RXS-0266~0269 先例，number_ledger v1.13）。
7. **零新 cargo feature 总裁决**：图形 RHI / artifacts v2 为加性通道；`vulkan-backend`（codegen）与 `vulkan`（运行时）既有 feature 复用，默认构建（全 feature off）零 GPU/SDK 依赖绿（§6.2）。

---

### 4.A 图形 RHI 化章（G4.2，RD-035 前的主面；RXS-0270~0279；验收门 G-G4-3）

> 定位：把 G3 语言/运行时层的图形能力（采样 RXS-0223~0230 / bindless RXS-0231~0235 / render graph RXS-0236~0241 / mesh-task RXS-0242~0246 / present RXS-0197/0220~0222）以 **RHI 库面形态**交付——零新语法，薄映射，全 `.rx`（apps/uc05-rhi 零 .rs 审计维持）。

#### A1. 图形 pass 类型面（→ RXS-0270）

- `Graph<C>` 新增已知方法（lang-item 薄映射，逐方法即逐 typeck 已知签名分支，RXS-0190 先例）：
  - `raster_pass(vs, fs) -> GfxPass<C>`——vertex+fragment 着色对（阶段着色 RXS-0153/0223，io_sig RXS-0159 既有）；
  - `mesh_pass(ms, fs) -> GfxPass<C>`——mesh+fragment（RXS-0243 入口契约：`#[numthreads]` + `#[outputs(topology,…)]`；task 前置条件臂，首期 mesh-only）；
  - compute `pass(kernel,…)` 0-byte 不变。
- `GfxPass<C>` 与 `Pass<C>` 同属 pass 句柄族（非 Copy affine，消费式声明链）；`GfxPass` 携带着色对反射集（§4.0-4）。
- **RT pass 条件臂**（§9 Q-RTArm）：`rt_pass(raygen, miss, closesthit)` 类型面**仅在执行臂同序列可达时条款化**（RT MIR lowering 最小集 + AccelStruct 资源面 + SBT）；不可达则不立类型面、登记 RD-036+——strict-only 拒半成品（门 G-G4-3 以 raster+mesh 满足，RT 缺失不构成门失败）。
- 着色合法性：图形 pass 声明出现在 `kernel`/`device fn` 体内 → RX3015（compute 面同点位，I8 扩展）。

#### A2. 图形资源面（→ RXS-0271）

- `Rhi<C>` 新增：`color_target(w,h)` / `depth_target(w,h)` / `texture2d(w,h)` / `sampler(SamplerDesc)`（RXS-0225 宿主采样器状态面薄映射）/ `texture_table()`（RXS-0235 薄映射，bindless 库化）——均产 `Res<C>` 族 affine 句柄（资源类标签进 cabi 资源类参数，镜像 rxrt_graph_resource 类 0/1/2/3 枚举扩展）。
- 元素/格式首期封闭：color/depth/readback 与 G3.5 同面（RGBA8 / D32F / f32 readback，RXS-0236 资源面口径）；纹理采样经既有采样方法族（着色侧 RXS-0223 0-byte）。

#### A3. 访问声明集与自动 barrier 覆盖图形 pass（→ RXS-0272；🔒 语义修订关联 §4.B2）

- `GfxPass` 访问声明封闭枚举（镜像 RXS-0236，**同一 graph.rs::AccessKind 单源**，§4.0-1）：`writes_rt(&res)` / `writes_depth(&res)` / `reads(&res)`（shader read，含采样纹理与 TextureTable）/ `reads_writes_uav(&res)`；`binds_sampler(&smp)` = **采样器绑定声明，非资源状态访问**（sampler 无 barrier 状态面，见下反射并集规则）；终端 `present_handoff(&res)`（→ A4）。
- **自动 barrier**：RHI 图形/混合图 sealed 后，访问声明经 §4.0-1 桥接（rhi.rs `RhiGraph` 直接构造 graph.rs `Graph`/`PassSpec`，同 crate）→ `derive_barriers()`（RXS-0238 状态机，双后端映射同源）→ `PlannedBarrier` 逐字回放（Vulkan 执行器：render pass begin/end + vkCmdPipelineBarrier；既有 `run_graph_offscreen` 为同模式先例）。**跨 pass happens-before = RXS-0239 既有承诺**（pass 粒度全序，重排修订见 §4.B2——G4.2 首期声明序 = 执行序不变）；compute pass 声明按 §4.0-1 映射（read→ShaderRead / write→UavReadWrite）落 `BufferSync`。
- 图合法性：read-before-write / 写写冲突 / 同 pass 同资源读写反馈 / 声明↔反射失配 → 装配期确定性拒（库层状态值零新码，镜像 RX6029/6030 口径，§4.0-3/4.0-4）。
- **图形着色对反射并集规则（评审 C-F5 disposition，→ RXS-0273）**：`raster_pass(vs,fs)`/`mesh_pass(ms,fs)` 的反射集 = **逐阶段函数签名资源形参的并集**（vs 的 `Texture2D` 形参与 fs 的同名/同槽资源按资源身份合并，重复计一）；**sampler 绑定与 TextureTable 计入反射并集但标「无状态访问」类**——barrier 相等域只核资源状态访问（color/depth/texture/uav/readback），sampler/table 另核「绑定完备性」（pass 用到而图未绑定 → 装配期拒，同库层状态值）；compute pass 反射机制（RXS-0257 kind-2 槽）0-byte。

#### A4. present 面库化（→ RXS-0274）

- 终端声明 `g.present(&back)`（或 `PresentHandoff` 访问）= 图的呈现终端：执行 = present 前布局迁移 + ① **headless readback 校验**（RXS-0222 纪律，CI 判据）② 窗口腿复用 RXS-0197/0198 typestate + C++ shim（D-130：窗/泵/输入不进语言，0-byte）——窗口腿 device 见证由 BLACKHOLE（G4.6）真实窗口路径承载，本章不以窗口为验收前提。
- 语义 = RXS-0240(c) present 终端胶合既有条款的 RHI 库化引用，零新语义本体。

#### A5. MIR→SPIR-V mesh/task 编码兑现深化（→ RXS-0275 + RXS-0246 修订行）

- 现状（已核）：`dxil_spirv.rs::emit_spirv_inner` 仅 vertex/fragment（其余阶段 `DxilError::unmappable`）；mesh/task/RT 的 G3.6 device 见证经 `vulkan_codegen.rs` 八个**手工构造** witness 发射器（`emit_mesh_min` 等，非 MIR lowering）；**mesh 在 .rx 侧仅有入口标注契约校验**（shader_stages.rs `check_mesh_entry`，RXS-0243/RX3017），**body 类型面与全部 conformance 语料均不存在**（评审 I-G4-IMPL-01 确认，本面为新建非「接线」）。
- 本章兑现（两段，均零新语法——known functions/builtins 面）：
  1. **mesh body 类型面（新建）**：per-vertex/per-primitive 输出数组声明形态（`#[outputs]` 标注对接 io_sig 扩展）、`mesh_set_outputs(vertices, primitives)` 已知函数、mesh builtins 阶段矩阵（`primitive_id` 等 RXS-0154 扩展面）——witness 发射器产物为 golden 参照；typeck 拒法沿 RX3012/3013/3017 族，零新码。
  2. **mesh/task 执行模型的 MIR lowering**（MeshEXT/TaskEXT，SPV_EXT_mesh_shader，SPIR-V 1.4 per-entry 分叉 RXS-0247 既有机制）——`#[outputs]` 接口块（per-vertex/per-primitive 输出数组）、`SetMeshOutputsEXT`、mesh builtins 编码；同场景 MIR lowering 产物 vs witness 模块语义等价（spirv-val 双口径校验 + 结构对照单测）。
- DXIL 腿：mesh/task 经 B 链 probe 已绿（RD-012 history，步骤 68）；本库面首期 device 门锚 Vulkan 腿，DXIL mesh 腿维持既有 probe 判据 0-byte（不扩面）。

#### A6. bindless 与采样库化（→ RXS-0276）

- `TextureTable`（RXS-0235）作为图资源绑定进 pass（`.reads_table(&table)`）：vk descriptor-indexing 运行时面既有（G3.4 `run_graphics_offscreen_bindless`），RHI 库化 = 资源类 + 槽位种类扩展（见 A7）；着色侧动态索引 + nonuniform 标注（RXS-0232）0-byte。
- 采样面：着色方法族 0-byte；RHI 侧只新增纹理/采样器资源与绑定声明（A2/A3）。

#### A7. 执行面：槽位种类扩展与 vk 执行入口（实现注，无独立条款号，挂 RXS-0272 IR 节）

- `rxrt_rhi_bind` 槽位种类自 kind-2（buffer 指针）扩：texture SRV / sampler / table / color-depth target 句柄（cabi 追加式，RXS-0194 0-byte 语义）；vk.rs 新增 RHI 图形执行入口（消费 `PlannedBarrier` + .rx 源 SPIR-V〔经 artifacts v2〕+ RHI 资源表；既有 `run_graphics_offscreen_v2`/`run_mesh_offscreen`/`run_graph_offscreen` 入口 0-byte 语义，新入口沿 U27/U30 审计模式登记 U31+）。
- 资源生命周期：transient 图形资源（color/depth target）与 buffer 同锚（readback / destroy 释放；章 B 别名复用的对象含图形 transient）。

#### A8. export(c) 图形导出面 + engine_host v3（→ RXS-0277）

- `apps/uc05-rhi/src/embed.rx` 追加图形导出（子集 v1 签名：标量 + 裸指针，如 `uc05_gfx_run_frame(out: *mut u32, w: i32, h: i32) -> i32`——整图封闭在一个 `#[export(c)]` host fn 内，EI1.4 同构）；生成头 CI 再生成逐字节比对（RXS-0254 守卫同面）。
- **engine_host v3**（`src/rurix-engine/harness/` 新增文件，v2/v1 资产 0-byte）：C++/D3D12（LUID 匹配）链 `rurix_rhi.lib`——raster 对照：D3D12 graphics pipeline（vs/ps，d3dcompiler 或预编 cso）；mesh 对照：D3D12 mesh pipeline（ms_6_5/ps_6_5，dxc 预编，Vulkan SDK dxc 在 provisioning）。
- **三方数值精确相等判据（评审 I-G4-IMPL-02 disposition，§9 Q-PixelCriterion 修订）**：.rx RHI（Vulkan）readback ↔ D3D12 readback ↔ host 参考值。**精确相等域 = 无纹理纯色内容 + 最近邻整数内容**——① 全屏/大三角形纯色（光栅化器内部像素确定性，边缘像素不入判据）；② 纹理面仅以 **nearest 过滤 + RGBA8 整数 texel fetch**（两后端均精确）参与，**线性/grad/aniso/浮点插值内容排除在跨后端精确相等域外**（该类内容按后端各自 vs host 参考校验，不进三方精确对照）；不设 ULP 容差——超出本域即换用例，不降判据强度、不引入容差。

---

### 4.B RD-035 执行面三项章（G4.3；RXS-0280~0289；验收门 G-G4-4）

> 定位：RD-035 backfill_condition 三条全部触发（承接期 = G4；三项彼此独立可分批，本章同伞形一次条款化）。未兑现前 evidence/uc05_invariant_matrix.json 的 I10 note 与 RXS-0262「诚实收窄」段字面维持，兑现后随条款修订同步更新（三方一致性机核维持，步骤 75 机制扩展）。

#### B1. transient 别名复用分配器 + 执行期峰值计数器（→ RXS-0280；I10 report_only → measured）

- **别名复用**：sealed 图上每个 transient 资源的生命期区间 = [首写 pass 序位， 末读 pass 序位]（rhi.rs 已精确跟踪 `resource_count`/声明序，RXS-0262 IR 节既有）；区间不重叠者共享同一设备分配（区间图着色分配器，纯 host safe 码，`#![forbid(unsafe_code)]` 面）；重叠者不得复用（正确性优先，着色数 = 并发上界）。**尺寸与对齐约束（评审 I-G4-IMPL-06 disposition）**：同槽资源组按**最大尺寸 + 最大对齐**分配，组内每成员的尺寸/对齐须被槽容量满足（逐成员核）；不满足者不入共享槽（着色按 （区间， 尺寸类， 对齐类） 三分量）。
- **执行期峰值计数器（评审 R-F3 措辞订正）**：执行**回放期随分配/释放事件记账并发存活字节峰值**——计数由 cabi 真实设备分配/释放调用驱动（每次 `cuMemAlloc`/vk 分配事件更新并发存活计数，峰值随真实执行走动），**非静态推算**；峰值 < 声明容量可由 device 执行证据见证（demo：≥2 对不重叠 transient → 峰值严格小于总量，入 evidence json）。
- **I10 升 measured**：矩阵 I10 自 report_only 改 measured（note/tiers 同步：机制 = 执行期峰值计数（回放期分配事件记账），证据级 = measured_local；evidence json + 步骤 79 门）。

#### B2. 依赖驱动重排 + 并行调度（→ RXS-0281 执行模型 / RXS-0282 正确性与新拦截项；🔒 RXS-0239 / RXS-0261 追加式修订）

- **执行模型**：sealed 图建依赖 DAG（RAW/WAW/WAR 边，既有推导同源）→ 拓扑分层；同层独立 pass 可换序、可**批级提交**（单 queue 一次提交多 pass，层间屏障，GPU 管线重叠；多 queue/async 仍 out-of-scope，§8）。
- **语义修订（追加式，严禁改写既有承诺字面）**：RXS-0239 追加「重排执行模型」段——**声明序定义依赖语义（happens-before 需求集），执行序可置换无依赖 pass；任意 i<j 存在依赖边者，pass i 的设备内存效应 happens-before 且对 pass j 可见的承诺不变**；RXS-0261 执行语义同步追加（顺序调度 → 依赖保持下的重排/批级调度）。
- **新拦截项 I11（重排正确性）——调度器与核验器两独立纯函数（评审 I-G4-IMPL-04 disposition，D6 互证先例）**：**调度器**产重排计划；**核验器**（独立函数，不导入调度器逻辑）自 sealed 图**独立重建依赖闭包**，逐边核「计划保持全部依赖序 + 层间屏障覆盖全部跨层依赖边」，违例 → **装配期确定性拒**；核验非调度器自证。**red_self_test 双向**：桩化调度器丢一条依赖边 → 核验器拦（RED）；桩化核验器放过违例计划 → 门检出（防「核验器被桩化」）；demo 图的手算期望调度作 golden 锚。
- I11 入不变量矩阵（tiers 与机制列更新，三方一致；漏拦即红）。
- **峰值计数器与重排交互**：重排改变并发上界（同层 pass 的 transient 区间视为重叠）——别名着色在重排后 DAG 上重算（B1 分配器输入 = 最终执行计划，单一事实源；「分配按旧计划、执行按新计划」的漂移窗口由 seal 后单次推导闭合：seal → 调度 → 着色 → 回放，四序固定）。

#### B3. RXS-0262 const 泛型定长容量 .rx 接线（→ RXS-0283；评审 R-F1/R-F2/I-G4-IMPL-03/I-G4-IMPL-08 disposition 修订）

- **形态（收窄定案，无 RD-007 依赖）**：`rhi.graph::<CAP>()`——lang-item **已知方法**调用点的 turbofish const 实参。**容量值不需要类型级 const 表示、不需要 RD-007 单态化**：编译器在已知方法分支**直接自调用点 AST 取 const 字面量并即时常量求值**（字面量/const item 求值 = 既有面 RXS-0062~0065），求值结果作为普通 `i64` 实参编入 `rxrt_rhi_graph_create(cap)` 类 cabi 调用——CAP 不进 `Graph<C>` 类型参数表（无 const 值类型级传播）。**「零新语法」承诺核算（R-F1）**：turbofish 调用形态为既有语法（`.rx` 泛型调用面），不新增产生式；本面 = 已知方法分支消费既有语法形态，非「const 泛型值进类型系统」。
- **编译期越界拒（新分析如实登记，R-F2）**：静态槽位记账 = **typeck/MIR 层新增的有界局部分析**（非「复用既有诊断通道」措辞——分析本体为新增编译器工作，诊断**码**复用既有 const/类型族）——在单函数体内沿 affine 单定义链前向扫描，按图值逐值记账 `Res` 声明数；声明第 CAP+1 个资源 → **编译期拒**（真实可达类别实现期判，必要时按合并序取码，§5.1）。
- **诚实边界（I-G4-IMPL-08）**：静态可枚举域 = **单函数体内的直线 builder 链**；循环/条件分支内创建资源、**跨函数（helper fn）组装图** → 一律 strict-only 编译期拒「non-static graph construction」（非静默降级运行期记账；EI1 全部 demo/嵌入构建均为单函数直线链，该拒法不缩小既有可表达面）。
- conformance/uc05/reject 新增 `transient_capacity_overflow.rx` / `nonstatic_graph_construction.rx` 等语料锚定（RD-035 明记「EI1.3 已如实不锚不存在的 reject 语料」——本章补齐）。
- RXS-0262「诚实收窄」段随兑现更新（追加式修订行：Vec 承载 → const 容量接线兑现，I10/容量面同步 B1）。

---

### 4.C artifacts v2 + .rx 单源 Vulkan RHI 章（RD-031 兑现；RXS-0290~0294；G4.2 前置切片 + G4.4；验收门 G-G4-5）

> 前置核实（G4_CONTRACT §7 ④ / RD-031 history 2026-07-16）：artifacts blob / `emit_gpu_artifact_globals` 在 main（src/rurixc/src/codegen.rs:99/1028）——backfill_condition 满足，条件臂取**落地**路径。

#### C1. artifacts blob v2 布局（→ RXS-0290；RXS-0209 IR2 defer 兑现）

- v1 = 48B：`version:u32=1 / reserved:u32 / ptx_ptr / ptx_len / cubin_ptr / cubin_len / sm_key[8]`（codegen.rs:1065-1071，rxrt-cabi artifacts.rs:51 解析，`version != 1` 即拒——RFC-0011 §4.7 明记的干净扩展缝）。
- **v2 = v1 48B 前缀兼容 + 追加段**：`version:u32=2`；v1 字段原位不变；尾部追加 `spirv_count:u64` + `spirv_entries_ptr:u64`（指向入口表：`{name_ptr,name_len,stage_tag:u32,spv_ptr,spv_len}` × count；stage_tag 映射既有 `ShaderStage` 枚举序）。v1 解析路径 0-byte（version 1 → 既有分支；version 2 → 新分支；其余 → 维持拒绝）；v2 含零入口表合法（compute-only CUDA 应用编出 v2 亦无 SPIR-V 时）。
- **NVIDIA PTX/cubin 路径逐字节不变**（RXS-0209 IR1 纪律）：v1 blob 产物、装载协商、PTX JIT ladder 0-byte。

#### C2. `@__rx_gpu_spirv` 段发射与多入口收集（→ RXS-0291；评审 C-F2/C-F4/I-G4-IMPL-05 disposition 修订）

- **feature 门控（C-F2）**：SPIR-V 发射**仅在 `vulkan-backend` cargo feature 启用时发生**（`vulkan_codegen`/`dxil_spirv` 模块现状即该 feature 门控）；**feature off → 产物与今日逐字节一致（v1 blob，零 SPIR-V 段）**，「默认构建（全 feature off）零 GPU/SDK 依赖绿」承诺保持；feature on → 按本节产 v2 blob。「单源 Vulkan RHI 通道」的可用性 = 工具链构建面（vulkan-backend on 的工具链），非运行期探测面——条款如实钉死，不宣称默认构建可达。
- `GpuArtifacts` 扩 `spirv_modules: Vec<(name, stage, Vec<u8>)>`；`emit_gpu_artifact_globals` 追加每入口 `@__rx_gpu_spirv_<i>` 全局 + 入口表全局 + blob v2 组装（codegen.rs:1028 同函数加性扩展；feature on 但无 SPIR-V 产物 → 空表 v2 或 v1，实现期按零漂移原则定并留痕）。
- **收集形态 = 每入口独立模块（C-F4/I-G4-IMPL-05：无 SPIR-V 合并、无链接器）**：`build_gpu_artifacts`（driver.rs:944）同一次编译内**新增 driver 侧入口迭代器**——枚举 MIR 全部 GPU 入口（compute kernel 与阶段着色函数），逐入口调用**与 `--target vulkan` 相同的 lowering 入口**（compute → `vulkan_codegen::lower_compute`；vertex/fragment → `emit_spirv_body_vulkan`；mesh → 章 A5 lowering），每入口产一独立 SPIR-V 模块；`build_and_emit_vulkan` 的单入口形态与 `--target vulkan` 单文件产出面 **0-byte 不变**（两路径共用 lowering 函数 = 单一事实源，非「一处两形态」含糊）。
- codegen 单测 + golden：v2 blob 布局 golden（字节序/字段偏移）+ 多入口表内容锚定 + v1 产物（feature off）零漂移断言。

#### C3. rxrt 解析与 DeviceArtifactSet 填充（→ RXS-0292）

- `rurix-rt-cabi/src/artifacts.rs` 解析 v2：SPIR-V 入口表 → `DeviceArtifactSet`——`with_spirv_fallback`（RXS-0209 IR1 槽位，现空置）按入口逐模块填充（单模块 = 现状槽；多模块 = 槽位泛化为按名索引，fatbin.rs 加性扩展 + 既有访问器 0-byte）；解析失败（版本/表畸形）→ RXS-0193 确定性诊断（不占 RX 码）。
- **U 系审计**：v2 解析沿用 artifacts.rs `unsafe fn parse` 既有审计面（U25 族），新增指针走查登记 U31+（单块单操作）。

#### C4. .rx 单源 Vulkan RHI 通道（→ RXS-0293 通道语义 / RXS-0294 device 见证判据）

- **后端选择（§9 Q-E）**：`Rhi::create_vk(&ctx)`（显式 Vulkan 后端构造；`Rhi::create` = CUDA 既有 0-byte）——无环境探测静默切换、无静默回退；Vulkan 不可用（无驱动/无扩展）→ 确定性 Err（RXS-0193 口径）。
- **compute 腿**：`rxrt_rhi_*` 现为 CUDA-only——Vulkan 变体：pipeline 自 SPIR-V 模块（按 kernel 名索引）+ descriptor set 自 marshalling 槽位（RXS-0208 既有 vk 映射：set 0 StorageBuffer 顺排 + push constants）+ dispatch + 计划同步点回放；`run_compute`（vk.rs:1043）为同模式先例。
- **graphics 腿**：即章 A 执行面（A7）——同一通道，G4.2 先落 graphics、G4.4 补齐 compute，通道语义一次条款化（本章）。
- **device 见证**：compute 图（saxpy 级）+ 图形图（章 A demo）各经 Vulkan 通道 device 真跑，数值对照 vs host 参考（+ vs CUDA 腿同图同参交叉对照）；spirv-val 全模块校验；RURIX_REQUIRE_REAL=1。

---

### 4.D C ABI v2 条件臂章（G4.5；RXS-0295~0299；验收门 G-G4-6）

> 判档门（先行于本臂一切实现）：以 **engine_host v3 图形嵌入的真实硬需求**判档（10 §3，争议向上取严）。**可证伪判据清单（评审 R-F4/I-G4-IMPL-07 disposition——「不可由子集 v1 等价表达」必须客观可核）**，逐条以 v3 嵌入面签名为输入：
> 1. **upcall 硬需求**：嵌入面是否需要 .rx 侧**调起宿主代码**（回调）？——数据指针无法承载「调用」语义，子集 v1 无替代表达 ⇒ 回调指针硬需求成立。
> 2. **外部固定 ABI**：被嵌入方是否为 ABI 不可改的既有外部 API（按值 struct 签名已冻结）？——engine_host v3 为本仓自建宿主，**天然不满足**（两侧皆可改，struct 按值恒可指针化替代 ⇒ struct 按值在本判据下对 v3 恒不成立）。
> 3. 任一条成立 → 对应面条款先行兑现 + ABI 往返真跑；**均不成立 → 登记 RD-036+ 存续**（RD-009 close 注先例），判档依据（v3 签名面逐条分析 + 本清单逐项结论）留痕契约 §8。**起草侧诚实预测：v3 嵌入面以子集 v1 可完整表达（无 upcall、无外部固定 ABI），判档大概率 = 不成立**——该结局合法（P-12：不以「完整」为名扩面；条件臂存在 ≠ 条件臂必须兑现，G-EA1-3/RXS-0249 先例）。

#### D1. repr(C) struct 按值（条件臂，→ RXS-0295；评审 C-F1 disposition 修订）

- 类型映射：`.rx` `repr(C)` struct（布局既有语言面）按值进导出签名 ↔ C struct 按值；**Windows x64 ABI（MSVC 精确规则）**：**仅当 struct/union 大小恰为 1、2、4、8 字节时按值传入整数寄存器（字段类型无关——含浮点字段亦走 GPR，MSVC x64 无 SSE 聚合分类）**；**其余一切大小（含 ≤8 的非 2 幂如 3/5/6/7 字节）一律由调用方在栈上复制并以隐藏指针传递**（callee 得副本，按值语义保持）。
- 合法性：仅 `repr(C)` 且无 affine/句柄字段的 struct 可按值导出；嵌套/数组字段按同规则递归；越界 → 编译期 strict 拒（RX6031 同族扩类别或新码，§5.1）。
- 生成头：struct 定义进生成头（逐字段 C 映射，单一事实源 = typeck C 映射）；ABI 往返真跑（C 侧构造 struct 按值传入 → .rx 侧逐字段读回断言；反向返回按值 → C 侧断言；**含 3/5/8 字节三尺寸哨兵**，钉死寄存器/指针分界）。

#### D2. 回调函数指针（条件臂，→ RXS-0296）

- 类型映射：`.rx` 函数指针类型（签名限子集 v1 + D1 类型）↔ C 函数指针（Windows x64 调用约定）；导出 fn 可接受/返回回调指针；调用回调 = 跨 ABI 间接调用（调用约定一致，无 thunk）。
- 运行期契约：回调指针有效性/生命周期为调用方前置条件（documented unsafe FFI boundary，RXS-0255 口径延伸）；**禁 panic 面延伸**——.rx 侧调用回调的栈帧不引入 unwind 跨 C 帧路径（by-construction 维持）；ABI 往返真跑（C 侧回调被 .rx 侧调起，数值回传断言）。

#### D3. 不成立路径（→ RD-036+ 登记）

判档不成立 → 本臂条款不 materialize（RXS-0295/0296 号随未消费作废声明 burned），登记 **RD-036+**（`export_c_extended_signatures_v2`：repr(C) struct 按值 / 回调指针 / 数组按值 / 跨堆所有权，RD-009 close 注先例），RFC 修订行留痕（不重开 RFC，G-EA1-3 先例）。

## 5. 下游 spec 条款映射（spec diff，10 §3 要件）

自 **RXS-0270** 起续号（G4 claim 段 0270~0299，number_ledger v1.13；**RXS-0266~0269 = EI1 earmark 余号 burned 跳号**）。**条款先行**（硬规则 7）：每 PR 条款 commit 先于实现 commit；每条 ≥1 `//@ spec:` 锚定；trace_matrix 全程全锚定；stable 快照加性重 bless 同 PR + bless_log 同 diff（步骤 49 硬红不可分 PR）；既有条款修订为**追加式修订行**（表头「版本」列名纪律，数据行避「版本」子串用「版号」）。**未消费号 close-out 作废声明 burned**（不落裸条款头）。

| 条款（拟） | 章 | 标题 | 落点 spec 文件 | 测试锚定计划（每条 ≥1） |
|---|---|---|---|---|
| RXS-0270 | A | RHI 图形 pass 类型面（raster/mesh；RT 条件臂；着色合法性 RX3015 扩展） | spec/rhi.md（扩章） | conformance/uc05 accept gfx_pass + reject（rhi_gfx_in_kernel RX3015） |
| RXS-0271 | A | RHI 图形资源面（color/depth target、texture2d、sampler、texture_table 薄映射） | spec/rhi.md | accept gfx_resources + reject（cross_brand gfx RX3006） |
| RXS-0272 | A | 图形 pass 访问声明集与自动 barrier（封闭枚举镜像 RXS-0236；推导单源 graph.rs；装配期拒） | spec/rhi.md | 步骤 76 device 出图 + reject（漏声明/写写冲突/反馈环）+ 推导 golden |
| RXS-0273 | A | 图形 pass 声明↔反射相等（I4 图形面扩展，编译器喂反射集，装配期拒） | spec/rhi.md | reject gfx_undeclared_access（库层状态 Err）+ 步骤 77 门 |
| RXS-0274 | A | present 面库化（终端 handoff + headless readback 判据 + RXS-0197 typestate 复用） | spec/rhi.md | 步骤 76 present 迁移 + readback 像素判据 |
| RXS-0275 | A | MIR→SPIR-V mesh/task 编码兑现深化（witness → MIR lowering；SPIR-V 1.4 分叉） | spec/vulkan_backend.md（追加）+ RXS-0246 修订行 | spirv-val 双口径 + MIR lowering vs witness 语义等价单测 + 步骤 76 mesh 出图 |
| RXS-0276 | A | RHI bindless 面（TextureTable 入 pass，descriptor-indexing 运行时面复用） | spec/rhi.md | 步骤 76 bindless 纹理表动态索引像素判据（四象限先例） |
| RXS-0277 | A | engine_host v3 嵌入面（export(c) 图形导出 + 三方数值精确相等判据 + 生成头逐字节守卫） | spec/rhi.md | 步骤 78 device 三方对照 + 篡改再生成 byte-diff RED |
| RXS-0278~0279 | A | **预留不落裸条款头**（图形面溢出顺位） | — | 未消费 → close-out 作废声明 burned |
| RXS-0280 | B | transient 别名复用分配器 + 执行期峰值计数器（I10 升 measured；区间着色；重排后重算） | spec/rhi.md | 步骤 79：峰值 < 声明容量 device 见证 + evidence json |
| RXS-0281 | B | 依赖驱动重排 + 并行调度执行模型（DAG 拓扑分层 + 批级提交；多 queue 仍除外） | spec/rhi.md | 步骤 79：重排 golden + 同图两跑逐字节确定 |
| RXS-0282 | B | 重排后 happens-before 正确性与 I11 拦截项（RXS-0239/0261 追加式修订；装配期确定性核验） | spec/rhi.md + render_graph.md/rhi.md 修订行 | 步骤 79：丢边篡改 RED 语料 + 矩阵 I11 行三方一致 |
| RXS-0283 | B | RXS-0262 const 泛型定长容量 .rx 接线（turbofish const 实参 + 编译期越界拒 + 静态可枚举边界） | spec/rhi.md | reject transient_capacity_overflow + accept 直线构建 + 步骤 79 |
| RXS-0284~0289 | B | **预留不落裸条款头** | — | 未消费 → 作废声明 |
| RXS-0290 | C | artifacts blob v2 布局（v1 前缀兼容 + SPIR-V 入口表；v1 解析路径 0-byte） | spec/vulkan_backend.md（追加；RXS-0209 IR2 兑现） | v2 布局 golden + v1 零漂移单测 + 解析红绿 |
| RXS-0291 | C | `@__rx_gpu_spirv` 段发射与多入口 SPIR-V 收集（同一编译单源；sentinel 口径） | spec/vulkan_backend.md | codegen 单测 + 入口表 golden + sentinel 断言 |
| RXS-0292 | C | rxrt artifacts v2 解析与 DeviceArtifactSet 填充（版本分支；畸形诊断 RXS-0193） | spec/vulkan_backend.md | 解析单测（v1/v2/畸形）+ U31+ 登记 |
| RXS-0293 | C | .rx 单源 Vulkan RHI 通道（显式后端选择 strict 无回退；compute 腿 RXS-0208 薄映射；graphics 腿 = 章 A） | spec/rhi.md + spec/vulkan_backend.md | 步骤 80：compute+graphics 双腿 device 数值对照 |
| RXS-0294 | C | Vulkan RHI device 见证判据（数值对照 + spirv-val + RURIX_REQUIRE_REAL） | spec/vulkan_backend.md | 步骤 80 + evidence json |
| RXS-0295 | D | repr(C) struct 按值 ABI（条件臂；Windows x64 布局与传递约定；合法性边界） | spec/export_c.md（追加） | 条件臂成立时：ABI 往返真跑 + reject 语料；不成立 → burned |
| RXS-0296 | D | 回调函数指针 ABI（条件臂；调用约定；documented unsafe 边界延伸） | spec/export_c.md（追加） | 条件臂成立时：C 侧回调往返真跑；不成立 → burned |
| RXS-0297~0299 | D | **预留不落裸条款头** | — | 未消费 → 作废声明 |

### 5.1 新错误码策略（预测；合并时以 registry 实号为准，不预留不预造）

**前提**：codegen 6xxx 段自 **RX6034** 续（EI1 RX6031~6033 已落；RX6009 burned 不用）；3xxx typeck 按合并序（现最高 RX3017）；7xxx 工具段自 **RX7023** 续；en/zh message-key 成对；registry/error_codes.json 只追加。**本表为预测**，materialize 时以合并时 registry 实号为准。

| 章节 | 类别（归属场景） | 段位 | 需新码 | 状态 |
|---|---|---|---|---|
| §4.B3 | non-static graph construction（循环/条件内建资源，静态不可枚举） | 2xxx/3xxx 段（实现期判真实可达类别；优先复用既有 const/类型诊断，RD-035 零新码预期） | ×1 | 条件 |
| §4.B3 | const 容量越界（声明第 CAP+1 资源） | 同上（复用既有 const 求值/类型诊断通道优先） | ×1 | 条件 |
| §4.D1/D2 | ABI v2 签名越界（非 repr(C) struct 按值 / 非法回调面） | RX6034+ 段（RX6031 同族扩类别优先） | ×1 | 条件（条件臂成立才评估） |

- **合计**：**需新码 0~3（全条件）**——图合法性/声明↔反射/别名分配/重排核验违例一律走**库层状态值零新码**（rhi.md §3/§5.1 先例）；mesh lowering 不可映射 → **RX6026 复用**（vulkan unsupported 同族）；artifacts v2 畸形/运行期失败 → **RXS-0193 诊断封口**（不占 RX 码）；affine/brand/typestate 复用 RX4001/RX4003/RX3006/RX2001~2004/RX3015；着色阶段违例复用 RX3012/RX3013/RX3017。

## 6. feature gate / tracking / 实现序（10 §3 要件）

### 6.1 前置与失败测试先行

- 本 RFC **Approved 合入先于任何实现 PR**（G-G4-2，10 §3 硬性）；**失败测试先行**（反 YAML-only）：RFC 合入时点，`ci/uc05_graphics_rhi_smoke.py`（步骤 76 拟）、`ci/uc05_graphics_invariant_gate.py`（步骤 77 拟）、`ci/uc05_engine_embed_v3_smoke.py`（步骤 78 拟）、`ci/uc05_exec_face_gate.py`（步骤 79 拟）、`ci/vulkan_rhi_channel_smoke.py`（步骤 80 拟）、`ci/blackhole_realtime_smoke.py`（步骤 81 拟）、artifacts v2 codegen（`@__rx_gpu_spirv` 段）/ mesh MIR lowering / RHI 图形库面（`raster_pass`/`mesh_pass`/`color_target` 等已知方法）/ 别名分配器 / 重排调度器 / const 容量接线 / spec RXS-0270~0299 条款体在 main **均不存在 = RED**（脚本名为拟名，随实现 PR 定案；步骤号一旦占用不复用，多余号作废声明）。

### 6.2 feature gate 总裁决

零新 cargo feature、零语言 gate：图形 RHI 为 always-on 库面加性（std::gpu 薄映射扩面）；artifacts v2 为 host codegen 加性通道（v1 路径 0-byte）；`vulkan-backend`（codegen）/ `vulkan`（运行时）/ `dxil-backend` / `shader-stages` 既有 feature 复用。**工具链构建面钉死（评审 C-F2 disposition）**：SPIR-V 发射仅在 `vulkan-backend` on 时发生；**默认构建（全 feature off）产物与今日逐字节一致（v1 blob）**、零 GPU/SDK 依赖绿（clippy/test 矩阵双验沿 G3/EI1 惯例）——「单源 Vulkan RHI 通道」可用性 = 工具链构建面（vulkan-backend on 的工具链 + `RURIX_RT_CABI_LIB` 指向 `vulkan` feature 构建的 cabi），非默认构建可达，不静默不冒充。

### 6.3 栈式 PR 计划（G4.2→G4.7 串行；条款 commit 先行 + 实现同 PR，G3/EI1 结构先例）

- **PR-A（G4.2 首切片，章 C 前置）**：artifacts v2 codegen——spec RXS-0290~0292 条款先行 + `GpuArtifacts` 扩 + `@__rx_gpu_spirv` 发射 + blob v2 组装/解析 + `DeviceArtifactSet` 填充 + codegen 单测/golden + U31+ 登记（无 CI 数字步骤，cargo test 面承载；G4_CONTRACT §7 ④ 工程前置）。
- **PR-B（G4.2 主面）**：图形 RHI 库面——spec RXS-0270~0273/0275 条款先行 + mesh MIR lowering + rhi.rs/vk.rs 执行面 + uc05 图形 demo（raster+mesh 出图）+ 步骤 76/77 + reject 语料 + g4.counter 登记与 evaluator 分支同 PR。
- **PR-C（G4.2 库化补齐）**：采样/bindless/present 库化——spec RXS-0274/0276 条款先行 + TextureTable 入 pass + present handoff + 步骤 76 覆盖扩（像素判据含 bindless 动态索引）。
- **PR-D（G4.2 嵌入）**：engine_host v3——spec RXS-0277 条款先行 + embed.rx 图形导出 + harness v3（新增文件，v2 0-byte）+ 步骤 78 三方对照 + 生成头逐字节守卫。
- **PR-E（G4.3）**：RD-035 三项——spec RXS-0280~0283 + RXS-0239/0261/0262 修订行 + 别名分配器 + 峰值计数器 + 重排调度器 + I11 拦截 + const 容量接线 + reject 语料 + 步骤 79 + 矩阵/报告三方一致。
- **PR-F（G4.4）**：Vulkan RHI 通道——spec RXS-0293/0294 条款先行 + compute 腿 Vulkan 变体 + 步骤 80 双腿 device 对照 + RD-031 处置留痕。
- **PR-G（G4.5）**：C ABI v2 判档——判档留痕（契约 §8）；（成立）spec RXS-0295/0296 + ABI 往返真跑 /（不成立）RD-036+ 登记 + RFC 修订行。
- **PR-H（G4.6）**：BLACKHOLE——归因 evidence + 修复 + REALTIME_OK + 帧对照 + 30fps evidence + 步骤 81（判档 Direct/Mini 执行期定）。
- **PR-I（G4.7）**：close-out（G-G4-8 清单）。

### 6.4 每 PR 不变量核验（全期硬约束）

既有零回归：dxil 套件（404+ 恒定）/ vulkan 套件 grow-only / 步骤 41~75 既有判据 0-byte 只增（步骤 69 blocked 探针恒跑 / 步骤 70 永久 gap）/ B 链 dxv validator + 签名门不旁路 / SPIR-V 1.4 分叉不动 1.0 路径 / RXS-0125·RXS-0149 手写路冻结 / engine_host v1·v2 资产 0-byte / EI1 compute RHI 路（步骤 72~75）零回归。LF byte-exact；counter/entries 不预造（与 evaluator 分支同 PR）；device measured + run URL 归 G4_CONTRACT §8；RURIX_REQUIRE_REAL=1 贯穿 device 段；trace 全程全锚定；新 unsafe U31+ 登记；GPU 实验全经 proc_guard。

## 7. 备选方案

1. **图形 RHI 专用推导（不复用 G3.5 graph.rs）**——**否决**：违 P-11（手写镜像层）；graph.rs 的 AccessKind/状态机已覆盖图形访问面（RXS-0236~0238），RHI 图形 pass 声明可无损映射；另起推导 = 第二事实源 + 双倍审计面（§4.0-1）。注：EI1 期 compute graph 未复用 graph.rs 系「零 .rs 应用判据 + compute-only 同步点语义」下的正解（RFC-0014 §7-2），本期图形面引入后推导单源回归 graph.rs，compute-on-CUDA 路径维持 rhi.rs 0-byte——两推导各服务单一后端，非同一面两份逻辑。
2. **图形 pass 走 CUDA 模拟（compute 伪装 raster）**——**否决**：违 strict-only（静默近似）；光栅化是 Vulkan/D3D12 固定管线语义，CUDA compute 无法等价；且放弃 G3 vk 底座 reuse。
3. **artifacts v2 另起文件通道（.spv sidecar）**——**否决**：RXS-0192 单源嵌入承诺（host 产物自包含）被破坏；sidecar 路径引入部署/完整性新面（哈希/防篡改）；RD-031 既定方向即 blob v2 bump（RFC-0011 §4.7 预留扩展缝）。
4. **RT pass 首期全量兑现**——**否决（条件臂化）**：RT MIR lowering（raygen/miss/closesthit + payload/attribute + trace_ray）+ AccelStruct 资源面 + SBT + DXR 宿主对照是独立大面；门 G-G4-3 以 raster+mesh 满足，RT 条件臂评估（§9 Q-RTArm），不达则登记 RD-036+ 不伪造。
5. **const 容量走运行期记账（汇编期拒）**——**否决**：RXS-0262 Legality 目标形态 = 编译期拒；运行期记账维持「Vec runtime-bounded」现状，不构成 RD-035 ③ 兑现；静态可枚举边界外 strict 拒（§4.B3）保严格性不缩水。
6. **重排直接引多 queue/async compute**——**否决**：RXS-0239 既有承诺面外（多 queue 语义本体另立）；首期单 queue 批级提交即可得管线重叠收益，多 queue 登 §8。
7. **C ABI v2 无条件兑现**——**否决**：P-12 克制压过完整性；无真实硬需求的 ABI 扩面 = 永久审计负债；判档驱动（章 D）。

## 8. 不做（范围红线）

| 不做项 | 理由（摘） | 登记去向 |
|---|---|---|
| RT pass 首期全量（执行臂不可达时） | §7-4 条件臂；门以 raster+mesh 满足 | **RD-036+**（未落地则登记） |
| 多 queue / async compute / split barrier | RXS-0239 既有承诺面外；语义本体另立 | **RD-036+**（需要时登记） |
| DXIL RT 腿（RD-034） | spirv-cross/LLVM 双上游钳制；步骤 69 探针恒跑维护，翻绿 = 复评信号 | RD-034 维持 open（不属本期兑现） |
| RD-027 毒径修复 | NVIDIA 上游侧不可修；MR-0011 护栏 + DRAFT 备包维持 | RD-027 维持 open |
| 窗口/输入进语言、render graph/ECS 进语言 | D-130 红线；06 :151「它们是库」 | 红线维持 |
| MSAA / blending / stencil / indirect draw | 当前零 deferred 登记，不静默带入（G3 同例） | 需要时先补登记再评估 |
| AMD 真卡见证（G-MB1-6） | 缺硬件 pending-hardware 不伪造；全部门锚 RTX 4070 Ti | G-MB1-6 维持 open |
| 跨堆所有权 / 数组按值 / 切片导出 | subset v1 红线（RD-009 close 注）；章 D 判档不含 | RD-036+（随判档登记） |
| `abi_stability_promise` | 维持 RXS-0180 L3；ABI 稳定承诺另期另裁 | 不立（EI1 先例） |
| 外部采纳 / 用户数宣称 | carve-out（G4_CONTRACT out_of_scope） | 不立 |

## 9. 未决问题 / 关键裁决

编号规则：`Q-<名>`。全部为 agent 拟裁（D-406 v2.0，Approved 即定案）；对抗性评审 disposition 可修订，修订落 §9.1 与修订记录。

| # | 裁决点 | 裁决 |
|---|---|---|
| Q-A | 图形图执行后端 | **拟裁**：含任一图形 pass 的图仅经 **Vulkan** 执行（artifacts v2 SPIR-V 通道）；compute-only 图维持 CUDA 既有路 0-byte；CUDA 遇图形 pass → 装配期确定性拒（strict 无回退，§4.0-2）。D3D12 腿 = engine_host v3 宿主侧对照面（非 RHI 执行后端） |
| Q-B | barrier 推导单源 | **拟裁**：图形/混合图 = graph.rs `derive_barriers`（RXS-0238）单源；compute-on-CUDA = rhi.rs `derive_syncs` 0-byte；无第三份推导（§4.0-1，§7-1） |
| Q-RTArm | RT pass 类型面 | **拟裁**：条件臂——mesh MIR lowering 落地后评估 raygen/miss/closesthit 最小集同序列可达性；可达则 `rt_pass` 条款化 + device 真跑（步骤 76 扩），不可达则不立类型面、登记 RD-036+；门 G-G4-3 以 raster+mesh 满足，不依赖本臂 |
| Q-PixelCriterion | 三方像素判据 | **拟裁（评审 I-G4-IMPL-02 修订）**：精确相等域 = ① 无纹理纯色内容（全屏/大三角形，边缘像素不入判据）② nearest 过滤 + RGBA8 整数 texel fetch；线性/grad/aniso/浮点插值内容排除（按后端各自 vs host 参考校验）；**不设 ULP 容差**，超域即换用例不降判据（§4.A8） |
| Q-E | Vulkan 后端选择 | **拟裁（评审 C-F2 修订）**：`Rhi::create_vk(&ctx)` 显式构造（`Rhi::create` = CUDA 0-byte）；无探测静默切换；Vulkan 不可用 → 确定性 Err（RXS-0193）。**工具链构建面钉死**：SPIR-V 发射仅在 `vulkan-backend` cargo feature on 时发生；feature off 产物与今日逐字节一致（v1 blob），「单源 Vulkan RHI 通道」可用性 = 工具链构建面，不宣称默认构建可达（§4.C2） |
| Q-F | const 容量形态 | **拟裁（评审 R-F1/I-G4-IMPL-03 修订）**：`rhi.graph::<CAP>()` = lang-item 已知方法调用点 turbofish const 实参，**字面量即时求值（既有 const eval）→ 普通 i64 cabi 实参**；**CAP 不进类型参数表，无 const 值类型级传播，无 RD-007 依赖**；turbofish 调用形态 = 既有语法（零新产生式）；静态槽位记账 = 单函数直线链有界局部分析（新增编译器工作如实登记，诊断码复用）；循环/条件/跨函数构建 → strict 拒 non-static construction（§4.B3） |
| Q-G | ABI v2 判档输入 | **拟裁（评审 R-F4/I-G4-IMPL-07 修订）**：判档 = engine_host v3 嵌入面签名逐项过**可证伪判据清单**（§4.D：① upcall 硬需求 ② 外部固定 ABI）；起草侧诚实预测 = 大概率不成立（v3 可全以子集 v1 表达）→ 登记 RD-036+ + RFC 修订行；逐项结论留痕契约 §8，不 rubber-stamp |
| Q-H | artifacts v2 布局 | **拟裁（评审 C-F2/C-F4 修订）**：v2 = v1 48B 前缀兼容 + 尾部 spirv_count + 入口表指针；v1 解析路径 0-byte；多入口按名索引、**每入口独立模块（无合并无链接器）**；feature off → v1 产物逐字节不变，feature on → v2（§4.C2） |
| Q-MeshScope | mesh lowering 首期范围 | **拟裁（评审 I-G4-IMPL-01 修订）**：mesh-only（无 task 前置）+ triangle topology + per-vertex 输出 + SetMeshOutputsEXT；**mesh body 类型面为新建**（输出数组声明形态 + `mesh_set_outputs` 已知函数 + builtins 阶段矩阵），非「接线」；task payload 为条件臂（同 Q-RTArm 评估窗）；witness 发射器为 golden 参照 |

## 9.1 对抗性评审记录（对抗性评审要求，10 §3 / §7 · [`../13_DECISION_LOG.md`](../13_DECISION_LOG.md) D-409）

**已完成 第 1 轮 2026-07-23**——由与起草者 Provenance **不同**的工具执行三镜头（correctness / redline / implementability）批判性（对抗性）评审，**评审 provenance `kimi-cli:kimi-for-coding`（独立 kimi-cli 实例，独立进程/零共享上下文，仅持评审提示词与仓库访问）≠ 起草 provenance `Kimi Code CLI (Kimi)`**（硬规则 2 可机验，`ci/check_contribution.py` 规则 4）。18 findings（3 blocker / 12 major / 3 minor）逐条 disposition：**18 条全部采纳并修**（3 blocker 正文实改 / 12 major 正文实改 / 3 minor 措辞与边界订正），无驳回、无空过。状态 Draft → Agent Approved（先于任何实现 PR，G-G4-2）。

**环境留痕（诚实边界，不冒充跨模型）**：首选跨模型评审者 `claude-code` 执行失败——claude CLI 全模型返回 `403 Request not allowed`（账号级不可用，本会话不可修；失败输出存 `.tmp/g4_rfc_review_opus.md` 现场）。改用独立 kimi-cli 实例执行：**工具级 provenance 相异成立**（`kimi-cli:kimi-for-coding` ≠ `Kimi Code CLI (Kimi)`），**但模型同族（kimi-for-coding 系）**——本轮为「跨工具、同模型族」评审，非 D-409 理想形态的「跨模型」。D-409 状态 = Proposed（13 §6，呈 owner 知会），本偏差如实登记；claude 可用后欢迎追加跨模型第 2 轮（修订行追加，不重开 RFC）。评审者标题计数「3 blocker / 13 major / 4 minor」与其实际枚举（12 major / 3 minor）差一，本段以实际枚举 18 条为准并更正。

| 字段 | 值 |
|---|---|
| 评审者 provenance | `Assisted-by: kimi-cli:kimi-for-coding`（独立实例；≠ 起草 `Kimi Code CLI (Kimi)`；同模型族偏差如实登记，见上环境留痕） |
| 评审轮次 | 第 1 轮，2026-07-23 |
| 结论 | **3 blocker / 12 major / 3 minor**；全部采纳并修，无驳回 |

**Findings 与 disposition**（每条一行；镜头前缀 C=correctness / R=redline / I=implementability）：

| # | Finding（评审者提出） | 严重度 | Disposition |
|---|---|---|---|
| C-F1 | §4.D1 Windows x64 struct 按值规则陈述不严：仅 1/2/4/8 字节按寄存器传递，≤8 非 2 幂（3/5/6/7）实际走调用方临时内存指针 | blocker | **采纳并修 §4.D1**：钉死 MSVC 精确规则（1/2/4/8 → GPR 且字段类型无关、无 SSE 分类；其余一切大小 → 调用方栈上复制 + 隐藏指针）+ ABI 往返测试含 3/5/8 字节三尺寸哨兵；§9.2 D-2 的「SSE 分类」SysV 措辞同步删除 |
| C-F2 | §4.0-2/§6.2/Q-E 称「复用既有 feature + 默认构建零依赖」与单源 Vulkan 通道矛盾——`vulkan_codegen`/`dxil_spirv` 由 `vulkan-backend`（默认 off）门控，默认 `rx build` 无法产 SPIR-V | blocker | **采纳并修 §4.C2/§9 Q-E/Q-H**：钉死工具链构建面——SPIR-V 发射仅在 `vulkan-backend` on 时发生；feature off → v1 blob 产物逐字节不变；「单源 Vulkan RHI 通道」可用性 = 工具链构建面，不宣称默认构建可达 |
| C-F3 | §4.0-1「唯一事实源 = graph.rs」但 rhi.rs（RHI 运行时本体）到 graph.rs 的 marshalling 路径未给出 | major | **采纳并修 §4.0-1/§4.A3**：桥接形态钉死——rhi.rs 与 graph.rs 同属 rurix-rt crate，gfx 图 `seal()` 直接构造 graph.rs `Graph`/`PassSpec`（同 crate 调用，无 cabi marshalling），访问枚举直接复用 graph.rs::AccessKind 单源 |
| C-F4 | §4.C2「扩 build_and_emit_vulkan」不成立——该函数单入口单模块，多入口需链接/合并机制 | major | **采纳并修 §4.C2/§9 Q-H**：收集形态 = 每入口独立模块（无合并无链接器）；新增 driver 侧入口迭代器调用与 `--target vulkan` 相同的 lowering 入口（单一事实源）；`build_and_emit_vulkan` 单入口形态与单文件产出 0-byte |
| C-F5 | 图形着色对（vs+fs/ms+fs）反射并集规则未定义；sampler/table 是否算「访问」未钉 | major | **采纳并修 §4.A3（→ RXS-0273）**：反射集 = 逐阶段函数签名资源形参并集（按资源身份合并）；sampler/table 计入反射并集但标「无状态访问」类——barrier 相等域只核资源状态访问，sampler/table 另核绑定完备性 |
| R-F1 | `rhi.graph::<CAP>()` 违「零新语法」——turbofish const 实参需 const 值传播，RD-007 未接通 | major | **采纳并修 §4.B3/§9 Q-F**：机制收窄——字面量在已知方法调用点即时求值（既有 const eval）→ 普通 i64 cabi 实参；CAP 不进类型参数表，无 const 值类型级传播，**无 RD-007 依赖**；turbofish 调用形态 = 既有语法（零新产生式） |
| R-F2 | §4.B3「静态记账复用既有诊断通道」低估——跨方法计数是全新编译期分析 | major | **采纳并修 §4.B3**：如实登记为 typeck/MIR 层新增有界局部分析（单函数体 affine 单定义链前向扫描）；诊断码复用既有族，分析本体为新增编译器工作 |
| R-F3 | §4.B1「device 采集」措辞——峰值为回放期 host 侧记账，非设备侧采样 | minor | **采纳并修 §4.B1**：措辞订正为「执行回放期随分配/释放事件记账（cabi 真实设备分配驱动，非静态推算）」；I10 measured 证据 = device 执行证据见证 |
| R-F4 | C ABI v2 判档「不可由子集 v1 等价表达」不客观（struct 按值恒可指针化）——rubber-stamp 风险 | major | **采纳并修 §4.D/§9 Q-G**：判档改**可证伪判据清单**（① upcall 硬需求——数据指针无法承载调用语义；② 外部固定 ABI——自建宿主天然不满足）；起草侧诚实预测 = 大概率不成立 → RD-036+ |
| R-F5 | 两推导并存削弱 P-11——混合图 compute pass 的 PlannedSync ↔ PlannedBarrier 等价性未声明未验证 | minor | **采纳并修 §4.0-1/§4.A3**：映射钉死（read→ShaderRead / write→UavReadWrite → BufferSync，pass 粒度全序 ⊇ CUDA 流序同步点）；行为等价由 G4.4 双腿交叉对照 device 见证，写入 RXS-0272 |
| I-G4-IMPL-01 | Mesh MIR lowering 严重低估——emit_spirv_inner 仅 vertex/fragment，mesh 需 `#[outputs]` 接口块/输出数组/SetMeshOutputsEXT/builtins/1.4 分叉，独立新 codegen 子项目 | major | **采纳并修 §4.A5/§9 Q-MeshScope**：如实登记 mesh body 类型面为**新建**（.rx 侧现仅入口标注校验，零 body 零语料——本会话复核确认）；工作两段化（body 类型面 + MIR lowering），witness 发射器为 golden 参照 |
| I-G4-IMPL-02 | 三方精确相等对纹理/采样内容不现实（插值/采样/viewport 精度差），无 ULP 回退 | major | **采纳并修 §4.A8/§9 Q-PixelCriterion**：精确相等域收窄为无纹理纯色 + nearest/RGBA8 整数 texel fetch；线性/grad/aniso/浮点插值排除（按后端各自 vs host 参考校验）；不设 ULP 容差，超域换用例不降判据 |
| I-G4-IMPL-03 | `graph::<CAP>()` 隐藏前置依赖 RD-007（const 泛型值单态化 inherited 未接通） | blocker | **采纳并修**：同 R-F1——机制收窄为调用点字面量即时求值 → i64 实参，CAP 不进类型参数表，RD-007 依赖显式解除（§4.B3/§9 Q-F） |
| I-G4-IMPL-04 | I11 自证式——调度器产计划又自核，共享 bug 无独立 oracle | major | **采纳并修 §4.B2**：调度器与核验器两独立纯函数（互不导入，D6 互证先例）；核验器自 sealed 图独立重建依赖闭包逐边核；red_self_test 双向（桩化调度器丢边被拦 + 桩化核验器被门检出）+ 手算期望调度 golden 锚 |
| I-G4-IMPL-05 | artifacts v2 多入口与 `--target vulkan` 单文件产出冲突——缺 SPIR-V 合并机制 | major | **采纳并修**：同 C-F4——每入口独立模块，无合并无链接器；两路径共用 lowering 函数单一事实源（§4.C2/§9 Q-H） |
| I-G4-IMPL-06 | 别名复用遗漏尺寸/对齐约束（共享分配须按最大尺寸+最大对齐） | minor | **采纳并修 §4.B1**：着色按（区间， 尺寸类， 对齐类）三分量；同槽组按最大尺寸+最大对齐分配，逐成员核满足性，不满足不入共享槽 |
| I-G4-IMPL-07 | C ABI v2 条件臂可能永不触发（struct 可指针化、回调被 RX6031 拒）——伪造逃生通道或无真实用例 | major | **采纳并修**：同 R-F4——可证伪判据清单 + 起草侧诚实预测（大概率不成立 → RD-036+）；条件臂存在 ≠ 必须兑现（P-12/G-EA1-3 先例），不成立为合法结局非伪造 |
| I-G4-IMPL-08 | §9.2 B-4 跨 helper fn 图构建未处理——跨过程资源计数超出现有能力 | major | **采纳并修 §4.B3**：静态可枚举域钉死为单函数体直线 builder 链；跨函数组装 → strict 拒 non-static construction（不缩小既有可表达面：EI1 全部构建均单函数直线链） |

## 9.2 已知风险与评审攻击面（起草侧自暴，供 §9.1 评审镜头用）

> **评审已消化（2026-07-23 第 1 轮）**：本节攻击面已由评审逐条覆盖或另发现，disposition 见 §9.1；下列条目补消化指针。

**章 A**
- **A-1 mesh MIR lowering 低估**：**〔评审确认并加重：I-G4-IMPL-01——body 类型面亦不存在，新建两段化，§4.A5/§9 Q-MeshScope 已修〕**。
- **A-2 跨后端「精确相等」判据的可达成性**：**〔评审确认：I-G4-IMPL-02——精确相等域收窄为纯色 + nearest 整数 fetch，不设 ULP 容差，§4.A8/§9 Q-PixelCriterion 已修〕**。
- **A-3 两推导并存的解释负担**：**〔评审确认：R-F5——compute→graph.rs 访问类映射与等价见证路径钉死，§4.0-1/§4.A3 已修〕**。
- **A-4 反射喂入对图形着色对**：**〔评审确认：C-F5——并集规则 + sampler/table 分类钉死，§4.A3（RXS-0273）已修〕**。

**章 B**
- **B-1 区间着色的正确性/漂移窗口**：**〔评审确认：I-G4-IMPL-06——尺寸/对齐三分量着色已修；漂移窗口由 seal→调度→着色→回放四序固定闭合，§4.B1/§4.B2 已修〕**。
- **B-2 I11 自证风险**：**〔评审确认：I-G4-IMPL-04——调度器/核验器两独立纯函数 + 双向 red_self_test + golden 锚，§4.B2 已修〕**。
- **B-3 峰值计数器「device 采集」口径**：**〔评审确认：R-F3——措辞订正为回放期分配事件记账，§4.B1 已修〕**。
- **B-4 const 容量跨函数构建**：**〔评审确认：I-G4-IMPL-08——静态可枚举域 = 单函数直线链，跨函数 strict 拒，§4.B3 已修〕**。

**章 C**
- **C-1 v2 blob 指针字段重定位（DLL/ASLR）**：评审未另发现；维持实现期验证项——v1 ptx_ptr 同构先例（EI1.2 DLL 产物已 device 真跑），v2 入口表指针同机制，PR-A 以 DLL+EXE 双形态单测锚定。
- **C-2 多入口 SPIR-V 双轨**：**〔评审确认：C-F4/I-G4-IMPL-05——每入口独立模块 + 共用 lowering 单一事实源，§4.C2 已修〕**。
- **C-3 sentinel 语义（feature off 的 v2 空表 vs v1）**：**〔评审确认并升格：C-F2——feature off → v1 产物逐字节不变（不产 v2 空表），工具链构建面钉死，§4.C2/§9 Q-E/Q-H 已修〕**。

**章 D**
- **D-1 判档客观性**：**〔评审确认并加重：R-F4/I-G4-IMPL-07——可证伪判据清单（upcall / 外部固定 ABI）+ 诚实预测，§4.D/§9 Q-G 已修〕**。
- **D-2 struct 按值 MSVC 边界**：**〔评审确认并升格：C-F1——MSVC 精确规则钉死（1/2/4/8 GPR，其余指针；无 SSE 分类），§4.D1 已修〕**。

## 10. 稳定化与 provenance

- **稳定化**（10 §5）：本期条款随 stable 快照加性重 bless（RXS-0180 L2 只增不破坏）；导出约定层（P-10「C ABI 导出约定」候选面）维持 RFC-0014 §10 两层区分（单 DLL 字节布局非稳定 / 约定本身经 RD-008 定型）——章 D 若兑现，其类型映射并入同一约定层候选，字节布局仍 L3。artifacts v2 blob 布局 = 工具链内部实现要求（非用户 stable ABI，RXS-0192/0209 同口径）。FCP-lite（advisory）下公开，agent 自主裁决合入。
- **Provenance**：`Assisted-by: Kimi Code CLI (Kimi)`（起草）。agent 自主决策；批准前置 = §9.1 对抗性评审完成（评审 provenance ≠ 起草，D-409/硬规则 2），批准后推进 §6.3 下游实现 PR。

## 11. 规范与实现依据

- **仓内**：milestones/g4/{G4_CONTRACT.md（§7 开工裁决/编号 claim）,G4_PLAN.md,CI_GATES.md（步骤 76~81 拟）}；milestones/ei1/EI1_CONTRACT.md §8.1（EI1 四项工程事实 + RD-035 登记）；milestones/g3/G3_CONTRACT.md §8.1（G3 五面 measured）；registry/deferred.json（RD-031/RD-035 原文；RD-027/RD-034 维护面）；registry/number_ledger.json v1.13（RXS-0270 起 / RXS-0266~0269 burned）；spec/rhi.md（RXS-0256~0265）、spec/render_graph.md（RXS-0236~0241）、spec/host_orchestration.md（RXS-0189~0199/0225/0235）、spec/shader_stages.md（RXS-0223/0224/0231/0232/0242~0245）、spec/vulkan_backend.md（RXS-0208/0209/0230/0246~0248）、spec/dxil_backend.md（RXS-0226~0229/0234/0249）、spec/export_c.md（RXS-0250~0255）、spec/edition.md（RXS-0180）；rfcs/0013（伞形体例）/ rfcs/0014（双面承载 + §9.1 格式）/ rfcs/0011（artifacts v2 扩展缝 §4.7）/ rfcs/0009（宿主编排）；src/rurixc/src/{codegen.rs（:99/1028 emit_gpu_artifact_globals）,driver.rs（:944 build_gpu_artifacts / :1381 compile_vulkan_target）,vulkan_codegen.rs（:501/:544/witness 发射器）,dxil_spirv.rs（:2141 emit_spirv_body_vulkan 最小子集）,resolve.rs（:674-691 RHI/Graph lang items）,mir_build.rs（RHI lowering 段）}；src/rurix-rt/src/{rhi.rs（RhiGraph/derive_syncs）,graph.rs（derive_barriers 单源）,vk.rs（run_compute/run_graphics_offscreen_v2/run_mesh_offscreen/run_ray_tracing_offscreen/run_graph_offscreen）,fatbin.rs（DeviceArtifactSet.spirv_fallback）}；src/rurix-rt-cabi/src/{artifacts.rs（v1 parse/DESC_LEN=48）,lib.rs（rxrt_rhi_* 段）}；src/rurix-engine/harness/{engine_host.cpp（v1）,uc05_engine_host.cpp（v2）}；apps/uc05-rhi（全 .rx 母本）；apps/realtime_run.log（BLACKHOLE E_NOTIMPL 证据）。
- **外部**：Vulkan SDK 1.3.296.0（spirv-val/spirv-cross/dxc）；VK_EXT_mesh_shader / SPV_EXT_mesh_shader；D3D12 mesh pipeline（ms_6_5）/ d3dcompiler；MSVC 14.44 link.exe；Windows x64 ABI 结构传递约定。

---

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| Draft v1.0 | 2026-07-23 | AI 起草初版（G4.1）：伞形四章——章 A 图形 RHI 化（RXS-0270~0279：图形 pass 类型面/资源面/访问声明+自动 barrier/反射相等/present 库化/mesh MIR lowering 兑现深化/bindless/engine_host v3）· 章 B RD-035 三项（RXS-0280~0289：别名复用+峰值计数器 I10 measured/重排+并行+I11/const 容量）· 章 C artifacts v2 + Vulkan RHI（RXS-0290~0294：blob v2/@__rx_gpu_spirv/解析填充/通道+device 见证）· 章 D C ABI v2 条件臂（RXS-0295~0299：repr(C) struct 按值/回调指针，判档门先行）。Q-A~Q-H + Q-RTArm/Q-MeshScope 拟裁；§5.1 新码 0~3 全条件；§7 备选七项；§8 红线十项；§9.2 攻击面自暴（A-1~A-4/B-1~B-4/C-1~C-3/D-1~D-2）。状态 **Draft**：Agent Approved 待 §9.1 对抗性评审（评审 provenance ≠ 起草）后翻，先于任何实现 PR（G-G4-2） | Full RFC（Draft） |
| v1.1 | 2026-07-23 | **对抗性评审 disposition 落实（第 1 轮，评审 provenance `kimi-cli:kimi-for-coding` 独立实例 ≠ 起草 `Kimi Code CLI (Kimi)`，三镜头 correctness/redline/implementability，D-409/硬规则 2）**：18 findings（3 blocker 正文实改 + 12 major 正文实改 + 3 minor 措辞订正）逐条落 §9.1，全部采纳并修无驳回。**三 blocker**——C-F1：§4.D1 钉死 MSVC 精确规则（1/2/4/8 GPR 字段类型无关，其余一切大小调用方复制+隐藏指针，无 SSE 分类；3/5/8 字节哨兵）；C-F2：§4.C2/§6.2/§9 Q-E/Q-H 钉死工具链构建面（SPIR-V 发射仅 vulkan-backend on；feature off → v1 blob 逐字节不变；单源 Vulkan 通道 = 构建面不冒充默认构建可达）；I-G4-IMPL-03（同 R-F1）：§4.B3/§9 Q-F const 容量机制收窄——调用点字面量即时求值 → i64 cabi 实参，CAP 不进类型参数表，**RD-007 依赖显式解除**。**major**——C-F3 桥接形态（rhi.rs 同 crate 构造 graph.rs 模型，AccessKind 单源）/ C-F4+I-G4-IMPL-05 每入口独立模块（无合并无链接器，共用 lowering 单源）/ C-F5 反射并集规则 + sampler/table 分类（RXS-0273）/ R-F2 静态记账如实登记为新增有界局部分析 / R-F4+I-G4-IMPL-07 ABI v2 判档改可证伪判据清单（upcall / 外部固定 ABI）+ 诚实预测大概率不成立 / I-G4-IMPL-01 mesh body 类型面如实登记为新建（两段化）/ I-G4-IMPL-02 三方精确相等域收窄（纯色 + nearest 整数 fetch，不设 ULP 容差）/ I-G4-IMPL-04 I11 调度器/核验器两独立纯函数 + 双向 red_self_test / I-G4-IMPL-08 静态可枚举域钉死单函数直线链（跨函数 strict 拒）。**minor**——R-F3 峰值计数器措辞订正（回放期分配事件记账）/ R-F5 compute→graph.rs 访问类映射与 G4.4 双腿等价见证 / I-G4-IMPL-06 别名着色尺寸/对齐三分量。评审者标题计数（13 major/4 minor）与实际枚举（12/3）差一，以实际枚举 18 条为准更正。**环境留痕**：首选 claude-code 全模型 403 不可用（账号级，本会话不可修），本轮为跨工具/同模型族评审——偏差如实登记 §9.1，不冒充跨模型；claude 可用后欢迎追加第 2 轮（修订行追加）。**状态 Draft → Agent Approved（2026-07-23，先于任何实现 PR，G-G4-2）** | Full RFC（Agent Approved） |
