# G9_CAPABILITY_MATRIX — UE5 级建造期能力 → Rurix 现状与缺口矩阵

> **所属**：G9 文档集（`milestones/g9/`；计划状态见 [G9_PLAN.md](G9_PLAN.md)——**G9 未立项，本文不构成契约/验收承诺**）。上游输入：[research/R4](research/R4_VIRTUAL_GEOMETRY_RT.md) · [R5](research/R5_GI_LIGHTING.md) · [R6](research/R6_GPU_DRIVEN_SUBMISSION.md) · [R7](research/R7_WORLD_AND_SPECIALTY_RENDERERS.md) · [R8](research/R8_PHYSICS.md) + [design/](design/) 五份模块设计草案（G9.0 冻结引用，内容以草案为准）+ 本文 §0 立项前事实基线。下游消费者（G9.1 治理波）：G9_CONTRACT / G9_CANDIDATE_DECISIONS / G9_ACCEPTANCE_MAP。
> **基准日**：2026-08-08（G8 closed 终态 + G8.7 P2 决策表 v1.0 + deferred.json 只读引用）。
> **行号纪律**：行号 `M##` **顺延 G8 矩阵**（G8 止于 M89 + M49a/M49b/M65b，本文自 **M90** 起），保持 RD 映射连续性；M## 为文档内部定位标识，**非 ledger 编号**。本文零编号占用——不新设/不消费任何 RFC/RD/RXS/SG/CI/U 编号，仅只读引用。
> **优先级纪律**：「建议 P 级」与「拟承接」均为计划建议值，G9 立项时经 G9.1 候选决策表重新裁决后硬化；**M52/M61 改判与 G9 分包规模未经用户裁决，相关行标「立项待裁决」，本矩阵不擅自定案**。
> **条件型 RD**：RD-039/040/041/044 维持 open 为法定输入；其分项 backfill 不得以「UE5 级目标」静默改写——触发证据（立项书 / measured workload / 资产需求）须逐分项留痕，deferred history 只追加（G8_PLAN §1.2 纪律继承）。
> **图例**：✅ 已交付 · 🟡 部分 · ⬜ 缺失；档位 A 语言/编译器 · B 运行时/RHI · C 引擎库 · D 工具/资产；验收五层级缩写 = **核** 核心等价 · **环** 功能闭环 · **降** 可降级 · **产** 可生产化 · **主** Vulkan 主线；4070Ti = 可否真机验证。

---

## 0. 立项前事实基线（行证据的共同来源）

1. **G8 已收口**：`milestones/g8/G8_CONTRACT.md` §8.26 `status: closed`（2026-08-06，flip commit `b4189e79`）；21/21 P0+go-P1 PASS、wave2~8a 聚合 11/11 PASS；最新 closeout 复核 `evidence/g8_wave8b_closeout_20260808T040705Z.json` `VERDICT=READY`（未提交，留工作树，立项前处置见 G9_PLAN §5）。
2. **十条 defer-to-G9+ 承接锚**（`milestones/g8/G8_P2_DECISIONS.md`，`ci/g8_p2_decisions_check.py` 机核「defer 必有 G9+ 承接锚」）：M06/M09（→D1）、M12/M16（→D2）、M33/M55（→D3）、M43/M48/M49（→D4）、M74（→D5）。本矩阵 §1~§5 逐行承接。
3. **存续 open（不阻断立项，为法定输入）**：RD-039/040/041/044 总体 open（G8_CONTRACT §8.26）；RD-034 DXIL RT/mesh blocked；RD-036/042/043 观察；G-MB1-6 AMD 真卡尾门缺硬件。
4. **G8 交付底座（D1~D5 只消费不重定）**：M01/M04 页格式 ABI（RXS-0328~0342）· M44 几何页 streamer · VisBuffer SW/HW diff=0 · M19 VSM 页缓存 · M50 RT 增量面 · M29~M32/M85 着色治理 · M37/M38 磁盘 I/O 链 · M79~M83/M88 资产闭环 · M66~M72 物理平台 · M89 单源 gfx submit · M24/M25 时域/超分。
5. **触发声明（立项时留痕）**：五份设计草案假定 G9 正式立项书即构成 RD-039 M06/M09 两分项的触发证据；立项时须在 `registry/deferred.json` history **只追加**登记 open-defer → G9 承接，不得改写 G8.7 决策表原文（D1 草案 §1 字面）。

---

## 1. D1 虚拟化几何与 RT 合流（详设：`design/G9_D1_VIRTUAL_GEOMETRY_RT.md`；调研：R4）

| 行 | 能力（子面） | G8 承接锚 | backfill / 触发条件字面 | UE5 基线 | 缺口要点 | 档位 | 建议 P 级 | 验收五层级 | 4070Ti | 拟承接 |
|---|---|---|---|---|---|---|---|---|---|---|
| M90 | cluster DAG 构建管线深化：monotonic 误差度量、簇对锁定、蒙皮元数据、CLAS 离线烘焙输入 | M06/M09 defer | RD-039「动态资产面出现时」「RT 与虚拟几何合流需求出现时」——以 G9 立项书为触发证据（history 只追加） | R4 §Nanite/DAG | G8 M01 静态 DAG 无误差度量/簇对锁定/CLAS 输入；算法自研不 vendoring | C/D | P0 | 核·环·产·主 | ✔ | G9.2 |
| M91 | 页格式 v2（RXPL 新 major）ABI 冻结 | D1 D-11；G8 R-G8-4 反向依赖禁令 | 新增 cluster 属性入页即触发：必须新 major + spec-first 冻结；M04 v1 ABI 0-byte 共存 | R4 | v1 页无包围球/骨骼元数据/CLAS 段 | C/D | P0 | 核·环·产·主 | ✔ | G9.2 |
| M92 | GPU cluster 感知蒙皮与骨骼植被：LBS kernel、Kerbl 保守包围球/法向锥、bone shader 可编程 API、距离分级动画更新率、Morph 非虚拟化旁路 | M06 defer「G9+ 虚拟几何评估窗」 | 同 M90 字面；UE5.5 CPU 蒙皮权宜路线拒绝（D1 D-1） | R4 §蒙皮 | 语言无蒙皮语义/skin cache；真实骨骼植被资产管线待建（R-D1-8） | A/B/C | P1 | 核·环·降·产·主 | ✔ | G9.3 |
| M93 | 误差驱动运行时 LOD/cluster 选择：`VisibleClusterSet` 输出、屏幕空间误差 cut 无重叠无空洞、未驻留页父簇兜底 | M06 矩阵行；M44 消费端 | 同 M90 字面；沿 G8.4 迟到页降级语义不重定 | R4 §Nanite | 静态 LOD cut 无蒙皮簇注入/无运行时误差驱动 | B/C | P0 | 核·环·降·主 | ✔ | G9.3 |
| M94 | CLAS 簇级 BLAS 与 RT 合流：当帧 multi-indirect 拼装、Cluster Template 实例化、AsManager 扩展；NV 主腿 + 传统 BLAS 回退腿 | M09 defer「G9+ RT×Nanite 合流窗」 | RD-039「RT 与虚拟几何合流需求出现时」；DMM 永禁（D1 D-7） | R4 §Mega Geometry | 无 CLAS/无当帧 BLAS 拼装；回退腿为正确性基线 | B/C | P0 | 核·环·降·产·主 | ✔ | G9.3 |
| M95 | VisBuffer/VSM/RT 单源真相集成：可见集一份三喂、动画分级作用于 AS 更新（静态帧零 AS 构建） | D1 [调研5] | 双世界错配防线；帧末一致性/provenance 校验负例为硬门 | R4 §单源真相 | 光栅/RT 可见性各自独立计算的现状 | B/C | P0 | 核·环·主 | ✔ | G9.3 |

## 2. D2 全局光照与光照缓存（详设：`design/G9_D2_GI_LIGHTING.md`；调研：R5）

| 行 | 能力（子面） | G8 承接锚 | backfill / 触发条件字面 | UE5 基线 | 缺口要点 | 档位 | 建议 P 级 | 验收五层级 | 4070Ti | 拟承接 |
|---|---|---|---|---|---|---|---|---|---|---|
| M96 | M17 Path Tracer 参照器：单向 PT + NEE/MIS/RR、megakernel 起步 wavefront 阶段化、固定 seed 确定性协议、pbrt-v4 对照 | G8 候选表 M17 no-go「G9+ 建造期前置」 | backfill「GI/材质画质门需要跨路径 golden 时（G9+ 建造期前置）」——**字面已命中，建议判 go**；G9.4 波内第一顺位 | R5 §参照器 | host `ref_tracer` 非 RT pipeline 级完整参照器 | C/D | P0 | 核·环·产·主 | ✔ | G9.4 |
| M97 | Surface Cache：离线 Card 参数化（≤12/mesh 可配）+ 运行时辐射度缓存；缺失覆盖**只丢能量不漏光** | M12 defer「G9+ GI 建造期」 | 承接锚字面；Card 图集页格式复用 M04 ABI 不私定 | R5 §Lumen | ⬜ | C/D | P0 | 核·环·降·主 | ✔ | G9.4 |
| M98 | 追踪降级链：L1 Screen Trace → L2 SWRT（Mesh/Global SDF）→ L3 HWRT（RayQuery + hit lighting 档）→ L4 Far Field；逐档可关可测禁静默 | M14 no-go 重判档（D2 即 M50 后评估窗） | M14 字面「M50 后评估，画质 measured 需求」——M50 已绿，需求方 = D2 自身画质门 | R5 §Lumen | ⬜（`RadianceTracer` 契约已预留 SDF 位） | B/C | P0 | 核·环·降·主 | ✔ | G9.4 |
| M99 | SPG 自适应细分 + Radiance Cache 双级（屏幕级 + 世界 clipmap 级）、product importance sampling | M11 世界辐射缓存观察（RD-040） | RD-040「屏幕探针远场缺失成为画质 **measured** 问题」——世界 clipmap 级须 measured 触发举证，未举证只做屏幕级 | R5 §SPG/RC | G8 1/16 均匀 probe + 3×3 滤波在位 | B/C | P1 | 核·环·降·主 | ✔ | G9.4 |
| M100 | 多灯直接光：低档 MegaLights 式固定随机选灯（默认）/ 高档 ReSTIR reservoir（可选）；海量灯阴影统一接口随动 | M15/M22 no-go | RD-040「多灯场景需求出现时」——立项时须附多灯 workload 证据；不足则 M15 维持 open-留档、只做低档；验证射线零跳过硬契约 | R5 §多灯 | ⬜ | A/C | P1 | 核·环·降·主 | ✔ | G9.4 |
| M101 | irradiance field 档位 L0–L3：共享 probe 着色与八面体编码内核只换空间索引；每档 AS 更新预算行 | M16 defer「G9+ GI 档位」 | 承接锚字面；DDGI 档 visibility 16×16 防漏光优先 | R5 §IF | ⬜ | B/C | P1 | 核·环·降·主 | ✔ | G9.4 |

## 3. D3 GPU-driven 提交与着色器系统（详设：`design/G9_D3_GPU_DRIVEN_SUBMISSION.md`；调研：R6）

| 行 | 能力（子面） | G8 承接锚 | backfill / 触发条件字面 | UE5 基线 | 缺口要点 | 档位 | 建议 P 级 | 验收五层级 | 4070Ti | 拟承接 |
|---|---|---|---|---|---|---|---|---|---|---|
| M102 | DGC 抽象层与三后端映射：IndirectCmdLayout / DgcBuffer、token 跨 API 最小公倍数、限制装配期 fail-closed | M55 defer「G9+ GPU-driven 提交」 | 承接锚字面；目标硬件 capability snapshot 实测确认为阻塞性前置（fail-closed） | R6 §DGC | ⬜（bindless 已绿 RXS-0231~0235，DGC 未用） | B | P0 | 核·环·降·产·主 | ✔ | G9.2 |
| M103 | descriptor buffer 全局表：「资源→全局 descriptor 索引」进 reflection/manifest（加性并存 set/binding）；索引分配律/回收进 spec | M55 defer | 承接锚字面；`VK_EXT_descriptor_heap` 只预留 feature 位不实现 | R6 §descriptor | ⬜ | B | P0 | 核·环·产·主 | ✔ | G9.2 |
| M104 | command build compute node 与 render graph 集成：AccessKind 新边 `StorageWrite→IndirectCommandRead`、零 CPU 回读结构性保证 | M55 defer | 触 G5 Barrier EB 三轴冻结面 → **必须 RFC 显式修订行**；RXS-0239 单 queue 全序字面不动 | R6 §范式 | ⬜ | A/B | P0 | 核·环·主 | ✔ | G9.2 |
| M105 | Indirect Execution Set 与 PSO/manifest 衔接：GPU 侧索引切换；D3D12 无对应物诚实降级 CPU 侧 PSO 切换 | M55 矩阵行字面「DGC 优先…descriptor buffer 作高性能后端」 | capability ID 区分路径；禁止静默模拟（P-01） | R6 §DGC | ⬜ | B | P1 | 核·环·降·产·主 | ✔ | G9.3 |
| M106 | shader library IR 函数级组合链接：编译期链接物化 SPIR-V/DXIL、链接拓扑进 manifest、interface hash 重算 | M33 defer「G9+ shader library 深化」 | 承接锚字面；v1 边界 = 函数级符号链接、禁跨 module 泛型单态化 | R6 §library | 🟡 模块系统 + 单产物嵌入已有 | A/B | P1 | 核·环·产·主 | ✔ | G9.3 |
| M107 | 变体预算与审计工具：工程级总预算门（硬失败）、死变体检测报告、axis/module 归属分解 | M29/M30/M85 治理面承接 | UE 1300 万→400 万变体教训；工具与 M106 同波不延后 | R6 §变体 | per-entry budget 已有，工程级总预算缺 | D | P1 | 环·产 | ✔ | G9.3 |
| M108 | SER 语言原语：HitObject 类型面、`reorderThread`/`hitObjectTraceRay`/`hitObjectInvoke`、capability `rt.ser` 可选、材质 flags coherence hint 位段预留 | M52 no-go 留档 | **改判提案：no-go → 语言层支持 + capability 可选——立项待裁决**；改判须 `deferred.json` history 只追加 override，禁静默改判；收益集中 NV 不承诺性能、渲染器集成延后 | R6 §SER | ⬜ | A/B | **立项待裁决**（建议 P2 可选） | 核·降·主 | ✔ | 裁决 go 后才排波 |
| M109 | mesh shader 可选 geometry pipeline：cluster 流入口、VS 光栅唯一 fallback、`mesh.task` capability 选择律 | M61 no-go 留档 | **改判提案——立项待裁决**；RD-039 双条件：「跨厂商收敛」按公开证据实质成立 +「measured」本机 4070 Ti 补齐；override 程序同 M108；顺序硬约束：排在 meshlet 格式与 GPU-driven 剔除之后 | R6 §mesh | 🟡 最小见证（RXS-0243/0246~0248） | A/B/C | **立项待裁决**（建议 P2 可选） | 核·降·主 | ✔ | 裁决 go 且 cluster 流就绪后 |

## 4. D4 大世界分区 / 专项渲染器族 / 显示管线（详设：`design/G9_D4_WORLD_AND_SPECIALTY_RENDERERS.md`；调研：R7）

| 行 | 能力（子面） | G8 承接锚 | backfill / 触发条件字面 | UE5 基线 | 缺口要点 | 档位 | 建议 P 级 | 验收五层级 | 4070Ti | 拟承接 |
|---|---|---|---|---|---|---|---|---|---|---|
| M110 | World Partition 数据模型与流送：单一持久世界 schema、2D cell、streaming source 距离环、三项预算契约逐帧 evidence、四事件接口、Data Layer 掩码位预留 | M43 defer「G9+ 大世界分区」 | 承接锚字面「大世界资产面出现时」 | R7 §WP | ⬜ | C/D | P0 | 核·环·降·产·主 | ✔ | G9.5（波内先行） |
| M111 | HLOD 烘焙管线：离线 Builder 按 Component 分发、产物即资产、运行时零合并、screen-size 互斥切换 | M43 defer | 同 M110；双构建确定性沿 M79 判据 | R7 §WP | ⬜ | C/D | P1 | 核·环·产·主 | ✔ | G9.5 |
| M112 | 大气体渲染器：Froxel 统一基础设施 + 雾前端 + 云前端（Perlin-Worley/weather map/时序上采样默认） | M48 defer「G9+ 大气特效」 | 承接锚字面；weather map 资产化走 M01/M85 通道 | R7 §大气 | ⬜ | C | P1 | 核·环·降·主 | ✔ | G9.5 |
| M113 | 水体：大洋 Tessendorf IFFT（位移/梯度/Jacobian + CDLOD）与浅水波方程双管线分离；浮力接口面预留不实现 | M49 defer「G9+ 专项渲染器」 | 承接锚字面；tiling-and-blending 防重复感 | R7 §水体 | ⬜ | C/D | P1 | 核·环·降·主 | ✔ | G9.5 |
| M114 | 毛发：Marschner R/TT/TRT 三瓣 + strand/card/mesh 几何三档；strand 档强制精确 OIT | M49 defer | 承接锚字面；强依赖 M120 精确档，排序在 OIT 落地之后 | R7 §毛发 | ⬜ | C/D | P2 | 核·环·降·主 | ✔ | G9.5 末 |
| M115 | 皮肤：Burley normalized diffusion 屏空单 pass + 扩散 profile 资产化 + pre-integrated LUT 回退档 | M49 defer | 承接锚字面；触 `MaterialClosure` 32B 须 RFC 修订，禁静默扩 | R7 §皮肤 | ⬜ | C/D | P1 | 核·环·降·主 | ✔ | G9.5 |
| M116 | 地形：GPU-driven heightfield、chunk ≡ cell（禁第二套分格）、LOD/剔除/缝合全 compute、toroidal 更新 | M49 defer | 承接锚字面；**禁依赖 SVT/RVT**（M40/42 G8 no-go 维持，D4 D17） | R7 §地形 | ⬜ | C/D | P1 | 核·环·降·主 | ✔ | G9.5 |
| M117 | 贴花：DBuffer 三通道帧图设计期占位 + screen-space cluster 化 + 前向回退档 | M49 defer | 承接锚字面；零用量也先冻结通道与 barrier 布局 | R7 §贴花 | ⬜ | C | P1 | 核·环·降·主 | ✔ | G9.5 |
| M118 | HDR 显示管线与可插拔 view transform：SDR/scRGB/PQ 三交换链路径运行时切换、ACES 1.3/2.0/AgX/中性四内置插件 | M45 no-go「HDR 显示设备资产/产品需求出现时」 | 拆两层：管线/插件面 SDR 上即可全量验证（先行）；HDR 设备标定条件未触发则 SKIP=not-triggered 或 open-留痕，**不假绿** | R7 §HDR | 🟡 present 现 FIFO + RGBA8/BGRA8 | B/C | P0（管线面） | 核·环·降·主 | 部分（标定需 HDR 设备） | G9.5 |
| M119 | 后处理栈：histogram 曝光+EV → bloom → DOF → tonemap → LUT → 输出变换，全程 HDR 线性域；曝光状态帧间持久 | M46 no-go「产品需求随 G9+ 建造期出现」 | G9 立项书即产品需求证据候选（立项时留痕）；与 TAA/TSR 时域链显式排序 | R7 §后处理 | 🟡 仅 soft-raster/uc06 tonemap 级 | C/D | P1 | 核·环·降·主 | ✔ | G9.5 |
| M120 | OIT 三档：默认 TAA 半透明 / 有界近似（WBOIT 起步、AVBOIT 目标）/ 精确 linked-list 仅毛发；排序 fallback 永保留 | M47 no-go「OIT 策略选型需 measured 对照」 | measured 对照 = **benchmark 门先行**（nvpro 七算法 harness）；无 benchmark 数据的默认档选型提交判 RED | R7 §OIT | ⬜ | A/B/C | P1 | 核·环·降·主 | ✔ | G9.5（benchmark 波内先行） |

## 5. D5 物理建造期（详设：`design/G9_D5_PHYSICS.md`；调研：R8）

| 行 | 能力（子面） | G8 承接锚 | backfill / 触发条件字面 | UE5 基线 | 缺口要点 | 档位 | 建议 P 级 | 验收五层级 | 4070Ti | 拟承接 |
|---|---|---|---|---|---|---|---|---|---|---|
| M121 | 统一 physics particle view：五域 `ParticleAdapter`、`PhysicsParticleRef` 名义类型、写路径仅 impulse/force；M68 damage journal 迁移为首个 consumer | M74 前置抽象 | M74 defer 字面「统一 Field 属建造期」的前置；单向事实源纪律 0-byte | R8 §Field | ⬜ | C | P0 | 核·环·主 | ✔ | G9.2（骨架）→G9.6（完整） |
| M122 | Gameplay Field 系统：三层解耦、首期 `FieldPhysicsType` 八枚举、三生命周期（persistent 显式注销全 journal）、过滤默认空匹配、World-Field 通道经 GpuScene 只读 buffer | M74 defer「G9+ gameplay Field」 | 承接锚字面；persistent 注册/注销/变更全 journal 化且 replay hash 一致为硬门 | R8 §Field | M68 damage/field journal 最小面在位 | C/D | P0 | 核·环·产·主 | ✔ | G9.2（骨架）→G9.6（完整） |
| M123 | 双通道确定性架构：lockstep-deterministic（永不异步化）vs async-decorative（零回写）；`deterministic_profile` 运行时断言 | M75 no-go「异步调度须独立判档」（RFC-0021 Q6） | **判档硬前置 = Jolt 单线程成本 measured**；测量不足则维持 no-go 留档 | R8 §异步 | 🟡 固定步 + accumulator 在宿主 | B/C | P1（条件制） | 核·环·降·主 | ✔ | G9.6 |
| M124 | 解析浮力模型：浸入体积/浸没质心 → 浮力+浮力矩+阻力 impulse，走 Field 通道（persistent field + `Buoyancy` 语义），确定性内置入 corpus | M77 no-go「未包装且无 gameplay 需求；联动 M49 defer」 | D5 建议 go：Field 统一抽象第二个真实用户；**禁旁路 API**（旁路即门红）；真双向流体耦合排除主线 | R8 §浮力 | ⬜ | C | P1 | 核·环·主 | ✔ | G9.6 |
| M125 | Jolt 5.3→5.6 升级 A/B：RFC-0021 §4.A4 七步程序逐字执行；新摩擦模型重点；layout 探针工具化 | G8.6a 纪律延续（corpus 已建成，评估窗开启） | 采纳臂三件事/失败臂钉 5.3；GPU compute 接口只评估不接权威（GPU 主刚体禁止线） | R8 §Jolt | 🟡 钉 5.3 honest stop-loss（G8.6a） | B/D | P1 | 环·产·主 | ✔ | G9.6 |
| M126 | Rapier 深造对标基准：新 Dynamic BVH / sparse voxel / persistent islands / glam 迁移 A/B | M65b no-go | RD-044「快路径被真实 workload 采用时」**字面不变**；基准先行不作 replay oracle；不成立则维持 no-go 留档 | R8 §Rapier | ⬜ | C | P2 | 环·产 | ✔ | G9.6 |
| M127 | 神经变形研究子轨：混合架构优先、离线工具链（corpus 即语料）、PhysicsAsset residual 通道预留 | `rfcs/0021` G9+ 研究轨留痕（行 122） | **无主线门、无 P0/P1 判据、不进 G9 收口硬门**；登记形式（独立 RD or 维持无归属留痕）**立项待裁决** | R8 §神经 | ⬜ | — | P3（研究轨） | —（不进验收） | — | 全程伴随，无硬门 |

---

## 6. 汇总

### 6.1 P0 地基清单（建议值，15 行；G9.1 决策表重裁后硬化）

| 波次 | P0 行 |
|---|---|
| G9.2 地基 | M90 DAG 深化 · M91 页格式 v2 ABI · M102 DGC 抽象 · M103 descriptor 全局表 · M104 AccessKind 新边 · M121 particle view 骨架 · M122 Field 骨架 |
| G9.3 几何×RT 合流 | M93 LOD/VisibleClusterSet · M94 CLAS 合流 · M95 单源真相集成 |
| G9.4 GI | M96 M17 参照器（波内第一顺位）· M97 Surface Cache · M98 追踪降级链 |
| G9.5 大世界×专项 | M110 分区数据模型（波内先行）· M118 显示管线/插件面 |
| G9.6 物理 | M121/M122 完整语义收尾 |
| G9.8a soak | 全部 P0 硬门绿后 soak；**禁止**条件实现后跳过 soak 直接 close |

### 6.2 统计

- **行数**：38 行（M90~M127），覆盖 D1×6 / D2×6 / D3×8 / D4×11 / D5×7 全部子面。
- **优先级分布（建议值）**：P0 = 15 · P1 = 18（含 M123 条件制）· P2 = 2（M114/M126）+ 2 立项待裁决（M108/M109，建议 P2 可选）· P3 研究轨 = 1。
- **档位分布**：含 A 档 5 行（M92/M100/M104/M106/M108/M109 中语言面）——伞形 RFC 语义面主体；其余重心 B/C/D。
- **4070 Ti 可验证性**：除 M118 设备标定层（需 HDR 设备，未触发则 SKIP 不充绿）外全部可真机验证——与 P-09 兼容。

### 6.3 承接锚映射总表（G8 行 → G9 行）

| G8 锚 | G8 决策字面 | G9 行 | 模块 |
|---|---|---|---|
| M06 骨骼/植被虚拟几何 | defer「G9+ 虚拟几何评估窗」 | M90/M92/M93 | D1 |
| M09 Mega Geometry 簇级 BLAS | defer「G9+ RT×Nanite 合流窗」 | M90/M94 | D1 |
| M12 Surface Cache | defer「G9+ GI 建造期」 | M97 | D2 |
| M14 HWRT hit lighting / Far Field | no-go「M50 后评估」 | M98 | D2 |
| M15/M22 MegaLights/ReSTIR + 海量灯阴影 | no-go「多灯场景需求出现时」 | M100 | D2 |
| M16 irradiance field 档位 | defer「G9+ GI 档位」 | M101 | D2 |
| M17 Path Tracer 参照器 | no-go「G9+ 建造期前置」（已命中） | M96 | D2 |
| M11 世界辐射缓存 | no-go 除非 measured | M99（世界 clipmap 级） | D2 |
| M33 shader library 组合链接 | defer「G9+ shader library 深化」 | M106/M107 | D3 |
| M52 SER | no-go 留档 | M108（**立项待裁决**） | D3 |
| M55 descriptor buffer/DGC | defer「G9+ GPU-driven 提交」 | M102/M103/M104/M105 | D3 |
| M61 mesh shader 第三光栅 | no-go 留档 | M109（**立项待裁决**） | D3 |
| M43 World Partition/HLOD | defer「G9+ 大世界分区」 | M110/M111 | D4 |
| M48 体积雾/云 | defer「G9+ 大气特效」 | M112 | D4 |
| M49 专项渲染器族 | defer「G9+ 专项渲染器」 | M113/M114/M115/M116/M117 | D4 |
| M45 HDR 管线 | no-go「HDR 显示设备资产/产品需求出现时」 | M118 | D4 |
| M46 后处理栈 | no-go「产品需求随 G9+ 建造期出现」 | M119 | D4 |
| M47 透明/OIT | no-go「OIT 策略选型需 measured 对照」 | M120 | D4 |
| M74 Physics Field | defer「G9+ gameplay Field」 | M121/M122 | D5 |
| M75 异步物理 tick | no-go「异步调度须独立判档」 | M123 | D5 |
| M77 水体/浮力 | no-go「ApplyBuoyancyImpulse 未包装」 | M124 | D5 |
| M65b Rapier 深造 | no-go「快路径被真实 workload 采用时」 | M126 | D5 |
| Jolt 5.6 评估窗 | G8.6a 纪律延续 | M125 | D5 |
| 神经变形 | rfcs/0021:122 G9+ 研究轨留痕 | M127 | D5 |

### 6.4 G9 成功判据草案（十条；G9.1 进 G9_CONTRACT 硬化为验收门；本矩阵只写判据草案，不 materialize CI 步骤）

渲染/平台侧：

1. **单源真相**：`VisibleClusterSet` 一份三喂光栅/RT/VSM；CLAS 腿与回退腿对同场景 ray query 逐命中一致；VRAM 占用 + AS 构建耗时 + CPU 构建带宽为 measured 硬指标，FPS 仅观察项（D1 D-9）。
2. **页格式 v2**：RXPL 新 major 编解码往返无损 golden；M04 v1 页 0-byte 兼容；篡改 digest 的页被拒。
3. **GI golden 门序**：M17 参照器门未绿，任何 GI 档位画质门不得验收；各档按「匹配深度」（1/2/full bounce）对 M17 golden；Surface Cache 缺失覆盖只丢能量不漏光负例 RED 臂有效。
4. **GPU-driven 提交**：DGC 链路零 CPU 回读（结构性断言 + 回读计数器=0）+ 与 CPU 录制等价场景像素级 golden；descriptor 全局索引与 shader 实际索引双向精确相等。
5. **着色治理**：IR 链接产物 interface hash 确定性 + 链接拓扑可回放；变体工程级总预算门硬失败有效。
6. **大世界流送**：预算契约字段逐帧 evidence；代表性大世界 soak（≥ G7 量级）hitch p99 ≤ measured 阈值；HLOD 双构建 hash 相等 + 运行时零合并断言。
7. **显示与透明**：四内置 view transform 插件逐一组 golden（含 AgX/ACES 已知差异记录）；OIT 默认档选型必须引 benchmark 数据（无数据选型提交判 RED）。

物理侧：

8. **Field 系统**：过滤默认空匹配 = 零影响显式断言；persistent field 注册/注销/变更全 journal 化且 replay 逐 tick hash 一致；World-Field 唯一出口 = GpuScene 只读 buffer、渲染侧零回写。
9. **浮力**：走 Field 通道求值（非旁路 API）；capture→replay 逐 tick hash 一致；变帧率输入同 tick 结果逐位一致。
10. **Jolt A/B**：RFC-0021 §4.A4 七步程序记录完整；采纳臂三件事/失败臂钉版证据齐备，两臂均诚实登记。

---

## 7. 门控维持与重审条件登记（只读引用，不改写既有注册表）

| 项 | 维持裁决 | 重审条件（字面来源） |
|---|---|---|
| DMM / displacement micromap | **永久禁止**（D1 D-7：NVIDIA 已归档，被 Mega Geometry 取代） | 任何 micromap 字样提案进 RFC 即一票否决 |
| Work Graphs（M56） | no-go 维持；render graph schema 预留 `reserved_` 前缀字段不接线 | RD-041 双条件字面：Vulkan 侧对应物成熟 + pass 内提交单元接缝预留 |
| async compute 第二腿（M59） | no-go 维持；DGC 全在单 queue 全序内表达 | RXS-0239 字面不动 + measured 收益证据（D3-Q7） |
| task shader（M62） | 不开放维持；RXS-0270 字面不动 | Amplification 语义出现真实消费方（D3-Q9：当前由 DGC 承担 fan-out） |
| GPU 主刚体 | 否决线维持（G6 裁决 + RFC-0017 + G8 矩阵 §12）；Jolt 5.6 GPU compute 只评估不接权威 | 矩阵 §12 五条件同时成立 + RD-043 触发 + 独立 Full RFC |
| 可微物理 | RD-042 观察维持，不进 D5 | RFC-0021 §2.4 |
| NRC / 神经 radiance cache | 观察项（D2 out） | SG-002 Tensor Core 族禁止面解除 |
| 帧生成 FG/MFG（M26） | 不进 G9 | RD-041 分项「独立层另判」字面 |
| SVT/RVT/sampler feedback（M40/41/42） | no-go 维持；D4 设计显式排除依赖 | 真实大纹理资产需求独立判档，不搭 D4 便车（R-D4-7） |
| USD（M86）/ MaterialX（M87） | 不进 G9 主线 | 真实资产需求 + TOST 法务标注（G8.7 字面） |
| GPU 粒子 VFX（M49a）/ present pacing（M49b） | 不进 G9 | RD-044 特效管线真实出现时 / latency measured 需求 |
| DXIL RT/mesh 腿（RD-034） | blocked 维持；D1~D3 仅 Vulkan 主腿 | 上游二选一解锁（spirv-cross RT 消费或 LLVM 签名钳制解除） |
| Safe GPU Operator Platform | **归属立项待裁决**（进 G9 独立轨道 or G10+）；与 UE5 渲染/物理前置无依赖 | G8_PLAN §1.1 留痕；G9_PLAN §5 待裁决表 |
| 神经变形权威化 | 「NN 权威禁止线」建议（D5-12）：NN 输出不得替代权威状态 | 研究子轨成果另行判档，不占主线门 |

---

## 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-08 | 初版：M## 顺延 G8（M90~M127，38 行）覆盖 D1~D5 全部子面；每行带 G8 承接锚 / backfill 触发字面 / 建议 P 级 / 验收五层级；M108/M109 改判与分包规模标「立项待裁决」不擅自定案；承接锚映射总表 + 成功判据草案十条 + 门控维持登记。零编号占用、零 registry 改动。 |
