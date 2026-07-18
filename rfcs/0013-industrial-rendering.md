# RFC-0013 — G3 工业渲染期五特性面（present / 采样超集 / bindless / render graph / mesh-task-RT）

| 字段 | 值 |
|---|---|
| RFC 编号 | RFC-0013（4 位制，编号永不复用，10 §9.5；G3_CONTRACT §7 v1.1 双轨分配：G3 = 单伞形 RFC-0013，RFC-0014 = EI1 earmark） |
| 标题 | G3 工业渲染期五特性面（present / 采样超集 / bindless / render graph / mesh-task-RT）——单伞形 Full RFC，五面各成章（MB1 rfcs/0011 单期伞形先例） |
| 档位 | **Full RFC**（10 §3：**06 §4.2 内存模型映射禁区增补 ×3**（隐式 LOD quad 导数 §4.B3 / texel fetch 越界 §4.B4 / storage image 写唯一写者纪律 §4.B5）+ **pass 边界 happens-before 语义本体**（§4.D4，RD-020 明记归 Full RFC）+ **新语法/类型系统**（intersection/callable 阶段、无界句柄数组、采样方法族）+ **FFI ABI 面**（uc04 shim ABI bump / `rxrt_table_*` / `rxrt_graph_*` / SBT 🔒）；AGENTS 硬规则 5；present 面判档争议向上取严 = Full（硬规则 8，契约 §7 ⑤） |
| 状态 | **Agent Approved（2026-07-18）**（§9.1 对抗性评审 D-409 三镜头 correctness/redline/implementability 已完成；唯一 blocker G-RED-1 合入前正文已实改，全文自洽）。合入 gated on **G-G3-1 归因开闸**（spike 期间零面 RFC 合入，闸门语义） |
| 承接里程碑 | G3（[milestones/g3/G3_CONTRACT.md](../milestones/g3/G3_CONTRACT.md)，验收门 **G-G3-2 ~ G-G3-6** 五面 + G-G3-7 锚定延续；主线 G3.2→G3.6 严格串行，[G3_PLAN.md](../milestones/g3/G3_PLAN.md)） |
| 关联条款 | 拟落 spec **RXS-0220 ~ RXS-0249**（30 条，切分：0220~0222 present / 0223~0230 采样 / 0231~0235 bindless / 0236~0241 graph / 0242~0249 mesh-RT，见 §5；溢出自 RXS-0270 顺续，0250~0269 = EI1 earmark）。落点 spec/{d3d12_runtime, shader_stages, host_orchestration, dxil_backend, binding_layout, vulkan_backend}.md + **新建 spec/render_graph.md**；RXS-0153/0155/0158/**0163~0165**/0174/0210 走既有条款加性修订行，不占新号（**RXS-0164（+0163/0165）落修订行把 unbounded→Unmappable 收窄为『非 bindless-SRV unbounded』、首期单 space0 收窄为『有界 space0 / bindless 自 space（类别）』，前向引用 RXS-0231/0233**，消解 binding_layout.md 单一事实源自相矛盾，SC-2） |
| 关联 deferred | RD-012 · RD-018 · RD-019 · RD-020 · RD-022 · RD-023 · RD-024 · RD-029（registry/deferred.json，全 open，本 RFC 兑现/处置对象；RD-027 = G-G3-1 闸门本体，非本 RFC 对象） |
| 依据决策 | D-406 v2.0（agent 完全自主）· G3_CONTRACT §7 开工裁决 v1.0 ①~⑨ + **v1.1 伞形更正** · D-130（窗/泵/输入不进语言）· D-131（compute=A / 图形=B）· D-207（PTX 收集根排除着色阶段）· D-409（对抗性评审）· 06 §4.2（13_DECISION_LOG 已锁决策，禁止重新发明） |
| Provenance | `Assisted-by: claude-code:claude-fable-5`（起草；五章并行起草 + 一致性核查后汇装）。agent 自主决策 |
| Agent 批准 | **Agent Approved 2026-07-18**（D-406 v2.0 agent 自主批准；§9.1 三镜头对抗性评审完成、blocker G-RED-1 正文收窄后全文自洽）。批准范围含全部 🔒 禁区章节（§4.A4 / §4.B3~B5 / §4.C2~C4 / §4.D4 / §4.D7 / §4.E7~E8） |
| 对抗性评审 | **已完成（2026-07-18，第 1 轮）**——评审者 provenance `Assisted-by: claude-code:claude-opus-4-8`（**≠ 起草 `claude-fable-5`**，跨模型镜头，D-409 / 硬规则 2，`ci/check_contribution.py` 规则 4 机核满足）；三镜头 correctness/redline/implementability 一次覆盖伞形全文；§9.2 攻击面为评审输入；记录落 §9.1（13 findings 全 disposition，blocker G-RED-1 正文已实改） |

---

## 1. 摘要

本 RFC 是 G3 工业渲染期的**单伞形 Full RFC**：把图形面现状（RFC-0007 首期收敛子集 = 显式 LOD 0 单纹理静态 sampler + 全静态绑定 + 手动 barrier + offscreen-only + vertex/fragment 两阶段）一次性推进到工业渲染五特性面，五面各成一章、共用一次对抗性评审、一次 Approved 合入即满足五面「RFC 前置」（契约 §7 v1.1 伞形执行语义）：

- **§4.A present**（G3.2，RD-019）：UC-04 deferred 渲染器接可见 win32 窗口 D3D12 flip-model swapchain 逐帧 present + resize 重建 + 逐帧 readback 数值校验；Vulkan 侧 `OUT_OF_DATE` 重建收尾。语言面零新语法（D-130）。
- **§4.B 采样超集**（G3.3，RD-022/023/024）：`sample` 隐式 LOD 化 + `sample_lod/grad/bias` + texel fetch + 可配置 sampler（静态属性 + 宿主 SamplerDesc）+ shadow 比较 + gather + storage image 写（首期**唯一写者纪律**，结构性禁止可竞写模式）；含 vk.rs graphics descriptor 运行时建面（后续三面共用底座）。
- **§4.C bindless**（G3.4，RD-018）：无界句柄数组 `[Texture2D<F>]` 仅签名形参 + 动态索引临时句柄 + `nonuniform` 标注 strict-only + 独占 set/space 分配律 + 越界 clamp 有界性 + std::gpu `TextureTable`。
- **§4.D render graph**（G3.5，RD-020）：Graph/Pass 声明式宿主库面（无新语法）+ 纯 host 自动资源状态推导 + 🔒 pass 边界 happens-before 语义本体 + 双后端执行器 + uc04 手动 `plan_barriers` 永续独立复核门。
- **§4.E mesh-task-RT**（G3.6，RD-012/RD-029）：六 RT 阶段 + mesh/task 全量类型面（intersection/callable 补齐、payload/attribute 契约升全量、AccelStruct/`trace_ray`）+ SPIR-V mesh/RT 编码与 1.4 per-entry 分叉 + Vulkan mesh 管线 / AS / SBT / TraceRays 运行时全量；DXIL 腿 probe-first 条件分支（RT 预判上游 blocked），RX6008 预留码正式改接。

全期纪律总纲：条款先行（硬规则 7）、真实红绿 device measured（本机 RTX 4070 Ti；AMD = G-MB1-6 尾门 pending-hardware）、不支持面一律编译期/装配期 strict-only 诊断、**全文无任何 UB 措辞**、probe 待定面以条件分支条款写入（G-EA1-3 读法先例），probe 结果落本 RFC 修订行不重开 RFC。

## 2. 动机

图形特性面自 G2.4/RFC-0007 起停在「首期收敛子集」，九条 open deferred 中八条（RD-012/018/019/020/022/023/024/029）是工业渲染的直接缺口：不能配置 sampler、不能 fetch、不能 mip、不能 bindless、不能自动 barrier、不能上屏、没有 mesh/RT。owner 2026-07-18 开工裁决（契约 §7 ②③）已定「RD-027 归因落地即开闸、五面全量推到底」；本 RFC 即五面的规范载体，把上述 deferred 从登记变成条款 + 实现 + measured 工程事实的入口。

**为何需要 Full RFC（而非 Direct/Mini）**：① **06 §4.2 内存模型映射禁区增补 ×3**——隐式 LOD / quad 导数语义（§4.B3）、texel fetch 坐标空间与越界语义（§4.B4）、storage image 写 + 首期唯一写者纪律（§4.B5，禁区结构回避），每条均属「写入 spec 的核心条款」禁区面，必须 Full RFC 全文批准；② **pass 边界 happens-before 语义本体**（§4.D4）——RD-020 reason 明记「barrier 并发/可见性/内存序语义本体另归 Full RFC（硬规则 5）」；③ **新语法 / 类型系统**——`intersection fn`/`callable fn` 两个新阶段关键字、无界数组类型形态、采样方法族与新句柄类型；④ **FFI ABI 面**——uc04 shim ABI bump、`rxrt_table_*`/`rxrt_graph_*` 新符号族、SBT 布局与 device address（均 🔒）；⑤ present 面单独看最轻，但判档争议向上取严 = Full（硬规则 8，契约 §7 ⑤ 已裁）。

**为何伞形单 RFC（而非五份）**：契约 §7 v1.1 更正裁决——owner 双轨分配 G3 = RFC-0013 单号（RFC-0014 = EI1 earmark，0015~0017 in-flight claim 撤回）；MB1 rfcs/0011 已有一份 RFC 承载 compute+graphics+present 全期的先例。伞形收益：五面跨章依赖（descriptor 底座 / 互证语料 / RT 输出通道 / present 胶水）在同一文档内钉死（§4.0），对抗性评审一次覆盖、逐章 findings；编号区间与错误码在同一张总表调停（§5），避免五份 RFC 各自预测互相撞号。

## 3. 指导级解释（用户视角）

### 3.1 present（§4.A）

`.rx` 侧没有任何新东西——RXS-0197 present typestate（CUDA↔D3D12 **interop** 帧机）**维持不动**；UC-04 deferred 渲染器的窗口 present 是**纯 D3D12 图形管线**（vertex/fragment）渲染 → flip-model swapchain 路径，**独立走 C++ shim**、不流经 RXS-0197 的 `.rx` typestate（无 cuda_done 生产者、无 interop fence 环节；两种 present 机制不混，SC-5）。「零新语法」因 UC-04 present 无 `.rx` 面而成立，与 RXS-0197 无关。变化发生在 demo/运行时层：UC-04 deferred 渲染器从「离屏出图 + 回读断言」升级为「可见窗口逐帧呈现 + 回读断言」，拖动窗口 resize 后继续出帧。present 不是「看起来动了」：每帧呈现前 backbuffer 回读做数值断言，无显示环境时 SKIP（dev-env degrade），`RURIX_REQUIRE_REAL=1` 翻硬红。

### 3.2 采样超集（§4.B）

```rx
fragment fn shade(inp: V,
                  alb: Texture2D<f32>,
                  #[sampler(filter = "linear", address = "wrap")] s: Sampler,   // 静态 sampler
                  sh:  Texture2D<f32>, cs: SamplerCmp,
                  rw:  TextureRw2D<f32>) -> Out {
    let a = alb.sample(s, inp.uv);                    // 隐式 LOD（fragment-only）
    let b = alb.sample_lod(s, inp.uv, 2.0);           // 显式 LOD（任意层）
    let c = alb.sample_grad(s, inp.uv, dx, dy);       // 显式梯度
    let d = alb.sample_bias(s, inp.uv, -1.0);         // 隐式 LOD + bias
    let t = alb.load(txy);                            // texel fetch，txy: vec2<u32>
    let k = sh.sample_cmp(cs, inp.suv, refz);         // shadow 比较，恒 LOD 0 → f32
    let g = alb.gather(s, inp.uv, 0);                 // gather4，分量字面量 0..=3
    rw.store(txy, vec4<f32>(a.x, b.y, c.z, 1.0));     // storage image 写
    ...
}
```

宿主侧另有同一状态空间的运行期 `SamplerDesc`（wrap-vs-clamp 像素对照即用它双跑）。违例全部编译期 strict-only 拒。

### 3.3 bindless（§4.C）

```rx
// 无界纹理表：仅签名形参位置合法；索引临时句柄仅立即 receiver
fragment fn fs(mats: [Texture2D<f32>], samp: Sampler,
               #[interpolate(flat)] mat_id: u32, uv: vec2<f32>) -> #[builtin(color)] vec4<f32> {
    mats[nonuniform(mat_id)].sample(samp, uv)
}
```

```rx
// 宿主（std::gpu，affine 纪律沿 RXS-0189）
let table = ctx.texture_table();        // TextureTable<C>
let idx0 = table.register(tex_a);       // 注册序即索引，返回稳定 u32
let idx1 = table.register(tex_b);
```

缺 `nonuniform(..)`、句柄逃逸、位置违例全部编译期拒；device 缺 descriptor-indexing feature → 确定性 Err 非 fake（P-01）。

### 3.4 render graph（§4.D）

```rx
let mut g = Graph::create(&ctx);
let albedo = g.color_target(w, h);
let normal = g.color_target(w, h);
let depth  = g.depth_target(w, h);
let lit    = g.color_target(w, h);
g.pass(geometry_vs, geometry_fs)
    .writes_rt(albedo).writes_rt(normal).writes_depth(depth);
g.pass(lighting_vs, lighting_fs)
    .reads(albedo).reads(normal).reads(depth).writes_rt(lit);
g.readback(lit, &mut pinned);
g.execute();     // 装配期核验 → 状态推导 → 单 queue 顺序提交
```

用户不写任何 barrier；漏声明访问、图结构违例在 `execute()` 装配期确定性 strict 拒——**不存在跑出错误图像或数据竞争的静默出口**。方法名终形随实现 PR 在已知签名纪律内定案（RFC-0009 §4.7 先例），语义面以 §4.D 条款为准。

### 3.5 mesh-task-RT（§4.E）

```rx
#[numthreads(32, 1, 1)]
#[outputs(topology = "triangles", max_vertices = 64, max_primitives = 42)]
mesh fn mesh_main(#[task_payload] p: &Payload, t: ThreadCtx<3>) { ... }   // set_mesh_outputs + 顶点/索引写出

raygen fn rg(tlas: AccelStruct, out_img: TextureRw2D<f32>) {
    let mut payload = HitInfo { ... };
    trace_ray(tlas, origin, 0.001, dir, 1000.0, &mut payload);            // 仅 raygen 体内合法，递归恒 1
    out_img.store(launch_id().xy, shade(payload));
}
```

九阶段补齐为十一阶段（`intersection fn` / `callable fn`）；payload/attribute 契约编译期逐字段比对，错配即拒。Vulkan 腿全量真跑；DXIL 腿 probe-first，上游 blocked 的腿以探针证据落尾门，不伪造。

## 4. 参考级设计

### 4.0 跨章一致性约定（汇装层裁决，五章共同事实源）

汇装前一致性核查（同 Provenance 独立 pass）发现的跨章矛盾在此统一钉死，各章条款以本节为准：

1. **Vulkan 原生 descriptor set 轴（§4.B7 × §4.C2）**：Vk-native 装饰形态 set = 类别轴 **0=CBV / 1=SRV / 2=UAV / 3=Sampler**（§4.B7 拟裁）；bindless 无界表独占 set **自 set4 起**按声明序递增（类别轴之后首个空闲 set）。两处同引 `src/rurixc/src/binding_layout.rs` 为**单一 binding-号事实源 + 按目标（B 链 / Vk-native）选择的两套 set 分配策略**（E-3 采纳：binding 号一处推导，set/space 分配为两套策略而非「一处推导」）；B 链形态装饰**字节不动**（零 golden 重 bless，合入门 = 混合有界+无界+多表+四类别齐全语料的 B 链字节 diff golden，§4.B7）；RTS0 侧无界表独占 space 自 space1，与类别轴无冲突。
2. **`TextureRw2D` 阶段面（§4.B1 × §4.E8）**：首期阶段列 = **fragment + raygen**。raygen 写 storage image 是 §4.E RT 输出通道（`vkCmdTraceRaysKHR` 后回读）的类型面前提，在采样章条款（RXS-0223）一次性钉死，mesh-RT 章不另行修订阶段矩阵。
3. **render graph 首期封闭枚举的不可表达面（§4.D2）**：graph 的访问声明/资源面**不含** bindless 表、storage image（TextureRw2D）资源、mesh/RT pass kind——三者显式登记 §8（RD-034+），RXS-0237「声明-反射双向精确相等」的域界定在首期封闭枚举资源面内；storage image 场景的 barrier 首期走 RXS-0169 手动编排路（§4.B5）。
4. **RT 管线 descriptor 布局（§4.E8）**：沿用 §4.B7 Vk-native set-per-class 装饰形态——TLAS 归 SRV 轴、storage image 归 UAV 轴，分属各自类别 set，不另立手排布局。
5. **vk.rs 新入口命名律**：同签名能力升级用 `_v2` 后缀（`run_graphics_offscreen_v2`），新管线类别用语义名（`run_mesh_offscreen` / `run_ray_tracing_offscreen`），graph 执行器入口定名 **`run_graph`**。
6. **三条资源下发路分工**：`GraphicsResource`（§4.B7，v2 入口的**有界 per-dispatch 资源**下发）/ `rxrt_table_*`（§4.C4，**无界表注册**）/ `rxrt_graph_*`（§4.D7，**图结构与访问声明**下发）——三者正交不重叠，graph 执行器消费前两者的产物、不重复建资源面。
7. **uc04 shim ABI 版本时间线**：见 §6.3——各章只写「bump 至时间线对应版」，不写死版号。
8. **probe 条件分支条款统一措辞**（§4.B6 SampleCmp/Gather、§4.C3 DXIL 腿、§4.E9 mesh/task probe 与 RT blocked 探针）：一律按 G-EA1-3 条件分支读法先例——probe 绿 = 全量臂激活；probe 红 = RD-034+ 尾门臂 + blocked 探针入 CI 防静默腐烂；probe 结果落本 RFC 修订行，不重开 RFC。
9. **合并序敏感号软化纪律**：新 RX 码 / U 号 / RD-034+ / trace 条数 / shim ABI 版号，正文一律相对措辞或引 §5/§6 预测表，以各 PR 合入时 registry/ledger/trace 再生实号为准；3xxx 段续号（自 RX3016）claim 须随 number_ledger 校准补（契约 v1.0 ⑥ 仅 claim 了 RX6027 与 RX7023）。

---

### 4.A present 章（G3.2，RD-019 兑现；RXS-0220~0222；验收门 G-G3-2）

> 定位：把 RD-019（「窗口 swapchain present 路径」deferred）按其 backfill_condition（补 spec 条款/实现/golden + 有显示环境的 device 见证）全量兑现。裁决地基 = RFC-0006 §9 Q-Present（offscreen-first，窗口 present 登 RD-019）。**语言面零新语法**：`.rx` 侧 present 面维持 RXS-0197/0198 typestate 0-byte 复用，全部增量在 C++ shim / rurix-rt 运行时层（D-130 红线）。

#### A1. D3D12 可见窗口 flip-model present 装配与呈现循环（→ RXS-0220）

- **swapchain 装配**：`IDXGIFactory2::CreateSwapChainForHwnd` + `DXGI_SWAP_EFFECT_FLIP_DISCARD`（flip-model 恒定，blt-model 不进本面，§8）；`BufferCount ∈ {2,3}`（默认 3）；format 与 lighting pass final 输出格式一致性经 host 装配核验（镜像 RXS-0167 PSO↔RT 一致性口径）。窗口为**可见** win32 窗（`WS_OVERLAPPEDWINDOW + ShowWindow`；与 vk.rs 既有隐藏窗形态区分——可见性为语义承诺，机器判据见 A3 与 Q-P-VisibleWindow）。
- **呈现循环**：每帧 record（deferred 三 pass 复用既有编排，RXS-0168 结构 0-byte）→ backbuffer 状态迁移锚点 `RENDER_TARGET → COPY_SOURCE`（readback copy）`→ PRESENT` → `Present(sync_interval, flags)` 逐帧 `S_OK`。状态迁移沿 RXS-0169 手动编排——本章不引入任何自动状态推导（自动推导 = §4.D）；缺 PRESENT 态迁移 = 装配核验显式拒（A5）+ debug layer 真跑翻红（G-G3-2 RED 判据）。
- **vsync/tearing 参数面**：`sync_interval ∈ {0,1}`；`sync_interval=0` 且请求 tearing 时须 `CheckFeatureSupport(DXGI_FEATURE_PRESENT_ALLOW_TEARING)` 探测通过 + `ALLOW_TEARING` 建链/呈现旗标成对；能力缺失 = 确定性运行期拒，不静默降级为 vsync（Q-P-TearingFail，P-01）。
- **消息泵**：shim 内非阻塞泵（镜像 vk.rs pump 形态）；泵只搬运 `WM_SIZE`/关闭请求两类事实，**不暴露输入事件面**（D-130 红线，§8）。

#### A2. swapchain 失效与重建（→ RXS-0221；含 Vulkan 侧收尾）

跨后端不变式：**swapchain 失效是正常路径不是错误**；重建序 = 等待 GPU idle → 释放全部 backbuffer 引用/尺寸依赖视图 → 重建 → 首帧重新校验。

- **D3D12 载体**：`WM_SIZE` → `ResizeBuffers(0, w, h, UNKNOWN, flags)`（缓冲数/格式恒定，尺寸取新客户区）；重建前 RTV/依赖资源全释放，重建后 RTV 重建 + **首帧 readback 再断言**（G-G3-2「ResizeBuffers 重建后再 readback 绿」判据）。测试驱动 = `SetWindowPos` 合成 resize（覆盖边界见 §9.2 P-2）。
- **Vulkan 载体（收尾）**：现状 `run_graphics_present`（src/rurix-rt/src/vk.rs:3068）对 `vkAcquireNextImageKHR`/`vkQueuePresentKHR` 仅接受 `VK_SUCCESS`/`SUBOPTIMAL_KHR`，`VK_ERROR_OUT_OF_DATE_KHR` 走 Err 终止——本章补齐：`OUT_OF_DATE`（与可选 `SUBOPTIMAL`）→ `vkDeviceWaitIdle` → 重建 swapchain/imageView/framebuffer（重查 surface caps extent）→ 重录后续帧；重建后首帧 readback 再断言。
- 重建核验失败（重建后格式/缓冲数漂移、视图未重建即录制）= host 侧可判定装配违例，strict-only 显式拒（A5）。

#### A3. present headless readback 校验与 SKIP 纪律（→ RXS-0222）

- **readback = present 面必要 device 证据**（MB1 W6 纪律，spec/vulkan_backend.md 反「present 无 headless 数值校验」先例）：逐帧 present 前 `COPY_SOURCE` 态 copy 到 readback buffer；断言点 ≥3——首帧 / resize 重建后首帧 / 末帧，判据与步骤 48 offscreen 同族（readback 布局复用 RXS-0170 / RX6022）。
- **SKIP 纪律**：无显示环境/非交互桌面 → device 段 SKIP = dev-env degrade 非 fake pass（步骤 41 先例，ci/realtime_present_smoke.py）；`RURIX_REQUIRE_REAL=1` 把 SKIP 翻硬红。SKIP 不占 RX 码（工具/环境层口径，spec/release.md §3）。
- **offscreen 不被替代**：步骤 48（ci/dxil_uc04_device_smoke.py）硬门 **0-byte 不动**；RD-019 close 留痕须明记「present 不得替代 offscreen 真跑」（RD-019 backfill_condition 原文即此）。

#### A4. 🔒 shim FFI ABI 面（每入口独立版本常量；非 stable）

**版本机制修正（E-1 评审采纳）**：现行 shim 对**单一共享常量** `kAbiVersion`（src/uc04-demo/shim/uc04_offscreen.cpp）做**精确相等**校验（`if (abi_version != kAbiVersion) return -1;`），Rust 侧镜像 `device.rs RX_UC04_ABI_VERSION` 原样透传——该机制无加性/无前向兼容（bump 会改两处源码常量、令 offscreen 入口**接受版本**与 `rx_uc04_abi_version()` **返回值**一并漂移，为旧版构建的调用方被新 shim 拒）。本 RFC 改用 **每入口独立版本常量 + `>=` 最小支持版本语义**：present 面新增独立入口 `rx_uc04_present_run(...)`（present 参数：宽高/帧数/sync_interval/tearing/resize 注入点）携其自有版本常量（恒 == 3、`>=3` 语义）；既有 `rx_uc04_offscreen_run` **入口版本常量恒 == 2、函数体字节不变**（`>=2` 语义，为 v2 构建的调用方在聚合版跃升后仍被接受 = 真加性/前向兼容，非 lockstep 拒绝）。**`rx_uc04_abi_version()` 返回值 = shim 聚合 ABI 版本，随每次入口增补而变化——非语义冻结、非兼容契约，纯 build-provenance 号**；原「`rx_uc04_abi_version` 语义字节不变」表述作废（E-1）。步骤 48 的 0-byte 守卫覆盖 offscreen 入口函数体 + 其自有版本常量（==2），**显式剔除聚合版本常量**（§6.6）。🔒 本 ABI 维持「实现确定、gate 后、非 stable」（承 RXS-0167 同级声明；stable 面随 RD-008）；host↔shim 二进制布局不冻结、不进语言 ABI 承诺。

#### A5. 诊断面（strict-only；装配期 6xxx）

不支持面与装配违例一律 host 侧确定性 6xxx 诊断、strict-only、无运行期 fallback（P-01；本章无 UB 面）：

- present 装配核验失败（swapchain desc ↔ final RT 格式/缓冲数失配、blt-model/不支持 swap effect 请求、缺 PRESENT 态迁移锚点）→ 新码（§5 码表预测 RX6027；镜像 RX6018~6022 装配期口径）。
- resize/重建核验失败（重建后格式/缓冲数漂移、视图未重建）→ 新码（§5 码表预测 RX6028；或与上条合码分变体，Q-P-CodeGranularity）。
- 纯运行期/环境失败（`Present` 返回 `DXGI_STATUS_OCCLUDED`/`DEVICE_REMOVED`、tearing 能力缺失、Vulkan surface 建失败）→ 确定性诊断 + 终止，**不占 RX 码**（06 §8.2 / RXS-0193 口径）。
- 新码 en/zh message-key 成对（bilingual 门），registry/error_codes.json 只追加。

#### A6. 语言面：零新语法（D-130 锁死）

RXS-0197/0198（spec/host_orchestration.md）present typestate（CUDA↔D3D12 **interop** 帧机）**维持不动、0-byte**：typestate `Present/Ready/Acquired/Presentable`、backbuffer 借用契约、fence 协议引 RXS-0142 单一事实源，全部不动。**UC-04 窗口 present 独立走 C++ shim + demo crate 驱动，不实例化 RXS-0197 typestate**（SC-5）——「零新语法」因 UC-04 present 无 `.rx` 面而成立，与 RXS-0197 无关（勿把两种 present 机制混为一谈）。本章不新增 lang item、不新增方法、不新增 RX3xxx/RX4xxx 类别。

---

### 4.B 采样超集章（G3.3，RD-022/023/024 兑现；RXS-0223~0230；验收门 G-G3-3）

> 定位：**06 §4.2 内存模型映射禁区（纹理路径）的第二次增补**（首次增补 = RFC-0007 首期收敛子集）。既有基座：类型面 RXS-0174（spec/shader_stages.md）、降级 RXS-0175 / 🔒 内存模型 RXS-0176（spec/dxil_backend.md）、实现锚 `lower_resource_sample`（src/rurixc/src/dxil_spirv.rs）。**严禁 UB 节**：一切不支持面 = 编译期 6xxx/3xxx 诊断 strict-only；一切运行期语义 **well-defined**（storage image 写取首期**唯一写者纪律**结构回避可竞写模式，无竞写即无 race、无「有界非确定」面、无需引 06 §4.2 uniform-size race 公理，§4.B5/G-RED-1）。

#### B1. 方法族类型面（→ RXS-0223）

| 方法 | 签名 | 阶段 | LOD 语义 |
|---|---|---|---|
| `sample` | `(Sampler, vec2<f32>) → vec4<F>` | **仅 fragment** | **隐式**（quad 导数，🔒 B3） |
| `sample_lod` | `(Sampler, vec2<f32>, f32) → vec4<F>` | fragment + vertex | 显式任意层 |
| `sample_grad` | `(Sampler, vec2<f32>, vec2<f32>, vec2<f32>) → vec4<F>` | fragment + vertex | 显式梯度 |
| `sample_bias` | `(Sampler, vec2<f32>, f32) → vec4<F>` | **仅 fragment** | 隐式 + bias（钳 [-16,16)） |
| `load` / `load_lod` | `(vec2<u32>[, u32]) → vec4<F>` | fragment + vertex | 无过滤整型取址（🔒 B4） |
| `sample_cmp` | `(SamplerCmp, vec2<f32>, f32) → f32` | fragment + vertex | **恒显式 LOD 0**（SampleCmpLevelZero 形态） |
| `gather` | `(Sampler, vec2<f32>, ⟨0..=3 字面量⟩) → vec4<F>` | fragment + vertex | 无 LOD（基层 2×2 单分量聚合） |
| `TextureRw2D<F>.load/.store` | `(vec2<u32>) → vec4<F>` / `(vec2<u32>, vec4<F>)` | **fragment + raygen**（首期，§4.0-2） | 无过滤（🔒 B5） |

新资源句柄类型：`SamplerCmp`、`TextureRw2D<F>`；均沿 RXS-0156 句柄纪律（仅签名形参、非值、不可存 let/结构体，RXS-0175 L4 维持——动态索引临时句柄归 §4.C，本章不放宽）。`mir.rs` 的 `MirResourceType` 加 `TextureRw2D(PrimTy)` / `SamplerCmp` 两变体；`class()` 归轴：`TextureRw2D → UAV(u)`、`SamplerCmp → Sampler(s)`。元素 F 首期：sample 族限 `f32`（过滤仅对浮点定义）；`load/store` 支持 `{f32, u32, i32}`（Q-S-Element）。违例一律 **RX3014 扩类别**（strict-only，经 UI golden；G-G3-3 门文）。

**`sample` 语义升级与既有路零回归（Q-S-SampleName）**：RXS-0174 现行 `sample` = 显式 LOD 0。本章把 `sample` 升级为**隐式 LOD**（对齐 D3D `Sample`/Vulkan `OpImageSampleImplicitLod` 业界语义，不发明自有名）；既有显式-LOD-0 语义由 `sample_lod(s, uv, 0.0)` **同一 lowering 路径逐字节承接**，uc04 既有语料迁移为 `sample_lod` 0 → 既有 golden（`dx.op.sampleLevel`）0-byte、步骤 48 判据不动。RXS-0174 落修订行记语义升级。

#### B2. sampler 状态面（→ RXS-0224 静态属性 / RXS-0225 宿主 SamplerDesc）

首期状态空间（两形态共用同一枚举集，单一事实源）：`filter ∈ {nearest, linear}`（min/mag/mip 三合一）+ `max_anisotropy: u32`（1=off；>1 时 Vulkan 侧探测 `samplerAnisotropy`，缺失 → 运行期确定性 Err，RFC-0011 §4.11 纪律）；`address ∈ {clamp, wrap, mirror, border}`（border 色限三预置）；`lod_bias`（钳 [-16,16)）、`min_lod`/`max_lod`。

- **形态 (a) 静态 sampler 属性 `#[sampler(...)]`**（RXS-0224）：挂在 `Sampler`/`SamplerCmp` 形参，状态编译期常量折叠 → D3D12 static sampler（RTS0 `D3D12_STATIC_SAMPLER_DESC`；现 `serialize_rts0` 恒写 `NumStaticSamplers = 0`，本面扩展）/ Vulkan immutable sampler。静态与动态 sampler 共用 s 轴按声明序分配 register（RXS-0164 per-class 轴纪律），静态者不占 descriptor table 槽位。无属性 = 现行静态默认（linear + clamp，RXS-0176 DS4 措辞向后一致）。属性键/值非法 → strict-only 拒（默认并入 RX3014 扩类别；独立可达类别 → §5 码表条件行）。
- **形态 (b) 宿主 `SamplerDesc`**（RXS-0225）：宿主运行时 API 面（rurix-rt/cabi + uc04 shim），同一状态空间的运行期对象；`SamplerCmp` 附 `compare ∈ {less, less_equal, greater, greater_equal}`。wrap-vs-clamp 像素对照（G-G3-3 门判据）走本形态。宿主面无新语法。

#### B3. 🔒 隐式 LOD 与 quad 导数（→ RXS-0227；06 §4.2 禁区增补之一）

- **语义对齐，不发明自有语义**（G-G3-3 门文）：`sample`/`sample_bias` 的 LOD 由 2×2 fragment quad 坐标导数按 D3D11.3 FL 与 Vulkan 规范的 quad 派生规则选取；条款措辞为「实现继承目标 API 的 quad 导数语义」，双后端（`dx.op.sample` / `OpImageSampleImplicitLod`）各自忠实降级。
- **非均匀控制流**：D3D/Vulkan 把发散控制流内的隐式导数判为未定义——Rurix **不引 UB**，改为**编译期合法性规则**：隐式 LOD 采样仅合法于 uniform 控制流。首期判定**结构性成立**：图形着色 body 现为 straight-line 切片（RXS-0171），不存在发散点，by-construction 全域满足、零新码。条款同时**前置锁定**：控制流一旦进入图形 body（后续里程碑），词法处于条件构造内的隐式 LOD 采样 → 编译期 6xxx strict-only 拒（保守近似一律拒，不做 uniformity 分析）——条件条款（§4.0-8），激活时点随图形控制流落地，不预造错误码。
- 派生链一致性（mip 单调性/各向异性截断）全部继承目标 API；不承诺跨后端逐位相同的 LOD 选取（过滤精度为实现近似，对照设计见 B8）。

#### B4. 🔒 texel fetch 坐标空间与越界（→ RXS-0228；RD-023 兑现）

- 坐标 = **非归一化整型** `vec2<u32>`，原点 (0,0) = 左上纹素（与 RXS-0176 DS2 归一化空间同向）；`load_lod` 的 `lod: u32` 选 mip 层，层内坐标以该层尺寸为界。无 sampler、无过滤：恒取单纹素。
- **越界语义 = 坐标钳制（clamp），codegen 注入**（Q-S-FetchOob）：lowering 在 `OpImageFetch` 前显式 emit `min(coord, size-1)`——两后端**同一语义源产同一确定性行为**，零 feature 依赖（不依赖 `robustImageAccess2` 探测、不依赖 D3D 零返回约定），**well-defined、无 UB 节**。
- `TextureRw2D` 的 `load/store` 越界同规则（store 越界 = 钳制后写，确定性）。

#### B5. 🔒 storage image 写与唯一写者纪律（→ RXS-0229；RD-024 兑现；06 §4.2 禁区结构回避）

首期**结构性禁止一切可竞写模式**（禁区的**结构性回避**而非未证内存模型断言）——与 §4.B4 越界 clamp / §4.C3 nonuniform 标注同族，由 typeck/codegen 强制、可 golden 断言，不引任何「有界竞写」内存序公理：

- `TextureRw2D<F>.store` = **普通（非原子）32-bit 分量 store**；首期元素分量全 32-bit（`{f32,u32,i32}`）。
- **唯一写者纪律（首期强制，结构约束，可 golden）**：storage image `store` 的目标坐标恒为**本 invocation 的位置标识 identity 映射**（raygen = `launch_id().xy`；fragment = 本 fragment 覆盖的目标像素坐标），使**每 texel 至多一个 invocation 写**成为 by-construction 事实——跨 invocation 同 texel 竞写模式**结构性不可构造**。坐标非本 invocation 位置标识派生、可产生多写者的 `store` 模式 → **编译期 6xxx strict-only 拒**（保守近似一律拒，不做别名分析；诊断通道随实现 PR，默认并入 RX3014/RX6023 扩类别或独立可达类别登 §5 码表条件行）。
- **无竞写即无 race**：唯一写者下不存在跨 invocation 同 texel 的未同步冲突写，故**本条无任何「有界非确定」内存序断言、无 UB 节**，§1「全文无任何 UB 措辞」在本条真成立（不需引 06 §4.2 uniform-size 有界区，也不移植 PTX 公理到 Vulkan/D3D 图形路）。
- **同 invocation 内** store→load 程序序可见（well-defined，单写者下无跨线程问题）。
- **下一 pass 消费可见性**：唯一写者纪律只封死同 pass 内跨 invocation 冲突；storage image 被后续 pass 读取的可见性保证点 = **pass/dispatch 边界 barrier**（UAV → SRV/UAV 状态迁移），首期编排走 RXS-0169 手动锚点路——**§4.D render graph 首期封闭枚举不承载 storage image 资源**（§4.0-3，§8 登记），storage image barrier 在 §4.D 自动推导域**之外**（G-RED-2），与 RXS-0176 DS6「采样语义假定 barrier 已就位、缺失归编排层拦截」同构分层。
- **可竞写模式（多写者同 texel）整体放开登 RD-034+ 另 Full RFC**（§8）：放开竞写须独立论证 Vulkan/D3D 内存模型下的非原子写可见性语义并给规范引文，非本期结构回避可承载；image atomics（`OpImageTexelPointer` + atomic）同显式不做（§8，RD-034+），跨线程协同走既有 `Atomic<T, Scope>` 缓冲路（RXS-0080），不在图像路重复建面。

#### B6. codegen 降级：SPIR-V opcode 全家 + B 链（→ RXS-0226）

扩展 `lower_resource_sample` 为方法族分发：

| 方法 | SPIR-V | ImageOperands | spirv-cross → HLSL | DXIL |
|---|---|---|---|---|
| `sample` | `OpImageSampleImplicitLod`(87) | — | `Sample` | `dx.op.sample` |
| `sample_lod` | `OpImageSampleExplicitLod`(88，已有) | Lod | `SampleLevel` | `dx.op.sampleLevel` |
| `sample_grad` | `OpImageSampleExplicitLod`(88) | Grad | `SampleGrad` | `dx.op.sampleGrad` |
| `sample_bias` | `OpImageSampleImplicitLod`(87) | Bias | `SampleBias` | `dx.op.sampleBias` |
| `load/load_lod` | `OpImageFetch`(95)+ 钳制序列 | Lod | `Load` | `dx.op.textureLoad` |
| `sample_cmp` | `OpImageSampleDrefExplicitLod`(90，Lod 0) | Lod | `SampleCmpLevelZero` | `dx.op.sampleCmpLevelZero` |
| `gather` | `OpImageGather`(96，分量字面量) | — | `Gather{Red,Green,Blue,Alpha}` | `dx.op.textureGather` |
| `TextureRw2D.load/.store` | `OpImageRead`(98)/`OpImageWrite`(99) | — | `RWTexture2D<float4>` 下标 | `dx.op.textureLoad/textureStore` |

- `TextureRw2D<F>` 的 `OpTypeImage` 带**显式 format**（f32→Rgba32f 等），规避 `shaderStorageImageWriteWithoutFormat` capability 依赖；`SamplerCmp` 走 depth-比较采样类型形态。B 链末尾 dxv validator + 签名门（RX6011/6012）不旁路不裁剪；全部 `.spv` 过 spirv-val。
- 子集外/模式外构造 → **RX6023 扩类别**（strict-only，RXS-0175 L2 通道复用）；**拟零新 6xxx 码**——独立可达类别（如 storage image 格式-元素失配）见 §5 码表条件行。
- **B 链 probe 先行**（条件分支条款，§4.0-8）：分离 image/sampler 形态的 `SampleCmp`/`Gather` 经 spirv-cross→dxc 的实测语料先行；probe 红 → 该子模式以 probe 证据诚实登 RD-034+ 尾门、条款按条件分支收窄激活。
- **PTX 腿：D-207 结构性不适用，如实标注**——采样面仅存在于图形着色阶段，PTX 收集根排除之（契约 out_of_scope `ptx_texture_path`）；条款正文落一句性事实标注，不承诺、不登记 OptiX 方向。

#### B7. Vulkan 运行时 graphics descriptor 建面（→ RXS-0230；运行时最大缺口，后续三面共用底座）

现状：`run_graphics_offscreen`（src/rurix-rt/src/vk.rs:1786）**零 descriptor 面**，纹理/采样器/storage image 在 Vulkan 腿运行时结构不可达。本面新建：

- **`run_graphics_offscreen_v2` 加性 API**（命名律 §4.0-5）：v1 签名与行为 0-byte 保留（MB1 语料零回归）；v2 追加 `resources: &[GraphicsResource]`（纹理含逐层 mip 数据、SamplerDesc、storage image）。内部建 `VkDescriptorSetLayout`（含 immutable samplers）/ `VkDescriptorPool` / `vkUpdateDescriptorSets` / `vkCmdBindDescriptorSets`，mip 链经 staging 逐层 upload + layout 迁移。新 unsafe FFI 逐处 `// SAFETY:` **折叠进 U27 扩注**（graphics FFI 边界内，0 新号，§6.4/E-2）。
- **绑定方案（Q-S-BindingScheme，本面最锐利设计点）**：现行 `infer_spirv_bindings`（binding_layout.rs:139）**硬编码 `set:0`**（:157）+ per-class binding——B 链经 spirv-cross 映射 register 正确，但**原生 Vulkan 消费下四类轴的 binding 0 互撞**。该函数**就地注释（:144-147）记录过一次真实 device bug**（旧全局递增 binding 令 sampler 落 `s1` vs RTS0 `s0`，lighting pass 采样不到 G-buffer），本改动直接触碰该核心循环故取证从严（E-3）。拟裁：**单一 binding-号事实源 + 按目标选择的两套 set 分配策略**（非「一处推导两形态」的含糊表述）——B 链形态维持现装饰**字节不动**（零 golden 重 bless）；Vk-native 形态由同一 lowering 切换 set 分配策略为 `set = 类别轴（0=CBV/1=SRV/2=UAV/3=Sampler）、binding = 类内序`（binding 号与 RXS-0164 register 推导同一事实源；bindless 无界表自 set4 起，§4.0-1）；该目标选择由既有 provenance/mode 旗标承载（现仅门控 UserSemantic 发射，本面显式扩为亦门控 descriptor-set 装饰）。**合入门（非仅 UI golden）= 「混合有界+无界并存 + 多表 + 四类别齐全」压测语料的 B 链 SPIR-V 字节 diff golden**（机核 B 链装饰字节不动，承 :144-147 device bug 教训）；conformance 另断言两形态除 Decorate 外指令流逐字相等（反双产物漂移）。

#### B8. device 见证与双后端数值一致性（G-G3-3 判据设计）

- **≥6 模式数值判据**（步骤 63；counter `g3.counter.sampling_superset_modes` ≥6）：① 隐式 LOD + mip 金字塔逐层异色（远近两采样点色异 = 证真 mip 选取）；② `sample_lod` 指定层选色；③ `sample_grad` 大梯度选高层；④ `load` 越界钳制断言；⑤ wrap-vs-clamp 像素对照（同 UV>1 两 SamplerDesc 双跑，像素必异）；⑥ `sample_cmp` shadow 双色；⑦ `gather` 角点 2×2 单分量断言；⑧ `TextureRw2D` **唯一写者 store**（identity 坐标，§4.B5）→pass 边界 barrier（RXS-0169 手动编排）→回读，且**可竞写模式（多写者同 texel）编译期拒**（唯一写者 golden：合法 identity-store accept + 非 identity 多写者 reject）；⑨ 多分量元素。逐项「篡改 → 像素变 = RED，复原 = GREEN」数据流红绿（RXS-0176 IR2 纪律）。
- **双后端一致性对照的诚实边界**：线性过滤权重精度为实现近似（D3D 仅保证定点下限），**逐位一致不可承诺**——对照设计：nearest-filter 与纹素中心采样模式**逐位比对**；linear/aniso 模式以容差带比对 + 结论只落「同判据双绿」。无显示/无 Vulkan → SKIP 三态，`RURIX_REQUIRE_REAL=1` 翻硬红；本机 RTX 4070 Ti measured + run URL 归契约 §8。

---

### 4.C bindless 章（G3.4，RD-018 兑现；RXS-0231~0235；验收门 G-G3-4）

> 定位：兑现 RD-018（「bindless / unbounded descriptor array / descriptor heap 直索引绑定推导」）。RFC-0005 §9 Q-Bindless 裁决 = 本期 defer 登 RD-018、backfill 时按 10 §3 判档接通 Full RFC——本章即该载体。现状锚点（实读核验）：`mir.rs` `ResourceCount::{One, Bounded(u32), Unbounded}` 已建模、`Unbounded` 推导侧 strict-only 拒（RX6013 `codegen.dxil_unmappable`，spec/binding_layout.md RXS-0163~0165）；句柄非值纪律 RXS-0156/0174；全仓零 descriptor-indexing 痕迹——从零建面。设计总纲：**把 `Unbounded` 从「显式不可映射」翻转为「合法降级路」，既有有界推导字节级零回归**；全章无任何 UB 措辞。**D-207 事实标注**：bindless 面同为图形着色阶段面，PTX 收集根排除之，PTX 腿结构性不适用（与 §4.B6 同口径）。

#### C1. 类型面：无界句柄数组与动态索引（→ RXS-0231/0232）

- **形态**：`[Texture2D<F>]`（切片样式类型文法，无新 token；F 沿 RXS-0156 首批 = f32）。**仅可作着色阶段函数签名形参**——返回位置/结构体字段/非着色阶段签名/嵌套/有界数组混写，一律违例（拟复用 RX3013「资源句柄违例」类别扩展；若评审判独立类别 → §5 码表条件行）。
- **索引表达式**：`table[idx]`，`idx : u32` 任意表达式。索引结果为**临时句柄**（类型 `Texture2D<F>`），**仅可作立即 receiver**（`table[i].sample(samp, uv)` / §4.B 方法族）——不可 `let` 绑定、不可传参、不可存字段（RXS-0156/0174 句柄非值纪律不破）。违例并入 RX3014 扩类别（UI golden）。
- **nonuniform 标注（strict-only）**：索引表达式须以 `nonuniform(expr)` 包裹，**唯一豁免 = 整型字面量常量索引**。缺失 → 编译期拒（新码，§5 码表预测 RX3016，3xxx 着色段，UI golden）。**不做 uniformity/divergence 推断**——保守全标合法（过标注仅性能保守，SPIR-V 合法），推断留后期（Q-B-Uniformity）。

#### C2. 绑定推导：独占 set/space 分配律（→ RXS-0233）

- `ResourceCount::Unbounded` 自 `descriptor_span` 的 `Unmappable` 拒绝路径翻转为合法路：
  - **SPIR-V 侧（Vk-native 形态）**：每个无界表独占一个 descriptor set，**自 set4 按声明序递增**（类别轴 set0~3 之后首个空闲 set，§4.0-1；与 §4.B7 同一 `binding_layout.rs` 分配律事实源）；表内 binding 0 单点。**B 链形态装饰字节不动**（零 golden 重 bless）。
  - **D3D12/RTS0 侧**：每个无界表独占一个 register space，**自 space1 按声明序递增**，descriptor range `NumDescriptors = unbounded(0xFFFFFFFF)`、`BaseShaderRegister = 0`——独占 space 分配律使 unbounded range 吞轴行为结构性无冲突。
  - 两侧 **binding-号同源单一事实源**（`binding_layout.rs` 一处推导），set/space 分配为**按目标（Vk-native/RTS0）选择的两套策略**（E-3；非「一处推导」的含糊表述），SPIR-V 装饰与 RTS0 序列化共同消费，沿 RXS-0163~0166 确定性纪律；有界路零漂移以「混合有界+无界+多表+四类别齐全」B 链字节 diff golden 为合入门（§4.0-1/§4.B7）。
- **RXS-0164 加性修订行**（SC-2 采纳；front-matter / §5 清单同列）：本条款把 spec/binding_layout.md RXS-0164（+必要时 0163/0165）的「unbounded→Unmappable」收窄为「**非 bindless-SRV unbounded**」、「首期单 space0」收窄为「**有界 space0 / bindless 自 space（类别）**」，并在 RXS-0164 处前向引用 RXS-0231/0233——消解 binding_layout.md 单一事实源内「unbounded→Unmappable / 单 space0」与「unbounded SRV 合法 / space1+」两组条款自相矛盾。
- 🔒（边界声明，沿 RXS-0163 先例）：set/space **具体数值**为实现确定、gate 后、非 stable，不冻结为 ABI；本条只承诺「独占性/声明序确定性/有界路零漂移」。
- 首期无界元素种类**仅 SRV 纹理**（`Texture2D<F>`）；无界 Sampler/CBV/UAV 表 → 维持 `Unmappable`/RX6013（不新码），见 §8。

#### C3. codegen 双腿（→ RXS-0234）

- **SPIR-V（共享语义源）**：`OpTypeRuntimeArray`（元素 = image 类型）+ `OpVariable`(UniformConstant) + `DescriptorSet/Binding` 装饰；capability `RuntimeDescriptorArray` + 标注索引处 `NonUniform` 装饰 + capability `ShaderNonUniform`；归属 `SPV_EXT_descriptor_indexing`（**Vulkan 1.2 core**，spirv-val 以 vulkan1.2 环境校验，承 RXS-0212）。索引降级为 `OpAccessChain`（runtime array）→ `OpLoad`（image）→ 立即消费，**不物化中间句柄 local**（MIR 层以 Rvalue 内联形态承载，镜像 RXS-0175 先例）。
- 🔒 **越界有界性（实现定义但有界，无 UB 措辞）**：codegen 对每个动态索引**强制发射 clamp**（`UMin(i, table_len - 1)`），`table_len` 由运行时经既有 marshalling 通道（RXS-0208 push-constant 槽尾部追加）提供 = 宿主 TextureTable 已注册计数。语义承诺：越界索引的观察结果为**实现定义**（clamp 后某已注册元素的采样值），访问恒有界于已注册表段，空槽结构性不可达。不依赖设备可选 robustness feature（Q-B-OOB）。
- **DXIL 腿（B 链，D-131；probe-first，§4.0-8）**：spirv-cross 将 runtime array 译为 HLSL unbounded `Texture2D t[] : register(t0, spaceN)` → dxc `-T *_6_0`（unbounded array + 动态索引 = SM6.0 资源能力面即可，不需 SM6.6 dynamic resources——此为 probe 前文献推断，probe 证据落地后回填）→ dxv + 签名门（RX6011/RX6012）不旁路；RTS0 unbounded range 与 HLSL register/space 经 RXS-0166 同构一致性门交叉核验。probe 红则 DXIL 腿以证据诚实落 RD-034+ 尾门，Vulkan 腿不受牵连。

#### C4. 运行时与宿主注册面（→ RXS-0235）

- **std::gpu `TextureTable<C>`**：非 Copy **affine** 句柄（纪律与 brand 契约全量复用 RXS-0189，零新借用码）；`ctx.texture_table() -> TextureTable<C>`、`table.register(tex) -> u32`（**注册序即索引，稳定单调**）、`table.len() -> u32`。**格式擦除**（Q-B-TableFormat）：`TextureTable<C>` 不带元素类型参数，host↔shader 元素类型错配 = 运行期确定性 Err（非 fake、非静默），条款显式声明该边界。kernel/device 体内调用 → RX3015（RXS-0189 着色格复用）。
- **Vulkan 运行时**：descriptor pool/set-layout 带 `UPDATE_AFTER_BIND` + `PARTIALLY_BOUND` binding flags；feature chain 探测（`VkPhysicalDeviceDescriptorIndexingFeatures` 四 bit：`shaderSampledImageArrayNonUniformIndexing` / `descriptorBindingSampledImageUpdateAfterBind` / `descriptorBindingPartiallyBound` / `runtimeDescriptorArray`）——**任一缺失 → 确定性 Err**（RXS-0193 封口，运行期不占 RX 段位），无静默降级（P-01）。无界表独占 set4+ 使有界 set0~3 完全不触 feature chain（零回归最强形）。注册写入仅发生在提交前；in-flight 期间不更新（§8）。
- 🔒 **cabi FFI 面**：新符号 `rxrt_table_*`（create/register/len/destroy）**只追加**，`rxrt_launch` 及既有符号面字节不变（RXS-0194「符号面只追加」纪律）；u64 句柄表/handle-0/poisoned 传播跨后端不变式维持。unsafe 新增集中 vk.rs，逐处 `// SAFETY:` **折叠进 U27 扩注**（graphics FFI 边界内，0 新号，§6.4/E-2）。
- **D3D12 腿运行时**（uc04 shim）：SRV descriptor heap 连续段按注册序填充，unbounded range 绑至该段基址；shim ABI bump 至 §6.3 时间线对应版。

---

### 4.D render graph 章（G3.5，RD-020 兑现；RXS-0236~0241；验收门 G-G3-5）

> 定位：契约 in_scope `render_graph_auto_barrier`。**本章存在的核心理由**：RD-020 明记「barrier 并发/可见性/内存序语义本体另归 Full RFC（硬规则 5）」——该本体在 D4（🔒）落笔。RFC-0006 §9 Q-Barrier 首期裁决（手动 barrier 编排、自动推导 defer）自此兑现升级；RXS-0169 手动核验器不废除，转为推导产物的独立复核门（D6）。

#### D1. 总纲

**声明式宿主库面，无新语法**。Graph/Pass 为编译器已知签名的 lang-item 宿主类型（RXS-0189 lang-item + RXS-0190 已知签名分支先例，零新文法产生式）；pass 以五类访问集方法声明读写面；**声明序 = 提交序**（不做重排，D2）；自动状态推导为 rurix-rt 纯 host safe 模块 `graph.rs`（D3）；pass 边界 happens-before 语义本体为 🔒 条款（D4）；D3D12 / Vulkan 双执行器消费**同一**推导产物（D5）；uc04 手动 `plan_barriers` 保留为独立复核门（D6）；cabi 只追加 `rxrt_graph_*` 符号（D7）。用户样例见 §3.4。

#### D2. 类型面与图合法性（→ RXS-0236/0237）

**类型规则**：`Graph<C>` / `PassBuilder` / 资源句柄（color/depth target、UAV 位复用 RXS-0189 `Buffer<C, T>`、readback 目的复用 `PinnedBuffer<C, T>`）为编译器 lang items，用户同名定义优先遮蔽、语义不变（RXS-0189 兜底纪律）；全部句柄**非 Copy affine**，move/借用违例复用 RXS-0054 / RXS-0057~0061 既有裁决，零新借用码。brand 契约沿 RXS-0189（跨 context 误用 → RX3006 复用）。Graph 系构造出现在 `kernel`/`device fn` 体内 → RX3015（零新码）。方法签名编译器已知（RXS-0190 口径）：元数/类型/方法名不符 → RX2003/RX2001/RX2004 复用。

**访问声明集（封闭枚举——本面「不支持即不可表达」）**：

| 声明方法 | AccessKind（同源枚举，D3/D5 单一事实源） |
|---|---|
| `writes_rt(t)` | `ColorAttachmentWrite` |
| `writes_depth(t)` | `DepthAttachmentWrite` |
| `reads(t)` | `ShaderRead` |
| `reads_writes_uav(b)` | `UavReadWrite` |
| `readback(t, dst)` | `CopySrcReadback`（源）+ `CopyDstReadback`（目的 buffer） |
| present 终端胶水（D5c） | `PresentHandoff` |

**首期不可表达面（§4.0-3）**：bindless 表声明、storage image（TextureRw2D）资源、mesh/RT pass kind 均不在封闭枚举内——凡含此三者的 pass 首期不可经 graph 表达，显式登记 §8（RD-034+），不静默。

**图合法性（装配期确定性核验，strict-only）**——`execute()`（或显式 `seal()`）时全量判定，违例 → 6xxx strict 拒（RFC-0006 §9 Q-Err「装配期可预测错误续用 6xxx」先例；新码 ×2 见 §5 码表预测 RX6029/RX6030）：

- **环 / 读未写**：声明序即全序，资源依赖边逆序即后向边 = 环，拒；读从未被写的 target 同族拒（**条款措辞锁 use-before-write 可达形态**）。
- **写写冲突**：同 pass 对同资源重复声明写 / 同资源同 pass 既 `reads` 又 `writes_rt`（`reads_writes_uav` 为唯一合法读写合并）→ 拒。跨 pass 顺序重写（ping-pong）合法（由 D4 全序覆盖）。
- **未声明访问 / 声明未用**：pass 的管线绑定反射面（RXS-0163~0166 单一事实源）与声明集**双向精确相等**核验（Q-G-OverDeclare），**相等域 = 首期封闭枚举资源面**（§4.0-3）；漏声明或声明未用 → 同码拒。
- **生命周期误用**：seal 后追加 pass / 重复 execute / 空图 execute → 拒。

#### D3. 自动资源状态推导（rurix-rt `graph.rs`，纯 host safe；→ RXS-0238）

- **输入** = 已 seal 图；**输出** = 确定性 barrier 计划：逐资源状态机（初态 `Undefined/Common`）沿声明全序推进，下一使用点所需状态 ≠ 当前状态即在该 pass 边界产出一条转换。**推导为纯函数**：同图 → 逐字节相同计划（golden 可锚定）；模块零 unsafe、零后端调用、无 GPU 依赖，单测恒跑。
- **资源类别分立（状态机诚实性）**：三种 barrier 形态——`Transition`（image/attachment：D3D12 states / Vulkan layout 迁移）、`BufferSync`（buffer：无 layout，仅 stage+access）、`UavSync`（同资源相邻 UAV 写-写/写-读：D3D12 UAV barrier / Vulkan memory barrier）。不把 buffer/UAV 硬套 image 迁移模型。
- **双后端映射同源**：`AccessKind → D3D12_RESOURCE_STATES` 与 `AccessKind → (VkImageLayout, VkPipelineStageFlags, VkAccessFlags)` 两张映射表同居 `graph.rs` 一处（P-11 单一事实源），执行器只**逐字重放**，禁止后端侧二次推导或语义重映射（含 shim C++ 侧）。映射锚点：`ColorAttachmentWrite→RENDER_TARGET / COLOR_ATTACHMENT_OPTIMAL`、`ShaderRead→PIXEL_SHADER_RESOURCE / SHADER_READ_ONLY_OPTIMAL`（RXS-0176 跨 pass RT→SRV 既有裁决的推广）、`CopySrcReadback→COPY_SOURCE / TRANSFER_SRC_OPTIMAL`、`UavReadWrite→UNORDERED_ACCESS / GENERAL`、`PresentHandoff→PRESENT / PRESENT_SRC_KHR`。

#### D4. 🔒 pass 边界 happens-before 语义本体（→ RXS-0239；禁区子节，本 RFC 全文批准对象）

- **承诺面（且仅此面）**：单 queue；声明序 = 提交序 = pass 粒度完成序。对任意 i < j，pass i 的全部 device 内存效应（RT/depth/UAV 写、copy 写）在 pass j 的任何访问**之前发生且可见**——**每个 pass 边界是全序同步点**。RAW / WAW / WAR 三类跨 pass 冲突全部被该全序裁定，可见性保证仅在 **pass 粒度**给出。
- **实现要求**：该保证由 D3 推导计划兑现；首期取**最保守 sound 同步掩码**（Vulkan：pass 边界 barrier 以覆盖生产/消费全阶段的保守 stage/access 掩码录制；D3D12：legacy ResourceBarrier 语义自含同步）。掩码窄化属性能优化，**不属本条承诺**（§8）。条款措辞与执行器实参须逐字段可对照。
- **pass 内不承诺、不触碰**：pass 内跨线程可见性/内存序仍由既有条款独占管辖——RXS-0079（shared/barrier 一致性）、RXS-0080（scoped atomics）、RXS-0068（barrier uniform 可达性）。本条不新增、不削弱、不重述任何 pass 内语义；pass 内 RT 自读（feedback loop）不可表达（D2 封闭枚举 + 写写冲突拒）。
- **严禁 UB**：本面无 UB 节、无实现自由竞争窗口。承诺面之外的一切构造走编译期诊断（复用）或装配期 6xxx strict 拒（D2）；运行期后端 API 失败走确定性诊断 + 终止 + poisoned 传播（RXS-0193/0194），无静默降级（P-01）。多 queue / async compute / split barrier 不在承诺面（§8），其不存在性即由本条全序措辞封死——条款不为未来扩张预留弱化措辞。

#### D5. 双后端执行器与 present 胶水（→ RXS-0240）

- **(a) D3D12 执行器**（uc04 shim，gate `d3d12-runtime`）：推导产物以 pass 数组 + 逐边界 barrier 数组经 shim ABI **数值透传下发**（shim ABI bump 至 §6.3 时间线对应版；C++ 侧零状态映射逻辑，枚举数值即 D3D12 原生常量，防第二事实源）；执行 = 逐 pass set RT/DSV → draw → 重放该边界 barrier 数组。与步骤 48 offscreen 路径同判据（G-G3-5）。
- **(b) Vulkan 执行器**（rurix-rt `vk.rs`，gate `vulkan`）：新入口 **`run_graph`**（命名律 §4.0-5）多 pass command buffer 录制（逐 pass render pass begin/end + 边界 `vkCmdPipelineBarrier`，layout/stage/access 全取自 D3 同源表），承 RXS-0207/RXS-0210 执行语义地基；现 `run_graphics_offscreen` / `run_graphics_present` 手写定点 barrier 路径 0-byte 保留。新 FFI unsafe 逐块 `// SAFETY:` **折叠进 U27 扩注**（graphics FFI 边界内，0 新号，§6.4/E-2）。
- **(c) present 终端 pass 胶水**：终端 pass 为 `PresentHandoff` 时，graph 只做 pre-present 状态迁移（推导产物之一）并把 backbuffer 交回 **§4.A present 会话（C++ shim present 链，SC-5）**，**不**吸收 present 会话生命周期、不建第二个 present 状态机；窗/泵/交换链维持 C++ shim（**D-130 0-byte 红线**）。与 §4.A 条款（RXS-0220~0222）单向衔接。

#### D6. 手动核验器 = 推导产物独立复核门（双实现互证；→ RXS-0241 后半）

uc04 `barrier::plan_barriers`（RXS-0169 / RX6021，main 已冻结，先于 graph.rs 存在）**永续保留、条款 0-byte 不动**。host 互证金标准（G-G3-5，恒跑纯 host 单测，无 GPU）：uc04 三 pass 图（`deferred::plan_deferred_passes` / RXS-0168 结构）经 graph.rs 推导出的 barrier 集，与 RXS-0169 `required_transitions` 手动锚点集**集合相等断言（双向）**。graph.rs 实现**禁止 import barrier.rs 任何推导逻辑**（oracle 独立性）。

#### D7. 🔒 cabi `rxrt_graph_*` 符号面（→ RXS-0241 前半）

`rxrt_graph_*` 为 RXS-0194 符号面的**只追加**延伸（含义冻结、布局不冻结为语言 ABI，RXS-0180 L3 口径）：拟 `rxrt_graph_create / rxrt_graph_pass / rxrt_graph_declare / rxrt_graph_readback / rxrt_graph_execute / rxrt_graph_destroy`（粒度 Q-G-CabiGranularity）。u64 句柄表、handle-0 = 失败、`diag` 失败行格式、poisoned 传播为跨后端不变式原样生效；既有 `rxrt_* / rxp_* / rxio_*` 符号含义零漂移。

**诊断总表**：编译期 = RX3015 / RX2001 / RX2003 / RX2004 / RX3006 / RX4001·RX4003（全复用，零新码）；装配期 = 新 6xxx ×2（图结构违例族 / 声明-反射失配族，§5 码表预测 RX6029/RX6030，en/zh 成对）；运行期后端失败不占 RX 段位（RXS-0193 口径）。

---

### 4.E mesh-task-RT 章（G3.6，RD-012 + RD-029 兑现；RXS-0242~0249；验收门 G-G3-6）

> 定位：六阶段全量——类型面补齐（intersection/callable + mesh/task 入口契约 + RT payload/attribute 契约升全量 + AccelStruct/trace_ray）、Vulkan 主腿全量（SPIR-V mesh/RT 编码 + 1.4 版本分叉 + vk.rs mesh 管线 / AS / SBT / TraceRays 运行时）、DXIL 腿条件分支（mesh/task probe-first，RT 预判上游 blocked）。承 RD-012（DXIL 侧，RX6008 预留改接）+ RD-029（Vulkan 侧，MB1 defer）。总纪律：不支持面一律编译期诊断（P-01 strict-only），无任何「未定义行为」措辞与运行期静默降级；运行期不可能实现的面走确定性 `Err`（不占 RX 码，RXS-0210 L3 先例）。

#### E1. 阶段全集补齐：`intersection` / `callable`（→ RXS-0242；RXS-0153 修订，不占新号）

- **现状**：`ast.rs` `ShaderStage` 九变体（Vertex/Fragment/Compute/Mesh/Task/RayGen/ClosestHit/AnyHit/Miss），缺 `Intersection`/`Callable`；spec/shader_stages.md RXS-0153 文法行同缺。
- **语法**：沿 RFC-0002 §9 Q1 前缀式，新增 `intersection fn` / `callable fn` 两关键字入 stage 集；文法产生式为 RXS-0153 既有 `<stage>` 备选集的**类别扩充**，以既有条款修订行落地，不开新条款号。
- **着色语义**：与既有九阶段同——kernel 入口着色（非直接可调用、设备上下文体）；PTX 收集根排除维持（D-207）。
- **诊断**：直接调用 / 阶段误用沿既有 RX3xxx 阶段契约通道，不新增文法级错误类别。

#### E2. mesh/task 入口契约（→ RXS-0243）

- **mesh 入口**：`mesh fn` 须携 `#[numthreads(x,y,z)]`（静态字面量）+ `#[outputs(topology = "triangles", max_vertices = N, max_primitives = M)]`。首期拓扑集 **triangles-only**（Q-M-MeshTopology）；缺任一标注 / 未知拓扑 / N、M 非正字面量 → 编译期拒（新码，§5 码表预测 RX3017；RX3011 语义为 I/O 字段标注，不硬套）。
- **mesh 体内已知 API**：`set_mesh_outputs(vertex_count, primitive_count)`（运行值须 ≤ 静态上限，静态可判越界编译期拒；运行期值语义由 `OpSetMeshOutputsEXT` 承载）+ 顶点输出数组写（`#[builtin(position)]` 与 varying 结构比照 vertex 输出）+ 三角形索引输出。**mesh 输出比照 vertex 输出参与 vs-out→fs-in 阶段间契约**——RXS-0155 既有措辞的兑现而非变更。
- **task 入口**：`task fn` 须携 `#[numthreads(x,y,z)]`；体内终结 API `emit_mesh_tasks(x, y, z, payload)`（其后不可达代码 → 编译期拒）。**task payload 契约**：`T` 与下游 `mesh fn` 的 `#[task_payload] p: &T` 形参**编译期逐字段比对**（名/类型/序），错配 → RX3012 类别扩充（RXS-0155 同码同点位）。payload `T` 限 POD 聚合。

#### E3. RT payload / attribute 契约升全量（→ RXS-0244；RXS-0155「保守上界」→ 显式类型契约，落修订行 + 新条款）

- **payload 声明形态**（Q-M-PayloadForm）：closesthit/anyhit/miss 以 `#[payload] p: &mut P` 标注式形参声明（对齐 RFC-0002 §9 Q2 属性式 I/O 先例）；raygen 在 `trace_ray` 调用点以 `&mut P` 实参给定。**编译期比对**：同编译单元内，raygen 的每个 `trace_ray::<P>` 与其可达 closesthit/miss 集的 `#[payload]` 形参类型逐字段一致，错配 → RX3012 类别扩充（raygen↔closesthit/miss 轴）。首期配对域 = **单编译单元 + 单 RT 管线三件套**（raygen×1 + miss×1 + closesthit×1，与步骤 67 语料同构）；多 payload / 多 hit group 的 SBT 序配对越出首期 → 编译期拒（不静默通过，Q-M-PairingDomain）。
- **attribute 契约**：intersection 经 `report_intersection(t, attr)` 产 hit attribute，closesthit/anyhit 以 `#[hit_attribute] a: &A` 消费；比对同上。首期 device 语料不含 intersection（accept-only，§8）；固定三角形几何的内建 attribute（重心坐标 `vec2<f32>`）为已知类型。
- **callable data 契约**：`execute_callable(index, data: &mut D)` ↔ `#[callable_data] d: &mut D`，比对同上；首期 accept-only。
- **RX3012 覆盖面声明（SC-3 采纳）**：RXS-0244 显式声明 RX3012 覆盖的是「**着色阶段间数据契约**」**超集**——task→mesh workgroup payload（非相邻 workgroup 传递）、raygen↔closesthit/miss 经 SBT 的**非相邻**payload 传递、callable data 均纳入其扩类别；其冻结 title 的「**插值限定**」维度对 RT / payload / callable 面**N/A**（仅 mesh 输出→fragment 仍属可复用的相邻 varying 面）。此为**只加类别不改既有语义**（07 §5「语义可加不可改」），非对 RXS-0155「相邻着色阶段类型契约」冻结语义的改派；若实现期判定 payload/attribute 契约错配为独立可达类别，则退回 §5.1 RX3018+ 条件行单列一个 3xxx 码（不预造）。
- **类型面承诺边界**：本契约只承诺**类型等价面**（字段名/类型/序编译期一致性）；payload 在管线间的**字节布局/寄存器承载不属承诺**（由 SPIR-V 存储类降级自然承载，镜像 RXS-0159 🔒 布局禁区口径）。

#### E4. AccelStruct 句柄、`trace_ray` 已知签名、RT builtins（→ RXS-0245）

- **`AccelStruct`**：不透明资源句柄，**仅可作 RT 阶段（raygen 为主）签名形参**；返回位置/结构体字段/非着色阶段签名 → RX3013 类别扩充（RXS-0156 位置纪律同构）。绑定轴 = **SRV**（`OpTypeAccelerationStructureKHR` descriptor），接 RXS-0163/0164 绑定推导新资源类别。
- **`trace_ray` 已知签名**（首期固定，收窄即显式）：`trace_ray(tlas: AccelStruct, origin: vec3<f32>, t_min: f32, dir: vec3<f32>, t_max: f32, payload: &mut P)`。ray flags 恒 opaque、cull mask 恒 0xFF、SBT offset/stride/miss index 恒 0（单三件套管线下唯一确定）；扩展参数越出首期 → 编译期拒。**递归深度恒 1**：`trace_ray` 仅在 `raygen` 上下文合法——**含经调用图可达 closesthit/anyhit/miss/callable 的 device fn 体内亦拒**（coloring 层阶段上下文可达性传播，非单点体内检查；对应语料随实现 PR），把运行期递归上限约束整体前移为编译期结构约束——不存在「越界递归」运行期路径，无需任何 UB 措辞。
- **RT builtins**（阶段×合法性矩阵，阶段不符 → 编译期拒）：`launch_id`/`launch_size`（全 RT 阶段）、`world_ray_origin`/`world_ray_direction`/`ray_t_min`（intersection/anyhit/closesthit/miss）、`hit_t`（anyhit/closesthit）、`primitive_index`/`instance_id`（intersection/anyhit/closesthit）、`hit_kind`（anyhit/closesthit）。命名沿 compute builtins snake_case 谱系（RXS-0202 体例）。独立可达新类别 → §5 码表条件行（RX3018+）。
- **anyhit 调用次数措辞纪律**：对同一 ray 的调用次数与序为**实现定义但有界**（Vulkan/DXR 双规范一致的遍历自由度），条款如实登记该自由度，**不**写成「未定义」。

#### E5. MIR→SPIR-V mesh/task 编码（→ RXS-0246）

- **执行模型**：`OpEntryPoint MeshEXT/TaskEXT` + capability `MeshShadingEXT` + `OpExtension "SPV_EXT_mesh_shader"`。
- **编码器落点（E-4 采纳，钉死）**：仓内存在**两个** SPIR-V 发射器——`dxil_spirv.rs`（vertex/fragment，围绕 I/O 签名 + 直线体降级 RXS-0171）与 `vulkan_codegen.rs`（GLCompute，`EXEC_MODEL_GLCOMPUTE`，已含 LocalSize/RuntimeArray/DescriptorSet 机制）。mesh/task 为 **workgroup 语义**（`#[numthreads]`/LocalSize/`TaskPayloadWorkgroupEXT`），与 `vulkan_codegen.rs` 的 compute 基建共性更大——**mesh/task 编码复用 `vulkan_codegen.rs` GLCompute/LocalSize/workgroup 基建**，不在 `dxil_spirv.rs` 重复实现 LocalSize/workgroup；mesh 的 vertex-out→fs-in I/O 装饰面（Position/varying Location）承 `dxil_spirv.rs` 既有装饰机制。**零漂移门显式跨两个发射器 golden 集界定**（`dxil_spirv.rs` 变；`vulkan_codegen.rs` 既有 GLCompute golden 不动 = 平凡安全，见 §6.5 PR-Mb/PR-Mc）。
- **execution modes**：`LocalSize`（承 `#[numthreads]`）+ `OutputVertices N` + `OutputPrimitivesEXT M` + `OutputTrianglesEXT`（承 `#[outputs]`）。
- **mesh 输出**：`Position` builtin 数组 + varying Location 数组（Output 存储类，复用既有装饰机制）+ `PrimitiveTriangleIndicesEXT`；`set_mesh_outputs` → `OpSetMeshOutputsEXT`。
- **task→mesh payload**：`TaskPayloadWorkgroupEXT` 存储类变量；`emit_mesh_tasks` → `OpEmitMeshTasksEXT`。
- **SPIR-V 版本**：mesh 入口随 E6 版本分叉走 1.4 口径 emit（interface 全量枚举）；精确最低合规版本以 spirv-val 实测核定（measured-first，不背书未实测的版本下限）。全部产物过 spirv-val 三态 gate（承 RXS-0212）。

#### E6. MIR→SPIR-V RT 六执行模型编码 + SPIR-V 1.4 per-entry 分叉（→ RXS-0247）

- **六执行模型**：`RayGenerationKHR` / `IntersectionKHR` / `AnyHitKHR` / `ClosestHitKHR` / `MissKHR` / `CallableKHR`，capability `RayTracingKHR` + `OpExtension "SPV_KHR_ray_tracing"`。
- **存储类**：`RayPayloadKHR` / `IncomingRayPayloadKHR` / `HitAttributeKHR` / `CallableDataKHR` / `IncomingCallableDataKHR`；`ShaderRecordBufferKHR` 不进首期（§8）。
- **指令族**：`OpTraceRayKHR` / `OpReportIntersectionKHR` / `OpIgnoreIntersectionKHR` / `OpTerminateRayKHR` / `OpExecuteCallableKHR`；`OpTypeAccelerationStructureKHR` + descriptor 装饰（承 RXS-0163 推导新类别）。
- **1.4 分叉（独立 PR，硬边界）**：RT 腿硬性要求 SPIR-V 1.4（`VK_KHR_ray_tracing_pipeline` 依赖 `VK_KHR_spirv_1_4`）；1.4 起 `OpEntryPoint` interface 须枚举**全部**被引用全局变量。分叉形态 = **per-entry 版本轴**（Q-M-SpirvVersion）：emitter header 版本字参数化为按入口选择——mesh/RT 入口 emit 1.4 + interface 全量；**既有 compute/vertex/fragment 入口维持 1.0 emit，产物字节零漂移**（既有 vulkan golden 不重 bless、DXIL B 路消费的 SPIR-V 字节不变）。零回归门：dxil 套件恒定 + vulkan 既有 golden 字节 diff 空 + `spirv-val --target-env vulkan1.2` 双口径皆 accept。
- **校验轴**：合规判定以 spirv-val 退出码为准，**不以驱动宽容度为准**（NVIDIA 驱动可能接受不合规组合，不得据此免分叉）。

#### E7. 🔒 Vulkan mesh 管线运行时（→ RXS-0248 前半；vk.rs 扩展）

- device 扩展 `VK_EXT_mesh_shader` + feature chain `VkPhysicalDeviceMeshShaderFeaturesEXT` 探测；**缺失 → 确定性 `Err`**（非 panic、不占 RX 码、无 fallback，RXS-0210 L3 纪律）。
- 新入口 `vk::run_mesh_offscreen(task_spv?, mesh_spv, fs_spv, W, H, clear, group_counts)`（命名律 §4.0-5）：graphics pipeline **无 vertex input / input assembly state**，stage 集 = (task?)+mesh+fragment；录制 `vkCmdDrawMeshTasksEXT(x,y,z)`；offscreen render pass + 回读像素——与 `run_graphics_offscreen`（U27）同构复用其骨架。
- FFI 纪律沿 U26/U27 审计模式（手写薄 loader、运行时解析零链接期符号、`#[repr(C)]` 逐字节对齐、句柄线性配对逆序销毁、`// SAFETY:` 单块单操作）；**mesh 管线 unsafe 折叠进 U27 扩注**（graphics FFI 边界内，0 新号，§6.4/E-2）。validation messenger fail-closed 承 RXS-0210。

#### E8. 🔒 AS 构建 / SBT / TraceRays 运行时（→ RXS-0248 后半）

- device 扩展集：`VK_KHR_acceleration_structure` + `VK_KHR_ray_tracing_pipeline` + `VK_KHR_deferred_host_operations` + bufferDeviceAddress（core 1.2 feature）；探测缺失 → 确定性 `Err`。
- **BLAS/TLAS**：单三角形 geometry → `vkGetAccelerationStructureBuildSizesKHR` 定尺寸 → device-local buffer（usage 含 `ACCELERATION_STRUCTURE_STORAGE` / `SHADER_DEVICE_ADDRESS`）→ `vkCmdBuildAccelerationStructuresKHR`（BLAS→barrier→TLAS 两段，单 queue 全序）；TLAS instance 经 `vkGetAccelerationStructureDeviceAddressKHR` 取 BLAS 地址。🔒 device address 为原始 GPU 指针面，**切出细审计 U30**（AS/SBT/device-address 全期唯一新 U 号，§6.4/E-2）。
- **RT 管线**：`vkCreateRayTracingPipelinesKHR`，shader groups = raygen(GENERAL) + miss(GENERAL) + hit(TRIANGLES_HIT_GROUP, closesthit)；`maxPipelineRayRecursionDepth = 1`（与 E4 编译期递归约束同源）。
- **🔒 SBT**：`vkGetRayTracingShaderGroupHandlesKHR` 取 handles；按 handleSize/handleAlignment/baseAlignment 对齐铺三 region（raygen/miss/hit 各单条目）；host-visible buffer + device address 填 `VkStridedDeviceAddressRegionKHR`。SBT 内**不嵌用户数据**（§8）；对齐算术为纯 host 可单测面（镜像 RXS-0210 协商 helper 先例）。
- **执行**：descriptor 布局沿 §4.B7 Vk-native set-per-class 形态（TLAS=SRV 轴、storage image=UAV 轴，§4.0-4）→ `vkCmdTraceRaysKHR(W,H,1)` → 回读像素。新入口 `vk::run_ray_tracing_offscreen(raygen_spv, miss_spv, chit_spv, vertices, W, H)`。
- **前置 probe**：`bin/vk_rt_probe`（AS build sizes/对齐、SBT stride、bufferDeviceAddress 驱动坑实测，证据先行入 evidence/，G3_PLAN §7）。

#### E9. DXIL 腿条件分支（→ RXS-0249；probe-first；RD-012 承接 + RX6008 改接）

- **现状锚点**：spec/dxil_backend.md RXS-0158 对应表已完整登记 mesh→`mesh`(SM6.5)/task→`amplification`(SM6.5)/RT→`library`(SM6.3) 映射但零实现；降级请求现沿 RXS-0157 RX6007 通道拒；**RX6008 为 RD-012 预留专用码**（registry 留痕，尚无正式条目）；D-131 裁定图形=B 链。
- **mesh/task 分支（probe-gated，§4.0-8）**：probe = 最小 mesh/task SPIR-V（E5 产物，含非空输出单三角形写出）→ `spirv-cross --hlsl`（mesh 支持度未实测，不预判）→ `dxc -T ms_6_5 / as_6_5` → dxv 签名门；证据 JSON + 报告入 evidence/。**绿** → mesh/task DXIL 全量落地（B 链贯通 + RXS-0158 表状态列修订）+ **RX6008 正式落 registry 并改接**（stage_deferred 拒改真降级；仍不支持的 RT 阶段拒绝码由 RX6007 改派 RX6008，语义=「DXIL 阶段降级不可用」，只加类别不改既有语义）+ CI 步骤 68 真实红绿。**红** → mesh/task 与 RT 同落 RD-034+ 尾门，probe 以 blocked 探针形态入 CI；RX6008 改接仍执行（拒绝通道自 RX6007 迁至专用码，RD-012 码位部分兑现，降级实现分量收窄留痕不伪 close）。
- **RT 分支（预判 blocked，以探针证据落地非以预判落地）**：双重上游钳制——① B 链：spirv-cross HLSL 后端无 `SPV_KHR_ray_tracing` 消费路径；② A 路：RT 为 DXIL library 多入口形态且 A-graphics 签名钳制未解（RD-015 open，上游 #90504/#57928）。处置 = **步骤 69 blocked 探针**：CI 恒跑最小 raygen SPIR-V → spirv-cross HLSL，**预期失败 = 探针 PASS（blocked 证据新鲜）**；上游某日翻绿 → 探针「意外成功」翻红提醒复评（对齐 RD-011/RD-015 跟踪纪律，防静默腐烂）。RT DXIL 全量登 RD-034+ 尾门，照 G-MB1-6 措辞越过 close-out 存续。

#### E10. 诊断汇总（全部编译期 strict-only；运行期缺能力 = 确定性 Err 不占码）

| 违例类别 | 层 | 通道 |
|---|---|---|
| 阶段误用/直接调用/`trace_ray` 越出 raygen 可达域/`emit_mesh_tasks` 后不可达 | typeck/coloring | 既有 RX3xxx 阶段契约通道扩类别 |
| mesh/task 入口标注缺失/非法 | typeck | 新码（§5 码表预测 RX3017） |
| task payload / RT payload / hit attribute / callable data 契约错配 | typeck | RX3012 类别扩充 |
| AccelStruct 位置违例 / `trace_ray` 签名违例 / builtin 阶段不符 | typeck | RX3013 类别扩充 + 独立可达类别新码（§5 码表条件行 RX3018+） |
| Vulkan 腿子集外构造 / SPIR-V emit / spirv-val 拒 | codegen | RX6026 类别扩充（RXS-0200 L2/L3 既有语义面） |
| DXIL 腿不支持阶段降级请求 | codegen | **RX6008 改接**（RD-012 预留码正式落 registry，introduced_in G3.6；RX6009：RD-013 已 close（2026-06-29，RXS-0171 body lowering 落地，未 materialize）→ 随 §6.5 PR-Me 的 error_codes.json errata 正式 burn，10 §9.5，SC-1） |
| 运行期 feature/扩展缺失（mesh shader / RT / BDA） | runtime | 确定性 `Err`，不占 RX 码（RXS-0210 L3 先例） |

## 5. 下游 spec 条款映射（spec diff，10 §3 要件）

自 **RXS-0220** 起续号（main 现最高 RXS-0219 @ spec/release.md；契约 §7 v1.1 区间分配；溢出自 RXS-0270 顺续，0250~0269 = EI1 earmark）。**条款先行**（硬规则 7）：每 PR 条款 commit 先于实现 commit；每条 ≥1 `//@ spec:` 锚定；trace_matrix 全程全锚定（N→N+Δ，以各 PR 合入时再生实测为准）；stable 快照加性重 bless 同 PR + bless_log 同 diff（步骤 49 硬红不可分 PR）；各落点文件修订表沿「表头『版本』列名、数据行用『版号』」纪律。RXS-0153/0155/0158/**0163~0165**/0174/0210 走既有条款加性修订行不占新号（**RXS-0164（+0163/0165）落修订行把「unbounded→Unmappable」收窄为「非 bindless-SRV unbounded」、「首期单 space0」收窄为「有界 space0 / bindless 自 space（类别）」，前向引用 RXS-0231/0233，消解 spec/binding_layout.md 单一事实源「unbounded→Unmappable / 单 space0」与 RXS-0233「unbounded SRV 合法 / space1+」自相矛盾，SC-2**）。

| 条款 | 章 | 标题 | 落点 spec 文件 | 要点（摘） | 测试锚定计划（每条 ≥1） |
|---|---|---|---|---|---|
| RXS-0220 | A | UC-04 可见窗口 flip-model swapchain present 装配与呈现循环 | spec/d3d12_runtime.md | FLIP_DISCARD 恒定；BufferCount∈{2,3}；Present(interval∈{0,1}) + tearing 参数面；逐帧迁移锚点 RT→COPY_SOURCE→PRESENT（RXS-0169 手动口径）；装配违例 strict-only（预测 RX6027）；RXS-0197 typestate 维持不动（UC-04 窗口 present 独立走 shim，SC-5） | uc04-demo 装配核验单测（accept/reject）+ ci/uc04_present_smoke.py device N 帧逐帧 S_OK |
| RXS-0221 | A | swapchain 失效与重建语义（D3D12 ResizeBuffers / Vulkan OUT_OF_DATE·SUBOPTIMAL） | spec/d3d12_runtime.md（单条承载；vulkan_backend.md RXS-0210 加性修订行引用） | 失效=正常路径；重建序 idle→释放→重建→首帧再校验；重建核验失败（预测 RX6028，可与 0220 合码） | resize 重建单测 + vk.rs 重建协商 host 单测 + 步骤 61「resize 后再 readback 绿」段 |
| RXS-0222 | A | present headless readback 校验与 SKIP 纪律 | spec/d3d12_runtime.md | readback=必要 device 证据（W6，三断言点）；布局复用 RXS-0170/RX6022；SKIP=dev-env degrade + REQUIRE_REAL 硬红；不占 RX 码；步骤 48 硬门不替代 | readback 断言单测 + SKIP/REQUIRE_REAL 三态 + red_self_test（篡改 PRESENT 迁移） |
| RXS-0223 | B | 采样方法族类型面（`sample` 隐式化 + SamplerCmp/TextureRw2D；TextureRw2D 阶段 = fragment+raygen） | spec/shader_stages.md | §4.B1 签名/阶段表；句柄纪律承 RXS-0156/0175 L4；元素 F 分方法限定；违例 RX3014 扩类别；RXS-0174 修订行记语义升级 | conformance/shader reject/accept + UI golden |
| RXS-0224 | B | 静态 sampler 属性 `#[sampler(...)]` | spec/shader_stages.md | 状态空间枚举/常量折叠；RTS0 static sampler / Vk immutable；s 轴共序（RXS-0164）；非法键值 strict 拒 | accept/reject 语料 + binding_layout 单测（NumStaticSamplers>0） |
| RXS-0225 | B | 宿主 SamplerDesc 状态面 | spec/host_orchestration.md | 同一状态空间宿主对象；compare 四值；aniso 探测确定性 Err；零新语法 | cabi/shim 单测 + 步骤 63 wrap-vs-clamp 对照 |
| RXS-0226 | B | 采样超集降级 opcode 全家（B 链贯通；条件分支子模式） | spec/dxil_backend.md | §4.B6 表全量；显式 format storage image；dxv/签名门不旁路；子集外 RX6023 扩类别；SampleCmp/Gather probe；PTX D-207 事实标注 | dxil graphics accept golden + reject→RX6023 + probe 证据 evidence/ |
| RXS-0227 | B | 🔒 隐式 LOD / quad 导数语义（06 §4.2 增补） | spec/dxil_backend.md | 继承 D3D/Vulkan quad 语义不自有发明；uniform 控制流合法性（首期 by-construction；控制流落地后词法保守拒，条件条款）；无 UB 节 | 步骤 63 模式①（mip 金字塔）+ accept 语料 |
| RXS-0228 | B | 🔒 texel fetch 坐标空间与越界钳制（06 §4.2 增补） | spec/dxil_backend.md | 整型非归一化坐标；codegen 注入 clamp；双后端同源确定性；零 feature 依赖 | 步骤 63 模式④ + codegen 单测（钳制序列存在） |
| RXS-0229 | B | 🔒 storage image 写 + 唯一写者纪律（06 §4.2 禁区结构回避） | spec/dxil_backend.md | 32-bit 字粒度；**唯一写者纪律**（identity 坐标 by-construction 每 texel 单写者，typeck/codegen 强制、可 golden）；**无竞写即无 race、无 UB 节**；可见性唯 pass 边界 barrier（首期 RXS-0169 手动路，§4.0-3）；**可竞写模式 + image atomics 登 RD-034+ 另 Full RFC** | 步骤 63 模式⑧（唯一写者 store→barrier→回读红绿 + 多写者编译期拒 golden）+ 类型面 reject 经 RXS-0223 锚 |
| RXS-0230 | B | Vulkan graphics descriptor 运行时建面（`run_graphics_offscreen_v2` 加性） | spec/vulkan_backend.md | v1 0-byte；v2 资源/immutable sampler/mip/storage image；装饰参数化（单一 binding-号事实源 + 两套 set 分配策略，B 链字节不动 + Vk set-per-class 0=CBV/1=SRV/2=UAV/3=Sampler）；U27 扩注（§6.4）；SKIP 三态 | vk.rs 单测 + 步骤 63 Vulkan 腿同判据 + v1 语料回归 + **混合有界+无界 B 链字节 diff golden**（合入门，E-3） |
| RXS-0231 | C | 无界资源句柄数组类型面（`[Texture2D<F>]` 仅签名形参） | spec/shader_stages.md | 切片样式文法无新 token；仅着色签名形参位；违例 RX3013 扩类别；句柄非值承 RXS-0156；首期仅 SRV 纹理 | conformance/shader reject（位置违例）+ accept + UI snapshot |
| RXS-0232 | C | 动态索引表达式与 `nonuniform` 标注（strict-only） | spec/shader_stages.md | 临时句柄仅立即 receiver（承 RXS-0174）；字面量常量豁免；缺失 → 新码拒（预测 RX3016）；逃逸并入 RX3014 扩类别 | reject（nonuniform_missing / handle_escape）+ accept 动态索引 0 诊断 |
| RXS-0233 | C | 无界数组绑定推导合法化与独占 set/space 分配律（+**RXS-0164（+0163/0165）加性修订行**，SC-2） | spec/binding_layout.md | Unbounded 自 Unmappable（RX6013）翻转；**落 RXS-0164 修订行收窄 unbounded→Unmappable 为「非 bindless-SRV unbounded」、单 space0 为「有界 space0 / bindless 自 space」，前向引用 RXS-0231/0233**；Vk-native 独占 set 自 **set4**（§4.0-1）、RTS0 独占 space 自 space1、按声明序；B 链装饰字节不动；有界路零漂移；非纹理无界维持 RX6013；🔒 数值非 stable | binding_layout 单测（accept 混合有界+无界确定性 / reject 无界 Sampler）+ 既有回归网 + serialize_rts0 字节确定性 + **RXS-0164 修订行核** |
| RXS-0234 | C | 🔒 descriptor indexing codegen 降级与越界有界性（双腿） | spec/dxil_backend.md | OpTypeRuntimeArray + RuntimeDescriptorArray/ShaderNonUniform + NonUniform 装饰（Vk1.2 core）；强制 clamp 至注册计数 = 实现定义但有界，无 UB 节；B 链 SM6.0 probe-first，红→RD-034+ 条件分支 | golden（spirv-val vulkan1.2 + B 链 dxv）+ codegen 单测（NonUniform/clamp 发射）+ probe 证据 JSON |
| RXS-0235 | C | std::gpu `TextureTable` 宿主注册面与运行时 feature chain | spec/host_orchestration.md | affine 承 RXS-0189；注册序即索引稳定单调；**格式擦除**（元素错配=运行期确定性 Err）；`rxrt_table_*` 只追加（RXS-0194）；四 feature bit 缺失确定性 Err；kernel 体内 RX3015 | accept/reject（move 后再用 / kernel 体内）+ feature-probe 单测（mock 缺 bit → Err）+ 步骤 64 device 段 |
| RXS-0236 | D | Graph/Pass 宿主库类型面与访问声明 | spec/render_graph.md（新建） | lang-item 已知签名（RXS-0189/0190）；五类访问封闭枚举 + PresentHandoff；affine 复用零新借用码；RX3015 同点位 | accept graph_deferred_three_pass（0 诊断）+ reject graph_in_kernel（RX3015） |
| RXS-0237 | D | 声明序=提交序与图合法性 | spec/render_graph.md | 声明全序无重排；环（use-before-write 可达形态）/写写冲突/生命周期违例（预测 RX6029）；声明-反射**双向精确相等，域=首期封闭枚举资源面**（预测 RX6030） | graph.rs reject 单测 ×4 族 + conformance reject graph_undeclared_read |
| RXS-0238 | D | 自动资源状态推导状态机 | spec/render_graph.md | 三 barrier 形态（Transition/BufferSync/UavSync）；推导纯函数确定性；AccessKind 双后端映射表单一事实源 | 推导计划 golden 单测 + 同图双跑逐字节等值断言 |
| RXS-0239 | D | 🔒 pass 边界 happens-before 语义本体 | spec/render_graph.md | 单 queue 全序同步点；RAW/WAW/WAR 仅 pass 粒度；保守掩码实现要求；pass 内归 RXS-0068/0079/0080 不触；严禁 UB 节 | D6 互证恒跑单测 + 步骤 65 device 数据流红绿（漏声明 read → strict 拒 RED） |
| RXS-0240 | D | 双后端 barrier 映射与执行器语义（`run_graph`） | spec/render_graph.md | 同源表逐字重放禁二次推导；D3D12 shim 数值透传 / Vulkan vkCmdPipelineBarrier；present 胶水交 RXS-0197 链（D-130 0-byte）；既有手写 barrier 路零回归 | 步骤 65（uc04 迁 Graph 重跑步骤 48 同判据 + Vulkan 同图）+ 映射一致性单测 |
| RXS-0241 | D | `rxrt_graph_*` C ABI 与手动复核门（🔒 FFI 延伸） | spec/render_graph.md | RXS-0194 只追加纪律；u64 句柄/handle-0/diag/poisoned 不变式；RXS-0169 `plan_barriers` 永续独立复核门，条款 0-byte | cabi 单测（符号面 + 句柄失败路）+ 互证 set-equality 恒跑单测 |
| RXS-0242 | E | 着色阶段全集：intersection/callable 与 RT 六阶段着色规则（+RXS-0153 修订行） | spec/shader_stages.md | 前缀式 `intersection fn`/`callable fn` 入 stage 集；kernel 入口着色/coloring 复用；PTX 收集根排除维持（D-207）；误用编译期拒 | accept rt_stages_full + reject intersection_direct_call |
| RXS-0243 | E | mesh/task 入口契约（#[numthreads] + #[outputs] + task payload 契约） | spec/shader_stages.md | 标注必备/静态字面量/triangles-only（缺失非法 → 预测 RX3017）；`set_mesh_outputs`/`emit_mesh_tasks` 已知签名；payload 逐字段比对（RX3012 扩类别） | accept mesh_task_entry + reject（missing_numthreads / bad_topology / task_payload_mismatch） |
| RXS-0244 | E | RT payload/attribute/callable data 显式类型契约（+RXS-0155 修订行） | spec/shader_stages.md | `#[payload]`/`#[hit_attribute]`/`#[callable_data]` 标注式形参；trace_ray 调用点比对；单编译单元三件套配对域；字节布局非承诺（🔒 边界声明）；**RX3012 声明覆盖「阶段间数据契约」超集、「插值限定」维度对 RT/payload/callable N/A（只加类别不改语义，SC-3）** | accept rt_payload_pair + reject rt_payload_mismatch + shader_stages.rs 单测 |
| RXS-0245 | E | AccelStruct 句柄、trace_ray 已知签名与 RT builtins 类型面 | spec/shader_stages.md | AccelStruct 仅 RT 签名形参（SRV 轴承 RXS-0156/0163）；固定签名 + raygen 可达域内合法（递归恒 1 编译期，含调用图传播）；builtins 阶段矩阵（独立类别 → 预测 RX3018+） | accept rt_trace_min + reject（accelstruct_return / trace_in_closesthit） |
| RXS-0246 | E | MIR→SPIR-V mesh/task 编码（MeshEXT/TaskEXT + SPV_EXT_mesh_shader） | spec/vulkan_backend.md | 执行模型/LocalSize/OutputVertices/OutputPrimitivesEXT/OutputTrianglesEXT；OpSetMeshOutputsEXT/OpEmitMeshTasksEXT；TaskPayloadWorkgroupEXT；**编码复用 vulkan_codegen.rs GLCompute/LocalSize 基建（E-4）**；spirv-val 三态 | accept vk_mesh_tri / vk_task_payload（spirv-val accept）+ emit 单测 + **跨两发射器零漂移 golden** |
| RXS-0247 | E | MIR→SPIR-V RT 六执行模型编码 + SPIR-V 1.4 per-entry 分叉 | spec/vulkan_backend.md | 六模型 + SPV_KHR_ray_tracing + 存储类族 + OpTraceRayKHR 族 + OpTypeAccelerationStructureKHR；mesh/RT 入口 1.4 + interface 全量；1.0 路字节零漂移 | accept vk_{raygen,miss,closesthit} + 1.0 路零回归单测（既有 golden 字节 diff 空） |
| RXS-0248 | E | 🔒 Vulkan mesh/RT 运行时执行语义（mesh pipeline + BLAS/TLAS + SBT + TraceRays） | spec/vulkan_backend.md | feature 缺失确定性 Err；mesh 管线无 vertex-input（unsafe 折叠 U27）；AS 两段构建 + device address；SBT 三 region 对齐律（🔒）；descriptor 布局沿 §4.B7 set-per-class（§4.0-4）；AS/SBT/device-address 切细审计 U30（§6.4/E-2）；validation fail-closed | host 单测（SBT 对齐/协商 helper）+ bin/vk_mesh、bin/vk_rt device 真跑（步骤 66/67 像素断言） |
| RXS-0249 | E | DXIL 腿条件分支：mesh/task probe-gated + RT blocked 探针 + RX6008 改接（+RXS-0158 表修订行） | spec/dxil_backend.md | probe 证据先行（spirv-cross→dxc ms_6_5/as_6_5→dxv）；绿=全量、红=RD-034+ 尾门；RT blocked 探针「意外绿翻红」防腐烂；拒绝通道 RX6007→RX6008 改接（RD-012） | reject 语料迁 RX6008 ± probe 绿时 accept ms_*.rx + probe 证据 evidence/ + 步骤 68/69 |

### 5.1 新错误码总分配表（预测；合并时以 registry 实号为准）

**前提**：6xxx 段自 **RX6027** 续（main 现最高 RX6026）；3xxx 段自 **RX3016** 续（main 现最高 RX3015）——**3xxx 续号 claim 为契约 v1.0 ⑥ 缺失项，须随 number_ledger 校准补**（契约仅 claim 了 RX6027 与 RX7023）。**本表为预测**：条件码按「不预留不预造」纪律不预分配，materialize 时以合并时 registry 实号为准（先合入面的条件码落地会使后续预测号右移）；en/zh message-key 成对（bilingual 门）；registry/error_codes.json 只追加。

| 预测号 | 章 | 类别（归属场景） | 状态 |
|---|---|---|---|
| RX6027 | A present | present 装配核验失败（swapchain↔RT 失配 / blt-model 请求 / 缺 PRESENT 态迁移锚点，§4.A5） | 确定 |
| RX6028 | A present | resize/重建核验失败（重建后格式/缓冲数漂移、视图未重建，§4.A5） | 确定（Q-P-CodeGranularity 合码则撤，后续预测号左移） |
| RX6029 | D graph | 图结构违例族（装配期：环/读未写/写写冲突/生命周期误用，§4.D2） | 确定 |
| RX6030 | D graph | 声明-反射失配族（装配期：漏声明/声明未用，§4.D2） | 确定 |
| RX6031+ | B sampling | storage image 格式-元素失配（若实现判定独立可达类别，§4.B6） | 条件 |
| RX3016 | C bindless | `nonuniform` 标注缺失（§4.C1） | 确定 |
| RX3017 | E mesh-RT | mesh/task 入口标注缺失/非法（§4.E2） | 确定 |
| RX3018+ | E mesh-RT | RT 已知签名/builtin 轴独立类别（视实现可达，§4.E4） | 条件 |
| （顺延） | B sampling | `#[sampler]` 非法键值独立类别（默认并入 RX3014 扩类别，§4.B2） | 条件 |
| （顺延） | C bindless | 无界数组位置违例独立类别（默认复用 RX3013 扩类别；若评审否决复用则新码，§4.C1） | 条件 |

- 合计：确定 6xxx ×4 + 3xxx ×2 = **6**（present 合码则 5）；含全部条件上限 **10**。
- **RX6008 = RD-012 预留码改接，单列不占新号**（§4.E9）：G3.6 正式落 registry 条目（introduced_in G3.6，语义「DXIL 阶段降级不可用」）；probe 绿的阶段从其拒绝集移除转真降级，blocked 阶段维持在拒绝集（只改集合成员不改码语义，Q-M-RX6008Scope）。
- **RX6009 处置（SC-1 采纳，跨 registry 一致性）**：权威 error_codes.json revision_log 现仍记「RX6009 留给 RD-013」，但 RD-013 已于 **2026-06-29 经 G-G2-4 close**（实现 RXS-0171 body lowering，从未 materialize RX6009）——**随 §6.5 PR-Me 的 error_codes.json errata 正式 burn**（10 §9.5 永不复用），使权威 registry 与 number_ledger/本 RFC 口径统一；PR-Me 的 errata 同时登记 RX6009 burn + RX6008 改接（见 §6.5）。
- RX7023 工具段：五章零消费。
- 复用扩类别（零新号）：RX3012/RX3013/RX3014/RX3015/RX6013/RX6023/RX6026 + RX2001/RX2003/RX2004/RX3006/RX4001/RX4003；语义可加不可改。
- 运行期/环境失败（Present 环境态、Vulkan feature 缺失、surface 建失败、SKIP）一律**不占 RX 码**（06 §8.2 / RXS-0193 / spec/release.md §3 口径）。

## 6. feature gate / tracking / 实现序（10 §3 要件）

### 6.1 前置与失败测试先行

- 本 RFC 合入 gated on **G-G3-1 归因开闸**（spike 期间零面 RFC 合入）；**RFC-0013 Approved 合入 = 五面「RFC 前置」一次性满足**（契约 §7 v1.1 伞形执行语义）。
- **失败测试先行**（反 YAML-only）：RFC 合入时点，`ci/uc04_present_smoke.py`（步骤 61）、步骤 62/63 采样脚本、`ci/bindless_smoke.py`（步骤 64）、`ci/render_graph_smoke.py`（步骤 65）、步骤 66/67 mesh/RT 脚本、`src/rurix-rt/src/graph.rs`、`spec/render_graph.md`、RXS-0220~0249 条款体在 main **均不存在 = RED**；各面实现 PR 落地转绿（脚本名为拟名，随实现 PR 定案）。
- **CI 步骤口径统一**：步骤 61~67（±68/69）已在 milestones/g3/CI_GATES.md v1.1 预登记为计划项，**workflow YAML 随各实现 PR 回填**实测命令与 run URL，数量不预占。

### 6.2 feature gate 总裁决：零新 cargo feature、零语言 gate

五面全部复用既有 gate，不暴露新用户组合维度（RFC-0005/0007/0009 复用先例；mesh/RT 的 Q-M-FeatureGate 同裁）：

| 面 | 类型面 | codegen | 运行时 |
|---|---|---|---|
| present | —（零新语法） | — | uc04-demo `d3d12-runtime` + rurix-rt `vulkan`（default off） |
| 采样超集 | `shader-stages` | `dxil-backend` + `vulkan-backend` | `vulkan` + shim |
| bindless | `shader-stages` | `dxil-backend` + `vulkan-backend` | `vulkan` + shim；TextureTable 为加性 stdlib 面无独立 gate |
| render graph | —（lang-item 宿主库） | — | `graph.rs` always-on 纯 host safe；执行器 `d3d12-runtime` / `vulkan` |
| mesh-task-RT | `shader-stages` | `vulkan-backend`（+`dxil-backend` probe 绿臂） | `vulkan` |

默认构建（全 feature off）零 GPU/SDK 依赖绿；CUDA 路零回归。

### 6.3 uc04 shim ABI 版本时间线（预测，§4.0-7；每入口独立版本常量，§4.A4/E-1）

按 G3.2→G3.5 合并序，聚合 ABI 版本单调递增，**各面能力一律经新增独立入口承载**（非扩既有入口签名）：**v3 = present**（`rx_uc04_present_run` 加性入口，§4.A4）→ **v4 = bindless**（SRV descriptor heap 段独立入口，§4.C4）→ **v5 = render graph**（pass/barrier 数组下发入口，§4.D5(a)）。**采样面 shim 增量（sampler heap / mip 链上传 / UAV barrier）走新增独立入口**（如 `rx_uc04_offscreen_v2` / 专用 sampler 入口，**SC-6 采纳**）**而非扩展 `rx_uc04_offscreen_run` 参数面**——既有 offscreen 入口签名 / 函数体 / 自有版本常量（==2）字节不动（步骤 48 0-byte 守卫，§6.6），采样能力另占入口 + 聚合版顺延（消解「不 bump 却扩 offscreen 参数面」与 §6.6/§4.A4 offscreen 0-byte 不变量的冲突）。各入口独立版本常量 `>=` 语义使既有调用方零重编译（加性 / 前向兼容）；聚合 ABI 版号（`rx_uc04_abi_version()` 返回值）以合并时 shim 实测为准，随入口增补变化（非语义冻结），正文各处不写死。

### 6.4 unsafe-audit U 号消费（折叠约定，E-2 采纳）

**审计粒度沿用 U26/U27 折叠约定**（`unsafe-audit/rurix-rt.md` 实录：U26 = **整个** Vulkan compute FFI 边界单 U；U27 = **整个** graphics 边界单 U，W6 win32 / W7 Android present 均折叠进 U27 无新号）——据此，G3 绝大部分 Vulkan 工作（present 重建 / 采样 descriptor 建面 / bindless feature chain + UPDATE_AFTER_BIND pool / graph 多 pass 执行器 / mesh 管线 `vkCmdDrawMeshTasksEXT`）均在既有 `run_graphics_*` FFI 边界内，**折叠进 U27 扩注 = 0 个新号**（既有 SAFETY 边界扩充，非新审计单元）。**唯一切出的新细审计 U 号 = AS 构建 / SBT 布局 / device-address**（`vkGetAccelerationStructureDeviceAddressKHR` 原始 GPU 指针算术 + SBT 对齐铺设）——该面是全期最高危 unsafe 面（裸指针 / 对齐），风险要求独立细审计，**自 U30**（U29 = EA1 预留显式跳让不回收）。即 G3 **净新增 1 个** U 号（U30 = AS/SBT/device-address），以 unsafe-audit ledger 合并时实号为准，任何章不写死具体号。**停止「沿用 U26/U27 折叠」与「预测 U30~U35 六个」并存的矛盾**；number_ledger claim 措辞（RFC 内引用处 §4.0-9 / §4.E8）同步校准为「G3 净消费 U30 一个（AS/SBT/device-address 细审计），余折叠 U27 扩注」。

### 6.5 栈式 PR 计划（G3.2→G3.6 严格串行合入；每 PR 条款 commit 先行 + 实现同 PR，EA1 #158/#159 结构先例）

**G3.2 present（步骤 61）**
1. **PR-P1**（条款+实现）：spec/d3d12_runtime.md 追加 RXS-0220~0222 + 修订行 + spec/README §4 区间登记 + vulkan_backend.md RXS-0210 加性修订行 → uc04 shim present 段（ABI v3）+ present/resize 装配核验（新码落码 en/zh）+ vk.rs OUT_OF_DATE/SUBOPTIMAL 重建收尾 + `//@ spec` 锚定 ×3 + trace 再生 + 快照重 bless。
2. **PR-P2**（CI 接线，可与 P1 合并为单 PR 三 commit）：步骤 61 `ci/uc04_present_smoke.py`（host 段恒跑装配核验；device 段 N 帧逐帧成功 + 首/重建后/末三点 readback + 内建 red_self_test；无显示 SKIP + REQUIRE_REAL 硬红）+ evidence JSON/schema + `g3.counter.uc04_present_frames` 与 budget_eval evaluator 分支**同 PR**（未知 id 强制 FAIL 纪律）。

**G3.3 采样超集（步骤 62/63；关键路径，descriptor 底座）**
3. **PR-S0（Vulkan descriptor 底座，独立先落地，四面共同前置；E-5 采纳）**：RXS-0230 条款先行（spec/vulkan_backend.md）→ `run_graphics_offscreen_v2`（v1 签名/行为 0-byte 保留）+ `VkDescriptorSetLayout`(+immutable sampler)/pool/`vkUpdateDescriptorSets`/bind 建面 + mip 链 staging 逐层上传 + layout 迁移 + storage-image UAV 路 → **折叠进 U27 扩注**（§6.4）→ vk.rs host 单测（建面/上传纯 host 可测部分）。**此底座 = 采样/bindless/graph/present 四面单点关键依赖**（Vulkan 腿此前零 descriptor 面，vk.rs:1786 无 resource 形参）——独立先落地解风险，非压进采样数值 PR；device 不可用时 SKIP 三态（`RURIX_REQUIRE_REAL=1` 翻硬红），底座数值见证滑动时下游四面走**尾门降级路径不阻塞条款合入**。
4. **PR-S1**（条款+前端+SPIR-V）：RXS-0223~0229 条款体（含 RXS-0229 唯一写者纪律 typeck）→ 方法族 typeck + mir 扩 → opcode 全家 + fetch 钳制 → conformance + UI golden + spirv-val → 步骤 62 host 段。RED 构造：篡改 opcode 常量 → spirv-val 拒。
5. **PR-S2**（B 链+绑定+shim）：spirv-cross probe 证据先行（SampleCmp/Gather 分离形态）→ binding_layout static sampler 序列化 + UAV 轴扩 → uc04 shim **新增独立 sampler 入口**（sampler heap / mip / UAV barrier，非扩 offscreen 参数面，SC-6/§6.3）→ 步骤 62 全量（含 RXS-0174~0176 既有路零回归断言）。
6. **PR-S3**（采样数值对照+device；底座已由 PR-S0 先落地）：RXS-0230 条款收口 → 绑定装饰参数化（**混合有界+无界 B 链字节 diff golden 为合入门**，§4.B7/E-3）→ 步骤 63（≥6 模式数值判据 + 双后端对照 + 模式⑧唯一写者 store→barrier→回读 + `g3.counter.sampling_superset_modes` 同 PR）。

**G3.4 bindless（步骤 64；依赖 G3.3 底座 PR-S0）**
7. **PR-K1**（条款+前端+推导+codegen）：RXS-0231~0234 条款体 → 类型面 → binding_layout 翻转 Unmappable + set4/space1 分配律 + **RXS-0164（+0163/0165）加性修订行（SC-2）** + 回归网 → RuntimeArray/NonUniform/clamp 发射 → B 链 probe 先行 → conformance/UI golden → 步骤 64 host 段 + **number_ledger.json 校准（补「3xxx 自 RX3016」入 reserved_in_flight[G3].RX_error，落 RX3016，避 EI1 轨撞号，SC-4）**。
8. **PR-K2**（运行时+宿主+device）：RXS-0235 条款体 → TextureTable + `rxrt_table_*` + feature chain + UPDATE_AFTER_BIND pool（**折叠 U27 扩注**，§6.4）→ D3D12 shim SRV heap 段独立入口（ABI v4）→ 步骤 64 device 段（≥4 纹理四象限动态索引，篡改注册序 RED，feature 缺失 Err 断言）+ `g3.counter.bindless_descriptor_smoke` 同 PR。

**G3.5 render graph（步骤 65；依赖 G3.2/G3.3 语料）**
9. **PR-G1**（纯 host）：spec/render_graph.md 六条款 + README §4 登记 + 快照重 bless → `graph.rs`（装配核验 + 状态推导）→ host 单测全家（互证 set-equality / reject 四族 / 推导 golden / 确定性双跑）→ 新 6xxx ×2 en/zh → conformance 语料 → trace 再生。
10. **PR-G2**（执行器+device）：`rxrt_graph_*` → D3D12 执行器（ABI v5 数组下发）→ Vulkan `run_graph`（**折叠 U27 扩注**，§6.4）→ uc04 迁 Graph API → present 终端胶水 → 步骤 65 接线 + `g3.counter.auto_barrier_hazard_redgreen` 同 PR。

**G3.6 mesh-task-RT（步骤 66/67 ± 68/69；置尾）**
11. **PR-M0**（前置双 probe 证据 PR）：① DXIL mesh probe（最小非空输出 SPIR-V→spirv-cross→dxc ms_6_5/as_6_5→dxv）；② `bin/vk_rt_probe`（AS sizes/SBT 对齐/BDA 驱动坑）。probe 结果决定 RXS-0249 分支走向，落本 RFC 修订行。
12. **PR-Ma**（类型面）：RXS-0242~0245 条款 + RXS-0153/0155 修订行 → ast/parser/coloring/shader_stages.rs 全量 + UI golden + 新码 en/zh + 快照重 bless + **number_ledger.json 校准（落 RX3017 时校准 3xxx 预留，避 EI1 轨撞号，SC-4）**。纯前端无 GPU。
13. **PR-Mb**（SPIR-V 1.4 分叉，独立 PR 隔离隐性大改）：emitter 版本轴参数化 + interface 全量收集；**零回归门显式跨两个发射器（`dxil_spirv.rs` 变 / `vulkan_codegen.rs` 不动）golden 集界定**（E-4）= dxil 套件恒定 + vulkan 既有 golden 字节 diff 空 + 双口径 spirv-val accept。**不含任何新阶段编码**。
14. **PR-Mc**（mesh Vulkan 腿）：RXS-0246 条款 + mesh/task 编码（**复用 `vulkan_codegen.rs` GLCompute/LocalSize 基建**，不在 dxil_spirv.rs 重复 workgroup，E-4）+ vk.rs mesh 管线（**折叠 U27 扩注**）+ `bin/vk_mesh` + 步骤 66（程序化网格像素判据 + 篡改 SetMeshOutputs RED）+ counter 同 PR。
15. **PR-Md**（RT Vulkan 腿）：RXS-0247/0248 条款 + RT 六模型编码 + AS/SBT/TraceRays（**切细审计 U30**，AS/SBT/device-address，§6.4/E-2）+ `bin/vk_rt` + 步骤 67（三件套命中/miss 双色 + 移动顶点 RED）。intersection/callable 语料 accept-only 诚实标注。
16. **PR-Me**（DXIL 腿收口）：RXS-0249 条款 + RXS-0158 表修订 + **error_codes.json errata 同时登记 RX6009 正式 burn（RD-013 已 close 未 materialize，10 §9.5）+ RX6008 改接**（SC-1）+ probe 分支落地（步骤 68 = mesh/task：绿→B 链全量红绿，红→blocked 探针形态；步骤 69 = RT blocked 探针恒建，预期失败=PASS、意外绿=翻红）+ RD-012 处置留痕 + RD-029 close + RD-034+ 登记（如触发）。

### 6.6 每 PR 不变量核验（全期硬约束）

步骤 48 脚本与 shim offscreen 入口 git diff 0-byte（**守卫覆盖 offscreen 入口函数体 + 其自有版本常量 ==2，显式剔除 shim 聚合版本常量 `rx_uc04_abi_version` 返回值**——后者随入口增补变化、非语义冻结，§4.A4/§6.3/E-1；采样等新能力一律走新增独立入口不扩 offscreen 参数面，SC-6）；步骤 41/54~58 既有判据 0-byte 只增；dxil 套件恒定 / vulkan 套件 grow-only；B 链 dxv + 签名门（RX6011/6012）不旁路不裁剪；vk.rs 既有入口手写 barrier 路 0-byte；RXS-0169 条款与 barrier.rs 0-byte；LF byte-exact（新文件 LF+尾换行，禁 Python 文本模式写，逐文件核 CR+尾字节）；counter/entries 不预造（登记与 evaluator 分支同实现 PR）；GPU 运行经 proc_guard（R-606）；VVL/驱动崩溃以**退出码**区分判定（反 grep stdout）；device measured + run URL 归契约 §8；trace 全程全锚定（相对计数，合入时再生实测）。

## 7. 备选方案

1. **五份独立 Full RFC（RFC-0013~0017）**：v1.0 原案，已由契约 §7 v1.1 更正裁决否决——越权 claim 了 EI1 earmark（RFC-0014），且五份文档使跨章依赖（§4.0）与错误码调停（§5.1）散落互撞。采纳单伞形（MB1 rfcs/0011 先例）。
2. **MIR→HLSL 第三发射路径**（绕 spirv-cross 直发 HLSL 以解 DXIL mesh/RT 上游钳制）——**否决**（自 §4.E 章论证并入，四条理由）：① 违 D-131 既裁 B 链单转译主干——新增第三发射面 = 每语义面双实现双漂移（采样/bindless/graph 全期条款须逐条镜像到 HLSL 文本发射器），维护面爆炸；② HLSL 为文本中间层，无 spirv-val 级机器可验 IR 合约，strict-only 校验门失去锚点；③ RD-015 记录 A 路成熟即迁回的既定路线，第三路径与其冲突；④ 体量评估越出 G3 timebox（五面全量已满载）。否决项非 defer 项，不登 RD；若未来上游双钳制长期不解且 RT DXIL 成硬需求，届时按 10 §3 重新判档。
3. **texel fetch 越界返回零向量**（替代 clamp）：D3D 天然零返回但 Vulkan 侧需 `robustImageAccess2` feature 探测，双后端行为对齐要多一条运行期分支——违最保守确定性，否决（Q-S-FetchOob）。
4. **SPIR-V 全局统一 binding + 全量图形 golden 重 bless**（替代装饰参数化）：扰动既有零回归不变量（dxil 套件恒定），收益不对称，否决（Q-S-BindingScheme）。
5. **全局升 SPIR-V 1.4**（替代 per-entry 分叉）：迫使全量 golden 重 bless + DXIL B 路消费面同步漂移，零回归门失去「字节 diff 空」机核锚点，否决（Q-M-SpirvVersion）。
6. **`rxrt_graph` 单符号整图序列化下发**（替代细粒度 build 面）：需新序列化 ABI 面且 diag 无法定位到违例 pass，过重，否决（Q-G-CabiGranularity）。
7. **present 扩参既有 offscreen 入口**（替代加性新入口）：迫使 offscreen 调用面重编译，违「既有冒烟判据 0-byte 只增」guardrail，否决（Q-P-ShimEntry）。
8. **SM6.6 `ResourceDescriptorHeap[]` 直索引语法糖**：无 SPIR-V 标准对应，结构性过不了 B 链单语义源纪律；unbounded array 已语义等价覆盖其能力面。收窄登 §8（RD-018 close 留痕显式写明，非静默砍面）。

## 8. 不做（范围红线）

各面 NOT_DO 合并如下；「登记去向」列 RD-034+ 者见本节末合并登记清单（以合入时 deferred.json 实际续号双侧标注）。

| 章 | 不做项 | 理由（摘） | 登记去向 |
|---|---|---|---|
| A | 窗口输入/事件循环/窗口管理 API 面 | D-130 红线；demo 只泵消息 | **SG-010 维持**（扩张诱惑出现登 gating 非提案） |
| A | present 替代 offscreen 验收 | RD-019 backfill 明记；步骤 48 硬门 | RD-019 close 留痕明记，不另立 RD |
| A | exclusive fullscreen / blt-model / 多窗口多 swapchain | flip-model 单窗即达诉求 | RD-034+（清单 ①） |
| A | HDR/色彩空间/VRR 深度面（waitable swapchain/latency 调优） | 首期仅 vsync/tearing 参数面 | RD-034+（清单 ①） |
| A | Android surface 生命周期/重建（surface lost） | G-MB1-7 已 measured 面不重开；on-device 无 runner | RD-034+（清单 ①） |
| A | present 性能门（帧时/延迟预算） | 首期 correctness-only（RFC-0011 Q-Perf 先例） | 不立，纵深期评估 |
| A/全 | MSAA backbuffer/blending/stencil/indirect | 契约 out_of_scope `msaa_blend_stencil_indirect`：零登记不静默带入 | 维持零登记 |
| B | 隐式 LOD 比较采样（SampleCmp 全隐式）、`gather_cmp` | 恒 LOD 0 已覆盖 shadow-map 主用例 | RD-034+（清单 ②） |
| B | image atomics（OpImageTexelPointer + atomic 族） | 跨线程协同走既有 RXS-0080 缓冲 Atomic 路 | RD-034+（清单 ②） |
| B | **可竞写 storage image 写模式（多写者同 texel）** | 首期唯一写者纪律结构回避（§4.B5/G-RED-1）；放开须独立论证 Vulkan/D3D 非原子写可见性 + 规范引文 | **RD-034+（清单 ②）另 Full RFC** |
| B | Texture1D/3D/Cube/数组纹理、MSAA 纹理 load | 维度族扩面；MSAA 按契约 out_of_scope 不代登 | RD-034+（清单 ②，MSAA 除外） |
| B | runtime 自动 mipmap 生成 | 测试语料宿主显式逐层上传 | RD-034+（清单 ②） |
| B | 压缩格式（BC/ASTC）、sRGB 自动转换、sampler feedback、min/max reduction | 无 deferred 载体，需求出现先登记再评估（14 §4） | 不登记不承诺 |
| B | PTX 腿纹理路（tex.2d）/ OptiX | D-207 结构性不适用事实标注 | 零登记不讨论 |
| B | compute（kernel）阶段 storage image / 采样 | 首期 TextureRw2D 限 fragment+raygen（§4.0-2）；DXIL compute=A 链无 B 链承接面 | RD-034+（清单 ②） |
| B | 动态索引句柄 / 无界数组 receiver | §4.C 专属；本章维持 RXS-0175 L4 | 归 §4.C，非登记项 |
| B | filter 精度跨后端逐位承诺、descriptor/opcode 二进制布局冻结 | 承 RFC-0007 §8 / RFC-0005 §4.5 🔒 不冻结纪律 | 维持 |
| C | SM6.6 heap 直索引语法糖 | 见 §7-8 | **RD-034+（清单 ③）+ RD-018 close 留痕显式收窄** |
| C | 无界 Sampler / CBV / UAV（StructuredBuffer）表 | 首期收敛 SRV 纹理表（G-G3-4 判据面） | 维持 Unmappable/RX6013；需求出现按 14 §4 补登，不预登 |
| C | uniformity/divergence 推断（自动免标） | 优化面非能力面；保守全标已语义完备 | 不登记（RD-018 close 留痕提及） |
| C | in-flight 期间 descriptor 活跃更新 | 触 GPU 时间线并发语义（§4.D 邻接禁区）；仅提交前注册 | 不登记（RXS-0235 显式拒，结构不可达） |
| C | `#[binding(...)]` 显式覆盖 / 多 space 用户面 | RFC-0005 §9 既有裁决维持，分配律纯推导 | 维持原裁 |
| C | bindless buffer device address（GPU 指针表）路线 | 与句柄非值纪律正面冲突，触 06 §4.2 禁区纵深 | 不登记不讨论（死亡路线邻接，诱惑出现登 SG） |
| D | pass 重排 / 依赖驱动调度优化 | 声明序=提交序是 D4 全序地基 | RD-034+（清单 ④，契约 out_of_scope `graph_reorder_multiqueue`） |
| D | 多 queue / async compute 重叠执行 | 跨 queue 可见性是另一内存模型面，须独立 Full RFC | RD-034+（清单 ④ 同条合并） |
| D | split barrier / D3D12 enhanced barriers API | 首期保守掩码下无收益 | RD-034+（清单 ④） |
| D | Vulkan stage/access 掩码最小化 | 正确性先行；窄化须与 D4 措辞联动修订 | RD-034+（清单 ④ 同族） |
| D | 资源 aliasing / transient 内存复用 | 状态机先在非 alias 世界证实互证闭环 | RD-034+（清单 ④） |
| D | **bindless 表 / storage image 资源 / mesh·RT pass kind 进 graph 面** | 首期封闭枚举不可表达（§4.0-3），显式留痕非静默 | **RD-034+（清单 ⑤）** |
| D | 图静态化到编译期 | 装配期核验已 strict 闭合，提前无诊断增益 | 不登记（真出现按 14 §4 补登） |
| D | 窗口/输入/事件循环进 graph 面 | D-130 红线；present 只做胶水 | SG-010 维持 |
| E | DXIL RT 全量降级 | 双上游钳制（spirv-cross 无 SPV_KHR_ray_tracing 消费 + RD-015 未解） | **RD-034+（清单 ⑥ 尾门）**，步骤 69 blocked 探针证据落地 |
| E | DXIL mesh/task 全量（若 probe 红） | probe-first：不以未实测转译链背书全量 | 条件 RD-034+（清单 ⑥；probe 绿则本条不存在） |
| E | MIR→HLSL 第三发射路径 | 见 §7-2 论证 | 否决项不登记 |
| E | ray query（inline RT，OpRayQuery） | 独立能力面，与六执行模型管线正交；零 deferred 登记不静默带入 | 不预登记；诉求出现先登 RD-034+ 再评估 |
| E | intersection/callable device 端到端见证 | 首期 device 语料收敛三件套；二者 accept-only（类型面+codegen+spirv-val 全量） | RD-034+（清单 ⑦） |
| E | ShaderRecordBufferKHR / 多 hit group / SBT 寻址参数非零 | SBT 布局 ABI 面扩张；单三件套下恒 0 唯一确定 | RD-034+（清单 ⑦ 同条收窄留痕） |
| E | AS 进阶面（compaction / refit / indirect build / motion blur） | 正确性首期外的吞吐/动态面 | 不预登记；诉求出现登 RD-034+ |
| E | mesh 拓扑扩集（lines/points）/ per-primitive varying / CullPrimitiveEXT | triangles-only 首期收敛 | RXS-0243 修订轴，诉求出现随修订扩或登 RD-034+ |
| E | PTX/OptiX 腿 | D-207 收集根排除；OptiX 契约明令不登记不讨论 | 零登记 |
| 全 | AMD device 见证 | G-MB1-6 硬件尾门独立存续；本机 RTX 4070 Ti measured 不充作 AMD | G-MB1-6/RD-032 维持，零新登记 |
| 全 | 外部采纳/用户数维度 | 契约 out_of_scope `production_adoption_claim` carve-out | 维持 |

**RD-034+ 合并登记意向清单**（五章 ~15 项登记意向去重合族为 7 条；执行期按 14 §4 追加，以合入时 deferred.json 实际续号双侧标注，不逐章散登防撞号）：

| # | 家族 | 收纳项 |
|---|---|---|
| ① | present 扩面族 | exclusive fullscreen / blt-model / 多窗口多 swapchain；HDR/色彩空间/VRR/waitable；Android surface 重建 |
| ② | 采样扩面族 | 隐式 dref + gather_cmp；维度族（1D/3D/Cube/数组 + MSAA load 之非 MSAA 部分）；自动 mipmap；compute 阶段图像面；image atomics；**可竞写 storage image 写模式（多写者同 texel，另 Full RFC）**；probe 红的 SampleCmp/Gather 子模式（条件） |
| ③ | bindless heap 直索引语法糖 | SM6.6 ResourceDescriptorHeap[]（RD-018 close 留痕显式收窄）；DXIL 腿 probe 红臂（条件） |
| ④ | graph 进阶族 | pass 重排 + 多 queue/async compute；split barrier + enhanced barriers；掩码窄化；aliasing/transient 复用 |
| ⑤ | graph 首期不可表达面 | bindless 表声明 / storage image 资源 / mesh·RT pass kind |
| ⑥ | mesh/RT DXIL 腿尾门 | DXIL RT 全量（恒，blocked 探针跟踪）；DXIL mesh/task（条件，视 probe） |
| ⑦ | RT device 见证扩充族 | intersection/callable 管线语料；SBT 用户数据/多 hit group/trace_ray 扩展参数轴 |

## 9. 未决问题 / 关键裁决（agent 自主判档拟裁，D-406 v2.0；批准即定案，逐项回填）

编号规则：`Q-<章>-<名>`（P=present / S=采样 / B=bindless / G=graph / M=mesh-RT）。

| # | 裁决点 | 拟裁 + 理由（摘） |
|---|---|---|
| Q-P-ShimEntry | shim ABI bump 形态 | **加性独立入口 `rx_uc04_present_run` + 版号 bump，offscreen 入口字节不变**——加性入口是步骤 48 0-byte 的结构保证（§7-7） |
| Q-P-RebuildHome | Vulkan OUT_OF_DATE 重建条款落点 | **RXS-0221 单条承载跨后端重建不变式，vulkan_backend.md RXS-0210 加性修订行引用**——重建序为后端无关不变式，单条防两文件语义漂移 |
| Q-P-VisibleWindow | 可见窗口的机器判据 | **可见性为语义承诺（WS_OVERLAPPEDWINDOW+ShowWindow），机器判据 = flip-model Present 逐帧 S_OK + readback 数值断言，不断言「人眼可见」**——scanout 内容不可编程回读，诚实边界写进条款（§9.2 P-1） |
| Q-P-TearingFail | tearing 能力缺失语义 | **确定性运行期拒（诊断+终止，不占 RX 码），不静默降级为 vsync**——P-01 无静默降级；能力探测属环境非语言违例 |
| Q-P-ReadbackCadence | readback 频度 | **逐帧 copy、断言首帧/重建后首帧/末帧三点**——smoke 帧数小逐帧成本可忽略，三点覆盖初始/重建/稳态 |
| Q-P-CodeGranularity | present 新码颗粒度 | **装配核验 ×1 + 重建核验 ×1（§5.1 预测 RX6027/RX6028）**；实现期若共享同一核验通道则合 ×1 分变体——镜像「按真实可达类别」纪律不预造 |
| Q-S-SampleName | `sample` 隐式化 vs 保持 LOD 0 | **隐式化 + 语料迁移**——业界语义对齐永久 API 不留惊讶；既有语义由 `sample_lod(s,uv,0.0)` 同路径承接、golden 0-byte；现在不改 1.x 后永远改不了 |
| Q-S-FetchOob | fetch 越界 zero vs clamp | **clamp（codegen 注入钳制）**——确定性、双后端同源、零 feature 依赖（§7-3）；代价数条 ALU，首期不立性能门 |
| Q-S-BindingScheme | Vulkan 原生 descriptor 冲突 | **同一 lowering 装饰参数化**（B 链字节不动零重 bless；Vk set-per-class）；conformance 断言两形态仅 Decorate 异（§7-4 否全局重 bless） |
| Q-S-Element | 元素类型覆盖 | **sample 族限 f32 分量；load/store 支持 {f32,u32,i32}；TextureRw2D 阶段 = fragment + raygen（§4.0-2）**——过滤仅对浮点定义；raygen 面为 RT 输出通道前提 |
| Q-S-UavOrder | storage image 写模式 | **§4.B5 首期唯一写者纪律**（结构性禁止可竞写模式；identity 坐标 by-construction 每 texel 单写者、typeck/codegen 强制可 golden + pass 边界唯一可见性点 + 无 image atomics）——**无竞写即无 race**，§1 无 UB 真成立，不移植 PTX/06 §4.2 uniform-size 公理到 Vulkan/D3D 图形路（G-RED-1 blocker 收窄）；storage image barrier 首期在 §4.D 自动推导域**之外**走手动 RXS-0169 路（§4.0-3 封闭枚举不承载），纳入 graph 为 forward-compat-only 非当前成立（G-RED-2）；**可竞写模式放开登 RD-034+ 另 Full RFC** |
| Q-S-CmpLod | sample_cmp LOD 形态 | **恒显式 LOD 0**——shadow-map 惯用形态；隐式 dref 与 quad 语义纠缠推 RD-034+，规避两重面相乘复杂度 |
| Q-B-IndexForm | 无界数组书写形态 | **`[Texture2D<F>]` 切片样式**——复用既有类型文法零新 token；具名类型会把宿主 affine 类型泄入 device 签名破坏分层 |
| Q-B-Uniformity | nonuniform 标注纪律 | **字面量常量豁免 + 其余强制标注缺失新码拒；不做推断**——过标注 SPIR-V 合法仅性能保守，欠标注是波内错采样事故面；先严后松 |
| Q-B-SetSpace | 独占分配律双侧形态 | **SPIR-V Vk-native 独占 set 自 set4（类别轴之后，§4.0-1）、RTS0 独占 space 自 space1，同源单一事实源产出**——UPDATE_AFTER_BIND 粒度 = per set-layout，有界 set0~3 完全不触 feature chain；两侧同源镜像 RXS-0164 分轴同口径教训 |
| Q-B-OOB | 越界有界性机制 | **codegen 强制 clamp 至注册计数（表长经 push-constant 尾槽下发）**——robustness2 为可选 feature 依赖之则承诺非确定；clamp 是唯一双腿同源、设备无关、可 golden 断言的机制 |
| Q-B-TableABI | cabi 承载 | **新 `rxrt_table_*` 符号族只追加**——RXS-0194 符号面含义冻结，kinds 枚举扩位触 MS1.2 已发布 marshalling 冻结面；新符号 = RFC-0011 §9 Q-Marshal 备选臂既认可模式 |
| Q-B-TableFormat | TextureTable 元素类型一致性 | **格式擦除（`TextureTable<C>` 无元素类型参数），host↔shader 元素错配 = 运行期确定性 Err**——条款显式声明该边界（RXS-0235）；备选 `TextureTable<C,F>` 泛型留后期加性升级 |
| Q-B-BLegGate | DXIL 腿 probe 红处置 | **条件分支条款写入（G-EA1-3 先例，§4.0-8）**：绿 → B 链腿全量 + 步骤 64 含 DXIL 段；红 → 腿落 RD-034+ 尾门 + blocked 探针入 CI，Vulkan 腿单独兑现 G-G3-4 |
| Q-G-SpecFile | graph 条款落点 | **新建 spec/render_graph.md**——host_orchestration 区间（RXS-0189~0199）头注已封闭；新语义面新文件是 vulkan_backend.md/d3d12_runtime.md 双先例；README §4 登记同 PR |
| Q-G-ApiShape | Graph API 形态 | **builder 方法链**——逐方法即逐 typeck 已知签名分支（RXS-0190 先例），诊断 span 精确到单条访问声明；结构体面超首期宿主子集（RD-026 边界） |
| Q-G-OverDeclare | 「声明未用」侧处置 | **双向 strict 拒（精确相等，域=首期封闭枚举资源面，§4.0-3）**——P-01 无静默；容忍未用声明 = 契约漂移温床；放宽是纯加性方向可后续 Mini 判档 |
| Q-G-Masks | Vulkan 首期同步掩码档 | **最保守 sound 掩码（pass 边界全阶段覆盖）**——D4 承诺以正确性为唯一判据；窄化登 RD-034+；条款与实参逐字段可对照是评审可核物 |
| Q-G-ManualDoor | RXS-0169 手动核验器保留期限 | **永续保留（非过渡兼容层），条款与 barrier.rs 0-byte**——双实现互证是本面唯一 host 金标准，砍掉即失去独立 oracle |
| Q-G-CabiGranularity | rxrt_graph_* 符号粒度 | **细粒度 build（create/pass/declare/readback）+ 单 execute**——镜像 rxrt_* 句柄面纪律；diag 可定位违例 pass（§7-6 否单符号整图） |
| Q-M-StageSyntax | intersection/callable 文法形态 | **前缀关键字直入 RXS-0153 stage 集，零新属性面**——与既有九阶段零不对称（RFC-0002 Q1 先例）；修订行不占新号 |
| Q-M-PayloadForm | payload/attribute 声明形态 | **标注式形参（`#[payload] p: &mut P` 等）**——对齐 RFC-0002 Q2 属性式 I/O 既裁；`&mut` 借用形态落既有借用检查零新所有权面 |
| Q-M-PairingDomain | payload 契约配对域 | **单编译单元 + 单三件套管线，配对不可判定即编译期拒（strict 无静默通过）**——与步骤 67 语料同构；多单元/多 payload 需 SBT 组装期核对面越出首期，宁拒不猜 |
| Q-M-SpirvVersion | 1.4 分叉形态 | **per-entry 版本轴（mesh/RT 入口 1.4，既有入口 1.0 字节不变）**——全局升 1.4 迫使全量重 bless + B 路消费面漂移（§7-5）；代价仅 emitter 版本参数化 |
| Q-M-MeshTopology | mesh 输出拓扑首期集 | **triangles-only，非 "triangles" → 编译期拒**——步骤 66 像素判据仅需三角形；扩集加宽验收矩阵不加宽证据价值 |
| Q-M-FeatureGate | 是否为 mesh/RT 开新 cargo feature | **不开**——沿三既有 gate（§6.2）；mesh/RT 是既有阶段面与既有后端的类别扩充非新目标；新 feature = 新组合矩阵无对应隔离收益 |
| Q-M-RX6008Scope | RX6008 改接语义边界 | **落码一次、标题覆盖「DXIL mesh/task/RT 阶段降级不可用」；probe 绿的阶段从拒绝集移除转真降级，blocked 阶段维持**——分支只改集合成员不改码语义（可加不可改） |

## 9.1 对抗性评审记录（对抗性评审要求，10 §3 / §7 · [`../13_DECISION_LOG.md`](../13_DECISION_LOG.md) D-409）

**已完成（第 1 轮，2026-07-18）**——评审者 provenance `Assisted-by: claude-code:claude-opus-4-8`（**≠ 起草 `claude-code:claude-fable-5`**，跨模型镜头，硬规则 2 可机验，`ci/check_contribution.py` 规则 4 满足 D-409）；三镜头 correctness / redline / implementability 对伞形全文一次覆盖；§9.2 为评审输入攻击面。13 条 findings 全部逐条 disposition，无空过；唯一 blocker G-RED-1 **合入前正文已实改**（§4.B5/RXS-0229 收窄为唯一写者纪律）；据此本 RFC 状态 Draft → **Agent Approved**。

| 字段 | 值 |
|---|---|
| 评审者 provenance | `Assisted-by: claude-code:claude-opus-4-8`（**≠ 起草 `claude-code:claude-fable-5`**；三镜头 correctness/redline/implementability） |
| 评审轮次 | 第 1 轮，2026-07-18 |
| 结论 | 1 blocker（正文已实改）+ 5 major（正文实改 / 落实现 PR）+ 7 minor（disposition + 轻措辞）；无驳回，全采纳 |

**Findings 与 disposition**（13 条；镜头 R=redline / SC=correctness / E=implementability；disposition：**采纳并改 §X** / **采纳落实现 PR** / **部分驳回**+理由）：

| # | Finding（评审者提出，摘） | 严重度 | Disposition |
|---|---|---|---|
| G-RED-1 | §4.B5/RXS-0229 storage image 竞写「有界非确定·非 UB」把 PTX 公理平移到 Vulkan/D3D 图形路 + 剥掉 06 §4.2 强制的 UB 边界标注（唯一击穿禁区红线处） | **blocker** | **采纳并改 §4.B5/RXS-0229（合入前正文已实改）**：收窄为首期**唯一写者纪律**（identity 坐标 by-construction 每 texel 单写者，typeck/codegen 强制、可 golden），删除全部「有界竞写 / PTX 公理平移」未证内存模型断言——无竞写即无 race、§1 无 UB 真成立；Q-S-UavOrder / §4.B8⑧ / §5 RXS-0229 行 / §1·§2 摘要同步；可竞写模式登 RD-034+ 另 Full RFC（§8） |
| SC-2 | RXS-0233 翻转 RXS-0164 的 unbounded→Unmappable/单 space0，却未把 RXS-0163~0165 列入修订清单，binding_layout.md 单一事实源自相矛盾（逼近 blocker） | major | **采纳并改 front-matter line 10 / §5 / §4.C2 / §5 RXS-0233 行**：RXS-0164（+0163/0165）入既有条款加性修订行清单；落修订行收窄 unbounded→Unmappable 为「非 bindless-SRV unbounded」、单 space0 为「有界 space0 / bindless 自 space（类别）」，前向引用 RXS-0231/0233 |
| SC-1 | RFC 称 RX6009 burned 但权威 error_codes.json 仍记「留给 RD-013」（RD-013 已 close 却未同步），跨 registry 活跃矛盾 | major | **采纳并改 §5.1/§4.E10 + 落实现 PR（§6.5 PR-Me）**：RX6009 改述为「RD-013 已 close（2026-06-29）未 materialize → 随 PR-Me error_codes.json errata 正式 burn（10 §9.5）」；PR-Me 明列 errata 同登 RX6009 burn + RX6008 改接 |
| E-1 | shim 单一 `kAbiVersion` 精确相等机制与「加性/offscreen 语义字节不变/v3~v5 逐面 bump」叙事直接冲突（bump 令 rx_uc04_abi_version 返回值 + offscreen 接受版本漂移） | major | **采纳并改 §4.A4/§6.3/§6.6**：改**每入口独立版本常量 + `>=` 语义**（offscreen 恒校验 ==2 永不变）；`rx_uc04_abi_version` 返回值随 bump 变化、非语义冻结（纯 build-provenance）；§6.6 0-byte 不变量显式剔除聚合版本常量 |
| E-2 | §6.4 预测 U30~U35（5~6 新号）与其所引 U26/U27「整 FFI 边界折叠单 U、present/Android 扩注无新号」约定不自洽，且用「1 个 U」审计最高危 AS/SBT/device-address 不足 | major | **采纳并改 §6.4 + 散引处（§4.B7/C4/D5b/E7/E8 + §5/§6.5）**：保留 U26/U27 折叠约定（G3 大部分 Vulkan 工作 = U27 扩注 0 新号），AS/SBT/device-address 切**单一细审计 U30**；不再预测 U30~U35 六个；number_ledger claim 措辞同步 |
| E-3 | 「binding 单一事实源」实为由 provenance 旗标选择的两套 set 策略，触碰有历史 device-bug 的核心函数（:144-147），缺混合签名字节 golden 门 | major | **采纳并改 §4.0-1/§4.B7/§4.C2**：措辞改「单一 binding-**号**事实源 + 按目标选择的两套 set 分配策略」；「混合有界+无界/多表/四类别齐全」B 链**字节 diff golden** 定为该 binding 改动合入门（机核 B 链字节不动，承 :144-147 device bug 教训），非仅 UI golden |
| E-5 | Vulkan descriptor 底座（四面承重依赖）压进单个 PR-S3 置于关键路径，无缓解/无 fallback | minor | **采纳并改 §6.5**：拆出 **PR-S0**（`run_graphics_offscreen_v2` + 建面 + mip 上传）独立、**先落地**作四面共同前置；采样数值对照另置 PR-S3；标注单点关键依赖 + device 不可用时 SKIP / 尾门降级路径 |
| G-RED-2 | Q-S-UavOrder「§4.B5 与 §4.D happens-before 本体可无缝叠加」与 storage image 出 graph 封闭枚举、走手动路的事实相冲突（误导实现者） | minor | **采纳并改 Q-S-UavOrder/§4.B5**：storage image barrier 首期手动 RXS-0169、在 §4.D 自动推导域**之外**；「无缝叠加」标为 forward-compat-only 非当前成立 |
| SC-3 | RX3012 冻结 title「相邻 varying 接口」对 RT/task/callable payload（非相邻传递、无插值维度）是语义拉伸 | minor | **采纳并改 §4.E3 + §5 RXS-0244 行**：RXS-0244 声明 RX3012 覆盖「着色阶段间数据契约」超集、「插值限定」维度对 RT/payload/callable 面 N/A，只加类别不改既有语义（07 §5）；实现期判独立类别则退 §5.1 RX3018+ 条件行 |
| SC-4 | number_ledger reserved_in_flight[G3].RX_error 无 3xxx 预留，G3 消费 RX3016/3017 有被并行 EI1 轨抢占的跨分支撞号风险，且无 PR 被指派去改 | minor | **采纳并改 §6.5 PR-K1/PR-Ma**：落 RX3016（PR-K1）/RX3017（PR-Ma）时附 number_ledger.json 校准，补「3xxx 自 RX3016」入 reserved_in_flight[G3].RX_error，避 EI1 撞号 |
| SC-5 | RXS-0197 是 CUDA↔D3D12 interop present typestate，UC-04 纯 D3D12 图形 present 不流经它，「复用/引 RXS-0197」是错误条款指引 | minor | **采纳并改 §3.1/§4.A6/§5 RXS-0220（+§4.D5c）**：措辞改「RXS-0197 present typestate 维持不动；UC-04 窗口 present 独立走 shim、零 .rx 面故零新语法」，两种 present 机制不混 |
| SC-6 | §6.3「采样 shim 增量经 offscreen 入口参数面承载不 bump」与 §6.6/§4.A4「offscreen 入口 0-byte」硬门直接冲突 | minor | **采纳并改 §6.3（并 E-1 一并处理）**：采样 shim 增量走**新增独立入口**（非扩 offscreen 入口），与 §6.6/§4.A4 offscreen 0-byte 不变量对齐 |
| E-4 | 仓内存在两个 SPIR-V 发射器（dxil_spirv.rs / vulkan_codegen.rs），mesh/task 与 vulkan_codegen.rs compute 基建共性更大却归入 vertex/fragment 发射器；单数「emitter 版本字参数化」措辞使零回归门漏掉跨发射器范围 | minor | **采纳并改 §4.E5/§6.5 PR-Mb/PR-Mc + §5 RXS-0246 行**：钉 mesh/task 编码器落点 = 复用 `vulkan_codegen.rs` GLCompute/LocalSize 基建；零漂移门显式跨两个发射器 golden 集界定 |

## 9.2 已知风险与评审攻击面（起草侧自暴，供 §9.1 评审镜头用）

> **评审已消化（2026-07-18，D-409 三镜头 correctness/redline/implementability，§9.1）**：下列自暴攻击面（P-1~M-5）已作为 §9.1 对抗性评审输入被逐镜头消化——**S-2**（竞写有界非确定的双后端可证性，起草侧自认「最易被击穿的内存模型主张」）经 blocker **G-RED-1** 正文收窄为唯一写者纪律（禁区结构回避）；**S-4**（装饰参数化「仅 Decorate 异」不变量）经 **E-3** 补「混合有界+无界 B 链字节 diff golden」合入门；**E-1/E-2/E-5** 等代码级验证并入 §9.1 findings 并落 disposition。以下条目保留供后续里程碑（控制流落地 PR、AMD 尾门等）复用。

**present（A）**
- **P-1「readback == 呈现内容」等价主张**：readback 在 present 前 copy，证明的是渲染产物非 scanout 像素；`Present` S_OK 仅证提交被 DWM 接受。条款措辞须把证据边界写死（RXS-0222 只 claim「呈现链路数值可校验」）。
- **P-2 合成 resize 覆盖缺口**：`SetWindowPos` 合成 `WM_SIZE` 不经过用户拖拽 sizing loop（`WM_ENTERSIZEMOVE`/模态泵）；evidence 须如实标注驱动方式。
- **P-3 可见窗口 × self-hosted 交互桌面 runner 稳定性**：遮挡/失焦可产 `DXGI_STATUS_OCCLUDED`，SKIP 判定与 fake-green 边界脆弱；OCCLUDED 处置若含糊会被攻「SKIP 通道可吞真红」。
- **P-4 RX 码边界（装配期 vs 运行期）在 present 面最模糊**：tearing 缺失/ResizeBuffers 失败/DEVICE_REMOVED 全判运行期不占码——须以「host 可判定性」为分界并在条款逐类列举。
- **P-5 Vulkan OUT_OF_DATE 在 NVIDIA/Windows 极难自然触发**：重建路径依赖合成触发，不同 ICD 触发时机差异大；AMD/Android 面照 G-MB1-6/7 尾门措辞不 claim。

**采样（B）**
- **S-1 spirv-cross 分离形态 SampleCmp/Gather 成熟度**：条件分支条款押注 probe；评审应攻击 probe 语料代表性（mip + 比较 + 分量选择组合位）。
- **S-2 竞写有界非确定的双后端可证性**：Vulkan 规范对非原子竞写的正文措辞是否足以支撑「有界」而非「未定义」，是本 RFC 最易被击穿的内存模型主张；若不可证，条款须再收窄（禁止可竞写模式或强制唯一写者纪律）。
- **S-3 隐式 LOD by-construction 论证寿命**：依赖图形 body straight-line（RXS-0171）现状；条件条款的「词法保守拒」若在控制流落地时漏接，隐式 LOD 会静默滑入发散区——评审应要求条件条款带机验锚（控制流落地 PR 的强制交叉测试）。
- **S-4 装饰参数化「仅 Decorate 异」不变量**：emitter 演进（§4.C 改 binding 面）后若失守，双形态语义分叉无声发生；评审应攻击断言覆盖面（含 gather/storage image 全 opcode 语料）。
- **S-5 双后端数值一致性容差设计**：容差宽则「一致」沦为弱判据，窄则厂商过滤精度差异产假红；评审应逼出容差量化依据与逐位模式在 ≥6 模式集中的占比下限。

**bindless（C）**
- **B-1「SM6.0 即可」是未 probe 的上游行为断言**：四环节（spirv-cross 译出/dxc 接受/dxv 通过/RTS0 对齐）任一失手即 DXIL 腿翻红；已置 probe-first 对冲，评审应攻击该措辞是否该降格为 assumed 标注（§4.C3 已注「probe 前文献推断」）。
- **B-2 clamp 与 NonUniform 组合保真**：`UMin(nonuniform_idx, len-1)` 经 spirv-cross 改写与 dxc 优化后是否保有 `NonUniformResourceIndex` 语义——丢失标注在 NVIDIA 大概率「碰巧对」，错采样只在 AMD/Adreno 波内分歧显形（AMD 恰是尾门）。评审应要求 HLSL 产物 golden 显式断言 NonUniformResourceIndex token 存在。
- **B-3 feature chain「确定性 Err」证真难度**：本机四 bit 全支持，缺失路径真设备不可达——mock 单测证分支逻辑非真探测路径；评审应攻击步骤 64 对该判据的取证形态是否够格 measured。
- **B-4 有界路零回归接触面比声称的大**：set4/space1 轴触 `infer_spirv_bindings`/`infer_register_assignments` 核心循环，per-class 计数口径此前已爆过三方失配 bug——「零字节漂移」须以混合签名（有界+无界并存+多表）语料压实。
- **B-5 临时句柄不逃逸的两层一致性**：typeck 拒 let 绑定与 MIR 不物化 local 是两处独立实现的同一不变式——评审应要求 MIR 层断言测试（降级产物无句柄型 local）而非仅 UI golden。

**render graph（D）**
- **G-1 互证金标准独立性可疑**：两实现思路同构则 set-equality 是自证非互证。可核物：graph.rs 无 barrier.rs import；RXS-0169 集 main 冻结先于 graph.rs 存在；断言双向。评审镜头 = diff 两实现的状态枚举与转换判定是否实质独立。
- **G-2 D4 承诺与 Vulkan 实参的措辞-实现缝**：执行器实际掩码若窄于条款措辞，可见性缺口是概率性 bug，VVL 零报错与像素判据都可能测不出。评审镜头 = 逐字段核对 `vkCmdPipelineBarrier` 实参与 RXS-0239 实现要求；质询「保守掩码」是否有精确定义而非形容词。
- **G-3「同源映射表」在 shim C++ 侧的失守面**：C++ 侧存在任何枚举复制/语义重映射（哪怕数值恒等 switch）即破单一事实源。评审镜头 = grep shim 源里的状态常量，应为数值透传。
- **G-4「环检测」条款可达性**：声明序构造下经典环不可构造；条款措辞已锁 use-before-write 可达形态（§4.D2），评审镜头 = 逐条核对 RXS-0237 reject 四族真可构造。
- **G-5 三 barrier 形态措辞失真**：UavSync（from==to）与「自转换非法」直觉冲突（RXS-0169 现即拒自转换）；uc04 三 pass 语料无 UAV——互证可能**因语料不含 UAV 而未证 UavSync**，该腿须有独立单测锚定，不得借互证门冒充已证。

**mesh-task-RT（E）**
- **M-1 per-entry 1.4 分叉零漂移主张**：emitter id 分配序/装饰 emit 序被 1.4 interface 收集扰动，1.0 产物可能「语义等价但字节漂移」。评审应要求 PR-Mb 附双口径产物 sha256 对照 + 1.0 路 golden 字节 diff 空的机核输出。
- **M-2 递归深度恒 1 的调用图封闭性**：`trace_ray` 出现在被 closesthit 调用的普通 device fn 体内时单点检查失效——条款已写「经调用图可达域亦拒」（§4.E4），评审应核对实现语料确覆盖间接层。
- **M-3「probe 绿 ⇒ DXIL mesh 全量可落」的证据跳跃**：最小语料过 dxc/dxv ≠ SetMeshOutputsEXT/payload/拓扑映射在真实语料保真；probe 语料须含非空输出（单三角形写出）而非空体（§4.E9 已内建），评审应核对。
- **M-4 合规轴 vs 驱动宽容度混同**：NVIDIA 驱动可能接受不合规组合——spirv-val 退出码三态 gate 为唯一合规判据、驱动行为仅作 device 证据；评审应核查步骤 66/67 脚本两轴未混同。
- **M-5 篡改 RED 判据依赖 VVL 报错**：VVL 对损坏 SPIR-V 有自身 SIGSEGV 前科（MB1 Adreno/MTE 上游 bug 实录），mesh/RT 扩展路径 VVL 覆盖更浅——RED 语料可能触发 VVL 崩溃而非报错。缓解 = 退出码三分类（校验报错红 / 工具自身崩溃 / 绿），评审应核查脚本非二分类。

## 10. 稳定化与 provenance

- **稳定化**（10 §5）：五面全部 gate 后（§6.2 既有 gate 复用）、**非 stable**——shim ABI / `rxrt_table_*` / `rxrt_graph_*` / set·space 数值 / SBT 布局均「实现确定、gate 后、不冻结」；stable 面冻结随 **RD-008** 届时定义。条款随 stable 快照加性重 bless（RXS-0180 L2 同 edition 只增不破坏）。两道硬件尾门（AMD G-MB1-6 / Android on-device）达成前，图形特性面不承诺 stable。
- **Provenance**：`Assisted-by: claude-code:claude-fable-5`（起草）+ `Assisted-by: claude-code:claude-opus-4-8`（§9.1 对抗性评审，三镜头，**≠ 起草**，D-409/硬规则 2）。agent 自主决策；批准前置 = §9.1 对抗性评审**已完成（2026-07-18 第 1 轮，13 findings 全 disposition、blocker G-RED-1 正文已实改）**，批准后推进 §6.5 下游实现 PR。

## 11. 规范与实现依据

- **仓内**：06_GPU_GRAPHICS_PROGRAMMING_MODEL.md §4.2/§8.2；milestones/g3/{G3_CONTRACT.md,G3_PLAN.md,CI_GATES.md}；registry/{deferred,error_codes,number_ledger,spike_gating}.json；spec/shader_stages.md（RXS-0153~0156/0174）、spec/dxil_backend.md（RXS-0155/0157~0162/0171/0175/0176）、spec/binding_layout.md（RXS-0163~0166）、spec/d3d12_runtime.md（RXS-0167~0170）、spec/host_orchestration.md（RXS-0142/0189~0199）、spec/vulkan_backend.md（RXS-0200~0213）、spec/device.md（RXS-0068/0079/0080）、spec/borrow.md（RXS-0054/0057~0061）、spec/edition.md（RXS-0180）、spec/release.md（RXS-0219 区间尾）；rfcs/0002/0005/0006/0007/0009/0011（先例与既裁）；src/rurixc/src/{dxil_spirv,binding_layout,mir,ast,shader_stages}.rs；src/rurix-rt/src/vk.rs；src/uc04-demo/{shim/uc04_offscreen.cpp,src/barrier.rs,src/deferred.rs}；ci/{dxil_uc04_device_smoke,realtime_present_smoke,budget_eval,check_guardrails,check_contribution}.py；unsafe-audit/rurix-rt.md（U26/U27 模式）。
- **外部**：DXGI flip-model / `IDXGIFactory2::CreateSwapChainForHwnd` / `ResizeBuffers` / `DXGI_FEATURE_PRESENT_ALLOW_TEARING`；D3D12 static sampler / RTS0 / unbounded descriptor range / SM6.0 资源能力面 / SM6.5 mesh·amplification / legacy ResourceBarrier；D3D11.3 FL quad 导数规则；Vulkan 1.2/1.3 规范（swapchain OUT_OF_DATE、immutable sampler、`VK_EXT_descriptor_indexing`（1.2 core）、`VK_EXT_mesh_shader`、`VK_KHR_acceleration_structure` / `VK_KHR_ray_tracing_pipeline` / `VK_KHR_deferred_host_operations` / `VK_KHR_spirv_1_4`、bufferDeviceAddress）；SPIR-V 1.0/1.4 规范（Image 指令族、`SPV_EXT_descriptor_indexing`、`SPV_EXT_mesh_shader`、`SPV_KHR_ray_tracing`、1.4 OpEntryPoint interface 规则）；SPIRV-Cross / dxc / dxv / spirv-val（既有 pin 集）。

---

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| Draft v1.0 | 2026-07-18 | AI 起草初版：五章（present/采样超集/bindless/render graph/mesh-task-RT）并行起草 + 一致性核查后汇装为单伞形全文（契约 §7 v1.1 裁决，MB1 rfcs/0011 先例）。汇装层落 §4.0 跨章约定（set 轴 set4 起、TextureRw2D 阶段扩 raygen、graph 封闭枚举不可表达面登记、RT descriptor 布局沿采样章形态、命名律、三资源路分工、shim ABI 时间线、probe 条件分支统一措辞、合并序敏感号软化）+ §5.1 新码总分配预测表（6xxx 自 RX6027、3xxx 自 RX3016，RX6008 改接单列）+ §8 RD-034+ 七族合并登记清单。状态 Draft：Agent Approved 待 §9.1 对抗性评审（评审 provenance ≠ 起草）后翻；合入 gated on G-G3-1 | Full RFC（Draft） |
| v1.1 | 2026-07-18 | 对抗性评审（D-409 三镜头 correctness/redline/implementability，评审 provenance `claude-code:claude-opus-4-8` ≠ 起草 `claude-fable-5`）disposition 落实 + blocker G-RED-1 正文收窄（§4.B5/RXS-0229 竞写有界非确定 → 首期**唯一写者纪律**，禁区结构回避非未证内存模型断言）。5 major 正文实改：SC-1（RX6009 随 PR-Me errata 正式 burn，跨 registry 一致性）、SC-2（RXS-0164（+0163/0165）入既有条款加性修订行清单）、E-1（每入口独立版号常量 + `>=` 语义取代精确相等门）、E-2（U 号折叠 U27 + AS/SBT/device-address 切 U30，停止预测 U30~U35）、E-3（混合有界+无界 B 链字节 diff golden 合入门）+ E-5（拆 PR-S0 descriptor 底座独立先落地作四面前置）。7 minor disposition + 轻措辞：SC-3/SC-4/SC-5/SC-6/G-RED-2/E-4。§9.1 填 13 条 findings disposition 表（全采纳无驳回）+ §9.2 补「评审已消化」注。状态 Draft → **Agent Approved**。编号不变（仍 RXS-0220~0249 共 30 条） | Full RFC（Agent Approved） |
