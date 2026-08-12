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

## 9. 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| v1.0 | 2026-08-12 | 新建（G9.4 spec-first，GI 波 M96~M101，硬规则 7 条款先行）：RXS-0357（M96 M17 Path Tracer 参照器：megakernel + NEE/MIS/RR + 起步范围冻结〔焦散/体积/specular 链 out〕+ 固定 seed 位级一致确定性协议〔累加序/RNG 流冻结、逐像素 sample count/方差导出、匹配深度 1/2/full 三 golden〕+ pbrt-v4 收敛曲线 measured 冻结容差带 + 改 seed/跳 RR/关 MIS 三臂 RED + 门序硬约束〔M96 未绿 M97~M101 任何画质门不得验收，机器阻断〕）/ RXS-0358（M97 Surface Cache：离线 Card 参数化 ≤12/mesh 可配 + 运行时辐射度缓存 + 只丢能量不漏光〔漏光像素计数=0〕+ Card 空洞漏光检测 RED 臂 + 图集复用 M04/M91 页 ABI 不私定 + 按匹配深度对 M96 golden）/ RXS-0359（M98 四级追踪降级链 L1 Screen Trace→L2 SWRT→L3 HWRT〔含 hit lighting 档〕→L4 Far Field + 逐档命中率/耗时计数逐帧 evidence + 逐级强关回归可检测〔强关后仍同 golden 即 RED〕+ 禁静默回退 + L4 未就绪 SKIP=not-triggered）/ RXS-0360（M99 屏幕级 SPG 自适应细分 + Radiance Cache 双级 + product IS 关闭方差回归 RED + 世界级 clipmap 未 measured 举证 not-triggered 不充绿）/ RXS-0361（M100 低档多灯直接光默认档 + 验证射线零跳过硬契约〔D2-Q4〕+ 高档 ReSTIR workload 证据不足 not-triggered 不充绿）/ RXS-0362（M101 IF 体素网格档位阶梯 L0~L3 + 共享 probe 着色/八面体编码内核只换空间索引 + 八面体编码线性域 + 每档 AS 更新预算行消费 AsStats + 超预算强制降档 RED）。**目标 spec 新建裁决**：RFC-0022 §5 映射表 GI 各行候选（rendering_platform.md / shader_stages.md / 资产管线 spec / conformance 协议章）裁定合并新建本文件（D2 GI 独立语义轴，候选文件本体 0-byte，头注留痕）。条款号自 ledger 实测 `RXS.next_free=357` 顺位领取（0357~0362 连续不跳号，0295/0296 burned 与 shadow_reserved 181~184 维持）。conformance 最小锚定语料同 PR 落（conformance/gi/{accept,reject}/，inert + `//@ spec` 锚定 + 预期诊断注释 + 转正路径旁注，G9.2/G9.3 spec 波先例）；symbolic key `g9.p0.m96/m97/m98.*`（G9.1 冻结字面）与 `g9.p1.m99/m100/m101.*`（G9.4 波 P1 全进裁决登记，G9_ACCEPTANCE_MAP §3 / CI_GATES §4A）0-byte 不动。零新 RX 码（诊断码实现期按实际可达类别领取不预造）、零新 U/RD/SG、零 src/ 改动、零 workflow 步骤。依据 [RFC-0022](../rfcs/0022-virtual-geometry-gi-semantics.md)（Agent Approved 2026-08-09）§4.6/§4.7/§4.8/§4.10/§7 + G9_ACCEPTANCE_MAP §2 M96/M97/M98 行 + §3 M99/M100/M101 行（判据逐字）+ G9_CANDIDATE_DECISIONS §2 RD-040 行与 v1.3 校准注 | **Full RFC**（RFC-0022） |
