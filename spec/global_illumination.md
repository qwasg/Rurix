# global_illumination.md — 全局光照（GI）语义面（G9.4 M96~M101）

> **地位**：D2 全局光照语义事实源——M96 M17 Path Tracer 参照器（megakernel +
> NEE/MIS/RR + 固定 seed 确定性协议 + pbrt-v4 容差带对照 + golden 门序硬约束）、
> M97 Surface Cache、M98 四级追踪降级链、M99 屏幕级 SPG 自适应细分 + Radiance
> Cache 双级、M100 低档多灯直接光默认档、M101 IF 体素网格档位阶梯（RFC-0022
> §4.6/§4.7/§4.8/§4.10 + §7 D2-Q4，Agent Approved 2026-08-09；
> G9_ACCEPTANCE_MAP §2 M96/M97/M98 行 + §3 M99/M100/M101 行〔G9.4 波 P1 全进
> 裁决登记，G9_CONTRACT §8.1 裁决①〕）。G8 已冻结的 `RadianceTracer` 契约面与
> temporal 时域底座（spec/rendering_platform.md / RFC-0019 §4.6.1）**字面
> 0-byte 不动**；本文件只承载 G9.4 GI 波新增语义。
>
> **档位**：Full RFC / RFC-0022。
>
> **编号**：RXS-0357~0362（G9.4 spec-first，自合入时实测
> `registry/number_ledger.json` `RXS.next_free = 357` 顺位领取，0357~0362
> 连续不跳号；编号永不复用，10 §9.5）。
>
> **新建裁决留痕（G9.4 spec PR）**：RFC-0022 §5 条款映射表 GI 各语义面的候选
> 目标 spec 为——「Card 参数化 cook profile schema / 缺失覆盖回退语义」=
> `spec/rendering_platform.md` 或资产管线 spec 修订；「追踪降级链 L1~L4 选档
> 契约」= `spec/rendering_platform.md` + `spec/shader_stages.md`；「probe 着色/
> 八面体编码共享内核、SPG 自适应细分判据、IF L0~L3 档位与 AS 更新预算行」=
> `spec/rendering_platform.md`；「golden 确定性协议」= conformance 协议章。本波
> 裁定**新建本文件**——六 M## 语义面同属 D2 全局光照独立语义轴（参照器 /
> 缓存 / 降级链 / 探针档位 / 多灯），与 rendering_platform.md 已冻结的
> reflection/canonical serialization/capability profile/PSO manifest 面
> （RXS-0304~0318/0347）不同轴、与 shader_stages.md 语言层类型面不同轴；
> rendering_platform.md / shader_stages.md / 资产管线 spec 本体 0-byte，
> conformance 锚定语料落 conformance/gi/（新建子目录）。新建裁决沿 G9.2
> virtual_geometry.md 新建先例（spec/README.md §4 登记 + 本头注留痕）。
>
> **多灯语义面边界**：RFC-0022 §7 逐字「多灯直接光（M100 MegaLights/ReSTIR）：
> 语义归 D2 后续 RFC/修订行，本章不冻结（RD-040 触发举证先行）」——RXS-0361
> 只冻结 RD-040 条件分项已判 go 的**低档默认面**（G9_CANDIDATE_DECISIONS §2
> M100 行 + v1.3 校准注字面），不构成对高档 ReSTIR 语义面的冻结。
>
> **门序硬约束（D2-Q7，机器阻断）**：M96（`g9.p0.m96.path_tracer_reference`）
> 未绿，M97~M101 任何画质门不得验收（任何 GI 档位画质门）——G9.4 波内第一
> 顺位；门序进 G9_ACCEPTANCE_MAP validator 机核，close-out 审计门序
> （RFC-0022 §4.10 / G9_PLAN:143 / G-G9-6 / G9_CAPABILITY_MATRIX §6.4 判据 3
> 逐字）。本文件全部下游条款（RXS-0358~0362）验收均以 RXS-0357 门绿为前置。

---

## 1. 范围与体例

- 体例 = FLS 风格（spec/README.md §2）；本文件**严禁 UB 节**——参照器、缓存、
  降级链与档位选择所有失败均为 typed `Err` / 确定性拒绝（fail-closed），不设
  未定义行为。
- 实现锚定（实现期命名，`src/rurix-render` 维持 `forbid(unsafe_code)` 纪律）：
  M96 megakernel 参照器与 pbrt-v4 对照 harness；M97 cook 期 Card 参数化器
  （`src/rurix-asset` cook 面扩写）与运行时 Card atlas 辐射度缓存；M98 四级
  追踪降级链选档器与逐档计数面；M99 SPG/Radiance Cache 屏幕级；M100 低档多灯
  直接光；M101 IF 档位与共享 probe 内核。
- 每条款 ≥1 `//@ spec: RXS-####` 测试锚定（traceability 矩阵全锚定，10 §4）。

## 2. 术语

- **NEE / MIS / RR**：next event estimation（显式光源采样）/ multiple
  importance sampling（多重要性采样）/ Russian roulette（轮盘赌路径终止）——
  参照器三大方差削减机制（RFC-0022 §4.10）。
- **megakernel 起步**：单一 compute/RT 入口承载全路径求值，接口按 wavefront
  阶段化切分（ray gen / intersect / shade / reservoir 各阶段独立可替换，为
  SER 与 hit-lighting 递归演进留位，D2-Q8）。
- **匹配深度**：每 GI 档位对拍 M96 golden 的 bounce 深度前提 = `{1, 2, full}`
  三档，各一 golden（RFC-0022 §4.10）。
- **Card**：mesh 表面离线参数化图集单元（Lumen 口径 ≤12/mesh 可配）；运行时
  辐射度缓存的空间载体（RFC-0022 §4.6）。
- **四级追踪降级链**：L1 Screen Trace → L2 SWRT（Mesh/Global SDF）→ L3 HWRT
  （RayQuery，含 hit lighting 档）→ L4 Far Field（RFC-0022 §4.7 表逐字）。
- **SPG**：屏幕空间 probe 网格（16 px/probe 基线 + 自适应细分 + 3×3 空间
  滤波，RFC-0022 §4.8）。
- **Radiance Cache 双级**：屏幕空间级（复用 probe 历史）+ 世界空间 clipmap
  级（绕相机分级；世界级未 measured 举证前 not-triggered，RFC-0022 §4.8）。
- **IF 档位阶梯**：irradiance field L0~L3——L0 屏幕空间 probe / L1 clipmap
  体积 probe（DDGI 基线）/ L2 空间哈希缓存 / L3 per-pixel 参考档
  （RFC-0022 §4.8 表逐字）。
- **验证射线零跳过**：GI 各档复用路径不得跳过验证射线（跳验证引入系统性变暗
  偏置、随场景复杂度放大、事后不可归因，D2-Q4，RFC-0022 §7 否决行）。
- **门序硬约束**：M96 未绿则 M97~M101 任何画质门不得验收（D2-Q7）。

---

## 3. 条款（RXS-0357，G9.4 M96 Path Tracer 参照器）

### RXS-0357 M17 Path Tracer 参照器 megakernel 架构、固定 seed 确定性协议与 pbrt-v4 容差带对照

**Legality**

1. **架构冻结（megakernel 起步 + NEE/MIS/RR）**（RFC-0022 §4.10 逐字；判据
   逐字引 G9_ACCEPTANCE_MAP §2 M96 行）：参照器 = **单向 PT + NEE/MIS/RR**，
   **megakernel 起步**，接口按 wavefront 阶段化切分（ray gen / intersect /
   shade / reservoir 各阶段独立可替换，D2-Q8）。**起步范围冻结**：焦散 / 体积 /
   specular 链明确 **out**（判据逐字引同上「megakernel 起步范围冻结（焦散/体积/
   specular 链明确 out）」）。
2. **固定 seed 确定性协议（硬）**（RFC-0022 §4.10 / §4.0-3 逐字；判据逐字引
   G9_ACCEPTANCE_MAP §2 M96 行）：**固定 seed 两次运行位级一致**——承 G8
   `ref_tracer` PCG32 对拍模式，**累加序与 RNG 流冻结**（采样维序、累加顺序、
   PCG32 流推进序均为协议面；canonical digest 不含路径、mtime、随机 seed）；
   **逐像素 sample count 导出 + 方差/收敛曲线进 evidence**；每 GI 档位定义
   「匹配深度」（1 / 2 / full bounce）作为对拍容差前提，1/2/full 三深度各一
   golden。
3. **pbrt-v4 容差带对照**（RFC-0022 §4.10 逐字；判据逐字引
   G9_ACCEPTANCE_MAP §2 M96 行）：正确性锚 = pbrt-v4（wavefront 架构文献
   基线），同场景同 spp **收敛曲线落入冻结容差带**；**容差带来源 = measured
   冻结**（先 measured 后冻结，禁手写掩盖，P-09）；工程模式 = 与实时管线共享
   场景/材质输入、不共享光照算法（golden diff 可归因到算法层而非输入层）。
4. **三臂 RED 独立有效**（判据逐字引 G9_ACCEPTANCE_MAP §2 M96 行）：**改
   seed（期望不同）、跳过 RR、关闭 MIS 三臂 RED 独立有效**——每臂独立于正例臂
   成立，任一臂失效（改 seed 不红 / 跳 RR 不红 / 关 MIS 不红）即漏检，本条款
   整体 FAIL。
5. **执行路径纪律**（RFC-0022 §4.10/§4.7 逐字）：PT 递归走 RT pipeline
   （消费 M50 多 hit group 增量面）；与 GI 各档 RayQuery 射线流**严禁混用同一
   射线流**（D2-Q9）；ray 生成与命中处理间加队列化中间层（SER 预留位，
   D2-Q13）。
6. **门序硬约束（D2-Q7，机器阻断）**（RFC-0022 §4.10 逐字；判据逐字引
   G9_ACCEPTANCE_MAP §2 M96 行）：**本条款门未绿，M97~M101 任何画质门不得
   验收**——G9.4 波内第一顺位，门序进验收映射 validator 与 close-out 审计
   机核，任何下游局部绿色不得冒充门序满足。**仅 host `ref_tracer` 非 RT
   pipeline 级输出不能充绿**。

**Implementation Requirements**

- 实现锚定（实现期命名，`forbid(unsafe_code)` 纪律维持）：megakernel 参照器
  + 固定 seed 确定性协议面（PCG32 流冻结 + 累加序冻结 + 逐像素 sample
  count/方差导出）+ pbrt-v4 对照 harness（收敛曲线 vs 冻结容差带机核）；
  device 侧 FFI 确需时按当时 `U.next_free` 实测顺位登记 unsafe-audit。
- RED 锚定计划（实现 PR 落）：改 seed / 跳 RR / 关 MIS 三臂各自独立 RED；
  两次运行位级一致 golden；pbrt-v4 收敛曲线容差带 golden（1/2/full bounce
  三深度各一）。
- 本 spec PR 先行落最小 RED 锚定占位语料
  `conformance/gi/reject/pt_seed_changed_nondeterministic.rx`（条款锚定占位，
  inert 锚定口径与转正路径见该文件头注释）；锚点目标（实现 PR 转正）= 改 seed
  不红即漏检负例（`ci/g9_path_tracer_reference_smoke.py` 门，symbolic key
  `g9.p0.m96.path_tracer_reference`，G9.1 冻结字面 0-byte 不动）。

---

## 4. 条款（RXS-0358，G9.4 M97 Surface Cache）

### RXS-0358 Surface Cache 离线 Card 参数化、运行时辐射度缓存与只丢能量不漏光契约

**Legality**

1. **离线 Card 参数化**（RFC-0022 §4.6 逐字；判据逐字引 G9_ACCEPTANCE_MAP §2
   M97 行）：cook 期每 mesh Card 参数化，默认上限 **≤12 Card/mesh**（Lumen
   口径）**可配**；超出按表面积/视角覆盖率裁剪，裁剪策略进 cook profile。
2. **运行时辐射度缓存**（RFC-0022 §4.6 逐字；判据逐字引 G9_ACCEPTANCE_MAP
   §2 M97 行）：Card atlas 驻留管理（稀疏更新，相机相关优先级）；命中点辐射度
   写入 = 完整材质求值 + 直接光 + 已缓存间接光（单帧延迟反馈，Lumen 同构）；
   **离线 Card 参数化与运行时辐射度缓存产物 digest 等于 golden**。
3. **缺失覆盖只丢能量不漏光（硬契约）**（RFC-0022 §4.6 逐字；判据逐字引
   G9_ACCEPTANCE_MAP §2 M97 行）：Card 未覆盖区域**只丢能量不漏光**——采样
   回退到下一级追踪结果或 ambient 项，输出非负、无低于 ambient 的黑色裂缝；
   **缺失覆盖只丢能量不漏光断言（能量差 measured 记录、漏光像素计数=0）**。
4. **Card 空洞漏光检测负例（RED 臂）**（判据逐字引 G9_ACCEPTANCE_MAP §2
   M97 行）：**Card 空洞漏光检测负例 RED 臂独立有效**——故意制造 Card 空洞的
   variant 必须被漏光检测臂判 RED，且该负例臂独立于正例臂成立。
5. **图集页 ABI 复用不私定**（RFC-0022 §4.6 逐字；判据逐字引
   G9_ACCEPTANCE_MAP §2 M97 行）：**Card 图集页格式复用 M04 版本化页 ABI
   （RXS-0328~0342）与 M91 页格式 v2（RXS-0344）既有面，禁止私定磁盘格式**。
6. **按匹配深度对 M96 golden 与门序**（判据逐字引 G9_ACCEPTANCE_MAP §2 M97
   行）：Surface Cache 输出**按匹配深度（1/2/full bounce）对 M96 golden
   验收**（容差前提 = RXS-0357 L2 匹配深度表）；门序硬约束（RXS-0357 L6）未
   满足前本条款门不得验收。

**Implementation Requirements**

- 实现锚定（实现期命名，纯 safe 方向维持）：cook 期 Card 参数化器
  （`src/rurix-asset` cook 面扩写）+ 运行时 Card atlas 驻留/辐射度缓存 +
  漏光检测校验面；unsafe 确需时按当时 `U.next_free` 实测顺位登记
  unsafe-audit。
- RED 锚定计划（实现 PR 落）：Card 空洞注入 variant → 漏光检测臂 RED（漏光
  像素计数≠0）；私定图集磁盘格式 variant → 装配期拒；参数化/缓存产物 digest
  golden。
- 本 spec PR 先行落最小 RED 锚定占位语料
  `conformance/gi/reject/surface_cache_card_hole_leak.rx`（条款锚定占位，
  inert 锚定口径与转正路径见该文件头注释）；锚点目标（实现 PR 转正）=
  `ci/g9_surface_cache_smoke.py` 门（symbolic key `g9.p0.m97.surface_cache`，
  G9.1 冻结字面 0-byte 不动）。

---

## 5. 条款（RXS-0359，G9.4 M98 追踪降级链）

### RXS-0359 四级追踪降级链选档契约、逐档计数面与禁静默回退

**Legality**

1. **四级链冻结**（RFC-0022 §4.7 表逐字；判据逐字引 G9_ACCEPTANCE_MAP §2
   M98 行）：**L1 Screen Trace**（屏幕空间高度场 ray march，HZB/深度，~50 m
   内、屏幕内，成本最低）→ **L2 SWRT**（Mesh SDF 近场逐对象 + Global SDF
   远场合并，compute SDF 步进，~200 m；`RadianceTracer` 契约已预留 SDF 实现
   位）→ **L3 HWRT**（Vulkan RayQuery 对 TLAS 追踪；命中着色两档 = 简单兜底
   求值〔默认〕/ **hit lighting** 完整材质求值〔高档，需 RayTracingQualitySwitch
   式材质简化开关，消费 M50 多 hit group 面〕）→ **L4 Far Field**（远场代理
   辐射度，~1 km 量级；本条款只冻结消费接口，资产生成归几何/资产模块）。
2. **选档契约与逐档计数面**（RFC-0022 §4.7 逐字；判据逐字引
   G9_ACCEPTANCE_MAP §2 M98 行）：逐 probe/逐像素按命中距离与覆盖优先级选档；
   每级独立开关 + 独立 evidence 计数面（命中率/射线量/耗时）；**L1/L2/L3/L4
   四级命中率/耗时计数非空且逐帧 evidence**。
3. **逐级强关回归可检测（RED 臂）**（RFC-0022 §4.7 逐字；判据逐字引
   G9_ACCEPTANCE_MAP §2 M98 行）：**逐级强制关闭后回归差异必须可检测——强关
   后输出仍同 golden 即 RED**（回归不可检测 = 降级链失效）；L3 关材质简化开关
   → 预算超限 RED。
4. **禁静默回退**（RFC-0022 §4.7 逐字；判据逐字引 G9_ACCEPTANCE_MAP §2 M98
   行）：**实际使用级别必须显式记录入 evidence，禁静默回退**（无计数降级即
   RED）。
5. **L4 依赖未就绪登记**（判据逐字引 G9_ACCEPTANCE_MAP §2 M98 行）：L4 Far
   Field 依赖 HLOD 接口未就绪时登记 **SKIP=not-triggered 不充绿**（条件未触发
   只表示决策已记录，不是成功）。
6. **射线流纪律与 golden 对拍**（RFC-0022 §4.7/§4.10 逐字；判据逐字引
   G9_ACCEPTANCE_MAP §2 M98 行）：GI 各档批量均匀射线全走 RayQuery+compute，
   RT pipeline 仅服务 M96 与未来 hit-lighting 递归，**严禁混用同一射线流**
   （D2-Q9），队列化中间层作为唯一交汇点（SER 预留位，D2-Q13）；**各档按匹配
   深度对 M96 golden**；门序硬约束（RXS-0357 L6）未满足前本条款门不得验收。

**Implementation Requirements**

- 实现锚定（实现期命名）：四级选档器与逐档计数面（命中率/射线量/耗时逐帧
  导出）+ L1 高度场 ray march / L2 SDF 步进 / L3 RayQuery（含 hit lighting
  档）/ L4 消费接口；`AsManager`/temporal 底座既有面 0-byte 复用。
- RED 锚定计划（实现 PR 落）：逐级强关后输出仍同 golden → RED；静默回退
  variant（实际级别未记录）→ RED；逐档计数面逐帧 golden。
- 本 spec PR 先行落最小 RED 锚定占位语料
  `conformance/gi/reject/tracing_fallback_silent_demotion.rx`（条款锚定占位，
  inert 锚定口径与转正路径见该文件头注释）；锚点目标（实现 PR 转正）=
  `ci/g9_tracing_fallback_chain_smoke.py` 门（symbolic key
  `g9.p0.m98.tracing_fallback_chain`，G9.1 冻结字面 0-byte 不动）。

---

## 6. 条款（RXS-0360，G9.4 M99 SPG + Radiance Cache）

### RXS-0360 屏幕级 SPG 自适应细分与 Radiance Cache 双级语义（世界级 not-triggered 登记）

**Legality**

1. **SPG 自适应细分（屏幕级判 go 面）**（RFC-0022 §4.8 逐字；判据逐字引
   G9_ACCEPTANCE_MAP §3 M99 行）：屏幕空间 probe 基线 **16 px/probe** +
   **自适应细分**（判据 = 深度/法线不连续性 + radiance 方差），**3×3 probe
   空间滤波**（≈48×48 屏幕有效滤波）；在 G8 既有 1/16 均匀 probe + 3×3 滤波
   底座上**增量、不重定底座**；细分判据阈值**先 measured 后冻结**（禁手写
   掩盖，P-09）；probe 历史/时域累积一律经 temporal 公共底座，**禁私写重
   投影**（D2-Q14，私写 variant 即 RED）。
2. **Radiance Cache 双级语义**（RFC-0022 §4.8 逐字；判据逐字引
   G9_ACCEPTANCE_MAP §3 M99 行）：**屏幕空间级**（复用 probe 历史）+ 世界空间
   clipmap 级（绕相机分级）；第一反弹采样 = BRDF×入射光 **product importance
   sampling**；**关 product IS → 方差回归可检测**（负例 RED 臂独立有效）；
   双级语义产物 digest 等于 golden。
3. **世界级 clipmap not-triggered 登记（RD-040 条件分项）**（RFC-0022 §4.8
   「世界级须 RD-040 measured 触发举证，未举证只做屏幕级」逐字；
   G9_CANDIDATE_DECISIONS §2 RD-040/M99 行 + v1.3 校准注）：**世界级 clipmap
   证据不足——未 measured 举证，登记 not-triggered 不充绿**；本条款只冻结
   屏幕级双级语义，世界级语义面留 RD-040 触发后只追加修订；不得以「UE5 目标」
   静默改写 backfill 字面，不得以屏幕级绿色冒充世界级已触发。
4. **门序与 golden**（判据逐字引 G9_ACCEPTANCE_MAP §3 M99 行）：屏幕级输出
   按匹配深度（1/2/full bounce）对 M96 golden 验收；门序硬约束（RXS-0357 L6）
   未满足前本条款门不得验收。

**Implementation Requirements**

- 实现锚定（实现期命名，纯 safe 方向维持）：SPG 细分判据与探针布局面 +
  Radiance Cache 屏幕级缓存（temporal 公共底座消费面，禁私写重投影）+ product
  IS 采样与方差计数面。
- RED 锚定计划（实现 PR 落）：关 product IS → 方差回归不可检测即 RED；私写
  重投影 variant → RED；屏幕级产物 digest golden。
- 本 spec PR 先行落最小 RED 锚定占位语料
  `conformance/gi/reject/radiance_cache_product_is_disabled.rx`（条款锚定
  占位，inert 锚定口径与转正路径见该文件头注释）；锚点目标（实现 PR 转正）=
  `ci/g9_spg_radiance_cache_smoke.py` 门（symbolic key
  `g9.p1.m99.spg_radiance_cache`，G9.4 波 P1 登记字面不动）。

---

## 7. 条款（RXS-0361，G9.4 M100 低档多灯直接光）

### RXS-0361 低档多灯直接光默认档、验证射线零跳过硬契约与高档 ReSTIR not-triggered 登记

**Legality**

1. **低档默认档判 go 面**（G9_CANDIDATE_DECISIONS §2 RD-040/M100 行逐字——
   RFC-0022 §7「多灯直接光语义归 D2 后续 RFC/修订行，本章不冻结」边界声明；
   判据逐字引 G9_ACCEPTANCE_MAP §3 M100 行）：**低档 MegaLights 式固定随机
   选灯为默认档**——多灯场景出图与 golden 相等（容差域经本条款面明示冻结，
   禁手写掩盖，P-09）；**选灯种子流固定、同输入双运行逐位一致**；海量灯阴影
   统一接口随动。
2. **验证射线零跳过硬契约（统计性偏置门）**（RFC-0022 §7 否决行 D2-Q4 逐字；
   判据逐字引 G9_ACCEPTANCE_MAP §3 M100 行）：GI 各档复用路径**不得跳验证
   射线**——跳验证引入系统性变暗偏置、随场景复杂度放大、事后不可归因；
   **跳验证射线注入负例 RED 臂独立有效**。
3. **高档 ReSTIR not-triggered 登记（RD-040 条件分项）**（G9_CANDIDATE_
   DECISIONS §2 RD-040/M100 行 + v1.3 校准注逐字；判据逐字引
   G9_ACCEPTANCE_MAP §3 M100 行）：**高档 ReSTIR reservoir 须附多灯 workload
   证据，证据不足不做到场登记 not-triggered 不充绿**——M15 维持 open-留档，
   不得以默认档绿色冒充高档已验收；高档语义面待 workload 证据后只追加重判。
4. **门序与 golden**（判据逐字引 G9_ACCEPTANCE_MAP §3 M100 行）：默认档输出
   按匹配深度（1/2/full bounce）对 M96 golden 验收；门序硬约束（RXS-0357 L6）
   未满足前本条款门不得验收。

**Implementation Requirements**

- 实现锚定（实现期命名，纯 safe 方向维持）：低档固定随机选灯器（种子流固定
  + 双运行确定性）+ 验证射线通道与零跳过计数面 + 海量灯阴影统一接口随动面。
- RED 锚定计划（实现 PR 落）：跳验证射线注入 → RED；选灯种子漂移（同输入
  双运行不一致）→ RED；默认档出图 golden。
- 本 spec PR 先行落最小 RED 锚定占位语料
  `conformance/gi/reject/multi_light_restir_tier_unproven.rx`（条款锚定占位，
  inert 锚定口径与转正路径见该文件头注释）；锚点目标（实现 PR 转正）=
  `ci/g9_multi_light_low_smoke.py` 门（symbolic key
  `g9.p1.m100.multi_light_low`，G9.4 波 P1 登记字面不动）。

---

## 8. 条款（RXS-0362，G9.4 M101 IF 档位阶梯）

### RXS-0362 IF 体素网格档位阶梯、共享 probe 着色与八面体编码内核律与每档 AS 更新预算硬契约

**Legality**

1. **档位阶梯 L0~L3 冻结**（RFC-0022 §4.8 表逐字；判据逐字引
   G9_ACCEPTANCE_MAP §3 M101 行）：**L0 屏幕空间 probe**（即 SPG 完整形态）/
   **L1 clipmap 体积 probe**（DDGI 基线：八面体 irradiance 8×8 + **visibility
   16×16——防漏光优先于提 irradiance 分辨率** + 每帧轮换更新摊销；DDGI
   Resampling 为演进项非首版）/ **L2 空间哈希缓存**（SHaRC 式空间哈希
   radiance 缓存，按需分级）/ **L3 per-pixel**（全分辨率逐像素追踪，参考档/
   截图档）。
2. **共享内核律**（RFC-0022 §4.8 D2-Q5 逐字；判据逐字引 G9_ACCEPTANCE_MAP
   §3 M101 行）：L0~L3 **共享 probe 着色与八面体编码内核，只换空间索引
   结构**——档间 golden 对拍可归因到索引结构而非实现差异；**共享内核同一
   函数实例断言**（各档复制实现即 RED）。
3. **八面体编码线性域**（判据逐字引 G9_ACCEPTANCE_MAP §3 M101 行）：probe
   数据的八面体编码在**线性域**进行——SRGB 编码注入即 RED（编码域错误属
   漏检即判本条款整体 FAIL 的负例臂）。
4. **每档 AS 更新预算硬契约**（RFC-0022 §4.8 D2-Q10 逐字；判据逐字引
   G9_ACCEPTANCE_MAP §3 M101 行）：每档定义强制含 **AS 更新预算行**，档位
   切换判据消费 `AsManager` 既有 `AsStats` 计数面；**超 AS 更新预算必须强制
   降档**（超限未降档即 RED）。
5. **档位切换确定性**（判据逐字引 G9_ACCEPTANCE_MAP §3 M101 行）：档位切换
   对同输入确定（双运行逐位一致）；切换阈值先 measured 后冻结（SPG/IF 调参
   阈值为实现确定、非 stable，RFC-0022 §10 逐字）。
6. **门序与 golden**（判据逐字引 G9_ACCEPTANCE_MAP §3 M101 行）：各档按匹配
   深度（1/2/full bounce）对 M96 golden 验收；门序硬约束（RXS-0357 L6）未满足
   前本条款门不得验收。

**Implementation Requirements**

- 实现锚定（实现期命名，纯 safe 方向维持）：共享 probe 着色/八面体编码内核
  （单实例，四档索引结构可替换）+ L0~L3 空间索引（屏幕 probe / clipmap 体积
  probe / 空间哈希 / per-pixel）+ 每档 AS 更新预算行与强制降档臂（AsStats
  消费面）。
- RED 锚定计划（实现 PR 落）：SRGB 编码注入 → RED；超 AS 预算未强制降档 →
  RED；档位复制实现（非同一内核实例）→ RED；档间对拍 golden 与切换双运行
  逐位一致 golden。
- 本 spec PR 先行落最小 RED 锚定占位语料
  `conformance/gi/reject/if_octahedral_srgb_encoding.rx` 与
  `conformance/gi/reject/if_as_budget_exceeded_no_demote.rx`（条款锚定占位，
  inert 锚定口径与转正路径见各文件头注释）；锚点目标（实现 PR 转正）=
  `ci/g9_if_tier_ladder_smoke.py` 门（symbolic key
  `g9.p1.m101.if_tier_ladder`，G9.4 波 P1 登记字面不动）。

---

## 9. 条款（RXS-0394，G11.4 M153 R3 灯种子集表达）

### RXS-0394 R3 灯种子集表达：光源集五元闭集 + 契约光照面单通道 + 点光源辐射链与 emissive 语义 + cornell 契约灯面 0-byte

**Legality**

1. **光源集五元闭集**（RFC-0028 §4.3.1 冻结语义；判据逐字引
   G11_CONTRACT §4.2 M153 行）：场景光源集 = `{ 契约 sun（方向光）,
   契约 sky（常量天光）, glTF 点光源集（包内 pointLight1~N 节点）,
   glTF 面光源集（area/spot 若包内存在）, glTF emissive 表面集
   （材质 emissiveFactor/emissiveTexture 非零面） }`——五元闭集，
   **缺类显式登记**（不得以缺类冒充空集）；bistro 包内实测 pointLight1~4
   四盏 + emissive 材质四件全部表达进渲染（4+ 盏实测消费，M153 门
   机核面）。
2. **契约光照面单通道**（RFC-0028 §4.3.2 / D-409 F4 修法逐字）：光源参数
   唯一事实源 = **契约光照参数面**（`milestones/g10/corpus/lighting_*.json`，
   M133 清单冻结面）；包内 glTF 字段为**派生输入**——经 corpus 派生链
   转入契约光照 JSON（语料修订走 M133 只追加修订程序：清单 digest 注册 +
   修订行），Rurix harness 与 UE build_scenes 双端同消费契约面；**禁止
   运行时双通道并存**（M130 契约 digest 一致性门序继承），glTF 字段直读
   绕过契约面即 RED；每盏光源的位姿/强度/色温/派生源 provenance 逐盏
   登记进门 evidence。
3. **点光源辐射链（host 参考管线口径）**：点光源对着色点辐照度
   `E = color_linear_rgb × I₀ × max(0, cosθ_emit) / d²`（朗伯余弦瓣——
   灯具单面发光口径，发光轴向背向取零；candela 点强 → 距离平方衰减，
   与契约 sun lux 链同单位面），出射 `L = E × max(n·l,0) × albedo/π ×
   可见性`（阴影射线，原点沿着色法线偏移 RAY_EPS，与 gi/tracer.rs 太阳
   阴影同口径）；**强度派生链**（RFC-0028 §9 Q3 字面：包内字段实测
   登记）= 灯具 emissive 通量换算——`Φ = Le × A × π`（朗伯半球通量，
   `Le = emissiveFactor × emissiveTexture 线性均值`，A = 关联灯具几何
   表面积）、**轴向点强 `I₀ = Φ / π = Le × A`**（朗伯发射 I(θ)=I₀·cosθ
   满足 ∫I dΩ = Φ），发光轴向 = 关联灯具 emissive 三角形面积加权平均
   法线，逐盏 provenance（节点位姿 / 关联灯具 / Le / A / 轴向 / 换算
   结果）进 evidence；换算链参数实现波 measured 标定后冻结（P-09），
   族外发明即口径漂移 RED。
4. **emissive 语义**（RFC-0028 §4.3.4 冻结语义）：emissive 表面出射辐射度
   `Le = emissiveFactor × emissiveTexture`（线性域），作为光源参与**直接
   可见与 GI 双级能量贡献**（主射线命中直出 + 世界辐射缓存沉淀喂回场景，
   RXS-0396 面）；emissive 强度/纹理消费口径与材质面（R1 修复面）解耦
   登记——材质未消费面如实登记，不以 emissive 表达冒充材质修复。
5. **cornell 契约 sun+sky 灯面 0-byte**（复测对照口径，G11_CONTRACT §4.2
   M153 行字面）：cornell 语料灯面维持契约 sun+sky（G10.3 生成器登记
   口径），`corpus/lighting_cornell_box.json` 0-byte；bistro 灯面表达
   不得回流改写 cornell 灯面（漂移即 RED）。
6. **门序**：本条款门验收以 RXS-0357（M96 golden）门绿为前置（RXS-0357
   L6 字面继承）；多灯开销与 G9 M100 低档 MegaLights 面联动评估登记——
   host 参考管线 4 盏逐灯直接求值（验证射线零跳过，RXS-0361 L2 口径
   继承），不新造低档选灯面；GPU 管线多灯 workload measured 对照面未
   产出，M100-high 维持 defer（G11.6 触发评估登记，承接锚字面 0-byte）。

**Implementation Requirements**

- 实现锚定（实现期命名，`forbid(unsafe_code)` 纪律维持）：A/B host 参考
  管线（g10_5_scene_render）点光源/emissive 消费面 + 契约光照 JSON
  闭集解析（fail-closed）+ 逐灯 provenance 登记块；派生链脚本
  （milestones/g11/harness 面）产 M133 修订。
- RED 锚定计划（实现 PR 落）：点光源未表达冒充修复 → RED；glTF 直读
  绕过契约面 → RED；cornell 契约灯面漂移 → RED。
- 本 spec PR 先行落最小锚定占位语料
  `conformance/gi/accept/light_seed_set_minimal.rx` 与
  `conformance/gi/reject/light_seed_gltf_direct_bypass.rx`（条款锚定
  占位，inert 锚定口径与转正路径见各文件头注释）；锚点目标（实现 PR
  转正）= `ci/g11_fix_r3_light_subset_smoke.py` 门（symbolic key
  `g11.p0.m153.fix_r3_light_subset`，G11.1 冻结字面 0-byte 不动）。

---

## 10. 条款（RXS-0395，G11.4 M154 R4 多反弹 GI 双级语义）

### RXS-0395 R4 多反弹 GI：屏幕探针近场 + 世界辐射缓存远场兜底双级语义、能量守恒口径与 host 同构兑现面

**Legality**

1. **双级语义**（RFC-0028 §4.1.1 冻结语义；判据逐字引 G11_CONTRACT §4.2
   M154 行）：近场 = 屏幕探针（G9 M99 已验收屏幕级 SPG + Radiance
   Cache 底座 0-byte 复用）；远场 = 世界空间辐射缓存（RXS-0396）。屏幕
   探针间接场查询失效（无有效屏幕覆盖/反照率不足/超出屏幕域）时**必须
   回落**世界缓存，**回落路径逐帧计数进 evidence**；禁止静默返回零辐射
   （远场能量丢失即 R4 差距成因面）。
2. **多反弹语义**（RFC-0028 §4.1.2 冻结语义）：间接光计算支持 **≥2 次
   反弹**；第二次及以上反弹的入射辐射度经世界缓存查询获得（屏幕探针只
   承担第一级近场面）；**反弹次数、每级能量计数进 evidence**；反弹截断
   处只丢能量不漏光（RXS-0358「只丢能量不漏光」口径继承——漏光适用面
   注：本语境漏光 = 双级合计后非物理正能量穿越遮挡的像素，判定 = 与
   M96 golden 按匹配深度对拍超容差带的漏光模式像素，计数 = 0 断言面
   沿 RXS-0358 口径继承）。
3. **能量守恒口径**（RFC-0028 §4.1.3 冻结语义）：双级合计的远场能量
   回归必须 measured 非零（对屏幕缓存物理不可达区域，判定阈 = RXS-0396
   L4 双锚面）；**逐级能量增量绝对值单调不增趋于零**（`|ΔE_{k+1}| ≤
   |ΔE_k|`，ΔE_k = 第 k 级迭代沉积总能量增量——多弹收敛口径：增量趋于
   零即收敛至均衡（采样噪声下允许小幅负增量），|Δ| 递增 = 能量发散即
   RED——不凭空造能）。
4. **host 参考管线消费面（双侧最小兑现面裁决，D-409 F1 修法）**：A/B
   host 参考管线（g10_5_scene_render）消费同一双级语义，形态 = **同构
   世界缓存的 host CPU 参考实现**（同一语义面双实现——解析式远场估计
   不构成「世界辐射缓存世界级」语义兑现，否决）；renderer 面 = 世界级
   缓存落地 + 远场能量回归判定锚（RXS-0396 L4），host 面 = G11.5 复测
   R4 delta 收敛断言的载体。**不以 host 参考管线多反弹冒充 GPU 管线
   世界级验收，不以 GPU 管线世界级落地冒充 host 臂 delta 收敛**（GPU
   管线双端面锚定 G14 不动）。
5. **门序**：本条款门验收以 RXS-0357（M96 golden）门绿为前置（RXS-0357
   L6 字面继承）；契约 digest 不等仍出 A/B 报告即 RED（M130/M139 门序
   字面，G11.5 复测继承）。

**Implementation Requirements**

- 实现锚定（实现期命名，纯 safe 方向维持）：世界缓存多反弹消费面 +
  回落路径/反弹级数/逐级能量计数导出 + host 同构实现（g10_5_scene_render
  面）；device/GPU 腿锚定 G14 不动。
- RED 锚定计划（实现 PR 落）：单反弹换皮冒充多反弹（逐级能量为零）→
  RED；回落计数缺失/静默零辐射 → RED；漏光像素注入 → RED。
- 本 spec PR 先行落最小锚定占位语料
  `conformance/gi/accept/gi_multibounce_two_level_minimal.rx` 与
  `conformance/gi/reject/gi_single_bounce_masquerade.rx`（条款锚定占位，
  inert 锚定口径与转正路径见各文件头注释）；锚点目标（实现 PR 转正）=
  `ci/g11_fix_r4_gi_multibounce_world_cache_smoke.py` 门（symbolic key
  `g11.p0.m154.fix_r4_gi_multibounce_world_cache`，G11.1 冻结字面
  0-byte 不动）。

---

## 11. 条款（RXS-0396，G11.4 M154 M99-clipmap 世界级辐射缓存承接）

### RXS-0396 世界辐射缓存世界级承接：空间哈希世界缓存 + 距离自适应辐射 LOD + 屏幕缓存失效回落 + 远场能量回归双锚判定（RXS-0360 世界级登记翻转修订行）

**Legality**

1. **修订行——RXS-0360 世界级 not-triggered 登记翻转**（RFC-0028 §4.4
   授权；G10.6 rejudged-go 承接锚逐字：「重判条件已命中（G10.6：R4 P0 +
   C1 P1 measured 举证落地）→ G11 画质修复期承接世界辐射缓存世界
   clipmap 级（只消费 G10.8b 锁定清单 R4/C1 行 + 本锚）；兜底 = 屏幕级
   SPG + Radiance Cache（g9.p1.m99 门绿）维持」）：RXS-0360 L3「世界级
   clipmap 未 measured 举证，登记 not-triggered 不充绿」登记**翻转为
   「世界级承接落地（G11.4 M154）」**——measured 举证 = `g10_gap_registry`
   R4 行（bistro HDR p90 delta = 4.697253086805343，evidence_digest
   sha256:d5f5d644…）+ C1 行（HDR 中位 ≈21×）双行，重判条件已命中
   （G10.6 重评窗核验，deferred.json RD-040 history 2026-08-15 行）。
   **RXS-0360 既有字面 0-byte 不改写**；世界级语义面由本条款承载；屏幕级
   SPG + Radiance Cache 兜底面（g9.p1.m99 门绿）维持不动——**不得以屏幕级
   绿色冒充世界级验收**（G11_CONTRACT §4.2 M154 行字面）。
2. **空间索引形态**（RFC-0028 §4.2.1 冻结语义）：世界空间哈希缓存——
   位置按距离自适应量化（**对数族**，量化函数族闭集 {对数族, 幂律族}
   取定）：`level(p) = clamp(floor(log2(1 + dist(p, camera) / d_ref)),
   0, LEVELS−1)`，格长 `s(ℓ) = s0 × 2^ℓ`；参数经实现波 measured 标定
   冻结（P-09）：`LEVELS = 4`、`s0 = scene_diag × 2^-8`、`d_ref =
   scene_diag × 2^-4`（scene_diag = 场景包围盒对角线实测——bistro
   25.962 m / cornell 958.659 单位，G11.4 实现波实测登记）；哈希冲突走
   **双哈希步长线性探测**（h1 定位 + h2 步长，探测上界闭集登记）；索引
   结构在线构建、**零离线预处理**（Surface Cache/Mesh Card 重资产路径
   继续后置）。
3. **辐射 LOD（clipmap 级）**（RFC-0028 §4.2.2 冻结语义）：按距离自适应
   的辐射度细节层级（每一级对应一个距离带的辐射度细节层级）；**层级数、
   每层覆盖距离带、每层命中率/耗时逐帧计数进 evidence**；禁静默降层级
   （降级路径显式登记，RXS-0359 禁静默回退口径继承）。
4. **回落语义**（RFC-0028 §4.2.3 冻结语义）：屏幕探针失效处回落世界
   缓存（RXS-0395 L1）；世界缓存级内未命中 → 更粗级查询（级间回落链）
   → 天光/常量环境项**末级兜底显式登记**；回落查询命中率、回落辐射度
   能量计数进 evidence。
5. **世界级验收判定（机核面，双锚同真，D-409 F3 修法）**：①**远场探针集**
   （屏幕缓存物理不可达的场景区域集——不投影进任何覆盖像素或被更近
   表面遮挡的场景表面点确定性采样，场景标定面登记，区域集实现波按
   G10 语料双场景登记）**能量回归 measured 达标定阈**（阈值由标定程序
   measured 产——「非零」字面不构成判定，任意噪声冒充能量回归即 RED）；
   ②**与 M96 golden 按匹配深度对拍一致**（RXS-0357 L2 匹配深度表与容差
   带 0-byte 引用，容差带 measured 后冻结〔P-09〕，L6 门序硬约束——M96
   golden 未绿本面不得验收）。双锚同真方为世界级；**UE 对拍面归 G11.5
   复测 delta 收敛（RXS-0393 面），不与 M96 golden 混用**。
6. **边界声明（D-409 F7 修法）**：世界级辐射缓存 **≠** RXS-0359 L4 Far
   Field 档——L4 为追踪降级链远场档（M98-l4 维持 defer，承接锚字面
   0-byte 不动），世界级缓存为辐射度复用缓存；两语义面不互冒充，世界级
   落地不构成 M98-l4 的静默兑现。

**Implementation Requirements**

- 实现锚定（实现期命名，纯 safe 方向维持）：空间哈希世界缓存（双哈希 +
  线性探测 + 距离自适应量化）+ 辐射 LOD 计数面 + 回落链计数面 + 远场
  探针集能量回归判定 + M96 golden 匹配深度对拍面（host 参考管线
  g10_5_scene_render 兑现，device/GPU 面锚定 G14）。
- RED 锚定计划（实现 PR 落）：世界级未落地冒充承接（远场探针集能量回归
  为零/低于标定阈）→ RED；屏幕级绿色冒充世界级（g9.p1.m99 evidence
  冒充 M154 evidence）→ RED；容差带外漏光 → RED。
- 本 spec PR 先行落最小锚定占位语料
  `conformance/gi/accept/world_radiance_cache_minimal.rx` 与
  `conformance/gi/reject/world_cache_farfield_zero_energy.rx`（条款锚定
  占位，inert 锚定口径与转正路径见各文件头注释）；锚点目标（实现 PR
  转正）= `ci/g11_fix_r4_gi_multibounce_world_cache_smoke.py` 门
  （symbolic key `g11.p0.m154.fix_r4_gi_multibounce_world_cache`，G11.1
  冻结字面 0-byte 不动）。

---

## 12. 条款（RXS-0397，G11.5b 天光直接 IBL 消费面）

### RXS-0397 天光直接漫反射 IBL 消费面（--sky-ibl）：全向口径 + 下半球黑半球混合 + GI 双重计数排除（RXS-0396 L4 末级兜底修订行）

**Legality**

1. **契约天光直接消费语义**（RFC-0028 §4.5 伞形「GI/天光遮蔽语义面」消费；
   measured 诊断事实源 = `milestones/g11/design/g11_5b_ldr_residual_diag.md`）：
   契约 sky（常量天光，cubemap_id=null ⇒ 常量辐射度 `L_sky = sky.intensity`
   同单位链，RXS-0392 口径维持）消费面扩为「**直接全向漫反射 IBL + GI 间接**
   双通道」——直接通道 = UE SkyLight 指定 cubemap 全向投递口径对齐面（G11.5b
   实测：该 UE 参照配置 `r.DynamicGlobalIlluminationMethod=0` 且
   `r.GenerateMeshDistanceFields=0` ⇒ movable SkyLight 无遮蔽机制可消费，按
   全向 IBL 投递，下半球黑读回 = true；SkyLight 漫反射实测占帧均值 95.4%、
   镜面 ≤0.03%）；直接项解析式 **`Lo_sky(x) = albedo(x) × L_sky ×
   (1 + n(x)·up)/2`**（下半球黑半球混合闭式；up = +Y；n = 着色法线，法线
   贴图扰动后消费面；解析式确定性、零采样面）。
2. **GI 双重计数排除**：旗标开时，世界缓存构建/渲染的间接估计子 **miss
   射线返回零辐射**（天空首反弹 = 主射线直接项单计数，禁直接项与 GI 收集
   并存双计数）；沉积面（命中点/探针点）直接项 += 同式天光项（天空二反弹
   及以上经缓存链接进入维持，RXS-0395 双级语义不变）；**旗标关 =
   RXS-0395/0396 字面口径逐字节 0-byte**（默认面帧 digest parity 门序面维持，
   破坏即 RED）。
3. **修订行——RXS-0396 L4 末级兜底**：旗标开时「天光/常量环境项末级兜底」
   由直接天光项承接（天光已单计数，禁重复注入），GI 零值 = 有效零间接，
   `last_resort_px` 计数显式登记维持；**RXS-0396 既有字面 0-byte 不改写**。
4. **镜面天光不消费登记**：天光镜面 IBL 项本条款不消费（G11.5b 实测份额
   ≤0.03% + 高光尾过冲防面）；维持 `c1_ue_specular_ibl` 残余登记（G15 画质
   量级收口面候选），不以本条款落地冒充镜面 IBL 闭环。
5. **消费面边界**：本条款消费面 = `--sky-ibl` 与 `--gi-multibounce` 组合
   （G11.5b 复测面双场景同消费）；单反弹 GI 组合与 GPU 管线面不在本条款
   消费面（GPU 面锚定 G14 不动）；cornell 契约 sun+sky 灯面参数 0-byte
   （同一消费语义双场景一致）；契约参数（相机/光照/seed/post）digest ==
   G10.5 锁定值 0-byte。
6. **门序**：契约 digest 不等仍出 A/B 报告即 RED（RXS-0395 L5 / M130/M139
   字面继承）；本条款闭环断言面 = G11.5 复测 R1 行收敛断言（M155 / M147
   `--phase g11.5`，RXS-0393 L2 quality_gap 款字面 0-byte）——**不以本条款
   落地冒充 delta 收敛，不改判据/阈值充绿**。

**Implementation Requirements**

- 实现锚定（实现期命名，纯 safe 方向维持）：host 参考管线
  `g10_5_scene_render` `--sky-ibl` 旗标面（主射线直接项 + GI miss 射线整零
  + 沉积直接项同式 + 末级兜底修订行口径）+ 渲染输出 `sky_ibl` 闭集登记块
  （enabled/mode/direct_sky_mean）；device/GPU 腿锚定 G14 不动。
- RED 锚定计划（实现 PR 落）：天光项双重计数（直接项与 GI miss 收集并存）
  → RED；旗标关默认面帧 digest 漂移（parity 破坏）→ RED；下半球黑口径翻转
  （(1−n·up)/2 反向或下半球不置黑注入）→ RED。
- 本 spec PR 先行落最小锚定占位语料
  `conformance/gi/accept/sky_ibl_direct_diffuse_minimal.rx` 与
  `conformance/gi/reject/sky_ibl_gi_double_count.rx`（条款锚定占位，inert
  锚定口径与转正路径见各文件头注释）；锚点目标（复测闭环断言面）=
  `ci/g11_ab_retest_closure_smoke.py` 门（symbolic key
  `g11.p0.m155.ab_retest_closure`）+ `ci/g11_fix_r1_material_subset_smoke.py`
  `--phase g11.5`（G11.1 冻结字面 0-byte 不动）。

---

## 13. 条款（RXS-0398，G12.2 M158 MIS 完整面生产化）

### RXS-0398 MIS 完整面生产化：光源采样（NEE）× BSDF 采样双策略逐顶点 balance heuristic 权重、多光源联合 PDF 与光源分布确定性、delta 光源退化、能量守恒口径（RXS-0357 参照器面生产化演进显式修订行）

**Legality**

1. **修订行——RXS-0357 参照器面生产化演进（G12，RFC-0029 §4.7 逐字
   承接）**：RXS-0357 既有字面 **0-byte**——起步范围冻结（焦散/体积/
   specular 链 out）维持、固定 seed 确定性协议维持、pbrt-v4 容差带维持、
   golden 门序硬约束 D2-Q7 维持；参照器面生产化演进语义（MIS 完整面 /
   俄罗斯轮盘生产化 / 采样策略升级与低差异序列 / 收敛判据生产化）经本
   修订行 + 新条款 **RXS-0398~RXS-0401** 承载（post-interlock
   actual-next-free allocation 领取，G12 立项裁决 6）。
2. **双策略逐顶点 MIS（balance heuristic）**（RFC-0029 §4.1 L1 冻结
   语义；判据逐字引 G12_CONTRACT §4.2 M158 行）：路径每个非首顶点处，
   光源采样（NEE）与 BSDF 采样构成双策略 MIS 对，权重 = **balance
   heuristic**——`w_nee = p_nee / (p_nee + p_bsdf)`、`w_bsdf =
   p_bsdf / (p_nee + p_bsdf)`（p_* 为该顶点处两策略各自 PDF 换算到同一
   测度；实现面取无零除安全形）。**delta 光源（点光）退化**：BSDF 策略
   对 delta 光源概率为零 → `w_nee = 1`，不产生除零；非 delta 光源
   （面光/发光面）双策略均可能非零，权重闭式计算。**权重缺失冒充 MIS
   即 RED**。
3. **多光源 MIS**（RFC-0029 §4.1 L2 冻结语义）：多光源场景 NEE 先按
   光源分布采样光源（离散 PDF），再按光源采样点（连续 PDF）——**联合
   PDF = 离散 × 连续**；MIS 分母含全部光源的 NEE 联合 PDF 之和（不重
   不漏）；**光源分布构建确定性**（同场景同分布 digest 进 evidence）。
4. **能量守恒**（RFC-0029 §4.1 L3 冻结语义）：白炉场景（全白 furnace，
   albedo=1 全反射闭域）MIS 渲染结果均值 = 入射能量（守恒容差 measured
   标定，禁手写 P-09）；**逐级能量增量单调不增**（RXS-0395 L3 口径继承
   ——每加一级反弹路径能量增量不增，采样噪声下允许小幅负增量，|Δ| 递增
   = 能量发散即 RED）；**只丢能量不漏光**（RXS-0358 口径继承：能量损失
   只允许数值截断方向，不允许负辐射或漏光注入）。**能量偏置注入即
   RED**。
5. **正确性锚 0-byte + 收敛不劣于**（G12_CONTRACT §4.2 生产化判据统一
   形态字面）：M96 既有判据 0-byte（起步范围冻结维持——MIS 完整面不改
   材质集合，Lambert/发光两类维持）；**固定 seed 位级确定性协议继承**
   （MIS 权重计算全 f32/f64 确定函数，无数据相关分支序差异）；golden
   门序 D2-Q7 维持（M96 未绿任何下游门不得验收）；**同 spp 收敛曲线不
   劣于参照器基线锚**（`g12_budget.json` 的 `g12.pt.ref_curve_*` 锚，
   direction=max，容差由 G12.2 标定程序 M166 measured 产出禁手写）——
   MIS 是方差削减升级，**收敛劣化冒充升级即 RED；确定性协议漂移即
   RED**。

**Implementation Requirements**

- 实现锚定（实现期命名，`src/rurix-render` 维持 `forbid(unsafe_code)`
  纪律）：生产化 host 面 `gi::path_trace::prod`（生产化场景装载/光源
  分布构建/MIS balance 权重/host oracle——与 device megakernel 公式面
  逐字同源，RXS-0357 host oracle 纪律继承：仅 host 输出不能充绿，门绿
  由 device 腿承载）+ device megakernel
  `src/rurix-render/kernels/g12_pt_production.rx` + harness
  `src/rurix-render/src/bin/g12_pt_production.rs`；M96 参照器既有面
  （`gi::path_trace` 既有条目/`g9_m96_path_tracer.rx`/harness）0-byte
  只消费不回写。
- RED 锚定计划（实现 PR 落）：权重缺失冒充 MIS（关 MIS 双策略裸加，
  输出仍同正例臂）→ RED；能量偏置注入（NEE 贡献人为增益）→ RED；
  收敛劣化冒充升级 → RED；确定性协议漂移（同 seed 输出分叉）→ RED。
- 本 spec PR 先行落最小锚定占位语料
  `conformance/gi/accept/mis_full_surface_minimal.rx` 与
  `conformance/gi/reject/mis_weight_missing.rx`、
  `conformance/gi/reject/mis_energy_bias_inject.rx`（条款锚定占位，
  inert 锚定口径与转正路径见各文件头注释）；锚点目标（实现 PR 转正）=
  `ci/g12_mis_full_surface_smoke.py` 门（symbolic key
  `g12.p0.m158.mis_full_surface`，G12.1 冻结字面 0-byte 不动）。

---

## 14. 条款（RXS-0399，G12.2 M159 俄罗斯轮盘生产化）

### RXS-0399 俄罗斯轮盘生产化：吞吐自适应终止概率、无偏补偿闭式、最小反弹保障与终止率/补偿计数面

**Legality**

1. **吞吐自适应终止概率**（RFC-0029 §4.2 L1 冻结语义；判据逐字引
   G12_CONTRACT §4.2 M159 行）：自第 N_min 级反弹起（**N_min ≥ 2 最小
   反弹保障**——低深度不早杀），路径按 `p_kill = clamp(1 − T/τ, 0,
   p_max)` 概率终止（T = 当前路径吞吐权重估计，τ = 吞吐参考阈——标定
   程序 measured 产禁手写 P-09；**p_max < 1 恒成立**——任何深度保留非零
   续行概率，禁截断偏置）。**早杀偏置注入（N_min 违反 / p_max=1）即
   RED**。
2. **无偏补偿**（RFC-0029 §4.2 L2 冻结语义）：续行路径权重乘以补偿
   因子 `1/(1 − p_kill)`——RR 补偿**闭式无偏**（estimator 期望不变）；
   补偿因子上界登记（防数值爆炸，钳制面显式登记进 evidence）。**补偿
   缺失冒充无偏即 RED**。
3. **计数非空**（RFC-0029 §4.2 L3 冻结语义）：RR 终止率（终止路径数/
   总路径数）、补偿因子分布（p50/p90/max）逐场景进 evidence——无计数
   面不得验收。
4. **跳 RR 偏移 RED 面继承**（RFC-0029 §4.2 L4；RXS-0357 L4 三臂 RED
   面继承）：关 RR 后同 seed 输出 digest 必须偏离正例臂对拍面——不偏离
   = 漏检 = 本条款整体 FAIL。
5. **正确性锚 0-byte + 收敛不劣于**：M96 既有判据 0-byte；固定 seed
   位级确定性协议继承（RR 终止判定与补偿计算全确定函数，终止不改后续
   采样起始索引——按索引寻址口径沿 RXS-0357 L2 字面）；**同 spp 收敛
   曲线不劣于基线锚**（`g12.pt.ref_curve_*` 锚，容差标定程序产）；收敛
   劣化冒充升级即 RED；确定性协议漂移即 RED。

**Implementation Requirements**

- 实现锚定（实现期命名，`forbid(unsafe_code)` 纪律维持）：吞吐自适应
  RR 参数面（τ 标定值消费、p_max/补偿钳制登记）+ 终止率/补偿计数导出
  （逐像素确定函数聚合，禁 atomic 顺序敏感累加）+ host oracle 同构兑现
  面（`gi::path_trace::prod`，公式面逐字同源）+ device megakernel
  `g12_pt_production.rx` 兑现面。
- RED 锚定计划（实现 PR 落）：早杀偏置注入（rr_min=0 / p_max=1）→
  RED；补偿缺失（续行不乘 1/(1−p_kill)）→ RED；跳 RR 偏移未检出 →
  RED（RXS-0357 三臂 RED 面继承）。
- 本 spec PR 先行落最小锚定占位语料
  `conformance/gi/accept/rr_throughput_adaptive_minimal.rx` 与
  `conformance/gi/reject/rr_early_kill_bias.rx`、
  `conformance/gi/reject/rr_compensation_missing.rx`（条款锚定占位，
  inert 锚定口径与转正路径见各文件头注释）；锚点目标（实现 PR 转正）=
  `ci/g12_russian_roulette_prod_smoke.py` 门（symbolic key
  `g12.p0.m159.russian_roulette_prod`，G12.1 冻结字面 0-byte 不动）。

---

## 15. 条款（RXS-0400，G12.2 M160 采样策略升级与低差异序列）

### RXS-0400 采样策略升级与低差异序列生产化：分层/低差异序列、序列索引确定性协议加性扩展与 RNG 流布局 provenance

**Legality**

1. **低差异序列面**（RFC-0029 §4.3 L1 冻结语义；判据逐字引
   G12_CONTRACT §4.2 M160 行）：采样维度序列自逐像素独立 PCG 流演进为
   **分层/低差异序列**——候选面 = Sobol 类低差异序列（确定性种子扰动）
   与分层采样（stratified per-dimension）；**选型经 benchmark measured
   裁决**（收敛曲线对照，选型证据进 evidence——本条款冻结选型面，采样
   选型是语义面选择非档位承诺，「仅测量不定档」纪律不适用，选型证据进
   evidence 即闭环）。
2. **确定性协议加性扩展**（RFC-0029 §4.3 L2 冻结语义）：低差异序列
   **索引推导确定性**——样本值 = f（像素索引， 采样索引， 维度， seed）
   确定函数寻址，无任何数据相关状态；**固定 seed 两次运行位级一致维持**
   （同 seed ⇒ 序列位级一致 ⇒ 输出位级一致；canonical digest 口径沿
   RXS-0357 L2 字面）；**RNG 流布局 provenance**（序列族/扰动面/寻址
   公式字面）进 evidence。RXS-0357 L2 既有字面 **0-byte**——本条款为
   **加性扩展**（新条款承载，修订行经 RXS-0398 L1 衔接）。**序列非确定
   冒充低差异即 RED；位级一致破坏未登记即 RED**。
3. **收敛不劣于**（RFC-0029 §4.3 L3 冻结语义）：同场景同 spp 收敛曲线
   measured **不劣于独立 PCG 流锚**（`g12.pt.ref_curve_*` 锚，容差标定
   程序产禁手写）——低差异序列的卖点是收敛加速，劣化即语义错误；**收敛
   劣化冒充升级即 RED**。
4. **正确性锚 0-byte**：M96 既有判据 0-byte（起步范围/确定性协议/
   pbrt-v4 容差带/golden 门序 D2-Q7）；M96 参照器既有流布局字面不动
   （生产化流布局为加性新面，不改 RXS-0357 冻结排布）。

**Implementation Requirements**

- 实现锚定（实现期命名，`forbid(unsafe_code)` 纪律维持）：采样器族
  （PCG 独立流承 G8 `ref_tracer` 对拍模式 / stratified 分层 / Sobol 类
  确定性种子扰动）同缓冲按索引寻址生成面 + 序列索引确定性机核（逐索引
  重求值 == 流内容）+ 双跑位级一致 + 流布局 provenance 登记；host 面
  `gi::path_trace::prod::sampler`，device 消费同一缓冲（RNG 流为输入
  不是结果，G7.4 先例同构）。
- RED 锚定计划（实现 PR 落）：序列非确定注入（同 seed 双跑流分叉）→
  RED；位级一致破坏未登记 → RED；收敛劣化冒充升级 → RED。
- 本 spec PR 先行落最小锚定占位语料
  `conformance/gi/accept/lds_deterministic_minimal.rx` 与
  `conformance/gi/reject/lds_nondeterministic_inject.rx`（条款锚定占位，
  inert 锚定口径与转正路径见各文件头注释）；锚点目标（实现 PR 转正）=
  `ci/g12_sampling_lds_upgrade_smoke.py` 门（symbolic key
  `g12.p0.m160.sampling_lds_upgrade`，G12.1 冻结字面 0-byte 不动）。

---

## 16. 条款（RXS-0401，G12.2 M161 收敛判据生产化）

### RXS-0401 收敛判据生产化：逐像素方差驱动自适应 spp 终止、spp 下界保障、收敛报告、误判率标定与全 spp golden 冻结带对拍

**Legality**

1. **方差驱动自适应终止**（RFC-0029 §4.4 L1 冻结语义；判据逐字引
   G12_CONTRACT §4.2 M161 行）：逐像素维护在线方差估计（Σx/Σx² 协议沿
   RXS-0357 L2 out_stats 面）；像素方差（或相对误差界）达阈即停采——
   **阈值标定程序产**（p100×k measured 入 `g12_budget.json` 禁手写
   P-09）；**spp 下界保障**（每像素最小采样数 ≥ N_floor——防早期方差
   欠估计早停；N_floor 值实现波冻结登记进 evidence）。**早停冒充收敛
   即 RED**。
2. **收敛报告**（RFC-0029 §4.4 L2 冻结语义）：逐像素 spp 分布
   （min/p50/p90/max）、方差分布、**未收敛像素计数**（达 spp 上界仍未
   达阈）非空进 evidence——无报告面不得验收；**未收敛像素缺报即
   RED**（报告计数与逐像素导出面独立重算不一致即缺报）。
3. **误判率标定**（RFC-0029 §4.4 L3 冻结语义）：**收敛误判率**（判
   收敛像素中相对全 spp 参照偏差超带比例）≤ 标定阈（标定程序产禁手写；
   对照集 = 自适应帧 vs 全 spp 参照帧同 seed 同场景）。
4. **golden 不偏离 + 帧型标签闭集**（RFC-0029 §4.4 L4 冻结语义）：
   固定全 spp golden 对拍不偏离冻结带
   （`milestones/g9/g9_m96_pbrt_tolerance_band.json` measured×2.0 带继承
   ——自适应帧与全 spp 参照同场景对拍，偏差超带即 RED）；**自适应帧
   不得冒充全 spp 参照**——evidence 帧型标签闭集
   `{adaptive, full_reference}`，混标即 RED。
5. **正确性锚 0-byte**：M96 既有判据 0-byte；固定 seed 位级确定性协议
   维持（自适应终止判定 = 样本值的确定函数，同 seed 同终止面同输出）；
   golden 门序 D2-Q7 维持；收敛曲线面不劣于基线锚口径沿 RXS-0398 L5
   字面。

**Implementation Requirements**

- 实现锚定（实现期命名，`forbid(unsafe_code)` 纪律维持）：逐像素在线
  方差 + 达阈停采 + spp 下界保障 + 逐像素 spp/收敛标志导出（禁 atomic
  顺序敏感累加）+ 收敛报告聚合面 + 误判率对照面（自适应帧 vs 全 spp
  参照帧同 seed 同场景）+ 帧型标签闭集登记；host 面
  `gi::path_trace::prod` 与 device megakernel `g12_pt_production.rx`
  公式面逐字同源。
- RED 锚定计划（实现 PR 落）：早停冒充收敛（spp 下界违反/阈值面外）
  → RED；未收敛像素缺报（报告计数 ≠ 逐像素导出面重算）→ RED；golden
  偏离冻结带 → RED；帧型标签混标 → RED。
- 本 spec PR 先行落最小锚定占位语料
  `conformance/gi/accept/adaptive_convergence_minimal.rx` 与
  `conformance/gi/reject/early_stop_masquerade.rx`、
  `conformance/gi/reject/unconverged_pixel_underreport.rx`（条款锚定
  占位，inert 锚定口径与转正路径见各文件头注释）；锚点目标（实现 PR
  转正）= `ci/g12_convergence_criterion_prod_smoke.py` 门（symbolic key
  `g12.p0.m161.convergence_criterion_prod`，G12.1 冻结字面 0-byte
  不动）。

---

## 17. 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| v1.0 | 2026-08-12 | 新建（G9.4 spec-first，GI 波 M96~M101，硬规则 7 条款先行）：RXS-0357（M96 M17 Path Tracer 参照器：megakernel + NEE/MIS/RR + 起步范围冻结〔焦散/体积/specular 链 out〕+ 固定 seed 位级一致确定性协议〔累加序/RNG 流冻结、逐像素 sample count/方差导出、匹配深度 1/2/full 三 golden〕+ pbrt-v4 收敛曲线 measured 冻结容差带 + 改 seed/跳 RR/关 MIS 三臂 RED + 门序硬约束〔M96 未绿 M97~M101 任何画质门不得验收，机器阻断〕）/ RXS-0358（M97 Surface Cache：离线 Card 参数化 ≤12/mesh 可配 + 运行时辐射度缓存 + 只丢能量不漏光〔漏光像素计数=0〕+ Card 空洞漏光检测 RED 臂 + 图集复用 M04/M91 页 ABI 不私定 + 按匹配深度对 M96 golden）/ RXS-0359（M98 四级追踪降级链 L1 Screen Trace→L2 SWRT→L3 HWRT〔含 hit lighting 档〕→L4 Far Field + 逐档命中率/耗时计数逐帧 evidence + 逐级强关回归可检测〔强关后仍同 golden 即 RED〕+ 禁静默回退 + L4 未就绪 SKIP=not-triggered）/ RXS-0360（M99 屏幕级 SPG 自适应细分 + Radiance Cache 双级 + product IS 关闭方差回归 RED + 世界级 clipmap 未 measured 举证 not-triggered 不充绿）/ RXS-0361（M100 低档多灯直接光默认档 + 验证射线零跳过硬契约〔D2-Q4〕+ 高档 ReSTIR workload 证据不足 not-triggered 不充绿）/ RXS-0362（M101 IF 体素网格档位阶梯 L0~L3 + 共享 probe 着色/八面体编码内核只换空间索引 + 八面体编码线性域 + 每档 AS 更新预算行消费 AsStats + 超预算强制降档 RED）。**目标 spec 新建裁决**：RFC-0022 §5 映射表 GI 各行候选（rendering_platform.md / shader_stages.md / 资产管线 spec / conformance 协议章）裁定合并新建本文件（D2 GI 独立语义轴，候选文件本体 0-byte，头注留痕）。条款号自 ledger 实测 `RXS.next_free=357` 顺位领取（0357~0362 连续不跳号，0295/0296 burned 与 shadow_reserved 181~184 维持）。conformance 最小锚定语料同 PR 落（conformance/gi/{accept,reject}/，inert + `//@ spec` 锚定 + 预期诊断注释 + 转正路径旁注，G9.2/G9.3 spec 波先例）；symbolic key `g9.p0.m96/m97/m98.*`（G9.1 冻结字面）与 `g9.p1.m99/m100/m101.*`（G9.4 波 P1 全进裁决登记，G9_ACCEPTANCE_MAP §3 / CI_GATES §4A）0-byte 不动。零新 RX 码（诊断码实现期按实际可达类别领取不预造）、零新 U/RD/SG、零 src/ 改动、零 workflow 步骤。依据 [RFC-0022](../rfcs/0022-virtual-geometry-gi-semantics.md)（Agent Approved 2026-08-09）§4.6/§4.7/§4.8/§4.10/§7 + G9_ACCEPTANCE_MAP §2 M96/M97/M98 行 + §3 M99/M100/M101 行（判据逐字）+ G9_CANDIDATE_DECISIONS §2 RD-040 行与 v1.3 校准注 | **Full RFC**（RFC-0022） |
| v1.1 | 2026-08-16 | 追加（G11.4 光照与 GI 修复波 spec-first，硬规则 7 条款先行；G11 已解锁 implementation_status=unblocked，G11_CONTRACT §8.1）登记 **RXS-0394 ~ RXS-0396**：RXS-0394（M153 R3 灯种子集表达：光源集五元闭集〔契约 sun/sky + glTF 点光源/面光源/emissive 表面，缺类显式登记〕+ 契约光照面单通道〔corpus/lighting_*.json 唯一事实源，glTF 字段 = 派生输入经 M133 只追加修订程序，直读绕过即 RED〕+ 点光源辐射链〔E = color×I/d²、L = E·ndl·albedo/π·vis，强度派生 = 灯具 emissive 通量换算 Φ=Le·A·π / I=Φ/(2π)，逐盏 provenance〕+ emissive 双级能量贡献语义 + cornell 契约 sun+sky 灯面 0-byte + M100 低档面联动评估登记〔不新造，M100-high 维持 defer〕）/ RXS-0395（M154 R4 多反弹 GI：屏幕探针近场 + 世界辐射缓存远场兜底双级语义〔失效必须回落 + 回落路径逐帧计数 + 禁静默零辐射〕+ 多反弹 ≥2 级〔第二次及以上经世界缓存查询 + 反弹级数/逐级能量计数 + 只丢能量不漏光 RXS-0358 口径继承〕+ 逐级能量单调不增 + host 同构世界缓存兑现面〔解析式否决；不冒充 GPU 管线世界级，GPU 面锚定 G14〕+ RXS-0357 L6 门序继承）/ RXS-0396（M154 M99-clipmap 世界级辐射缓存承接：**RXS-0360 世界级 not-triggered 登记翻转修订行**〔G10.6 rejudged-go 承接锚逐字 + measured 举证 R4 行 4.697253086805343 / C1 行 ≈21×；RXS-0360 既有字面 0-byte〕+ 空间哈希世界缓存〔对数族量化 level=clamp(floor(log2(1+dist/d_ref)),0,LEVELS−1)，s(ℓ)=s0×2^ℓ，LEVELS=4 / s0=scene_diag×2^-8 / d_ref=scene_diag×2^-4 实测标定冻结〔bistro 25.962 m / cornell 958.659 单位〕+ 双哈希步长线性探测 + 在线构建零离线预处理〕+ 距离自适应辐射 LOD clipmap 级〔层级/距离带/命中率计数 + 禁静默降级〕+ 级间回落链 → 天光末级兜底显式登记 + 世界级双锚判定〔远场探针集能量回归达标定阈 + M96 golden 匹配深度对拍，UE 对拍归 G11.5 不混用〕+ ≠RXS-0359 L4 Far Field 边界声明〔M98-l4 defer 0-byte〕）。条款号自 ledger 实测 `RXS.next_free=394` 顺位领取（0394~0396 连续不跳号，0295/0296 burned 与 shadow_reserved 181~184 维持）。零新 RX 码；零新 U/RD/SG；conformance 最小锚定语料六件（conformance/gi/accept 三件：light_seed_set_minimal.rx / gi_multibounce_two_level_minimal.rx / world_radiance_cache_minimal.rx；reject 三件：light_seed_gltf_direct_bypass.rx / gi_single_bounce_masquerade.rx / world_cache_farfield_zero_energy.rx；inert + `//@ spec` 锚定 + 预期 RED 注释 + 转正路径旁注，G9.2~G11.2 spec 波先例）同 PR 落；symbolic key `g11.p0.m153/m154.*`（G11.1 冻结字面，G11_ACCEPTANCE_MAP §1 / CI_GATES §4）0-byte 不动；trace_matrix 重生成 CRLF 字节纪律维持（375→378 全锚定）；stable 快照因条款计数 375→378 同 PR 重 bless（RXS-0180 L2 加性演进，error_codes/editions/subcommands 三段 0 变化）。依据 [RFC-0028](../rfcs/0028-g11-gi-quality-closure.md)（Agent Approved 2026-08-16，D-409 评审后）§4.1/§4.2/§4.3/§4.4/§5 + G11_CONTRACT §4.2 M153/M154 行（判据逐字）+ G11_ACCEPTANCE_MAP §1。既有 spec 条款字面 0-byte（只追加新条款/修订记录行；§9 修订记录节号顺延 §12，节体 0-byte），不触红线/禁区。`Assisted-by: Kimi-K3（G11.4 波）` | **Full RFC**（RFC-0028） |
| v1.2 | 2026-08-16 | 追加（G11.5b 追加子波 spec-first，硬规则 7 条款先行；G11.5 R1 行整波 FAIL 停线后诊断修复面，G11_CONTRACT §8.5b）登记 **RXS-0397**（天光直接漫反射 IBL 消费面 --sky-ibl：全向口径〔UE SkyLight 指定 cubemap 无遮蔽投递对齐——G11.5b measured 诊断 g11_5b_ldr_residual_diag.md：UE 侧 SkyLight 漫反射占帧均值 95.4%、镜面 ≤0.03%、r.DynamicGlobalIlluminationMethod=0/距离场关机制取证〕+ 下半球黑半球混合闭式 Lo = albedo×L_sky×(1+n·up)/2 + GI 双重计数排除〔旗标开时间接估计子 miss 射线整零，天光首反弹 = 直接项单计数；沉积直接项同式〕+ **RXS-0396 L4 末级兜底修订行**〔旗标开时末级兜底由直接项承接，RXS-0396 既有字面 0-byte〕+ 镜面天光不消费登记〔c1_ue_specular_ibl 维持 G15 候选〕+ 消费面边界〔--gi-multibounce 组合面；cornell 契约灯面 0-byte〕+ 门序继承 + 闭环断言面 = M155/M147 g11.5 phase 字面不改判据）。条款号自 ledger 实测 `RXS.next_free=397` 顺位领取（0397 连续不跳号，0295/0296 burned 与 shadow_reserved 181~184 维持）。零新 RX 码；零新 U/RD/SG；conformance 最小锚定语料两件（accept sky_ibl_direct_diffuse_minimal.rx + reject sky_ibl_gi_double_count.rx；inert + `//@ spec` 锚定 + 转正路径旁注，G9.2~G11.4 spec 波先例）同 PR 落；symbolic key `g11.p0.m155/m147.*`（G11.1 冻结字面，G11_ACCEPTANCE_MAP §1 / CI_GATES §4）0-byte 不动；trace_matrix 重生成 CRLF 字节纪律维持（378→379 全锚定）；stable 快照因条款计数 378→379 同 PR 重 bless（RXS-0180 L2 加性演进，error_codes/editions/subcommands 三段 0 变化）。依据 [RFC-0028](../rfcs/0028-g11-gi-quality-closure.md)（Agent Approved 2026-08-16，D-409 评审后）§4.5 伞形「GI/天光遮蔽语义面」+ §5 映射表 + G11_CONTRACT §4.2 M155 行判据逐字承接 + 主会话 G11.5b 裁决（先诊断修复后评 metric，禁改判据充绿）。既有 spec 条款字面 0-byte（只追加新条款/修订记录行；§12 修订记录节号顺延 §13，节体 0-byte），不触红线/禁区。`Assisted-by: Kimi-K3（G11.5b 波）` | **Full RFC**（RFC-0028 伞形 §4.5 面承接） |
| v1.3 | 2026-08-17 | 追加（G12.2 生产化核心波 spec-first，硬规则 7 条款先行；G12 已解锁 implementation_status=unlocked，G12_CONTRACT §8.1 G-G12-3 互锁 READY）登记 **RXS-0398 ~ RXS-0401**：RXS-0398（M158 MIS 完整面生产化：NEE×BSDF 双策略逐顶点 balance heuristic〔w_nee=p_nee/(p_nee+p_bsdf)，delta 光源退化 w_nee=1 无除零〕+ 多光源 MIS〔联合 PDF=离散×连续，光源分布构建确定性 digest〕+ 能量守恒〔白炉均值=入射能量 measured 标定容差 + 逐级能量增量单调不增 RXS-0395 口径继承 + 只丢能量不漏光 RXS-0358 口径继承〕+ 正确性锚 0-byte + 同 spp 收敛不劣于 g12_budget ref_curve 锚〔容差 M166 标定产〕+ **RXS-0357 参照器面生产化演进显式修订行**〔RXS-0357 既有字面 0-byte，起步范围冻结维持〕）/ RXS-0399（M159 俄罗斯轮盘生产化：吞吐自适应 p_kill=clamp(1−T/τ,0,p_max)〔τ 标定程序产 + p_max<1 恒成立 + N_min≥2 最小反弹保障〕+ 无偏补偿闭式 1/(1−p_kill)〔上界登记〕+ 终止率/补偿计数非空 + 跳 RR 偏移 RED 面继承 RXS-0357 三臂）/ RXS-0400（M160 采样策略升级与低差异序列：stratified/Sobol 类候选 benchmark measured 裁决选型 + 确定性协议加性扩展〔样本值=f(像素,采样,维度,seed) 确定函数寻址 + 固定 seed 位级一致维持 + RNG 流布局 provenance；RXS-0357 L2 既有字面 0-byte〕+ 收敛不劣于独立 PCG 流锚）/ RXS-0401（M161 收敛判据生产化：逐像素方差驱动自适应 spp 终止〔阈值标定程序产 p100×k 入 g12_budget + spp 下界 N_floor 保障〕+ 收敛报告〔spp 分布/方差/未收敛像素计数非空〕+ 误判率 ≤ 标定阈 + 全 spp golden 对拍不偏离 measured×2.0 冻结带 + 帧型标签闭集 {adaptive, full_reference}）。条款号自 ledger 实测 `RXS.next_free=398` 顺位领取（0398~0401 连续不跳号，0295/0296 burned 与 shadow_reserved 181~184 维持）。零新 RX 码；零新 U/RD/SG；conformance 最小锚定语料十一件（conformance/gi/accept 四件：mis_full_surface_minimal.rx / rr_throughput_adaptive_minimal.rx / lds_deterministic_minimal.rx / adaptive_convergence_minimal.rx；reject 七件：mis_weight_missing.rx / mis_energy_bias_inject.rx / rr_early_kill_bias.rx / rr_compensation_missing.rx / lds_nondeterministic_inject.rx / early_stop_masquerade.rx / unconverged_pixel_underreport.rx；inert + `//@ spec` 锚定 + 预期 RED 注释 + 转正路径旁注，G9.2~G11.5b spec 波先例）同 PR 落；symbolic key `g12.p0.m158/m159/m160/m161.*`（G12.1 冻结字面，G12_ACCEPTANCE_MAP §1 / CI_GATES §4）0-byte 不动；trace_matrix 重生成 CRLF 字节纪律维持（379→383 全锚定）；stable 快照因条款计数 379→383 同 PR 重 bless（RXS-0180 L2 加性演进，error_codes/editions/subcommands 三段 0 变化）。依据 [RFC-0029](../rfcs/0029-g12-path-tracer-productionization.md)（Agent Approved 2026-08-17，D-409 评审后）§4.1/§4.2/§4.3/§4.4/§4.7/§5 + G12_CONTRACT §4.2 M158/M159/M160/M161 行判据逐字承接 + G12_ACCEPTANCE_MAP §1。既有 spec 条款字面 0-byte（只追加新条款/修订记录行；§13 修订记录节号顺延 §17，节体 0-byte），不触红线/禁区。`Assisted-by: Kimi-K3（G12.2 生产化核心波）` | **Full RFC**（RFC-0029） |
