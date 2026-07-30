# RFC-0016 — G5 原生渲染器期伞形:渲染调度 render graph 引擎库 / RHI 图形派发桥 / 虚拟化几何 / VSM / 屏幕探针 GI / 光追与 AS 管理 / 材质场景流送 / 时域重建

| 字段 | 值 |
|---|---|
| RFC 编号 | RFC-0016(4 位制,编号永不复用,10 §9.5;rfcs/README §5 自由池首号;G5_CONTRACT §7 ② claim 登记,number_ledger v1.27 reserved_in_flight[G5]) |
| 标题 | G5 原生渲染器期单伞形 Full RFC 八章:章 A 渲染调度 render graph 引擎库(声明式 pass 读写声明/四趟编译/EB 三轴屏障推导/transient 池化别名/编译期校验/异步 compute 车道/图 dump)· 章 B RHI 图形派发桥执行面(rxrt_rhi_submit gfx pass 自「仅参 barrier 推导」升「真派发」接 vk.rs 图形执行器;VB/IB/descriptor/SPV 入口经 artifacts v2 传递;present handoff 产品化)· 章 C 虚拟化几何(rurix-geom-build 离线 meshlet+层级 DAG / GPU 实例簇两级剔除 / 64 位 VisBuffer SW+HW 双路光栅 / 材质 classify-resolve)· 章 D VSM 虚拟阴影(方向光 clipmap 栈 / 16K 虚拟 128×128 页 / 页标记分配失效 / 共享物理页池 / 多视图深度光栅 / 投影采样)· 章 E 屏幕探针 GI(1/16 均匀探针 / ray query 单反弹 / SH / 平面加权插值 / 3×3 滤波 / 时域累积)· 章 F 光追效果与 AS 管理(BLAS 缓存 / refit 分级 / TLAS 快速重建 / ray query 封装 / RTAO / 硬阴影 / 时域滤波)· 章 G 材质场景流送(单层闭合 32B / GPU scene 扁平化 / PSO precache / 页式流送三预算 / 两级实例化)· 章 H 时域重建(MV/Halton jitter/历史验证公共底座 / TAA / TSR 类超分 / UpscaleBackend trait) |
| 档位 | **Full RFC**(伞形,G4 RFC-0015 单伞形先例:一份 RFC 承载全期各面,一次对抗性评审、一次 Approved 合入即满足各面「RFC Approved 前置」;八章覆盖两个新 crate + RHI 执行面升级 + 七个渲染子系统,跨章一致性约定(§4.0)体量远超 Mini 承载;判档争议向上取严 = Full,硬规则 8) |
| 状态 | **Agent Approved**(2026-07-29;§9.1 对抗性评审〔评审 provenance `cursor:claude-fable-5` ≠ 起草 provenance `cursor:kimi-k3-max`,三镜头,D-409〕完成,7 findings 逐条 disposition〔2 blocker + 4 major 正文实改 + 1 minor 留痕〕,先于任何实现合入,G-G5-2) |
| 承接里程碑 | G5([milestones/g5/G5_CONTRACT.md](../milestones/g5/G5_CONTRACT.md),验收门 **G-G5-2 ~ G-G5-9**;主线 G5.0→G5.5 波次推进,[G5_PLAN.md](../milestones/g5/G5_PLAN.md)) |
| 关联条款 | **预期零新语言语义条款**(渲染器为引擎库,06 §8.3 :149-151 render graph/ECS「它们是库」——不进语言;G5_CONTRACT §7 ④);章 B 复用既有条款面 RXS-0270~0277(图形 RHI 库面)/ RXS-0280~0283(执行面三项)/ RXS-0290~0294(artifacts v2 + Vulkan 通道)与推导单源 RXS-0236~0241,预期零修订;**确需时自 RXS-0297 顺位消费**(number_ledger v1.27 claim;以合并时 spec 实际为准,未消费不占号、不落裸条款头;G4 条件臂未消费两号已 burned 跳号不复用,number_ledger v1.21) |
| 关联 deferred | **RD-036**(C ABI v2 超界硬需求存续,维护对象,本期不兑现)· **RD-034**(DXIL RT 腿 blocked,维护对象;章 F 全走 Vulkan ray query)· 执行期新 RD 自 **RD-037** 起(七报告 P3+/长线评估项登记:Work Graphs/mesh nodes、ReSTIR GI/PT、帧生成 FG/MFG、SVT、Surface Cache/Mesh Card、Mega Geometry 簇级 BLAS、SMRT 完整版、MegaLights、多层材质 slab、Assemblies 全功能、Nanite Foliage/骨骼、NRD/vendor 降噪接入评估等,§8) |
| 依据决策 | D-130(窗口/输入不进语言红线——present 窗口腿维持 C++ shim/运行时层)· D-406 v2.0(agent 完全自主)· D-409(对抗性评审,评审 provenance ≠ 起草)· 06 §8.3(render graph/ECS 是库)· 04 P-01(strict-only)/ P-09(证据压过进度:性能数字 measured 写 evidence 不进硬门)/ P-11(推导单源)/ P-12(克制压过完整性)· RD-034(DXIL RT blocked 维持;步骤 69 探针恒跑) |
| Provenance | `Assisted-by: cursor:kimi-k3-max`(起草)。agent 自主决策;批准前置 = §9.1 对抗性评审完成 |
| Agent 批准 | **Agent Approved 2026-07-29**——§9.1 对抗性评审(评审 provenance `cursor:claude-fable-5` ≠ 起草 `cursor:kimi-k3-max`,三镜头,D-409)完成,2 blocker + 4 major 正文实改 + 1 minor 留痕逐条 disposition(§9.1),先于任何实现合入(G-G5-2) |
| 对抗性评审 | **已完成 第 1 轮 2026-07-29**——见 §9.1;评审 provenance `cursor:claude-fable-5`(主线)≠ 起草 `cursor:kimi-k3-max`(独立子代理上下文),三镜头 correctness/redline/implementability(D-409,硬规则 2 可机验,`ci/check_contribution.py` 规则 4);跨工具独立实例不可得之偏差如实登记 §9.1 环境留痕 |

---

## 1. 摘要

本 RFC 是 G5 原生渲染器期的**单伞形 Full RFC**(G4 RFC-0015 单伞形先例:一次对抗性评审、一次 Approved 合入即满足八面「RFC Approved 前置」)。G5 把「rurix 拥有原生游戏引擎渲染器」自调研结论推进到 measured 工程事实——上游事实源 = **渲染器调研/ 七份调研报告**(2026-07-28,只读),各报告 P0–P2 主线全量落地,P3+/长线评估项登记 RD-037+ 存续。八章:

- **章 A — 渲染调度 render graph 引擎库(报告5 P0–P2,G5.2-A,验收门 G-G5-3)**:新 crate `src/rurix-render`(`#![forbid(unsafe_code)]`,纯 host)——声明式 pass 读写声明(Frostbite 式应用指定线性序)+ **四趟编译**(剔除/生命周期/屏障/车道)+ **EB 三轴 Barrier{sync,access,layout}×before/after 内部规范形式**(AnKi 简化 stage 集)+ **transient 池化别名**(区间不相交共享 + UNDEFINED handoff)+ **编译期校验**(漏声明/越期句柄/读写冲突确定性拒,注入错误声明必被捕获)+ **异步 compute 车道**(FencePair 注入,候选三条件纪律,一键回退)+ **图 dump JSON**。跨帧资源一律 imported 外部资源;流送屏障图外 acquire/release。全部效果章(C~H)的公共前置。
- **章 B — RHI 图形派发桥执行面(G5.2-B,验收门 G-G5-4)**:`rxrt_rhi_submit`(src/rurix-rt-cabi/src/lib.rs:1838)中 gfx pass 自「仅参 barrier 推导」(lib.rs:1883~1885 现状锚,已核)升「**真派发**」——VB/IB/descriptor 自 RHI 资源与 marshalling 槽位传递、SPV 入口自 artifacts v2 入口表(`@__rx_gpu_spirv`,RXS-0291)按名索引,接 vk.rs 既有图形执行器(`run_rhi_graphics_offscreen`,vk.rs:13406,RXS-0272,U31)通道扩面;present handoff 产品化。**复用 RXS-0270~0294 既有条款面,零新语言语义**;既有 compute 路(CUDA/Vulkan,步骤 72~75/76~81)零回归。本章是全部七报告效果的**工程前置**(无真派发则一切效果不出图)。
- **章 C — 虚拟化几何(报告1 P0–P2,G5.2-C+G5.3-C,验收门 G-G5-5)**:离线 crate `src/rurix-geom-build`(meshlet ≤128 tri/簇、层级 DAG 分组简化保边界、自身/父级误差包围球、序列化预留页表字段)+ 运行时 **GPU 实例/簇两级剔除**(视锥/背面锥/LOD cut,HZB 预留——§9 Q-B)+ **64 位 VisBuffer**(u64 = depth30|cluster27|tri7;atomicMax u64 SW 光栅 + HW 间接绘制双路,共享同一写出格式)+ **材质 classify/resolve**(tile 分桶 + 16 位材质槽 ID 窄缓冲)。CPU 参照剔除器(host 蛮力)对拍逐簇一致。
- **章 D — VSM 虚拟阴影(报告3 P0–P1,G5.3-D,验收门 G-G5-6)**:方向光 **clipmap 栈**、16K×16K 虚拟地址空间/128×128 页、32 位页表项、**页标记(主深度屏幕反馈)/分配/失效**三 pass、**共享物理页池**(固定预算,非 sparse binding)、**多视图 shadow_depth_raster**(每 clipmap 级一视图,接口首日按多视图设计)、投影采样(硬阴影;SMRT 降档 §9 Q-C)。
- **章 E — 屏幕探针 GI(报告2 P0–P1,G5.3-E,验收门 G-G5-6)**:**1/16 均匀屏幕探针 + ray query 单反弹 + SH(L1)投影 + 平面加权插值 + 3×3 探针空间滤波 + 时域累积**(章 H 底座复用);追踪层接口 =「**输出命中点辐射度**」统一契约(ray query 本期唯一实现,SDF/ReSTIR 未来同接口可替换);TLAS 与 ray query 封装与章 F **同一份代码**。
- **章 F — 光追效果与 AS 管理(报告4 P0–P1,G5.2-F+G5.3-F,验收门 G-G5-6)**:**AS 管理器**(BLAS 构建缓存网格哈希键/动态 refit 分级决策树/TLAS 快速重建/compaction 时机+显存监控)+ **ray query 封装**(RHI 暴露 AS 句柄与构建/更新命令)+ **RTAO/硬阴影**效果 pass + **时域滤波**(章 H 底座)。DXIL RT 腿维持 blocked(RD-034),**全走 Vulkan ray query**;与离线路径追踪器同结构对拍(同 TLAS 同几何)。
- **章 G — 材质场景流送(报告6 P0–P2,G5.2-G+G5.3-G,验收门 G-G5-7/G-G5-8)**:**单层 principled 材质闭合 32B 定长**(albedo/F0/roughness/normal/emissive 打包)+ GPU MaterialTable + **GPU scene 扁平化实例表**(唯一事实来源,增量更新)+ **PSO precache**(变体预测器 + 加载期后台编译 + **运行时编译告警** demo 侧归零)+ **通用页式流送运行时**(128KB 页、PageRequest 反馈驱动、StreamingBudget{io,transcode,upload} 三预算每帧重置、LRU 页池、staging 图外)+ **两级实例化**(部件实例组,单层限制)。
- **章 H — 时域重建(报告7 P0–P1,G5.2-H+G5.3-H,验收门 G-G5-7)**:**公共底座**(完整 MV + Halton jitter + 深度/法线历史验证 + disocclusion + 邻域裁剪;历史颜色/深度为**外部资源**双缓冲)+ **TAA** + **TSR 类超分**(输入/输出分辨率解耦)+ **UpscaleBackend trait**(自研主实现 + vendor 后端留口)。**禁效果 pass 私写重投影**(代码审计)——章 D/E/F 的时域滤波一律经本底座。

```
apps/uc06-renderer>  meshlet 场景 → 两级剔除 → VisBuffer(SW/HW) → 材质 classify/resolve
                   → VSM + 屏幕探针 GI + RTAO/硬阴影 → TAA/TSR → readback   # G5.4 全管线 demo
```

「引擎采纳」维度显式 carve-out(G5_CONTRACT out_of_scope `production_adoption`);达成表述 =「渲染器工程闭环 measured 落地」,不宣称社会事实。性能预算类数字(GI<2ms@1080p / RT 单效果<1ms / 阴影≤3ms / 图构建<1% 帧 CPU 等报告基线)为对标参考,**measured 写 evidence 不进硬门**(P-09)。

## 2. 动机

G4 close-out 已把「.rx 声明图形 pass、自动 barrier、artifacts v2 SPIR-V 通道、engine_host v3 三方数值相等」做成 measured 工程事实(步骤 76~81)。但「rurix 拥有原生游戏引擎渲染器」还差整层(G5_CONTRACT 定位口径,已核实):

1. **gfx pass 不真派发**——`rxrt_rhi_submit` 阶段 2 只派 compute pass;gfx pass「仅参 barrier 推导」(src/rurix-rt-cabi/src/lib.rs:1883~1885 注释钉死:gfx 派发要 vs/fs SPIR-V + 顶点数据,声明式库面未承载)。vk.rs 图形执行器 `run_rhi_graphics_offscreen`(vk.rs:13406)为 Phase 1 最小面(单 raster pass + color target + 回读),VB/IB/多 pass/纹理 descriptor 供给面未通。这是章 B 的主面,也是一切效果的工程前置。
2. **渲染器模块零存在**——七报告假设的渲染器模块(render graph 引擎库/几何/阴影/GI/RT/流送/时域)在仓库**零存在**(G5_CONTRACT 定位口径已核;各报告 §3 的「rurix 现状映射」表系按假想引擎结构推断,其假设路径如 `render/graph.rs`、`mesh/streaming.rs` 等**均非本仓真实路径**,本期全部新建,不虚构承接)。G3.5 `src/rurix-rt/src/graph.rs` 是 .rx RHI/语言运行时面的图与 barrier 推导(RXS-0236~0241),不是引擎级渲染器库;EI1 `src/rurix-rt/src/rhi.rs` 是 RHI 库面运行时。引擎渲染器层整个缺位。
3. **七报告 P0–P2 主线是收敛的工业共识路径,可执行性已被上游证据钉死**——调度(报告5:Frostbite 声明式 + EB 三轴 + RDG transient/异步语义)、几何(报告1:meshoptimizer clusterlod.h + Bevy 0.14 同语言参照)、阴影(报告3:StratusGFX SVSM 最小实现参照)、GI(报告2:GI-1.0 两级缓存 + Lumen 模块辞典)、RT(报告4:ray query 最小闭环 + Khronos 混合实践)、材质流送(报告6:单层闭合 + PSO precache + 通用页式流送)、时域(报告7:TSR 公开旋钮体系 + 公共底座)。P0–P2 不需要学术创新,需要按 rurix 工程纪律(measured-first、strict-only、推导单源)裁剪落地。

**为何伞形单 RFC**:G4 RFC-0015 先例——同期八面共享一套跨章一致性约定(§4.0:推导层面分工/后端分工/冻结接口/性能口径/unsafe 纪律),一次对抗性评审覆盖全文(D-409),各面失败测试先行判据不变(RFC 合入时点各面 CI 脚本与 crate 在 main 不存在 = RED);章 B 复用 G4 既有条款面,预期零新语言语义条款,伞形 Full 一并承载判档(G5_CONTRACT §7 ④:rfc_required = RFC-0016 单号伞形八章)。

## 3. 指导级解释(用户视角)

### 3.1 章 A — 声明式帧图:读写声明进,屏障出

渲染器作者只声明「每个 pass 读什么、写什么、上哪条车道」,**帧内零手写屏障**(API 为 Draft 拟形,以 §4.0-3 冻结接口为准):

```rust
let mut g = RenderGraph::new();
let depth  = g.create(ResourceDesc::texture("depth", w, h, Format::D32F)); // transient
let vis    = g.create(ResourceDesc::visbuffer(w, h));                       // transient R64Uint
let tlas   = g.import(external_tlas);          // 跨帧/外部资源:import 不进 transient 池
let history = g.import(taa_history);           // 章 H 历史:外部资源双缓冲
g.add_pass(PassDesc::graphics("cull_raster").writes(&[vis, depth]).reads(&[scene, clusters]));
g.add_pass(PassDesc::compute("rtao").queue(Queue::AsyncCompute)             // 异步车道候选
              .reads(&[depth, tlas]).writes(&[ao_raw]));
let plan = g.compile()?;   // 四趟:剔除 → 生命周期 → 屏障(EB 三轴) → 车道(FencePair 注入)
                           // 漏声明写 / 越期句柄 / 读写冲突 → 编译期确定性拒(非运行期炸)
execute(plan);             // transient 别名后峰值 < 无别名峰值;dump() 产 JSON 图
```

### 3.2 章 B — 同一个 .rx 图,真出图

. rx 用户视角**零改源**:G4 的 `apps/uc05-rhi/src/gfx_demo.rx` 形态(raster pass + 访问声明 + present)不变,变化在运行时——`rxrt_rhi_submit` 阶段 2 新增 gfx 派发臂,vs/fs SPIR-V 自 artifacts v2 入口表按名索引、VB/IB 经 cabi 追加绑定传入,vk.rs 图形执行器真 draw,headless readback 像素断言。漏声明/写写冲突维持装配期确定性拒(RXS-0272 面 0-byte)。

### 3.3 章 C~H — 子系统组装

uc06 demo 作者看到的一条管线:geom-build 离线把 glTF 转成 meshlet 包(簇 DAG + 误差球 + 预留页字段)→ GPU scene 实例表(章 G)→ 两级剔除 + VisBuffer(章 C)→ 材质 classify/resolve 查 MaterialTable(章 G)→ VSM(章 D)+ 屏幕探针 GI(章 E)+ RTAO/硬阴影(章 F)→ TAA/TSR(章 H)→ readback。每个子系统都是 render graph 里的声明式 pass(章 A),跨帧状态(历史/页表/探针缓存)一律 import 外部资源。

## 4. 参考级设计

### 4.0 跨章一致性约定(汇装层裁决,八章共同事实源)

1. **推导单源与层面分工(P-11)**:**.rx RHI/语言运行时面**的图与 barrier 推导唯一事实源维持 `src/rurix-rt/src/graph.rs`(RXS-0236~0241)**0-byte**——章 B 派发桥消费其 `PlannedBarrier` **逐字回放,禁二次推导**(U31 契约同);**引擎渲染器库面**(章 A)的 EB 三轴推导唯一事实源 = `src/rurix-render` 的 graph_compile 模块(新建)。两推导**各自服务单一层面**,无第三份:graph.rs 的 AccessKind 状态机服务 .rx 声明面(G3.5/EI1/G4 既有承诺面),graph_compile 的 EB 三轴(sync/access/layout 细分 + AnKi 简化 stage 集)服务引擎库内部规范形式——两者不共享代码、不交叉调度同一张图、不互相喂产物(RFC-0015 §7-1「两推导各服务单一后端」同构裁决,§7-1)。
2. **后端分工(strict 无回退)**:**渲染器主线 = Vulkan**——章 C 的 `VK_KHR_shader_atomic_int64`、章 E/F 的 `VK_KHR_ray_query`、章 A 的 synchronization2 级屏障映射,均为**能力查询 fail-closed**(缺特性 → 确定性 Err,RXS-0193 口径,无静默回退无 mock);既有 **CUDA compute 路(步骤 72~75)零回归 0-byte**;**DXIL RT 腿维持 blocked**(RD-034,步骤 69 探针恒跑),本期 RT 全走 Vulkan ray query。
3. **冻结接口清单(照抄 G5_PLAN §2,G5.2 开工前固化,波次内不得漂移)**:
   - `Barrier { sync_before/after, access_before/after, layout_before/after }`(EB 三轴,AnKi 简化 stage 集)
   - `PassDesc { name, queue: Graphics|AsyncCompute, reads: Vec<ResAccess>, writes: Vec<ResAccess> }` + `ResourceDesc`/`ResourceId`(transient vs imported)
   - `ClusterRecord`(≤128 tri/簇,含锥剔除+误差包围球字段)与序列化布局(预留页表字段)
   - VisBuffer 位格式:u64 = depth:30 | cluster:27 | tri:7
   - `MaterialClosure` 32B 定长(albedo/F0/roughness/normal/emissive 打包)
   - `PageRequest`/`StreamingBudget { io, transcode, upload }`
   - `UpscaleBackend` trait(输入颜色/深度/MV/reactive/曝光 → 输出目标分辨率颜色)
   - 跨帧资源(TAA 历史/VSM 页表/GI 探针历史)一律外部资源 import,不入 transient;流送屏障图外 acquire/release
4. **性能数字口径(P-09)**:本期只做**机制正确性 + 度量埋点**;各阶段 GPU 时间戳、异步重叠量、transient 峰值、流送三预算水位等 **measured 写 evidence 不进硬门**(G5_CONTRACT out_of_scope `perf_budget_hard_gates`);报告预算基线(GI<2ms@1080p / RT 单效果<1ms / 阴影≤3ms / 图构建<1% 帧 CPU)作对标参考记录,BENCH_PROTOCOL 口径另期收紧,不伪造充绿。
5. **P3+ 项 RD 存续清单**:七报告 P3+/长线评估项(清单见 §8 表)**按报告自身建议登记 RD-037+ 存续**(以合入时 deferred.json 实际为准)——不实码、不算失败、不伪造(P-12 克制压过完整性)。
6. **合并序敏感号软化**:RD-037+ / 步骤 82+ / U32+ / 确需 RXS-0297+ 正文一律相对措辞或引拟分配表,以各 PR 合入时 registry/ledger 实号为准(number_ledger 只追加纪律)。
7. **unsafe 与 crate 纪律**:`src/rurix-render` 核心 crate **`#![forbid(unsafe_code)]`**(G5_CONTRACT guardrails);`src/rurix-geom-build` 首选纯 Rust 实现(clusterlod 式算法逐文件移植),确需 FFI(meshoptimizer 类 C 库)则 unsafe 集中登记 **U32 起**;vk.rs 手写 FFI 扩展沿 U26/U27/U30/U31 审计模式 U32 起续号;**新增 cargo feature 仅限透传 gate**(§9.1 R-2 修订:rurix-render/uc06-renderer 的 `vulkan` feature = `rurix-rt/vulkan` 单纯转发,uc04-demo `real-shim` 先例;不新增任何语义性 feature)——rurix-render host 逻辑(四趟编译/校验/dump/CPU 参考器)always-on 纯 host 可测,默认构建(全 feature off)零 GPU 依赖绿维持。

---

### 4.A 渲染调度 render graph 引擎库章(报告5 P0–P2;G5.2-A;验收门 G-G5-3)

> 定位:一切效果章的公共前置——声明式 pass + 自动屏障 + transient 别名 + 编译期校验 + 异步车道。新 crate `src/rurix-render`,纯 host,`#![forbid(unsafe_code)]`。**不新增任何 shader 阶段**(报告5 §5),这是它必须先行的原因。

#### A1. 声明式 pass 与资源面(→ 冻结接口 §4.0-3)

- **Frostbite 式应用指定线性序**(非启发式线性化):pass 按注册序声明,逐 pass 显式 `reads`/`writes`(`ResAccess` 集);资源 = `ResourceDesc`(录制期描述符)+ `ResourceId`(transient vs imported 两类)——录制期只建描述符,执行期才落物理资源(RDG 语义)。
- execute 闭包与声明分离(声明喂编译器,闭包喂执行器);pass 强制携带 name 与源码位置(调试上下文纪律,报告5 §6)。
- **不可变资源免追踪**(Godot RAG 技巧):带初始数据创建且无修改标志的资源不建 AccessTracker,tracker 规模量级下降(静态几何/纹理直接受益)。

#### A2. 四趟编译(graph_compile)

sealed 图经固定四趟产出执行计划,**趟序固定闭合漂移窗口**(RFC-0015 §4.B2 seal→调度→着色→回放四序先例同构):

1. **剔除(cull)**:未被任何输出引用的 pass/资源剔除(RDG pass/resource culling 语义);
2. **生命周期(lifetime)**:逐 transient 资源计算 [首用 pass, 末用 pass] 区间;
3. **屏障(barrier)**:EB 三轴推导(§4.A3),产出逐 pass 边界的 BarrierBatch;
4. **车道(lane)**:异步 compute 候选判定 + FencePair 注入(§4.A5);车道划分改变并发上界时,生命周期区间在该趟后冻结为最终执行计划的输入(单一事实源)。

#### A3. EB 三轴屏障推导

- **内部规范形式**:`Barrier { sync_before/after, access_before/after, layout_before/after }`(D3D12 Enhanced Barriers 三轴模型;§4.0-3 冻结接口),Vulkan 后端映射 `vkCmdPipelineBarrier2` 族;**stage/access 枚举采 AnKi 简化集**——PC/主机实证收敛为 graphics/compute 两大 stage(传输队列正交另计),25+ stage 完整枚举的组合爆炸挡在引擎内部。
- 推导算法(Frostbite/Granite 参照):逐资源追踪「上一次使用的 sync/access/layout」,pass 声明新使用时 layout 变化或读写冲突则发射屏障;连续只读链 fake flush(access=0)避免无效缓存失效;**EB 实用规则两条入规范**:buffer 同队列内可并发读写无中间屏障(indirect 参数缓冲/计数器受益);布局转换可 split barrier 提前发射隐藏延迟(车道趟注入点)。
- **golden 锚定**:手算期望屏障序列作 golden,推导输出逐条比对(host 单测,G-G5-3)。

#### A4. transient 池化别名 + 峰值审计

- **别名**:生命周期区间不相交的 transient 共享同一物理分配(池化);**handoff 正确性靠屏障**——旧主 `access_after = NO_ACCESS` 结束,新主 `layout_before = UNDEFINED` + DISCARD 进入(EB aliasing barrier 示例),GPU 可丢弃残留缓存写。
- **尺寸/对齐分池**(RFC-0015 §4.B1 三分量着色先例同思想):同槽组按最大尺寸+最大对齐分配,逐成员核满足性,不满足者不入共享槽。
- **峰值审计**:逐池记录每帧最高水位(high_water);验收 = 别名后峰值 < 无别名峰值**非平凡成立**(≥2 对不相交 transient 的 demo 图,host 单测,G-G5-3)。

#### A5. 编译期校验(RED 自检)

- 三类违例**编译期确定性拒**(库层错误枚举,零新 RX 码,§5.1):① **漏声明**(pass 实际读写未在 reads/writes 声明——执行器回调声明集与实际资源访问核验);② **越期句柄**(句柄与生命周期绑定,末用后访问直接报错——RDG 纪律照抄,别名共享下防花屏);③ **读写冲突未声明**(同 pass 同资源读写反馈/写写冲突)。
- **RED 自检**:注入错误声明(漏声明写/声明后越期使用/读写冲突未声明)**必被图编译器捕获**(RDG setup 验证翻版;反 YAML-only——校验器被桩化则自检红)。

#### A6. 异步 compute 车道(FencePair 注入)

- 调度模型(RDG 语义):pass 标 `AsyncCompute` 后,沿依赖图找图形管线最后一个生产者插 fence(`FencePair { signal_after, wait_before, value: u64 }`,timeline semaphore),异步段跑完在首个图形消费者 join 回图形队列;不支持异步的平台自动回落图形队列。
- **候选三条件纪律**(报告5 §2.4,三方证据收敛):**时长 ≥0.5ms 量级、无图形管线依赖(只读 GBuffer/只写自有缓冲)、消费者距离生产者足够远**;首批候选 = AO 计算与滤波/GI 滤波与探针更新/降噪空间滤波趟;明确不上异步 = 主光栅、阴影渲染、间接参数准备。异步段参数准备必须图内自包含(bindless 或常量一次绑齐,不读图形队列中间状态)。
- **验收口径**(§9 Q-D):机制正确性(fence 注入/车道划分/回落)入硬门 host 单测 + **一键回退开关**;重叠量时间戳 measured 写 evidence 不进硬门(vkguide「未必有收益」与 Godot 5–15% 纪律,P-09)。

#### A7. 图 dump JSON

每帧可导出 JSON 图(pass/资源/区间/屏障/车道全量),喂观测与「这一帧为什么没重叠」事后查询;G-G5-3 验收「图 dump(JSON)可产」。

---

### 4.B RHI 图形派发桥执行面章(G5.2-B;验收门 G-G5-4;复用 RXS-0270~0294 既有条款面)

> 现状锚(已核):`rxrt_rhi_submit`(src/rurix-rt-cabi/src/lib.rs:1838)阶段 1 装配核验 + gfx pass `add_gfx_pass` 注入 graph 参与 `derive_barriers`;阶段 2 派发循环只派 compute pass——「gfx pass 派发要 vs/fs SPIR-V + 顶点数据,`.rx` RHI 声明式库面未承载 → gfx pass 仅参 barrier 推导」(lib.rs:1883~1885 注释)。vk.rs `run_rhi_graphics_offscreen`(vk.rs:13406,RXS-0272,U31)= Phase 1 最小面(单 raster pass 写 color target + barrier plan 逐字回放 + readback,禁二次推导)。

#### B1. 供给面升级:VB/IB/descriptor/SPV 入口

- **SPV 入口**:gfx pass 的 vs/fs 着色函数名经既有 `rxrt_rhi_raster_pass(vs, fs)`/`rxrt_rhi_mesh_pass(ms, fs)` C 字符串面传入(RXS-0270 0-byte);执行期自 artifacts v2 入口表(`@__rx_gpu_spirv`,RXS-0291;`DeviceArtifactSet` 按名索引,RXS-0292)取 SPIR-V 模块——**G4 已通的同一通道,零新机制**。
- **VB/IB 绑定**:顶点/索引缓冲自 RHI buffer 资源(kind-2 槽位既有面,EI1 marshalling 机制复用)经 **cabi 追加式符号**绑定进 gfx pass(RXS-0194 追加式 0-byte 语义口径;句柄生命周期沿 `drop_rhi_children` 级联清理模式);首期显式 VB/IB 形态,vertex pulling 归 §9 Q-E/RD-037+。
- **descriptor 供给**:纹理/采样器/TextureTable 沿 RXS-0271/0273/0276 既有声明面与 vk descriptor 映射(RXS-0208)执行;声明↔反射双向相等装配期拒(RXS-0273 0-byte)。

#### B2. 双通道裁决与真派发臂(§9.1 R-1 修订)

- **主通道(本期硬门,G-G5-4)= 引擎库 Rust 级多 pass 图形执行器**:vk.rs 同 crate 新模块(`render_exec`,feature `vulkan` gate)——接受 SPIR-V 模块/资源描述/pass 序列(raster/compute 混合)/屏障计划(由章 A graph_compile 产出,执行器逐字回放禁二次推导)/readback 请求,内建 pipeline cache 与能力探测(`VK_KHR_shader_atomic_int64` 查询面);**既有 `run_rhi_graphics_offscreen` 及 G3 `run_graphics_offscreen*`/`run_mesh_offscreen`/`run_graph_offscreen` 入口 0-byte 语义**,扩面为同文件/新模块新入口,沿 U31 审计模式登记 U32+。渲染器七章效果全部经此通道出图——它是「rurix 原生渲染器」的执行脊柱。
- **次通道(条件臂)= .rx 声明式 gfx submit 真派发**:`rxrt_rhi_submit` 阶段 2 gfx 派发臂(供给面 B1)——缺口小则本期落地;实现期核实缺口涉 rurixc lowering 扩面(声明式库面承载 VB/IB 数据)则**如实登记 RD-037+ 存续,不降判据不伪造**(RD-034 honest 存续先例);门 G-G5-4 以主通道满足。
- **判据**:≥1 raster pass 绘制三角形 device 真跑,**非空着色器清色不变量** + headless readback 像素断言(RTX 4070 Ti,`RURIX_REQUIRE_REAL=1`;G-G5-4)。

#### B3. present handoff 产品化

- RXS-0274 终端声明的执行面补齐:present 前布局迁移 + ① headless readback 判据(RXS-0222 纪律,CI 主判据)② 窗口腿维持 RXS-0197/0198 typestate + C++ shim(D-130 红线 0-byte,BLACKHOLE G4.6 已 device 见证的 `present-real` feature 链复用)。
- 语义本体零新增——既有条款面的执行兑现,非新承诺。

#### B4. 零回归与条款面

- **既有 compute 路零回归**:CUDA 路(步骤 72~75)0-byte;Vulkan compute 腿(RXS-0293,步骤 80)0-byte;gfx 面既有判据(步骤 76~78)0-byte 只增。
- **条款面**:访问声明/barrier/present/artifacts v2/Vulkan 通道语义由 RXS-0270~0277/0280~0283/0290~0294 全覆盖,**预期零新条款零修订**;实现期若发现执行面语义确超既有字面(如 VB/IB 绑定声明面需锚定),走 §5 条件消费路径(spec/rhi.md 追加式修订行或自 RXS-0297 顺位),不留裸条款头。

---

### 4.C 虚拟化几何章(报告1 P0–P2;G5.2-C + G5.3-C;验收门 G-G5-5)

> 定位:Nanite 类路线的工业共识路径裁剪——离线 meshlet+DAG、GPU 两级剔除、VisBuffer 双路光栅、材质 classify/resolve。**mesh shader 第三路径不做**(报告1:Bevy 纯计算管线证明非必需,优化项非地基,§7-3)。

#### C1. 离线构建 crate `src/rurix-geom-build`(报告1 P0)

- 纯 host 离线工具 crate(首选纯 Rust;FFI 评估见 §4.0-7):任意输入网格 → meshlet 化(**≤128 tri/簇**,验收 128±20%;簇生成原则 = 顶点局部性优先、簇界对齐剔除粒度)→ **层级 DAG**(邻簇分组 + 保边界简化锁定组边界顶点防裂缝 + 再分簇递归)→ **自身/父级误差包围球** → 序列化布局(**预留页表字段**:页号/常驻标志,P4 流送反向约束——流送前全部页标常驻)。
- **`ClusterRecord`**(§4.0-3 冻结):顶点/索引池偏移、三角形数、剔除球(中心 + 8bit 半径缩放)、法向锥(轴 + 8bit 角 + 8bit 顶点偏移,3B 剔除字段方案)、自身/父级 LOD 误差球、材质槽 ID、页号(预留);32–48B/簇量级。
- 验收:100% 输入网格转换成功;三角形守恒、边界锁定、包围球包含性单测(host,G-G5-5 前半)。

#### C2. 运行时 GPU 两级剔除 + LOD cut(报告1 P1)

- `instance_cull`(1 线程/实例)+ `cluster_cull`(1 线程/簇,subgroup 压缩):视锥 + 背面锥剔除;**LOD cut 判定**并入簇剔除——每簇并行检查「自身误差不可感知(<1px)且父级误差可感知」,恰构成 DAG 上一个 cut,无需簇间通信。
- `compact_draw_args`:分箱计数 → DispatchIndirect/DrawIndirect 参数(单线程组前缀和);**CPU 只发一次 draw** 的 GPU-driven 形态。
- **HZB 预留**(§9 Q-B):剔除 pass 的 HZB 输入接口与 mip 链资源面预留;两阶段 HZB(首遍/重建/补漏)为 P2 尾条件臂。
- **CPU 参照剔除器**(host 蛮力逐簇视锥/背面锥)与 GPU 剔除**逐簇一致对拍**(device,G-G5-5);遮挡剔除(若条件臂落地)允许保守误差但不得漏可见簇。

#### C3. 64 位 VisBuffer + SW/HW 双路光栅(报告1 P2)

- **VisBuffer**:render graph transient **u64 storage buffer**(width×height 元素;§9.1 R-5 修订——`R64_UINT` image 的 64 位 image 原子支持面窄,storage buffer 的 `shaderBufferInt64Atomics` 是 `VK_KHR_shader_atomic_int64` 主承诺面),像素负载 **u64 = depth:30 | cluster:27 | tri:7**(§4.0-3 冻结;无符号整数原子比较同时完成深度测试与可见性记录)。
- **SW 光栅**(小三角形):compute pass,组内先逐顶点变换入 group shared,再 1 线程/三角形 scanline,**atomicMax u64** 写 VisBuffer;硬性需求 `VK_KHR_shader_atomic_int64`(能力查询 fail-closed,§4.0-2);深度位采用与硬件深度缓冲相同量化(SW/HW 一致性前提)。
- **HW 光栅**(大三角形):复用既有图形通路 + DrawIndirect 间接绘制,**与 SW 路共享同一 64 位写出格式**;SW/HW 分箱阈值(边长 ~32 像素级参照)可调参。
- 验收:SW/HW 同场景**逐像素 diff 容差 = 0(整数域)**(G-G5-5);SW/HW 三角形计数可视化。

#### C4. 材质 classify / resolve

- `mat_classify`:VisBuffer → tile×材质分桶列表(逐 tile 原子计数 + 前缀和,标准 GPU 分桶);`mat_resolve`:分桶列表 + MaterialTable(章 G)→ **16 位材质槽 ID 窄缓冲** → GBuffer(UE5 窄缓冲 40% 提速路径参照——材质解析为独立窄缓冲 pass,非从 VisBuffer 反查)。
- 输出接章 G 单层材质闭合求值与章 D/E/F 光照输入;材质 resolve 输出对拍参考(G-G5-5)。

---

### 4.D VSM 虚拟阴影章(报告3 P0–P1;G5.3-D;验收门 G-G5-6)

> 定位:以 StratusGFX SVSM 为骨架模板、UE VSM 文档为规格说明;方向光直接以 clipmap VSM 起步(不建 CSM);**SMRT 软阴影降档**(§9 Q-C)。

#### D1. clipmap 栈与虚拟地址空间

- 方向光 **clipmap 栈**:以相机为中心、半径逐倍扩大的虚拟图组(UE 默认 6–22 级参照,空级几乎免费;级数按地平线距离可裁剪);每级 = 16K×16K **虚拟地址空间**,物理分配按 **128×128 页** 按需。
- **32 位页表项**(StratusGFX 参照):帧标记/物理页索引/驻留位/脏位;页表纹理每灯每级 KB 级,常驻跨帧。

#### D2. 页标记/分配/失效三 pass

- `shadow_page_mark`:主深度缓冲逐像素反投影到阴影空间,计算所需页坐标与 clipmap 级,标记 used(屏幕反馈);
- `shadow_page_alloc`:页标记 → 紧凑请求列表 + 物理页分配/驱逐(**LRU 带帧龄延迟**;近处级优先、新页优先于驱逐);
- `shadow_invalidate`:失效三源——图元移动(标脏包围盒覆盖页)/灯移动(标脏全图)/级联原点切换(标脏环形更新带);**本期保守失效起步**(P0/P1 简化),事件管线与章 G scene 变更通知共用同一事件总线(避免两套脏标记)。

#### D3. 共享物理页池与多视图深度光栅

- **共享物理页池**(固定预算纹理数组,128²×N 页;**非 sparse binding**——共享池起步驱动行为更一致,StratusGFX 双后端经验,§7-5);页池水位/驱逐率进度量埋点。
- **多视图 `shadow_depth_raster`**:脏页列表 → 物理页池深度,**每 clipmap 级一个视图**;接口首日按多视图设计(VSM = 章 C cluster 剔除管线的多视图客户——报告3 §2.5,P4 合流预留;视图数超上限拆 pass 的容量规划照 `MAX_VIEWS_PER_CULL_RASTERIZE_PASS` 教训)。
- **投影采样**(`shadow_project` 并入光照着色):页表 + 物理池 → 阴影可见性(硬阴影;SMRT 降档 §9 Q-C)。
- 页表/物理页池 = **跨帧外部资源 import**(§4.0-3 纪律);验收 = 页表分配/失效正确性 host 单测 + device 深度对拍(G-G5-6)。

---

### 4.E 屏幕探针 GI 章(报告2 P0–P1;G5.3-E;验收门 G-G5-6)

> 定位:GI 内核最小闭环——屏幕探针 + ray query + 时域累积;Lumen 当模块辞典不当产品目标(报告3 §6 同款克制)。

#### E1. 探针管线(全 compute pass)

- **1/16 均匀屏幕探针**(每探针少量光线)→ **ray query 单反弹追踪**(章 F 同一份封装,TLAS 由 AS 管理器供给)→ **SH(L1)投影**(带边八面体参照 SimLumen,硬件双线性采样友好)→ **平面加权插值**(plane-aware,薄几何泄漏缓解)→ **3×3 探针空间滤波**(等效大核屏幕滤波)→ **时域累积**(章 H 公共底座,禁私写重投影)。
- 重要性采样首期 = BRDF PDF 单因子(光照 PDF 上一帧重投影归 P2 后,如实标注)。

#### E2. 追踪层统一契约

- 接口 =「**输出命中点辐射度**」:ray query 为本期唯一实现;SDF 软追踪/ReSTIR 未来实现同一接口,追踪层可替换(报告2 §3.2 阶段不变量,P0 冻结)。
- 探针缓存历史(SH + 深度锚点)= 跨帧外部资源双缓冲;GI 支路为异步车道首批候选(章 A6 三条件满足时)。

#### E3. 验证与 device 腿条件臂(§9.1 R-3 修订)

- **能量守恒检查**:关滤波关累积仅单反弹,对比参考间接能量曲线——不凭空造能/丢能;
- device 真跑与 **CPU 参考追踪器方向一致性对拍**(G-G5-6);滞后与闪烁分离度量(运动序列逐帧 SSIM + 变化区域响应时间)。
- **device 腿条件臂**:`rayQueryEXT` 在 compute shader 的 SPIR-V 编码依赖工具链供给面(rurixc vulkan_codegen RT 面现为 `emit_*_min` 见证形态)——ray query 编码通道实现期核实:通则 GI/RTAO device 腿全量;**不通则 device 腿降档为「vk.rs G3.6 RT 底座(`run_ray_tracing_offscreen` 形态)最小见证 + host 参考器全量对拍」并登记 RD-037+ 存续,不伪造 device 绿**;门 G-G5-6 的 GI/RT 面以「AS 管理 host 单测 + CPU 参考对拍 + device 腿到其真实证据边界」满足(measured-first / blocked-honest 高于全量表述,G4 先例)。

---

### 4.F 光追效果与 AS 管理章(报告4 P0–P1;G5.2-F + G5.3-F;验收门 G-G5-6)

> 定位:最小 RT 闭环 = TLAS 管理 + ray query 效果 + 时域滤波;RT pipeline/SBT 不做(§7-4);DXIL RT blocked 维持(RD-034),全走 Vulkan ray query。

#### F1. AS 管理器(「加速结构债」正面回应)

- **BLAS 构建缓存**:按网格哈希键复用(静态几何一次构建 + compaction);
- **动态 refit 分级决策树**:refit 用于变形、rebuild 用于顶点数变化的拓扑改变(Khronos 混合实践基线),策略显式成文 + 监控;
- **TLAS 快速重建**:实例数 <10k 亚毫秒级参照;实例增删 compaction 时机成文;AS 显存占用有界监控。
- RHI 面:AS 句柄 + 构建/更新命令 + scratch 管理——G3.6 vk.rs AS/SBT 既有运行时面(RXS-0248,U30 审计面)复用扩注,U32+ 登记。

#### F2. ray query 封装与效果 pass

- 封装 = `accelerationStructureEXT` 描述符 + `rayQueryEXT` 指令流的着色器工具面(章 E **同一份代码**——「GI 一套 RT、阴影一套 RT」分裂否决,报告4 §3.1);**无需 raygen pipeline/SBT/命中着色体系**。
- 效果:**RTAO**(GBuffer 法线半球余弦采样,Vulkan 官方教程形态)+ **硬阴影**(向光源一根光线,输出可见性缓冲);效果缓冲抽象(可见性/AO 辐射度契约)。
- **时域滤波**:章 H 公共底座(重投影 + 历史验证 + 邻域统计),禁效果 pass 私写重投影;静态场景收敛(帧间差趋零,G-G5-6)。

#### F3. 验证

- **同结构对拍**:与离线路径追踪器**同 TLAS 同几何**逐像素对比(几何错误与采样错误不互相甩锅);动态序列回归(移动物体无漏光);AS 构建时间与显存进度量埋点。

---

### 4.G 材质场景流送章(报告6 P0–P2;G5.2-G + G5.3-G;验收门 G-G5-7/G-G5-8)

> 定位:材质定型 + 变体不炸 + 场景 GPU 化 + 资源流送——四条线共享「驻留按页、请求由渲染反馈驱动、每帧工作量预算化」一套思想;**先建通用流送运行时,再接资源类型**。

#### G1. 单层 principled 材质闭合(32B 定长)

- **`MaterialClosure` 32B 定长**(§4.0-3 冻结):albedo/F0/roughness/normal/emissive 打包 + flags;参数有明确定义域与默认值(原理化参数化 = 未来自动降阶前提,Substrate 教训);**单层闭合**——多层 slab 混合/分层归 P3+(§8),预留拓扑字段(coverage weight)位。
- **MaterialTable**:GPU 定长数组,材质 ID 索引(章 C classify/resolve 直接消费;逐像素材质分类着色 = 唯一渲染路径,Blendable 等价物思想——固定字节、速度可预期)。

#### G2. GPU scene 扁平化

- 实例表 SoA:`InstanceRecord`(变换/包围球/网格 DAG 根句柄/材质 ID/部件组指针,64–96B/实例量级)GPU 常驻;scene 变更**增量更新**;**唯一事实来源**(章 C 剔除、章 F BLAS/TLAS 实例、章 D 失效事件同源消费)。
- **两级实例化**:实例表项 = 网格 | 部件实例组(`PartInstanceGroup { part_mesh, transforms_base, count }`)——剔除 descent 多一次间接寻址 + 变换复合;限制照抄 Assemblies:**单层、不跨 assembly 去重**(P2 最小面;全功能归 §8/RD-037+)。

#### G3. PSO precache + 运行时编译告警

- **变体预测器**:材质 × 几何类型 × pass 集(深度/阴影/velocity/base 有限集)→ shader key 预测集,加载期后台异步编译;**permutation 源头减量**(按项目渲染需求关整类特性置换的纪律成文)。
- **运行时编译即告警**:渲染线程上发生编译 = 告警进日志 + CI 断言;**demo 侧告警归零**(G-G5-7)。

#### G4. 通用页式流送运行时(报告6 P1–P2)

- **页式运行时**(资源类型无关):**128KB 页**槽位固定池 + LRU;**`PageRequest { resource_id, page_index, priority, frame_requested }`** 反馈驱动(章 C LOD cut 选中未驻留页 → 请求队列;优先级类目);**root page(DAG 顶层)常驻**——永远有可渲染的东西(章 C 序列化预留字段兑现);**staging 图外上传**,消费点以 acquire 屏障接入图内(章 A 纪律)。
- **`StreamingBudget { io, transcode, upload }` 三预算每帧重置**(Fast Geometry Streaming 思想):超支告警;页池水位/驱逐率/pop-in 计数进度量埋点(数字不进硬门,§4.0-4)。
- 纹理层(§9.1 R-4 修订降档):几何页与纹理页 = 两种注册资源类型接入同一页式运行时;页解包**确定性**(解包页与离线参考逐字节一致)host 单测;**KTX2/BasisU 真转码器接入归 RD-037+**(第三方转码面超本期时间盒),本期页 payload 为未压缩/简单打包档,转码分档接口留口。

---

### 4.H 时域重建章(报告7 P0–P1;G5.2-H + G5.3-H;验收门 G-G5-7)

> 定位:公共底座一次投资处处受益——TAA/TSR/章 D 阴影滤波/章 E GI 累积/章 F RT 降噪全部经同一底座;**禁效果 pass 私写重投影**(代码审计,G-G5-7)。

#### H1. 公共底座

- **完整 MV**:屏幕空间速度 RG16F 随主几何 pass MRT 输出;几何速度主面本期实载,**蒙皮/WPO 通道预留**(接口按三类速度设计;骨骼/植被资产为 P3+ 项,本期无资产验证——§8 如实标注);
- **Halton jitter** 进投影矩阵(全局 uniform);
- **历史验证三件套**:深度/法线一致性测试 + **disocclusion 检测** + **邻域裁剪**(AABB/variance clipping,鬼影直接克星);
- 历史颜色/深度 = **外部资源双缓冲**(R11G11B10 带宽档 / RGBA16F 质量档;历史分辨率可独立超采样,§4.0-3 纪律)。

#### H2. TAA 与 TSR 类超分

- **TAA** = 底座上的薄 pass(重投影 + 验证 + 邻域裁剪 + 累积);
- **TSR 类超分**:输入/输出分辨率解耦——**与 TAA 同一 kernel,仅分辨率映射不同**;机制照抄公开旋钮面:闪烁时域分析(按**时长**判定、与目标帧率解耦)、收敛加速、拒绝抗锯齿质量档分档;**不做锐化**(锐化归 tonemapper 后可选 pass)。
- **reactive mask 双通道**:自动通道(透明/粒子 pass R8 附加输出)+ 手工通道(材质级「永不累积」标记);与深度/法线后验验证取并集。

#### H3. UpscaleBackend trait

- 冻结接口(§4.0-3 照抄):输入颜色/深度/MV/reactive/曝光 → 输出目标分辨率颜色;**自研 TSR 类 = 主实现**(任何平台保底);**vendor 后端留口**(FSR 3.1 开源/DirectSR 形态参照;本期不接 SDK,接入评估归 RD-037+);帧生成 FG/MFG = P3+ 独立层(§8)。
- 验收:TAA/TSR 静态场景收敛对拍超采样参考(**SSIM 门禁**,host 参考实现);伪影回归集(disocclusion/薄几何/高速运动/透明后运动)入库。

---

## 5. 下游 spec 条款映射(spec diff,10 §3 要件)

**预期零新语言语义条款**(渲染器为引擎库,06 §8.3;G5_CONTRACT §7 ④)。章 B 为既有条款面的执行兑现,非新承诺;章 A/C~H 为引擎库内部面(crate 内部契约,非 spec 条款面)。**条款先行纪律对条件消费路径保持**(硬规则 7):实现期确需新条款时,spec commit 先于实现 commit,自 **RXS-0297** 顺位消费(number_ledger v1.27 claim;G4 条件臂未消费两号已 burned 跳号不复用);**未消费不占号、不落裸条款头**。

| 章 | 既有条款复用面(零修订承诺) | 条件消费路径(确需时) |
|---|---|---|
| B | RXS-0270(gfx pass 类型面)/ 0271(资源面)/ 0272(访问声明+自动 barrier,U31)/ 0273(反射相等)/ 0274(present 库化)/ 0276(bindless)/ 0280~0283(执行面三项)/ 0290~0292(artifacts v2)/ 0293~0294(Vulkan 通道+device 见证);推导单源 RXS-0236~0241;cabi 追加 RXS-0194 口径 | VB/IB 绑定声明面确需锚定 → spec/rhi.md 追加式修订行;超出 → RXS-0297 顺位 |
| A/C~H | 无(引擎库内部面;冻结接口 §4.0-3 = crate 内部契约) | 确需语言语义时先判档(争议向上取严)→ RXS-0297 顺位 + RFC 修订行留痕 |

### 5.1 新错误码策略(预测;合并时以 registry 实号为准)

**预期零新 RX 码**(number_ledger v1.27 claim):渲染器库面违例走**库层错误枚举/状态值**(Rust `Result`,镜像 RX6029/6030「图违例走库面诊断」口径的非 RX 码侧);rurix-geom-build 离线工具违例走退出码 + 诊断文本。确需时:codegen 自 RX6034 续 / 工具类自 RX7023(en/zh message-key 成对,registry/error_codes.json 只追加)。

## 6. feature gate / tracking / 实现序(10 §3 要件)

### 6.1 前置与失败测试先行

- 本 RFC **Approved 合入先于任何实现 PR**(G-G5-2,10 §3 硬性);**失败测试先行**(反 YAML-only):RFC 合入时点,`ci/renderer_graph_smoke.py`(步骤 82 拟)、`ci/renderer_draw_smoke.py`(83 拟)、`ci/renderer_visbuffer_smoke.py`(84 拟)、`ci/renderer_lighting_smoke.py`(85 拟)、`ci/renderer_temporal_smoke.py`(86 拟)、`ci/uc06_renderer_smoke.py`(87 拟)、`src/rurix-render` crate、`src/rurix-geom-build` crate、`apps/uc06-renderer`、各面 shader 语料与 host 参考器在 main **均不存在 = RED**(脚本名为拟名,随实现 PR 定案;步骤号一旦占用不复用,多余号作废声明 burned)。

### 6.2 feature gate 总裁决

零新 cargo feature、零语言 gate(§4.0-7):rurix-render/rurix-geom-build 为 always-on host 库面;device 执行经 rurix-rt 既有 `vulkan` feature 与 `vulkan-backend` codegen 面(RFC-0015 §6.2 工具链构建面口径沿用);默认构建(全 feature off)零 GPU/SDK 依赖绿(clippy/test 矩阵双验沿 G3/EI1/G4 惯例)。

### 6.3 波次 PR 计划(照 G5_PLAN §1 波次;条款 commit 先行 + 实现同 PR,G3/EI1/G4 结构先例)

- **G5.2 底座六面(并行)**——PR-A 章 A render graph 底座(四趟编译/EB 屏障 golden/别名峰值/校验 RED 自检/dump;步骤 82)+ PR-B 章 B 派发桥(供给面 + 真派发臂 + present;步骤 83)+ PR-C 章 C 离线 geom-build(meshlet/DAG/序列化/CPU 参照剔除器)+ PR-D 章 G 前半(材质闭合/MaterialTable/GPU scene/PSO precache)+ PR-E 章 H 前半(MV/jitter/历史验证底座 + TAA)+ PR-F 章 F 前半(AS 管理器 + ray query 封装)。集成门 = 全 workspace build/test 绿 + G-G5-3/G-G5-4。
- **G5.3 效果六面(并行,gated on G5.2)**——PR-G 章 C 运行时(两级剔除/VisBuffer 双路/classify-resolve;步骤 84)+ PR-H 章 D VSM + PR-I 章 E GI + PR-J 章 F 效果(RTAO/硬阴影 + 时域滤波;步骤 85 三面合)+ PR-K 章 H TSR + UpscaleBackend(步骤 86)+ PR-L 章 G 流送运行时 + 两级实例化。集成门 = G-G5-5/G-G5-6/G-G5-7。
- **G5.4 合流**——PR-M `apps/uc06-renderer` 全管线 demo + 步骤 87 + evidence schema + g5_budget counter(evaluator 分支同 PR)+ P3+ 项 RD-037+ 登记(G-G5-8)。
- **G5.5 close-out**——PR-N 全量回归冻结 + 门终审表 + RD/SG 处置 + status flip(G-G5-9)。

### 6.4 每 PR 不变量核验(全期硬约束)

既有零回归:dxil 套件恒定 / vulkan 套件 grow-only / 步骤 41~81 既有判据 0-byte 只增(步骤 69 blocked 探针恒跑 / 步骤 70 永久 gap)/ compute RHI 路(步骤 72~75)与 gfx 面(步骤 76~81)零回归 / engine_host v1·v2·v3 资产 0-byte。LF byte-exact;counter/entries 不预造(与 evaluator 分支同 PR);device measured + run URL 归 G5_CONTRACT §8 面;`RURIX_REQUIRE_REAL=1` 贯穿 device 段(缺 provisioning SKIP = dev-env degrade,mock/SKIP 不充绿);trace 全程全锚定(条件消费条款落时);新 unsafe U32+ 登记;GPU 实验全经 proc_guard;evidence/ 只增不删。

## 7. 备选方案

1. **rurix-render 复用 G3.5 graph.rs 作推导**——**否决**:层面错位——graph.rs 是 .rx RHI/语言运行时面的 AccessKind 状态机(RXS-0236~0241 既有承诺面 0-byte),EB 三轴(sync/access/layout 细分 + AnKi 简化 stage + split/aliasing barrier)是引擎库内部规范形式;crate 边界独立(`#![forbid(unsafe_code)]`);两推导各自单源服务单一层面,不复制不共享(§4.0-1)。
2. **全自动命令追踪式图(Godot RAG 自动派)**——**否决**:声明式零迁移成本 + 编译期可校验(注入错误声明必被捕获);全自动派调试上下文丢失为作者自认弱点;其两技巧(不可变免追踪/层级分组)已摘入章 A。
3. **mesh shader 第三光栅路径**——**否决(延后)**:Bevy 纯计算管线达实用性能证明非地基;Vulkan 侧多厂商扩展行为差增加调试面;归 P3+ 评估(§8)。
4. **RT pipeline/SBT 首期满载**——**否决**:SBT 记录布局/管线库/双后端语义差是十倍级工程量;ray query 覆盖阴影/AO/GI 探针全部本期需求(NRD/RTXDI 硬件门槛即 inline trace);「命中点需多样化材质着色」真实出现时再评估(与 GI hit lighting 同步)。
5. **VSM 页池走 sparse binding(VK_KHR_sparse_binding)**——**否决(首期)**:共享物理页池驱动行为更一致(StratusGFX 双后端经验);sparse 只给机制不给策略,留作优化项。
6. **通用材质图/多层 Substrate 首期**——**否决**:Epic 默认 Blendable(固定内存、速度可预期)证据;单层闭合 + 拓扑字段预留保升级路径;多层归 P3+。
7. **GPU 侧调度(Work Graphs/mesh nodes)入图架构**——**否决**:工具链成熟度与 Vulkan 缺位(2026 仍评估项);图架构预留「pass 内部提交单元可替换」接缝,不为它改抽象;归 P3+ 与报告1 P4 评审合并(§8)。

## 8. 不做(范围红线)

| 不做项 | 理由(摘) | 登记去向 |
|---|---|---|
| 七报告 P3+/长线项:Work Graphs/mesh nodes、ReSTIR GI/PT、帧生成 FG/MFG、SVT/RVT、Surface Cache/Mesh Card、Mega Geometry 簇级 BLAS、SMRT 完整版、MegaLights、多层材质 slab、Assemblies 全功能(嵌套/跨 assembly 去重)、Nanite Foliage/骨骼(Skinning)、曲面细分/位移、cluster 流送 P4(压缩/父页引用/超显存)、世界辐射缓存(GI P2)、自适应探针(GI P3)、SDF 软追踪(GI P4)、RT pipeline/SBT(RT P2)、SER/OMM、ReSTIR DI 多灯(RT P3)、NRD/vendor 降噪接入、DirectSR/FSR 3.1 SDK 接入、蒙皮/WPO MV 资产验证 | 报告自身建议 + P-12 克制;本期 P0–P2 主线全量落地,P3+ 不实码不伪造 | **RD-037+**(执行期逐条登记,以合入时 deferred.json 实际为准) |
| 性能预算硬门(GI<2ms/RT<1ms/阴影≤3ms/图构建<1% 帧 CPU) | 机制正确性优先;measured 写 evidence 不进硬门(P-09);BENCH_PROTOCOL 另期收紧 | G5_CONTRACT out_of_scope 维持 |
| DXIL RT 腿 | spirv-cross/LLVM 双上游钳制;步骤 69 探针恒跑,翻绿 = 复评信号 | RD-034 维持 open |
| 窗口/输入进语言;render graph/ECS 进语言 | D-130 红线;06 §8.3「它们是库」 | 红线维持 |
| mesh shader 第三光栅路径 | 报告1:优化项非地基(§7-3) | RD-037+(随 P3+ 评估) |
| HZB 两阶段剔除(条件臂不达时) | §9 Q-B 条件臂;门 G-G5-5 以单阶段满足 | RD-037+(不达则登记) |
| SMRT 软阴影 | §9 Q-C 降档;采样端算法零新数据结构,后续期可独立承接 | RD-037+ |
| 引擎采纳/下载量/用户数宣称 | carve-out(沿 MS1/EA1/EI1/G4 先例) | 不立 |
| AMD 真卡见证(G-MB1-6) | 缺硬件 pending-hardware 不伪造;全部门锚 RTX 4070 Ti | G-MB1-6 维持 open |

## 9. 未决问题 / 关键裁决

编号规则:`Q-<名>`。全部为 agent 拟裁(D-406 v2.0,Approved 即定案);对抗性评审 disposition 可修订,修订落 §9.1 与修订记录。

| # | 裁决点 | 裁决 |
|---|---|---|
| Q-A | 渲染器着色器供给面 | **拟裁(§9.1 R-6 修订)**:效果 pass 着色器首选 **.rx 源经 rurixc `--target vulkan` 产 SPIR-V 模块**(G3 vk 运行时底座同形态通道,smoke 脚本驱动编译或 build 期供给);**禁止运行期在线编译**;渲染器效果 shader 的手写 SPIR-V 二进制不入仓(可审计性);**测试见证极小模块例外**:执行器单测的最小 VS/FS/compute 见证模块允许沿 vk.rs 既有测试供给通道(既有资产/程序化生成),不受效果 shader 纪律约束;某效果 .rx 表达不达 → 登记 RD-037+ 评估,不私开通道 |
| Q-B | HZB 两阶段剔除入本期与否 | **拟裁(条件臂,降档 P2 尾)**:单阶段剔除(视锥/背面锥/LOD cut)入本期硬门;HZB mip 链资源面与剔除输入接口**预留**;两阶段 HZB(首遍剔除/重建/补漏重测)为 **P2 尾条件臂**——G5.3-C 集成时剔除对拍已绿且时间盒可达则落,不达则登记 RD-037+;**门 G-G5-5 不依赖两阶段** |
| Q-C | SMRT 软阴影口径 | **拟裁(降档)**:本期章 D = 硬阴影投影采样(页表+物理池,规格面完整);SMRT(沿光线多采样,Source Radius/Angle)降档 **RD-037+**——采样端算法零新数据结构,后续期可独立 Mini/承接期兑现;门 G-G5-6 以 VSM 深度对拍 + 硬阴影满足 |
| Q-D | 异步车道验收口径 | **拟裁**:**机制正确性入硬门**(FencePair 注入/车道划分/平台回落 host 单测 + 候选三条件静态核验 + 一键回退开关);**重叠量时间戳 measured 写 evidence 不进硬门**(报告5 vkguide「未必有收益」与 Godot 5–15% 证据纪律;P-09);无效候选按回退开关下线不留痕 |
| Q-E | 顶点获取形态(章 B/章 C) | **拟裁**:首期**显式 VB/IB 缓冲绑定**(cabi 追加式,章 B1);vertex pulling(无绑定直索 storage buffer)与压缩 meshlet 顶点格式归 RD-037+ 评估;HW 路 VisBuffer 写出格式与 SW 路共享不变(§4.C3) |
| Q-F | NRD/vendor 降噪与超分 SDK 接入 | **拟裁(留口不接)**:本期自研时域滤波(章 H 底座)承载 RT/阴影降噪;NRD(ReLAX/ReBLUR)、FSR 3.1、DirectSR 接入评估登记 **RD-037+**(第三方 C++/SDK 集成面超本期时间盒);UpscaleBackend trait 与降噪输入契约(MV/深度/法线同构)**留口先行**,接入时不改底座 |

## 9.1 对抗性评审记录(D-409)

**已完成 第 1 轮 2026-07-29**——评审 provenance **`cursor:claude-fable-5`(G5 主线,独立于起草)≠ 起草 provenance `cursor:kimi-k3-max`**(D-409/硬规则 2 可机验);三镜头 correctness/redline/implementability;7 findings 逐条 disposition(2 blocker 正文实改 + 4 major 正文实改 + 1 minor 留痕):

| # | 镜头 | finding | disposition |
|---|---|---|---|
| R-1(blocker) | implementability | 章 B 原文把「.rx 声明式 gfx submit 真派发」当唯一通道承诺——其缺口若涉 rurixc lowering 扩面(声明式库面承载 VB/IB)则超时间盒,且引擎渲染器(Rust 库)本不经 cabi 声明面驱动,主消费者是章 A 编译产物 | **采纳并修 §4.B2**:双通道裁决——主通道 = 引擎库 Rust 级多 pass 执行器(vk.rs `render_exec`,本期硬门 G-G5-4);.rx submit 真派发降为条件臂,缺口大则 RD-037+ honest 存续 |
| R-2(blocker) | correctness | §4.0-7 原文「零新 cargo feature」与工程事实矛盾——rurix-render/uc06-renderer 需 `vulkan` 透传 feature 才能维持「默认构建零 GPU 依赖绿」(uc04-demo real-shim 先例正是此形态) | **采纳并修 §4.0-7**:改为「新增 feature 仅限透传 gate,不新增语义性 feature」 |
| R-3(major) | implementability | 章 E/F 的 device 腿隐含假设 `rayQueryEXT` compute 编码通道已在工具链就位——rurixc vulkan_codegen RT 面现为 `emit_*_min` 见证形态,GI/RTAO 的 .rx→SPIR-V ray query 编码未经核实,device 全量承诺有伪造风险 | **采纳并修 §4.E3**:device 腿条件臂——通则全量,不通则降档「G3.6 RT 底座最小见证 + host 参考器全量对拍」+ RD-037+ 存续,不伪造 device 绿 |
| R-4(major) | implementability | §4.G4 原文承诺 KTX2 转码分档(BC7/BC6H)——真转码器为第三方级工程量,超时间盒即会诱发降质量赶工或伪造 | **采纳并修 §4.G4**:降档为页式运行时 + 确定性解包 + 转码接口留口;KTX2/BasisU 接入归 RD-037+ |
| R-5(major) | correctness | §4.C3 原文「transient R64Uint 全屏纹理」——Vulkan 64 位 **image** 原子支持面窄(`shaderImageInt64Atomics` 覆盖率差),工业实现(Nanite/Bevy)用 storage buffer 承载 VisBuffer;冻结契约 types.rs 亦无 R64 纹理格式 | **采纳并修 §4.C3**:VisBuffer = u64 storage buffer(`shaderBufferInt64Atomics` 主承诺面) |
| R-6(major) | correctness | §9 Q-A 原文「手写 SPIR-V 二进制不入仓」一刀切会卡死执行器单测的最小见证模块供给(vk.rs 既有测试已有供给通道先例) | **采纳并修 Q-A**:效果 shader 纪律维持;测试见证极小模块例外沿 vk.rs 既有通道 |
| R-7(minor) | redline | §6.1 失败测试先行措辞「RFC 合入时点 main 不存在 = RED」在单会话波次执行下的时序语义需澄清(脚手架 crate 已先于 RFC 批准存在于工作树) | **留痕不改正文**:失败测试先行的实质 = 实现面不先于 RFC Approved **合入 main**;本期波次执行的 commit 序以「治理/RFC 批准先于实现合入」为准,工作树先行搭建不构成合入;G-G5-2 判据不变 |
| 红线镜头总核 | redline | D-130(窗口/输入不进语言)/ 06 §8.3(库不进语言)/ RD-034(DXIL RT blocked)/ P-09(数字不进硬门)/ P-12(P3+ 不实码)逐条核过 | 零红线违反;§8 表与 G5_CONTRACT out_of_scope 一致 |

**环境留痕**:评审与起草同会话跨模型(起草 = kimi-k3-max 子代理独立上下文;评审 = claude-fable-5 主线,读全文 + 对照 G5_CONTRACT/G5_PLAN/number_ledger v1.27/types.rs 冻结契约/vk.rs 现状),符合「评审 provenance ≠ 起草 provenance」字面;跨工具独立实例评审不可得于本会话,偏差如实登记(RFC-0015 §9.1 跨工具同模型族评审偏差留痕先例)。

**结论**:7 findings 全部 disposition 完毕(6 正文实改 + 1 留痕),状态 **Draft → Agent Approved(2026-07-29)**,先于任何实现合入(G-G5-2)。

## 9.2 已知风险与评审攻击面(起草侧自暴,供 §9.1 评审镜头用)

**章 A**
- **A-1 两推导层面分工的解释负担**(graph.rs vs graph_compile):§4.0-1 钉死「各自服务单一层面、不交叉调度同一张图」;攻击点 = 未来是否出现「同一资源两侧调度」的灰色面——以「章 B 桥内一切 barrier 由 graph.rs 产、章 A 图内一切 barrier 由 graph_compile 产」双层互斥闭合。
- **A-2 异步车道收益不及预期**:Q-D 机制/收益分离口径 + 一键回退;攻击点 = 候选三条件的「≥0.5ms」量级判定在无度量前是静态声明——如实承认首批准入为声明式,时间戳 evidence 回填后修订候选集。

**章 B**
- **B-1 cabi 追加面句柄生命周期**:VB/IB 绑定符号的 affine 句柄级联清理沿 `drop_rhi_children` 模式复用;攻击点 = 跨 rhi 误用与销毁后旧句柄——接收方核验 + 级联失效(既有模式)承载,实现期红绿锚定。
- **B-2 执行器扩面回归风险**:Phase 1 最小面 → 多 pass/VB-IB/纹理扩面在同文件新臂,既有入口 0-byte;攻击点 = PlannedBarrier 回放与 draw 录制的交错序——逐字回放纪律不变,交错序 golden 锚定。

**章 C**
- **C-1 DAG 构建质量(最高危)**:裂缝/误差失真 → 运行时 popping;P0 即引入簇界包含性与误差单调性单测;分组简化首选成熟算法组合,不自研简化器(报告1 §7)。
- **C-2 SW 光栅边角**:细长/退化三角形、浮点 snapping、与硬件深度精度一致性——深度位同量化为前置;SW/HW diff 容差 = 0 整数域的可达性依赖此前提,若实现期证伪则域收窄为「深度一致 + 覆盖差异枚举留痕」并修订本段(不静默降判据)。
- **C-3 硬件碎片化**:`VK_KHR_shader_atomic_int64` 排除老硬件——能力查询 fail-closed;传统逐网格 LOD+GBuffer 回退路径兼作 Nanite 路径对拍基准(不视为浪费)。

**章 D**
- **D-1 失效风暴**:保守失效起步的成本画像(每帧失效页数)进度量埋点与告警;WPO 植被类风暴源本期无资产(蒙皮/WPO 归 P3+),风险部分休眠,如实标注。

**章 E/F**
- **E-1 屏幕探针薄几何/强视差泄漏**:平面加权 + 滤波缓解不根除(报告2 §6 自认);泄漏场景入回归集留痕,不宣称根除。
- **F-1 加速结构债**:BLAS 全量重建掩盖更新策略缺失——决策树/compaction 时机 P0 即成文 + 监控,不留隐式策略。

**章 G/H**
- **G-1 流送各自为政**:几何/纹理/PSO 后台编译共享同一预算调度器(§4.G4);攻击点 = PSO 编译不经页池——共享「每帧工作量预算」调度而非共享页池,如实区分。
- **H-1 底座不完整诱发效果各自打补丁**:MV 三类速度通道预留但蒙皮/WPO 本期无资产验证(§8);审计项 = 禁私写重投影(G-G5-7 代码审计)兜底。

## 10. 稳定化与 provenance

- **稳定化**(10 §5):预期零新语言语义条款 → stable 快照零 spec 面变更(条件消费落时随快照加性重 bless,RXS-0180 L2 只增不破坏);`src/rurix-render` / `src/rurix-geom-build` crate API 与 §4.0-3 冻结接口 = **引擎库内部契约**(RXS-0180 L3 口径,非 stable ABI),G5.2 固化后波次内不得漂移(G5_PLAN §2),期外演进走普通 PR 判档。FCP-lite(advisory)下公开,agent 自主裁决合入(D-406 v2.0)。
- **Provenance**:`Assisted-by: cursor:kimi-k3-max`(起草)。agent 自主决策;批准前置 = §9.1 对抗性评审完成(评审 provenance ≠ 起草,D-409/硬规则 2),批准后推进 §6.3 下游实现 PR。

## 11. 规范与实现依据

- **仓内**:milestones/g5/{G5_CONTRACT.md(§7 开工裁决/编号 claim),G5_PLAN.md(§1 波次/§2 冻结接口),CI_GATES.md(§2 步骤 82~87 拟分配),g5_budget.json};渲染器调研/调研报告1~7(上游事实源,只读);milestones/g4/G4_CONTRACT.md §8.8(G4 close-out);registry/number_ledger.json v1.27(reserved_in_flight[G5];RXS next_free 297 / CI 步骤 next_free 82 / RD next_free 37 / U next_free 32);registry/deferred.json(RD-034/RD-036);spec/rhi.md(RXS-0256~0265/0270~0289)、spec/render_graph.md(RXS-0236~0241)、spec/vulkan_backend.md(RXS-0208/0209/0246~0248/0290~0294)、spec/export_c.md(RXS-0250~0255)、spec/edition.md(RXS-0180);rfcs/0015(伞形体例母本)/ rfcs/0013(G3 伞形五章)/ rfcs/0014(EI1 双面);src/rurix-rt/src/{rhi.rs(RhiGraph/exec_face),graph.rs(derive_barriers 单源),vk.rs(:13406 run_rhi_graphics_offscreen,U31;G3.6 AS/SBT 面 U30),fatbin.rs(DeviceArtifactSet.spirv_fallback)};src/rurix-rt-cabi/src/{lib.rs(:1838 rxrt_rhi_submit;:1883~1885 gfx 仅参推导现状锚),artifacts.rs(v2 解析)};src/rurixc/src/{codegen.rs(@__rx_gpu_spirv 发射),driver.rs(build_gpu_artifacts),vulkan_codegen.rs,dxil_spirv.rs,mir_build.rs(RHI lowering)};src/rurix-engine/harness/engine_host_v3.cpp(G4.2 嵌入对照);apps/uc05-rhi/src/{gfx_demo.rx,embed.rx}(gfx 母本)。
- **外部**:Vulkan SDK 1.3.296.0(spirv-val);VK_KHR_ray_query / VK_KHR_shader_atomic_int64 / VK_KHR_synchronization2;D3D12 Enhanced Barriers 规范;Frostbite FrameGraph(GDC 2017);UE RDG 官方文档;Godot 4.3 RAG;AnKi simplified pipeline barriers;Granite render graph;meshoptimizer clusterlod.h(zeux);Bevy 0.14 virtual geometry(JMS55);StratusGFX Sparse VSM(ktstephano);UE VSM/SMRT/TSR/PSO precaching/Fast Geometry Streaming/Nanite Assemblies 官方文档;GI-1.0(Boissé,AMD);RTXGI/SHaRC 与 RTXDI(NVIDIA);Khronos Vulkan RT 混合渲染最佳实践;FSR 3.1 开源 / DirectSR(留口参照)。

---

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| Draft v1.0 | 2026-07-29 | AI 起草初版(`Assisted-by: cursor:kimi-k3-max`,G5.1):伞形八章——章 A 渲染调度引擎库(四趟编译/EB 三轴/transient 别名/编译期校验/异步车道/dump)· 章 B RHI 派发桥(真派发臂/供给面/present 产品化,复用 RXS-0270~0294)· 章 C 虚拟化几何(geom-build/两级剔除/VisBuffer 双路/classify-resolve)· 章 D VSM(clipmap/页标记分配失效/共享页池/多视图深度/投影)· 章 E 屏幕探针 GI(ray query 单反弹/SH/平面插值/3×3 滤波/时域累积)· 章 F 光追与 AS 管理(BLAS 缓存/refit 分级/TLAS/RTAO/硬阴影)· 章 G 材质场景流送(32B 闭合/GPU scene/PSO precache/页式流送三预算/两级实例化)· 章 H 时域重建(公共底座/TAA/TSR/UpscaleBackend)。Q-A~Q-F 拟裁;§5 预期零新条款;§7 备选七项;§8 红线九项;§9.2 攻击面自暴(A-1/A-2/B-1/B-2/C-1~C-3/D-1/E-1/F-1/G-1/H-1) | Full RFC(Draft) |
| v1.1 | 2026-07-29 | **D-409 对抗性评审完成 + 状态翻 Agent Approved**(评审 provenance `cursor:claude-fable-5` ≠ 起草 `cursor:kimi-k3-max`):7 findings 逐条 disposition(§9.1)——R-1 章 B 双通道裁决(主通道 = Rust 级执行器 `render_exec`,.rx submit 降条件臂)/ R-2 §4.0-7 feature 口径改「仅限透传 gate」/ R-3 章 E/F device 腿条件臂(ray query 编码通道核实,不通则降档 honest 存续)/ R-4 §4.G4 KTX2 真转码降档 RD-037+ / R-5 VisBuffer 改 u64 storage buffer / R-6 Q-A 测试见证模块例外 / R-7 失败测试先行时序语义留痕。字段表状态/批准/评审三行同步 | Full RFC(**Agent Approved**) |
