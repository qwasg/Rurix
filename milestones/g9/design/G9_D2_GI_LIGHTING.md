# G9_D2 — 全局光照与光照缓存（GI 建造期）模块设计草案

> **DRAFT 设计提案——G9 未立项，不构成契约/验收承诺**
> 本文件是 G9 模块 D2 的设计草案，供 G9 立项评审使用。G9 契约（CONTRACT / CI_GATES / ACCEPTANCE_MAP / budget）未定稿前，本文任何「门 / 判据 / 波次」均为建议性质，**不**构成验收承诺；G9 立项后须经治理流程升格为契约文本方可作为验收依据。所有编号（RXS/RD/U/RX/CI step）一律待立项后按 actual next_free 领取，本文不 claim 任何号。
> **G9.0 冻结引用**：2026-08-08 起，本文作为 G9.0 文档集不可变基线附件被 [G9_PLAN.md](../G9_PLAN.md) 冻结引用；正文 0-byte，后续变更只追加修订记录（追加于文末）。

---

## 1. 定位与承接锚

**D2 = G9 UE5 级渲染器建造期的全局光照（GI）与光照缓存模块。** 目标是把 G8 前置期建成的 RT 增量面、GI 三核与 CPU 参考对拍体系，升级为一套**可降级、可生产化、有跨路径 golden 参照**的完整 GI 子系统：以 Lumen 全链路为主参照架构，以 MegaLights/ReSTIR 处理多灯直接光，以 irradiance field 档位阶梯覆盖画质/性能光谱，以 M17 Path Tracer 参照器作为一切画质门的 golden 基准。

**G8 承接锚（法定输入，逐字引用）**：

| 锚 | 来源 | 字面结论 | D2 处置 |
|---|---|---|---|
| M12 Surface Cache | `milestones/g8/G8_P2_DECISIONS.md:13` | defer-to-G9+，「G9+ GI 建造期」 | 本模块核心子系统（§4.1） |
| M14 HWRT hit lighting / Far Field | `milestones/g8/G8_P2_DECISIONS.md:14` | no-go，open-留档，「M50 后评估，无画质 measured 需求方」 | M50 已绿；D2 即 M50 后的评估窗，hit lighting 作为追踪降级链最高档、Far Field 作为末档重新判档（§4.2） |
| M15 MegaLights / ReSTIR | `milestones/g8/G8_P2_DECISIONS.md:15` | no-go，RD-040「多灯场景需求出现时」 | 多灯场景需求 = UE5 级渲染器建造期的既定目标面，作为 RD-040 backfill 触发的 candidate workload 立项时举证；多灯直接光子系统（§4.4） |
| M16 irradiance field 档位 | `milestones/g8/G8_P2_DECISIONS.md:16` | defer-to-G9+，「G9+ GI 档位」 | 档位阶梯子系统（§4.5） |
| M17 Path Tracer 参照器 | `milestones/g8/G8_CANDIDATE_DECISIONS.md:125`（§10） | no-go，backfill =「GI/材质画质门需要跨路径 golden 时（G9+ 建造期前置）；G8.7 复审」，open-留 G8.7/G9+ | 触发条件字面命中——D2 各画质门均需要跨路径 golden。M17 参照器为 D2 **第一前置**（§4.6），golden 对拍门先于一切 GI 档位验收（§7） |
| 波次归口 | `milestones/g8/G8_PLAN.md:227-232`（§2.5b） | 「HDR / 后处理 / 透明·OIT / Path Tracer 参照器 M45/M46/M47/M17」「Path Tracer 依赖 G8.2 RT 增量面」 | M17 与 G8.5b 归口一致；D2 消费 G8.2 RT 增量面成果 |

**现有基础（G8 交付，D2 直接消费，不重做）**：

- **W3 GI 三核**：`src/rurix-render/src/gi/`（tracer/probe/sh/interpolate/filter/temporal/pipeline）——屏幕探针 GI host 全组件 + Vulkan RayQuery device 腿；`src/rurix-render/src/rt/`（bvh/as_manager/ref_tracer/effects/denoise）——RTAO/硬阴影 + CPU 参考对拍金标准（PCG32 确定性、`RAY_EPS` 对拍契约常量）。
- **VSM** 完整页缓存（`src/rurix-render/src/shadow/`，G8.5a 门已绿）。
- **TAA/TSR** 生产契约（G8.5b 门已绿）。
- **M50 RT 增量面**：多 hit group / SBT 用户数据 / stack sizing / pipeline library（G8.2 strategic_override 交付）——hit lighting 与 M17 RT pipeline 腿的底座。
- **追踪层统一契约已冻结**：`gi::tracer::RadianceTracer`（`gi/mod.rs` 头注：「P0 冻结，SDF/ReSTIR 未来同接口可替换」）——D2 的 SWRT SDF 与 ReSTIR 接入点已在 G8 预留。

---

## 2. 范围 in / out

### 2.1 in（D2 交付面）

1. **Surface Cache**：mesh 离线 Card 参数化 + 运行时命中点辐射度缓存（Lumen 式，缺失覆盖只丢能量不漏光）。
2. **追踪降级链**：Screen Trace → SWRT（Mesh/Global SDF）→ HWRT（RayQuery，含 hit lighting 简化材质档）→ Far Field（远场代理），逐档可关、可测。
3. **Screen Probe Gather + Radiance Cache**：屏幕空间 probe gather（自适应细分 + 空间滤波）+ 双级 radiance cache（屏幕空间 + 世界空间 clipmap），第一反弹 product importance sampling。
4. **多灯直接光**：低档位 = MegaLights 式固定随机选灯 + 解析/随机阴影拆分估计；高档位 = ReSTIR reservoir 复用（可选增强），海量灯阴影统一接口随动（承接 M22 联动关系）。
5. **irradiance field 档位阶梯**：L0 屏幕 probe → L1 clipmap 体积 probe → L2 空间哈希缓存 → L3 per-pixel，共享 probe 着色与八面体编码内核，只换空间索引。
6. **M17 Path Tracer 参照器**：单向 PT + NEE/MIS/RR，固定 seed 确定性，逐像素 sample count 导出 + 方差/收敛曲线，每 GI 档位定义「匹配深度」（1/2/full bounce）。

### 2.2 out（不进 D2）

| 项 | 处置 |
|---|---|
| NRC / 神经 radiance cache（RTXGI 2.0 第三腿） | 观察项；GPU tensor/神经网络属既有 SG 禁止面，且训练基建超 D2 |
| RD-034 DXIL RT 腿 | 维持 blocked；D2 全部 RT 走 Vulkan 主线 |
| SER（VK_EXT_ray_tracing_invocation_reorder） | 演进项；仅在接口层预留队列化中间层，不实现 |
| Far Field HLOD 资产生产链（HLOD1 代理生成） | 归 G9 资产/几何模块；D2 只定义消费接口与 golden |
| 透明 GI / 焦散 / 体积光照 | 后续 G9 子模块或 G9+ |
| 多 GPU / WebGPU / 编辑器 GUI | 承 G8 §1.5 out-of-scope 口径 |
| 任何 `spec/`/`conformance` 改动 | 立项后 spec-first 流程；本草案期 0-byte |

---

## 3. 依赖前置

| 前置 | 状态 | D2 消费方式 |
|---|---|---|
| G8.2 M50 RT 增量面（多 hit group / SBT 用户数据 / stack sizing / pipeline library） | closed（G8.2 退出门） | hit lighting 材质记录、M17 RT pipeline 腿 |
| W3 GI 三核 + RayQuery device 腿（`gi/`、`rt/`） | closed（G8 交付） | L0 屏幕 probe 档直接在其上增量；`RadianceTracer` 契约复用 |
| CPU 参考对拍体系（`ref_tracer` PCG32 确定性） | closed | golden 协议与 device/host 对拍 harness 的既有模式 |
| VSM 页缓存 + 时域公共底座（TAA/TSR temporal） | closed | 阴影可见性查询、probe 历史重投影（禁私写重投影纪律延续） |
| RD-037 单源 gfx submit / M04 页格式 ABI / AsManager | closed | probe 图集/radiance cache 资源生命周期与驻留 |
| G9 立项 + 契约四件套 + 非空 measured budget | **未满足** | D2 一切实现波次的硬前置 |
| M17 golden 对拍门绿 | **D2 内部第一门** | GI 各档位验收的前置门（见 §7 门序） |

---

## 4. 模块分解

### 4.1 Surface Cache（承接 M12）

- **离线侧**：cook 期对每 mesh 做 Card 参数化（默认上限 12 Card/mesh，Lumen 口径；超出按表面积/视角覆盖率裁剪，裁剪策略进 cook profile 可配）。Card 图集页格式复用 M04 版本化 ABI，禁止 D2 私定磁盘格式。
- **运行时侧**：Card atlas 驻留管理（稀疏更新，相机相关优先级）；命中点辐射度写入 = 完整材质求值 + 直接光 + 已缓存间接光（单帧延迟反馈，Lumen 同构）。
- **缺失覆盖语义（硬契约）**：Card 未覆盖区域**只丢能量不漏光**——采样回退到下一级追踪结果或 ambient 项，严禁产生负值/黑色裂缝；该语义进负例 RED 臂（§7）。
- **接口**：`RadianceTracer` 契约之上新增「Surface Cache 命中/未命中」二级查询，对上层 GI 档位透明。

### 4.2 追踪降级链（承接 M14 重判档）

四级降级，每级独立开关 + 独立 evidence 计数面（命中率/射线量/耗时）：

| 级 | 机制 | 覆盖范围 | 调研依据 |
|---|---|---|---|
| L1 Screen Trace | 屏幕空间高度场 ray march（HZB/深度） | ~50 m 内、屏幕内 | Lumen 第一层；成本最低 |
| L2 SWRT | Mesh SDF（近场逐对象）+ Global SDF（远场合并），RayQuery 不适用处走 compute SDF 步进 | ~200 m | Lumen SWRT 两层；`RadianceTracer` 契约已预留 SDF 实现位（G8 `gi/mod.rs` 头注） |
| L3 HWRT | Vulkan RayQuery 对 TLAS 追踪；命中着色两档：简单兜底求值（默认）/ **hit lighting** 完整材质求值（高档，需 RayTracingQualitySwitch 式材质简化开关，消费 M50 多 hit group） | 全场景、视距内 | Lumen HWRT + Hit Lighting（调研 1）；M14 的 measured 需求方 = D2 自身画质门 |
| L4 Far Field | 远场代理辐射度（HLOD1 式代理，~1 km 量级）；D2 仅消费接口，资产生成归几何模块 | 视距外 | Lumen Far Field（调研 1）；M14 Far Field 分项同步重判 |

**降级选择契约**：逐 probe/逐像素按命中距离与覆盖优先级选档，选择结果入 evidence（禁静默回退）。

### 4.3 Screen Probe Gather + Radiance Cache

- **SPG**：屏幕空间 probe 放置（基线 16 px/probe + 自适应细分，Lumen 口径），3×3 probe 空间滤波（≈48×48 屏幕有效滤波）。在 G8 既有 1/16 均匀 probe + 3×3 滤波（`gi/probe.rs`/`gi/filter.rs`）上增量：自适应细分判据 = 深度/法线不连续性 +  radiance 方差。
- **Radiance Cache 双级**：屏幕空间级（复用 probe 历史，时域公共底座重投影）+ 世界空间 clipmap 级（绕相机分级，承接 G8 M11「世界辐射缓存」观察项的 measured 触发）；第一反弹采样 = BRDF×入射光 **product importance sampling**（调研 1）。
- **与档位阶梯关系**：本子系统是 L0 档的完整形态；L1+ 复用其 probe 着色与八面体编码内核（§4.5）。

### 4.4 多灯直接光（承接 M15/M22）

| 档位 | 机制 | 调研依据 |
|---|---|---|
| 低档（默认） | **MegaLights 式**：每像素固定随机选灯 + 解析无阴影 DI 与随机阴影拆分估计（S/U 比值法）+ hidden light budget 控总成本 | 调研 2：1 ray/pixel 预算下完整 ReSTIR reservoir 复用需 2–3× 验证射线、常数成本过高，故 ReSTIR 仅作可选增强（Narkowicz & Costa, SIGGRAPH 2025） |
| 高档（可选） | ReSTIR reservoir 时空复用；候选演进：ReGIR 世界空间网格 reservoir（RTG2 2021）、GRIS+shift mapping 跨域复用（Lin 2022）、PSMS-ReSTIR/RGB ReSTIR 时空相关性伪影修复（2025/2026） | 调研 2 |

- **验证射线纪律（硬）**：任何复用路径不得跳过验证射线——GI-1.0 教训：跳验证引入系统性变暗偏置（调研 3）；该断言进 RED 臂（统计性亮度偏置门）。
- **海量灯阴影统一接口**随本子系统联动交付（M22 在 G8 决策表中「随 M15/RD-040」）。
- **RD-040 触发举证**：立项时须附多灯场景 workload 证据（UC 场景灯数/成本曲线），作为 RD-040 backfill「多灯场景需求出现时」的字面兑现，禁止无证据主线化（G8 §1.2 纪律延续）。

### 4.5 irradiance field 档位（承接 M16）

四级阶梯，**共享 probe 着色内核与八面体编码，只换空间索引结构**：

| 档 | 空间索引 | 说明 | 调研依据 |
|---|---|---|---|
| L0 | 屏幕空间 probe | 即 §4.3 SPG | Lumen SPG |
| L1 | clipmap 体积 probe | DDGI 基线：八面体 irradiance 8×8 + **高分辨率 visibility 16×16（防漏光优先于提 irradiance 分辨率）** + 每帧轮换更新摊销；DDGI Resampling（2021）补直射/非漫反射项为演进 | Majercik JCGT 2019（调研 3） |
| L2 | 空间哈希缓存 | SHaRC 式空间哈希 radiance 缓存，按需分级 | RTXGI 2.0（GDC 2024）行业走向「哈希缓存+按需分级」（调研 3） |
| L3 | per-pixel | 全分辨率逐像素追踪，参考档/截图档 | Lumen ReferenceMode 同位（调研 1） |

- **AS 更新预算（硬契约）**：每档位定义须含 AS 更新预算行——HWRT AS 更新成本 >100 k 实例时显著（调研 5），档位切换判据必须消费 AsManager 既有 `AsStats` 计数面。

### 4.6 M17 Path Tracer 参照器（D2 第一前置）

- **架构**：单向 PT + NEE/MIS/RR 起步；**megakernel 优先**但接口按 wavefront 阶段化切分（ray gen / intersect / shade / reservoir 各阶段独立可替换，为 SER 演进留位）（调研 4）。
- **双锚对照**：正确性锚点 = pbrt-v4（wavefront 架构文献基线）；工程模式 = UE Path Tracer 式「与实时管线共享场景/材质输入，不共享光照算法」——golden diff 可归因到算法层而非输入层（调研 4）。
- **确定性协议（硬）**：固定 seed + 逐像素 sample count 导出 + 方差/收敛曲线进 evidence；每 GI 档位定义「匹配深度」（1 bounce / 2 bounce / full bounce）作为对拍容差前提。
- **执行路径纪律**：M17 的 PT 递归走 RT pipeline（消费 M50）；与 GI 各档的 RayQuery 射线流**严禁混用同一射线流**（Arm 最佳实践，调研 5）；ray 生成与命中处理间加队列化中间层，为 SER 预留。

---

## 5. 关键设计决策表

| # | 决策 | 理由 | 调研引用 |
|---|---|---|---|
| D2-Q1 | 主架构对齐 Lumen 全链路（Surface Cache + 四级降级 + SPG + Radiance Cache），而非自研替代 | UE5 级等价是项目既定验收基线；Lumen 是唯一公开完整工程化细节的实时 GI 架构 | 调研 1（SIGGRAPH 2022 Wright et al.） |
| D2-Q2 | Surface Cache 缺失覆盖只丢能量不漏光，且进负例 RED 臂 | 漏光比丢能量视觉上更不可接受；该语义是 Lumen 鲁棒性核心 | 调研 1 |
| D2-Q3 | 多灯直接光低档默认 MegaLights 式固定随机选灯，ReSTIR 仅高档可选 | 1 ray/pixel 预算下完整 reservoir 复用常数成本过高（2–3× 验证射线） | 调研 2（SIGGRAPH 2025） |
| D2-Q4 | 任何复用路径禁止跳验证射线，偏置门进验收 | 跳验证引入系统性变暗偏置，且偏置随场景复杂度放大、事后不可归因 | 调研 3（GI-1.0, arXiv 2023） |
| D2-Q5 | irradiance field 阶梯共享 probe 着色与八面体编码内核，只换空间索引 | 避免四套实现四套 bug；内核同源使档间 golden 对拍可归因到索引结构 | 调研 3 |
| D2-Q6 | DDGI 档 visibility 分辨率（16×16）优先于 irradiance 分辨率（8×8） | 防漏光优先；漏光是高频可见伪影，irradiance 分辨率不足只是软误差 | 调研 3（Majercik JCGT 2019） |
| D2-Q7 | M17 参照器先于一切 GI 档位建造，golden 门为各档前置 | 无跨路径 golden 的画质门不可验收（防假绿治理原则）；M17 backfill 字面即此 | `G8_CANDIDATE_DECISIONS.md:125`；调研 4 |
| D2-Q8 | M17 用 megakernel 起步、接口按 wavefront 阶段化切分 | 工程简单优先；阶段化接口为 SER 与 hit-lighting 递归留演进位 | 调研 4 |
| D2-Q9 | GI 各档追踪全走 RayQuery+compute；RT pipeline 仅 M17 与未来 hit-lighting 递归；严禁混用同一射线流 | 批量均匀射线是 RayQuery 甜区（AMD 上常快于 RT pipeline）；混用破坏两边性能特征 | 调研 5（Arm 最佳实践） |
| D2-Q10 | 每 GI 档位定义强制含 AS 更新预算行，档位切换消费 AsStats | >100 k 实例时 AS 更新成本显著，不计量则档位切换判据失真 | 调研 5 |
| D2-Q11 | hit lighting 需 RayTracingQualitySwitch 式材质简化开关，消费 M50 多 hit group | 命中点完整材质求值成本不可控；M50 增量面正是为此战略 override | 调研 1；`G8_CANDIDATE_DECISIONS.md:10` |
| D2-Q12 | Far Field 只定义消费接口，HLOD 代理生成归几何/资产模块 | 资产生成链与 GI 运行时解耦，避免 D2 范围爆炸 | 调研 1（Far Field=HLOD1 代理） |
| D2-Q13 | SER 不实现，仅预留队列化中间层 | VK_EXT_ray_tracing_invocation_reorder 2025-11 刚落地，驱动面未稳；接口预留成本极低 | 调研 5 |
| D2-Q14 | probe 历史/时域累积一律经 temporal 公共底座，禁私写重投影 | 承 G8 RFC 章 H 纪律与 G-G5-7 审计点；私写重投影是历史伪影主源 | `src/rurix-render/src/rt/mod.rs` 头注 |

---

## 6. 波次建议

```text
D2.0 治理包（契约/RFC/决策表/验收映射/measured budget；M15 的 RD-040 触发举证）
  → D2.1 M17 Path Tracer 参照器（megakernel + 确定性协议 + pbrt-v4 对拍）★ 第一前置
  → D2.2 Surface Cache 离线 Card 参数化 + 运行时缓存（L3 追踪档先行）
  → D2.3 追踪降级链 L1 Screen Trace / L2 SWRT SDF / L4 Far Field 接口 + hit lighting 档
  → D2.4 SPG 自适应细分 + Radiance Cache 双级（product importance sampling）
  → D2.5 多灯直接光（低档 MegaLights 式 → 高档 ReSTIR 可选增强）
  → D2.6 irradiance field 档位 L1 DDGI → L2 空间哈希 → L3 per-pixel 参考档
  → D2.7 全档 soak + close-out
```

- 波次内可蜂群并行，波次间串行；**D2.1 未绿，D2.2 起任何画质门不得验收**（门序硬约束）。
- D2.2 与 D2.3 的 L3 档互为依赖，建议同波或 D2.3-L3 提前；L4 Far Field 依赖几何模块 HLOD 接口，未就绪时登记 SKIP=not-triggered，不充绿。
- 条件实现刚绿不得当日进 D2.7（承 G8.8a 纪律）。

---

## 7. 验收门草案

**通用门型（每门四件套）**：断言 + device 真跑对拍 + golden + 负例 RED 臂；evidence schema 名按下表。所有门须经 G9 ACCEPTANCE_MAP 映射后才生效。

| 门 | 断言要点 | device 对拍 | golden | 负例 RED 臂 | evidence schema（建议名） |
|---|---|---|---|---|---|
| **G-D2-1 M17 参照器（前置门）** | 固定 seed 两次运行位级一致；逐像素 sample count 导出非空 | RT pipeline device 路径真跑；与 CPU 参考（复用 `ref_tracer` PCG32 模式）方向一致性对拍 | pbrt-v4 同场景同 spp 收敛曲线对比（容差带）；1/2/full bounce 三匹配深度各一 golden | 改 seed / 跳 RR / 关 MIS 三臂必须 RED（确定性/收敛性破坏可检测） | `g9_d2_m17_pt_golden_v1` |
| **G-D2-2 Surface Cache** | 缺失覆盖回退路径存在且只丢能量（输出非负、无低于 ambient 的黑色裂缝） | Card atlas device 写入/采样真跑，与 host 参考对拍 | Card 参数化 cook golden（同资产双构建 hash 相等，复用 M79/M04 模式） | 故意制造 Card 空洞 → 漏光检测臂必须 RED | `g9_d2_surface_cache_v1` |
| **G-D2-3 追踪降级链** | 四级各自命中率/射线量/耗时计数非空；降级选择入 evidence 禁静默 | L1–L3 各档 device 真跑；SWRT SDF 与 RayQuery 同场景对拍 | 每档对 M17 参照器 1-bounce golden（容差按匹配深度表） | 逐级强制关闭 → 能耗/画质回归可检测；L3 hit lighting 关材质简化开关 → 预算超限 RED | `g9_d2_trace_fallback_v1` |
| **G-D2-4 SPG + Radiance Cache** | 自适应细分判据可复现；重投影全经 temporal 公共底座（审计点） | probe gather + 双级 cache device 真跑 | 对 M17 2-bounce golden；product importance sampling 与均匀采样方差对比 golden | 私写重投影 / 关 product IS → 方差或伪影回归 RED | `g9_d2_spg_radiance_cache_v1` |
| **G-D2-5 多灯直接光** | hidden light budget 生效（灯数增长时射线成本有界）；验证射线零跳过（统计性亮度偏置门） | 低档/高档各 device 真跑 | 对 M17 NEE/MIS 直接光 golden；ReSTIR 档与低档方差对比 | 跳验证射线 → 系统性变暗偏置检测臂必须 RED（D2-Q4）；时空复用相关性伪影场景（PSMS-ReSTIR 类用例）回归臂 | `g9_d2_many_light_v1` |
| **G-D2-6 irradiance field 档位** | 档间共享内核（着色/八面体编码同一函数实例）；每档 AS 更新预算行非空 | L1 DDGI / L2 哈希缓存 device 真跑；逐档轮换更新摊销计数 | 每档对 M17 golden；DDGI visibility 16×16 防漏光场景 golden | visibility 降采样 → 漏光检测臂 RED；超 AS 更新预算 → 档位强制降级臂 | `g9_d2_irradiance_field_v1` |
| **G-D2-7 soak** | 全档代表性场景 soak（阈值不低于 G-G7-8 量级）；budget_eval --strict 零 estimated/skip | One True Device Frame 连续 resource provenance | — | — | `g9_d2_soak_v1` |

**门序（硬）**：G-D2-1 未绿 → G-D2-2~6 全部不得验收；任一门 RED 臂失效（破坏后仍绿）→ 该门整体判无效，不得 close。

---

## 8. 风险与止损

| ID | 风险 | 预警 | 止损 |
|---|---|---|---|
| R-D2-1 | M17 参照器范围爆炸（做成完整渲染器） | 波次 D2.1 超预算 50% | 钉死范围：单向 PT+NEE/MIS/RR、megakernel、仅 golden 用途；焦散/体积/specular 链明确 out |
| R-D2-2 | 无 golden 的画质门假绿 | 某档验收绕过 G-D2-1 | 门序硬约束进 ACCEPTANCE_MAP validator；close-out 审计门序 |
| R-D2-3 | Surface Cache 漏光（薄几何/未覆盖区） | 负例臂首次运行即绿（臂失效） | 漏光检测臂独立评审；G-D2-2 不绿则 D2.4 不起 |
| R-D2-4 | 复用路径系统性偏置（跳验证/时域过复用） | 亮度统计漂移但逐帧难察 | D2-Q4 偏置门（与 M17 全量参考的长期均值对比）；验证射线计数入 evidence |
| R-D2-5 | AS 更新成本失控（>100 k 实例） | AsStats 更新耗时超档位预算 | 档位定义含预算行（D2-Q10）；超预算强制降档路径必须已验收 |
| R-D2-6 | RayQuery 与 RT pipeline 混用同一射线流致性能特征互污 | 混用代码进 review | D2-Q9 架构纪律 + review 检查表；队列化中间层作为唯一交汇点 |
| R-D2-7 | 时空复用相关性伪影（ReSTIR 档） | 高档位出现 streak/拖影回归 | PSMS-ReSTIR/RGB ReSTIR 类负例场景入库；ReSTIR 维持可选增强、低档可回退 |
| R-D2-8 | Far Field 依赖几何模块 HLOD 未就绪 | D2.3 L4 长期 SKIP | 登记 SKIP=not-triggered 不充绿；接口 golden 先行，消费侧后补 |
| R-D2-9 | M15/RD-040 触发证据不足被指静默主线化 | 立项评审质疑 | D2.0 治理包强制附多灯 workload 证据；不足则 M15 分项维持 open-留档、D2.5 只做低档 |
| R-D2-10 | 条件实现刚绿即 close | 跳过 D2.7 soak | D2.7 为 close 前置硬门（承 G8 R-G8-9） |

---

## 9. spec / RFC 需求

均待 G9 立项后按 spec-first 流程提出（spec 条款 PR 先于实现 PR）；本草案期 `spec/`/`conformance` 0-byte。

| 需求 | 内容要点 | 归属建议 |
|---|---|---|
| RFC-G9-α GI 档位语义 | L0–L3 档位定义、切换判据、AS 更新预算行格式、降级链选择契约（禁静默回退）、「匹配深度」表 | 新 RFC（对标 G8 RFC-0019 渲染平台语义） |
| RFC-G9-β Surface Cache 格式 | Card 参数化 cook profile schema、atlas 页格式（复用 M04 ABI 的修订行）、缺失覆盖回退语义 | 资产管线 RFC 修订（对标 G8 RFC-0020） |
| RFC-G9-γ RT pipeline 递归与 M17 语义 | hit group 递归深度/stack sizing 扩展、M17 wavefront 阶段化接口、RayQuery/RT pipeline 混用禁令的语义化、队列化中间层（SER 预留位） | G8 RFC-0019 RT 章修订 |
| RFC-G9-δ golden 确定性协议 | 固定 seed、逐像素 sample count 导出、方差/收敛曲线 evidence 字段、device/host 对拍 harness 统一约定（承 `ref_tracer` PCG32 模式） | conformance 协议章 |
| spec 条款 | `spec/vulkan_backend.md` RayQuery 章扩展（SWRT/Global SDF、probe tracing 批量均匀射线模式）；`spec/shader_stages.md` hit lighting 材质简化开关（RayTracingQualitySwitch 式）语义 | 各自 spec 文件修订 |

---

## 10. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v0.1 | 2026-08-07 | 首版 DRAFT。基于 G8 收口事实（M12/M14/M15/M16 P2 决策行、M17 §10 no-go 行、W3 GI 三核/VSM/TAA-TSR/M50 现状）与五项调研结论撰写；零号 claim、零 spec/src/conformance 改动。 |
