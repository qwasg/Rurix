# RFC-0022 — 虚拟几何与 GI 语义（G9 伞形三章之一）

| 字段 | 值 |
|---|---|
| RFC 编号 | RFC-0022（4 位制，编号永不复用，10 §9.5；编号按 2026-08-09 实测 `registry/number_ledger.json` namespaces.RFC next_free=22 领取，registry 登记由 G9.1 治理波统一落） |
| 标题 | 虚拟几何与 GI 语义（G9 伞形三章之一） |
| 档位 | **Full RFC**（cluster DAG/LOD 运行时语义、页格式 v2 磁盘 ABI 演进、CLAS 簇级 AS 构建与 capability profile 门控、VisibleClusterSet 单源真相契约、GI 追踪降级链与档位语义、时域材质语义扩展、Path Tracer 参照器确定性协议；触及运行时语义、FFI/unsafe 相邻边界与资产 ABI 面，10 §3 / AGENTS 硬规则 5） |
| 状态 | **Agent Approved（2026-08-09）** |
| 承接里程碑 | G9（G9.2~G9.4，验收门 G-G9-4~6） |
| 关联条款 | 拟落 spec **RXS-####~**（区间不预写推测号，G9.2 互锁开放后按 actual next_free 领取；当前实测自由起点 RXS-0344 仅为快照非 claim）；候选落点见 §5 |
| 依据决策 | D-406 v2.0 · D-409 · P-13 · 10 §7 · G9.1 立项六项裁决（已定案） · RD-039 M06/M09 分项 backfill 字面（G9 立项书为触发证据）· RD-040 世界辐射缓存 measured 触发字面 · [RFC-0019](0019-rendering-platform.md) §4.5（capability/profile）/§4.6（时域语义）/§4.7（M28 边界）· G9.0 不可变 ref `1d9460a1`（[D1 草案](../milestones/g9/design/G9_D1_VIRTUAL_GEOMETRY_RT.md) D-1~D-12 · [D2 草案](../milestones/g9/design/G9_D2_GI_LIGHTING.md) D2-Q1~Q14）· [G9_CAPABILITY_MATRIX](../milestones/g9/G9_CAPABILITY_MATRIX.md) M90~M98 |
| Provenance | `Assisted-by: Kimi Code CLI (Kimi) rfc0022-drafter` |
| Agent 批准 | **Agent Approved 2026-08-09**；agent 依 10 §7/P-13/D-406 v2.0 自主批准；批准只表示语义评审完成，**不构成实现许可**（G9.2 互锁仍为独立硬门） |
| 对抗性评审 | **完成**（D-409 第 1 轮）：评审 provenance `Assisted-by: Kimi Code CLI (Kimi) rfc0022-adversarial-reviewer`（独立实例，与起草无共享上下文）≠ 起草 provenance `Assisted-by: Kimi Code CLI (Kimi) rfc0022-drafter`；结论「有条件通过」，6 findings（1 major + 5 minor）全部 disposition，含 1 条跨文档移交（F-6 → G9.1 治理波）；同工具族偏差说明与 findings 表见 §9.1 |

---

## 1. 摘要

本 RFC 是 G9 伞形三章的第一章，定义 G9 建造期 D1（虚拟化几何与 RT 合流）与 D2（全局光照与光照缓存）两模块的语义面：

1. **M90 cluster DAG 深化**：monotonic 误差度量不变量、簇对锁定语义、蒙皮元数据模型、CLAS 离线烘焙输入；
2. **M91 页格式 v2**：RXPL 新 major ABI 语义面，M04 v1 0-byte 共存，未知版本 fail-closed；
3. **M94 CLAS×RT 合流**：当帧 multi-indirect 拼装语义，NV 主腿 + 传统 BLAS 回退腿（正确性基线），capability profile 门控；
4. **M93/M95 VisibleClusterSet 单源真相**：光栅/BLAS/VSM 一份三喂，帧末 provenance 校验负例为硬门；
5. **M97 Surface Cache**：离线 Card 参数化（≤12/mesh 可配），缺失覆盖**只丢能量不漏光**不变量；
6. **M98 四级追踪降级链**：L1 Screen Trace → L2 SWRT → L3 HWRT hit lighting → L4 Far Field，逐档可关可测禁静默；
7. **probe 编码与档位阶梯**：八面体编码共享内核、SPG 自适应细分、Radiance Cache 双级、IF L0~L3 阶梯（M99/M101 语义先行）；
8. **材质时域语义面**：蒙皮/WPO/object transform 三类速度通道消费；
9. **M96 Path Tracer 参照器**：megakernel 起步、固定 seed 确定性协议、pbrt-v4 对照、golden 门序硬约束（M96 未绿，M97~M101 任何画质门不得验收（任何 GI 档位画质门））。

```text
资产离线侧                          运行时任一帧
ClusterDag builder ──RXPL v2 页──▶ streamer（M44 只消费）
  ├─ monotonic 误差 + 簇对锁定         │
  ├─ 蒙皮元数据                        ▼
  └─ CLAS 烘焙输入              误差驱动 DAG cut ──▶ VisibleClusterSet
                                      ▲                │ 一份三喂
                          蒙皮 kernel（skin cache）    ├─▶ 光栅 VisBuffer
                                                       ├─▶ RT BLAS 拼装（CLAS 主腿 / 传统回退腿）
                                                       └─▶ VSM 页标记
                                                                │
                                              GI：追踪降级链 L1→L4 → probe/IF 档位
                                                                │
                                              M96 Path Tracer 参照器 ── golden 对拍（门序硬约束）
```

本 RFC 是 **G9.1 governance-only** 交付物。即使随后 Agent Approved，也只表示语义评审通过，**不会解锁任何 `src/`、`spec/`、`conformance/` 实现**；G9.2 互锁（对标 G8.2）是独立硬门。§4 全部类型/数据结构/状态机级设计为**拟议语义（Draft）**，批准前不构成契约。

## 2. 动机、范围与治理门

### 2.1 为什么需要 Full RFC

G8 冻结的虚拟化几何底座（cluster DAG、页式流送、VisBuffer）只有静态 meshlet DAG——无误差度量、无簇对锁定、无蒙皮元数据、无 CLAS 输入；RT 侧 M50 增量面已绿但几何/RT 两个世界的可见性各自独立计算。D1 要把底座升级为动态资产可用的完整 Nanite 类管线并合流进 RT AS；D2 要在其上建造可降级、有跨路径 golden 参照的完整 GI 子系统。这些面涉及：运行时 LOD 选择语义、磁盘页格式 ABI 演进（触 M04 v1 冻结面相邻）、device AS 构建（unsafe 相邻）、capability profile 门控类型规则、时域材质语义扩展、以及一切 GI 画质门的 golden 判定协议——都不是 Direct/Mini 可安全承载的局部实现选择。

承接锚字面（法定输入，G9.1 立项裁决已定案）：

- RD-039 M06/M09 两分项（「动态资产面出现时」「RT 与虚拟几何合流需求出现时」）：以 G9 正式立项书为触发证据，`registry/deferred.json` history **只追加**登记 open-defer → G9 承接，禁静默改写 G8.7 决策表原文；
- M17 Path Tracer 参照器 backfill「GI/材质画质门需要跨路径 golden 时（G9+ 建造期前置）」字面已命中（D2 各画质门均需跨路径 golden），G9_CAPABILITY_MATRIX M96 行记「建议判 go」；
- M14（HWRT hit lighting / Far Field）「M50 后评估，画质 measured 需求」——M50 已绿，需求方 = D2 自身画质门，M98 重判档成立；
- RD-040 世界辐射缓存「屏幕探针远场缺失成为画质 **measured** 问题」——M99 世界 clipmap 级须 measured 触发举证，未举证只做屏幕级。

### 2.2 双门互锁：RFC 批准不等于实现开工

| 门 | 允许动作 | 禁止动作 |
|---|---|---|
| G9.1 governance-only（本波） | 起草/评审/批准 RFC；冻结 P0 key 命名空间、evidence schema 目标路径与 §5 spec 映射计划 | 不改 `src/`、`spec/`、`conformance/`；不 materialize 数字 CI 步骤；不预建空 schema 壳/空脚本占位；不领取 RXS/RD/U/RX 共享在途号 |
| G9.2 implementation gate | G9.1 决策表/验收映射/measured budget 齐备，互锁 validator 读不可变 refs 全绿后，spec-first 落条款与 RED | 互锁任一红时不得以 RFC Approved 或立项裁决替代机器事实 |

### 2.3 in-scope

| 面 | 本 RFC 冻结内容 | P0 key（G9.1 冻结命名空间） | 最晚波次 |
|---|---|---|---|
| M90 DAG 深化 | 误差度量 monotonic 不变量、簇对锁定、蒙皮元数据、CLAS 烘焙输入 | `g9.p0.m90.cluster_dag_deepening` | G9.2 |
| M91 页格式 v2 | RXPL 新 major ABI 语义、v1 0-byte 共存、未知版本 fail-closed | `g9.p0.m91.page_format_v2_abi` | G9.2 |
| M93 LOD/可见集 | 误差驱动 cut 无重叠无空洞、未驻留父簇兜底、VisibleClusterSet 载荷 | `g9.p0.m93.visible_cluster_set` | G9.3 |
| M94 CLAS 合流 | 当帧 multi-indirect 拼装、Template 实例化、双腿语义、capability 门控 | `g9.p0.m94.clas_rt_convergence` | G9.3 |
| M95 单源真相 | 一份三喂、动画分级作用于 AS 更新、帧末 provenance 校验 | `g9.p0.m95.single_source_truth` | G9.3 |
| M96 PT 参照器 | megakernel+wavefront 接口、固定 seed 确定性、pbrt-v4 对照、golden 门序 | `g9.p0.m96.path_tracer_reference` | G9.4（波内第一顺位） |
| M97 Surface Cache | Card 参数化、运行时辐射度缓存、只丢能量不漏光 | `g9.p0.m97.surface_cache` | G9.4 |
| M98 追踪降级链 | L1~L4 逐档契约、hit lighting 档、降级选择 evidence | `g9.p0.m98.tracing_fallback_chain` | G9.4 |
| M92 蒙皮/骨骼植被 | 语义面随 §4.2 冻结（P1）；gate key 不在本 P0 命名空间，G9.1 决策表另立 | — | G9.3 |
| M99 SPG+RC / M101 IF 档位 | 语义面随 §4.8 冻结（P1）；gate key 同上另立 | — | G9.4 |

key/脚本/evidence schema 三方逐字一致冻结：脚本形如 `ci/g9_<slug>_smoke.py --gate <key>`；evidence schema 目标路径 `milestones/g9/g9_m<##>_<slug>_evidence_schema.json`（**只冻结路径，本波不预建文件**）。

## 3. 指导级解释（用户视角）

### 3.1 动态资产进入虚拟化几何

美术导入骨骼植被资产后，builder 在离线侧产出：带 monotonic 误差的 cluster DAG、每簇蒙皮元数据（最大影响骨数、骨骼索引集、包围体膨胀系数）与 CLAS 构建输入，全部打包进 RXPL v2 页。运行时蒙皮 kernel（`.rx` compute）按簇读骨骼 palette，输出蒙皮后顶点与保守包围球/法向锥进 skin cache；剔除、光栅、RT 三方只消费 skin cache，不存在第二份蒙皮结果。距离分级更新率（全速 / 1/2 / 1/3 / 1/4）降级时，包围体按最大未更新帧数放大，剔除不错杀。Morph 资产走非虚拟化传统 vertex path，GpuScene 以 instance flag 区分，禁止混入 DAG。

### 3.2 一份可见集，三个消费者

运行时 selection 对 DAG 求屏幕空间误差 cut——投影簇误差 ≤ 阈值则选该簇，否则下降子簇，cut 上每簇恰好选中一次（无重叠无空洞）；选中簇页未驻留时用父簇兜底并登记迟到页（沿 G8.4 迟到页降级语义）。selection 输出 `VisibleClusterSet`（紧凑数组：cluster stable id + LOD level + 蒙皮版本 + 变换 id），同一份数组：喂光栅 VisBuffer（64 位格式不变）、经 multi-indirect device 构建拼成当帧 BLAS、喂 VSM 阴影页标记。任何消费者独立再算一遍可见性，帧末 provenance 校验即 RED。

### 3.3 GI 档位与参照器

GI 射线按四级降级链选档：近处屏幕内走 L1 Screen Trace，中距走 L2 SWRT（Mesh/Global SDF），全场景视距内走 L3 HWRT RayQuery（高档开 hit lighting 完整材质求值），视距外走 L4 Far Field 代理。每一档可独立开关、有独立命中率/射线量/耗时计数；逐 probe/逐像素的选档结果写入 evidence，禁止静默回退。Surface Cache 未覆盖的区域只丢能量不漏光——回退到下一级追踪结果或 ambient 项，绝不产生负值/黑色裂缝。所有画质门以 M96 Path Tracer 参照器为 golden 基准：固定 seed 两次运行位级一致，与 pbrt-v4 同场景同 spp 收敛曲线对照；M96 未绿，M97~M101 任何画质门不得验收（任何 GI 档位画质门）。

## 4. 参考级设计（拟议语义，Draft）

### 4.0 跨面不变量

1. **单源真相**：`VisibleClusterSet` 是当帧几何可见性的唯一事实源；光栅/RT/VSM 与 GI 追踪底座不各自重算可见性（D1 D-8，[调研5]）。
2. **strict-only / fail-closed**：误差单调性破坏、页 digest 篡改、未知页版本、capability snapshot 不满足、cut 空洞、provenance 错配均确定性拒绝；不得静默继续或静默降级。
3. **deterministic**：同一资产+同一 builder 版本双构建字节一致（沿 M79 判据）；M96 固定 seed 两次运行位级一致；canonical key/哈希不含路径、mtime、随机 seed。
4. **正确性基线与优化腿分离**：传统 BLAS 回退腿是正确性基线，CLAS 腿是 VRAM/构建带宽优化腿；两腿对同场景 ray query 逐命中一致（D-6/D-9）。
5. **非 stable ABI**：CLAS/BLAS 物理字节、device address、driver 构建耗时数值为实现确定、非 stable；页格式 schema/digest 规则、误差语义、provenance 规则由 spec 冻结。

### 4.1 M90 — cluster DAG 误差度量与簇对锁定

在 G8 `ClusterDag` 上扩展（builder 离线，host 纯 Rust，`forbid(unsafe_code)` 纪律维持）：

- **误差度量**：每簇记录 `parent_error` / `cluster_error`（几何近似误差 + 包围球）。monotonic 不变量：DAG 每一边 `parent_error ≥ cluster_error` 逐边成立；builder 校验发现破坏单调性的输入/中间态必须 typed Err 拒绝（fail-closed，无 UB）。运行时选择即求误差 cut（[调研1]）。
- **簇对锁定（cluster pair locking）**：边塌缩时锁定相邻簇边界，避免 LOD 裂缝。算法与 nv_lod_cluster_builder 同源但**独立自研实现，不 vendoring NVIDIA 代码**（D-10：许可与确定性双构建纪律；「边塌缩/簇对锁定」术语引自 D1 草案转述，R4 U4）。
- **CLAS 离线烘焙输入**：builder 输出每簇 CLAS 构建输入（三角形簇 + 簇级 AABB），随资产页打包；运行时 CLAS 构建退化为 device 侧拼装而非几何处理（[调研3]）。
- **输出**：走 RXPL 新 major（§4.5），与 M04 v1 共存；双构建确定性沿 M79 判据。

### 4.2 蒙皮元数据模型与保守包围体（M90/M92 语义面）

- **蒙皮元数据**（每簇，随页烘焙）：最大影响骨数、骨骼索引集、蒙皮包围体膨胀系数（Kerbl 保守界所需输入）。
- **保守包围体**：蒙皮 kernel 输出蒙皮后顶点 + 保守包围球/法向锥写回 skin cache——LBS 权重下包围球半径按各骨最大位移保守放大，法向锥按骨旋转角保守放大（Unterguggenberger/Kerbl/Pernsteiner/Wimmer，《Conservative Meshlet Bounds for Robust Culling of Skinned Meshes》，CGF 40(7)，2021，R4 §3.1 著录勘定）。不变量：任意姿态序列下蒙皮后顶点 100% 落在保守包围球内；法向锥覆盖真实法向。
- **拒绝 CPU 蒙皮权宜路线**（D-1）：不做「CPU 蒙皮喂静态 cluster」——击穿 cluster 包围体/误差度量、吞吐低、有视角 LOD 瑕疵（机制表述引自 D1 草案转述，R4 U3；Epic 官方可复核「无几何 LOD / 不支持 Morph」）。
- **距离分级动画更新率**（D-3）：按相机距离分档（全速 / 1/2 / 1/3 / 1/4，10m 内全速），分级状态进场景表；降级帧顶点缓冲逐位不变，保守包围体按最大未更新帧数放大；恢复全速无跳变越界。
- **Morph 旁路**（D-2）：Morph 走非虚拟化传统 vertex path，禁入 DAG。

### 4.3 M94 — CLAS 离线烘焙消费与当帧拼装语义

- **主腿（NV CLAS）**：`VK_NV_cluster_acceleration_structure`——离线烘焙的 CLAS 输入随页流送；当帧用 multi-indirect device 构建把 `VisibleClusterSet` 涉及的 CLAS 拼成 BLAS；静态重复几何用 Cluster Template 实例化共享底层 AS（D-5，[调研3]）。BLAS 生命周期归 `AsManager` 单所有者扩展：新增 `ClasBlasKey` = 可见簇集合 digest，沿用 G8 显式策略 + `AsStats` 计数面纪律。
- **回退腿（非 CLAS 厂商）**：同一份 `VisibleClusterSet` 走传统 triangles BLAS（按对象/按 LOD 段分组）。**回退腿是正确性基线，CLAS 腿是性能/VRAM 优化腿**（D-6）；两腿对同一场景 ray query 逐命中一致（任意 hit 集合 + 最近 hit 距离容差 0），命中流 digest golden 沿 G7 RayQuery 对拍体例。AMD 侧无 CLAS 等价物落地（DGF 路线观察中，R4 §5.3），回退腿在可见期内不可省略。
- **capability profile 门控**：沿 RFC-0019 §4.5 体例，CLAS 腿以语言层 capability ID（拟 `rt.clas`，符号名非 vendor extension 名）表达 requirement；profile 不提供该 capability 时构建期只产出回退腿 variant；运行期 device capability snapshot 不满足所选 profile → 装载 fail-closed，禁止运行时发现不支持后静默换腿。两条腿都是 manifest 显式 variant，选择发生在构建/装配期。**加性修订义务**：`rt.clas` 触 `spec/shader_stages.md` RXS-0311 capability ID 闭集（v1 十项冻结，条款明示加性演进走修订行，闭集外由 `capability.unknown_id` 拒）——入集必须经该条款的**加性修订行**，禁止静默扩闭集。
- **验收指标语义**（D-9）：CLAS 收益硬指标 = `vram_as_bytes` / `as_build_ms` / CPU 侧构建带宽；FPS 仅观察项（AW2 落地数据：VRAM −300MB、RTX 4060 +42%、2080 Ti +13%、4090 几乎无帧率收益，R4 §4.3 全数值复核——以 FPS 验收 CLAS 必然假绿）。静态帧零 AS 构建：降级簇 CLAS/BLAS 引用不变，只有全速更新簇触发 refit/重拼。
- **DMM 禁止线**：位移走 WPO/tessellation 既有面；任何 micromap 提案直接拒绝（D-7，NVIDIA 官方归档原文，R4 §5.4）。

### 4.4 M93/M95 — VisibleClusterSet 单源真相契约

- **载荷**：紧凑数组 = cluster stable id + LOD level + 蒙皮版本 + 变换 id；由 §4.1 DAG 的误差 cut 产生，cut 无重叠无空洞（每叶路径恰好一簇）；未驻留页父簇兜底 + 迟到页登记。
- **一份三喂**（D-8）：
  1. **光栅**：VisBuffer 64 位格式（depth30|cluster27|tri7）不变，蒙皮簇经 skin cache 顶点进 SW/HW 双路，diff=0 判据维持；
  2. **RT**：当帧 BLAS 拼装的输入数组直接由 selection 输出 memcpy/device copy 派生，**禁止独立再算一遍可见性**；
  3. **VSM**：阴影页标记用同一可见集 + 灯光视角 selection，蒙皮簇阴影包围体与相机路径同源。
- **动画分级作用于 AS 更新**：降级簇 AS 引用不变（静态帧零 AS 构建），全速簇触发 refit/重拼（[调研2][调研5]）。
- **帧末 provenance 校验（硬门负例）**：校验三方消费者输入 digest 与 `VisibleClusterSet` digest 精确一致；可见集与 BLAS 输入故意错开一簇的 variant 必须被帧末一致性校验 RED——这是单源真相防假绿核心（D1 §7 `clas_blas.merge_parity` 负例臂）。
- **帧级 evidence 计数**：`visible_clusters`、`clas_builds`、`blas_refit`、`anim_update_tier_histogram`、`vram_as_bytes`、`as_build_ms`。

### 4.5 🔒 M91 — 页格式 v2（RXPL 新 major）ABI 语义面

新增 cluster 属性（误差/包围球/骨骼元数据/CLAS 输入段）入页即触 G8 R-G8-4 反向依赖禁令，必须新 major（D-11）：

- **v2 语义**：RXPL major=2，新 schema_digest preimage；簇误差/包围球/骨骼元数据/CLAS 输入段布局、编解码往返无损。
- **v1 共存律**：M01/M04 v1 页格式 ABI（RXS-0328~0342 冻结面）**0-byte 保持**；v1/v2 页可在同一流送系统共存，G8.4 streamer 只消费冻结 ABI、迟到页降级语义不重定；禁止在实现波次中途重定 v1 ABI。
- **未知版本 fail-closed**：loader 对未知 major/篡改 schema_digest/section_digest 的页必须确定性拒绝，不得按猜测布局解析。
- 双构建确定性沿 M79 判据；物理段偏移/padding 差异进 environment/evidence，不进语义哈希。

### 4.6 M97 — Surface Cache 语义

- **离线侧**：cook 期每 mesh Card 参数化，默认上限 **12 Card/mesh**（Lumen 口径）可配；超出按表面积/视角覆盖率裁剪，裁剪策略进 cook profile。Card 图集页格式**复用 M04 版本化 ABI，禁止私定磁盘格式**。
- **运行时侧**：Card atlas 驻留管理（稀疏更新，相机相关优先级）；命中点辐射度写入 = 完整材质求值 + 直接光 + 已缓存间接光（单帧延迟反馈，Lumen 同构）。
- **缺失覆盖不变量（硬契约）**：Card 未覆盖区域**只丢能量不漏光**——采样回退到下一级追踪结果或 ambient 项，输出非负、无低于 ambient 的黑色裂缝；该语义进负例 RED 臂（故意制造 Card 空洞 → 漏光检测臂必须 RED）。
- **接口**：`RadianceTracer` 契约（G8 已冻结，SDF/ReSTIR 位已预留）之上新增「Surface Cache 命中/未命中」二级查询，对上层 GI 档位透明。

### 4.7 M98 — 四级追踪降级链

| 级 | 机制 | 覆盖范围 |
|---|---|---|
| L1 Screen Trace | 屏幕空间高度场 ray march（HZB/深度） | ~50 m 内、屏幕内，成本最低 |
| L2 SWRT | Mesh SDF（近场逐对象）+ Global SDF（远场合并），compute SDF 步进 | ~200 m；`RadianceTracer` 契约已预留 SDF 实现位 |
| L3 HWRT | Vulkan RayQuery 对 TLAS 追踪；命中着色两档：简单兜底求值（默认）/ **hit lighting** 完整材质求值（高档，需 RayTracingQualitySwitch 式材质简化开关，消费 M50 多 hit group） | 全场景、视距内 |
| L4 Far Field | 远场代理辐射度（HLOD1 式代理，~1 km 量级）；本 RFC 只定义消费接口，资产生成归几何/资产模块 | 视距外 |

降级选择契约：逐 probe/逐像素按命中距离与覆盖优先级选档；每级独立开关 + 独立 evidence 计数面（命中率/射线量/耗时）；**选择结果入 evidence，禁静默回退**。逐级强制关闭 → 能耗/画质回归必须可检测（负例 RED 臂）；L3 关材质简化开关 → 预算超限 RED。GI 各档批量均匀射线全走 RayQuery+compute；RT pipeline 仅服务 M96 与未来 hit-lighting 递归；**严禁混用同一射线流**（D2-Q9，Arm 最佳实践），队列化中间层作为唯一交汇点（SER 预留位，D2-Q13）。

### 4.8 probe 编码与 irradiance field 档位（M99/M101 语义先行）

- **共享内核**：L0~L3 共享 probe 着色与**八面体编码**内核，只换空间索引结构（D2-Q5）——档间 golden 对拍可归因到索引结构而非实现差异。
- **SPG**：屏幕空间 probe 基线 16 px/probe + 自适应细分（判据 = 深度/法线不连续性 + radiance 方差），3×3 probe 空间滤波（≈48×48 屏幕有效滤波）；在 G8 既有 1/16 均匀 probe + 3×3 滤波上增量。probe 历史/时域累积一律经 temporal 公共底座，禁私写重投影（D2-Q14，承 G8 时域纪律）。
- **Radiance Cache 双级**：屏幕空间级（复用 probe 历史）+ 世界空间 clipmap 级（绕相机分级）；**世界级须 RD-040 measured 触发举证，未举证只做屏幕级**（G9_CAPABILITY_MATRIX M99 行字面）。第一反弹采样 = BRDF×入射光 product importance sampling；关 product IS → 方差回归可检测（负例臂）。
- **IF 档位阶梯**：

| 档 | 空间索引 | 说明 |
|---|---|---|
| L0 | 屏幕空间 probe | 即 SPG 完整形态 |
| L1 | clipmap 体积 probe | DDGI 基线：八面体 irradiance 8×8 + **visibility 16×16（防漏光优先于提 irradiance 分辨率）** + 每帧轮换更新摊销；DDGI Resampling 为演进项非首版 |
| L2 | 空间哈希缓存 | SHaRC 式空间哈希 radiance 缓存，按需分级 |
| L3 | per-pixel | 全分辨率逐像素追踪，参考档/截图档（Lumen ReferenceMode 同位） |

- **AS 更新预算（硬契约）**：每档位定义强制含 AS 更新预算行，档位切换判据消费 AsManager 既有 `AsStats` 计数面（D2-Q10；「>100 k 实例时 AS 更新成本显著」阈值沿用调研轮综合口径、公开来源仅佐证趋势——G9.2 后须以本仓 measured 数据复测钉死，R5 §6.2 标注）。

### 4.9 材质时域语义面——三类速度通道消费

沿用 RFC-0019 §4.6.1 运动约定（jitter-free 输出像素坐标）：`previous_output_pixel = current_output_pixel + motion_vector`。时域管线（TAA/TSR 及 GI probe 历史）消费三类速度通道：

1. **object transform 通道**：刚体变换 MV，G8 既有语义不变；
2. **WPO 通道**：WPO current/previous 双时刻求值，RFC-0019 §4.6.1 语义 0-byte 沿用；
3. **蒙皮变形通道（新增语义）**：蒙皮簇 MV 必须由 current/previous 两时刻蒙皮姿态分别求值派生（skin cache 双时刻读）。动画更新率降级帧若无法提供 previous 姿态求值，该对象覆盖像素必须**显式标 history invalid**——这是可见降级，禁止写零 motion 伪装（RFC-0019 §4.6.1 同口径延伸至蒙皮路径）；禁止用邻域 motion 外插填补蒙皮簇 velocity。

**与 RD-041 的边界**：本节只冻结 M92 蒙皮链路消费时域底座所需的 MV 语义面；RD-041「蒙皮/WPO MV 通道资产验证」分项维持 no-go（G9_CANDIDATE_DECISIONS §3，实现留 G9.7），本节不触发该分项。

### 4.10 M96 — Path Tracer 参照器语义

- **架构**：单向 PT + NEE/MIS/RR；**megakernel 起步**，接口按 wavefront 阶段化切分（ray gen / intersect / shade / reservoir 各阶段独立可替换，为 SER 与 hit-lighting 递归演进留位）（D2-Q8）。
- **双锚对照**：正确性锚 = pbrt-v4（wavefront 架构文献基线，同场景同 spp 收敛曲线对比容差带）；工程模式 = UE Path Tracer 式「与实时管线共享场景/材质输入，不共享光照算法」——golden diff 可归因到算法层而非输入层。
- **确定性协议（硬）**：固定 seed（承 G8 `ref_tracer` PCG32 对拍模式）两次运行位级一致；逐像素 sample count 导出 + 方差/收敛曲线进 evidence；每 GI 档位定义「匹配深度」（1 / 2 / full bounce）作为对拍容差前提，1/2/full 三深度各一 golden。改 seed / 跳 RR / 关 MIS 三臂必须 RED。
- **执行路径纪律**：PT 递归走 RT pipeline（消费 M50 增量面）；与 GI 各档 RayQuery 射线流严禁混用同一射线流（§4.7）；ray 生成与命中处理间加队列化中间层（SER 预留）。
- **golden 门序（硬约束）**：**M96（`g9.p0.m96.path_tracer_reference`）未绿，M97~M101 任何画质门不得验收（任何 GI 档位画质门）**——G9.4 波内第一顺位；门序进 G9_ACCEPTANCE_MAP validator 机核，close-out 审计门序（D2-Q7 / G9_PLAN:143 / G-G9-6 / G9_CAPABILITY_MATRIX §6.4 判据 3）。

## 5. 下游 spec 条款映射（spec diff 计划，G9.2 互锁后 materialize）

条款号一律 **RXS-####（拟，G9.2 互锁开放后按 actual next_free 逐条领取）**；当前实测自由起点 RXS-0344 仅为快照非 claim。**spec 条款 PR 先于实现 PR**（硬规则 7）；每条 materialize 时至少一个 `//@ spec: RXS-实际号` 锚点，trace_matrix 全锚定。

| 条款（拟） | 标题 | 目标 spec（候选） | 测试锚定计划（每条 ≥1） |
|---|---|---|---|
| RXS-#### | RXPL major=2 段布局、新 schema preimage、v1/v2 共存律、未知版本拒绝 | `spec/geometry_pages.md` 追加新章（v1 条款 0-byte） | 编解码往返无损 golden；v1 页 0-byte 兼容；篡改 digest 的页 RED；未知 major RED |
| RXS-#### | DAG 误差度量 monotonic 不变式与簇对锁定语义 | 新建 `spec/virtual_geometry.md`（候选） | 单调性破坏 fixture → builder typed Err RED；双构建 byte golden |
| RXS-#### | 误差驱动 cut 无重叠无空洞不变式、未驻留父簇兜底律 | 同上 | 空洞（父子同选/都不选）注入 → selection 校验 RED；逐帧选中簇 id 序列 golden |
| RXS-#### | LBS 蒙皮 kernel 确定性律（palette 读取序、浮点累加序）、skin cache 布局与失效律 | 新建 `spec/skinned_geometry.md`（候选）或 `spec/shader_stages.md` | GPU/CPU Kerbl 参照逐簇对拍 golden；顶点 100% 落包围球断言 |
| RXS-#### | Kerbl 保守界公式冻结（膨胀系数定义、法向锥放大律）、降级帧放大律 | 同上 | 故意缩小包围系数 variant → 剔除错杀断言失败 RED；降级帧顶点缓冲逐位不变 |
| RXS-#### | `VisibleClusterSet` 单源真相契约（生产者/消费者、帧末一致性校验义务） | `spec/rendering_platform.md` 追加章 | 可见集与 BLAS 输入错开一簇 → provenance 校验 RED；旁路独立再算可见性 variant RED |
| RXS-#### | CLAS 构建输入页内布局、当帧拼装语义、回退腿等价性定义、capability `rt.clas` profile 门控 | `spec/rendering_platform.md` + `spec/vulkan_backend.md` + `spec/shader_stages.md`（RXS-0311 capability ID 闭集加性修订行） | CLAS/回退腿逐命中一致 golden（距离容差 0）；snapshot 不满足 → 装载 RED；静态帧非零 `clas_builds`/`blas_refit` RED |
| RXS-#### | Card 参数化 cook profile schema（≤12/mesh 可配）、图集复用 M04 ABI、缺失覆盖只丢能量不漏光回退语义 | `spec/rendering_platform.md` 或资产管线 spec 修订 | Card 空洞 → 漏光检测臂 RED（输出非负、无黑色裂缝）；同资产双构建 hash golden |
| RXS-#### | 追踪降级链 L1~L4 选档契约、逐档计数面、禁静默回退、hit lighting 材质简化开关语义 | `spec/rendering_platform.md` + `spec/shader_stages.md` | 逐级强制关闭 → 回归可检测 RED；关材质简化开关 → 预算超限 RED |
| RXS-#### | probe 着色/八面体编码共享内核、SPG 自适应细分判据、IF L0~L3 档位定义与 AS 更新预算行格式、DDGI visibility 16×16 | `spec/rendering_platform.md` | 档间共享内核同一函数实例断言；visibility 降采样 → 漏光检测 RED；超 AS 预算 → 强制降档臂 |
| RXS-#### | golden 确定性协议：固定 seed、逐像素 sample count 导出、方差/收敛曲线 evidence 字段、匹配深度表 | conformance 协议章 | 改 seed / 跳 RR / 关 MIS 三臂 RED；pbrt-v4 容差带 golden；两次运行位级一致 |
| RXS-#### | 蒙皮变形 MV 双时刻求值、降级帧 history invalid 义务 | `spec/rendering_platform.md`（时域章追加） | 蒙皮两帧解析运动 golden；零 motion 伪装 variant → provenance/history 断言 RED |

- **错误码策略**：G9.1 零 RX claim。G9.2 后优先复用已冻结的接口/资源/后端不支持类别；只有实现证明出现新的、用户可行动、可独立到达的诊断类别时，才按当时各段 `next_free` 只追加并同步 en/zh message key。builder/装配错误优先用 typed `Err`，不为每个状态预造 RX。

## 6. feature gate / tracking / 实现序（G9.2 互锁后生效）

### 6.1 Gate 命名空间（G9.1 冻结，三方逐字一致）

| 覆盖面 | canonical gate key | smoke 脚本 | evidence schema 目标路径（只冻结路径） |
|---|---|---|---|
| M90 DAG 深化 | `g9.p0.m90.cluster_dag_deepening` | `ci/g9_cluster_dag_deepening_smoke.py` | `milestones/g9/g9_m90_cluster_dag_deepening_evidence_schema.json` |
| M91 页格式 v2 | `g9.p0.m91.page_format_v2_abi` | `ci/g9_page_format_v2_abi_smoke.py` | `milestones/g9/g9_m91_page_format_v2_abi_evidence_schema.json` |
| M93 可见集 | `g9.p0.m93.visible_cluster_set` | `ci/g9_visible_cluster_set_smoke.py` | `milestones/g9/g9_m93_visible_cluster_set_evidence_schema.json` |
| M94 CLAS 合流 | `g9.p0.m94.clas_rt_convergence` | `ci/g9_clas_rt_convergence_smoke.py` | `milestones/g9/g9_m94_clas_rt_convergence_evidence_schema.json` |
| M95 单源真相 | `g9.p0.m95.single_source_truth` | `ci/g9_single_source_truth_smoke.py` | `milestones/g9/g9_m95_single_source_truth_evidence_schema.json` |
| M96 PT 参照器 | `g9.p0.m96.path_tracer_reference` | `ci/g9_path_tracer_reference_smoke.py` | `milestones/g9/g9_m96_path_tracer_reference_evidence_schema.json` |
| M97 Surface Cache | `g9.p0.m97.surface_cache` | `ci/g9_surface_cache_smoke.py` | `milestones/g9/g9_m97_surface_cache_evidence_schema.json` |
| M98 降级链 | `g9.p0.m98.tracing_fallback_chain` | `ci/g9_tracing_fallback_chain_smoke.py` | `milestones/g9/g9_m98_tracing_fallback_chain_evidence_schema.json` |

M92/M99/M100/M101 为 P1 面，gate key 不在本 P0 命名空间，由 G9.1 决策表硬化后另立；本 RFC 只冻结其语义面。**门序硬约束**：M96 未绿 → M97~M101 任何画质门不得验收（任何 GI 档位画质门），进 ACCEPTANCE_MAP validator 机核。

### 6.2 真实 RED/GREEN

| 面 | RED（必须先可复现） | GREEN（不得以较弱见证替代） |
|---|---|---|
| M90 | 单调性破坏 fixture；双构建字节漂移 | ≥3 真实资产（含 ≥1 骨骼资产）全量构建 + 误差表 byte golden |
| M91 | 篡改 digest 的页；未知 major 的页 | v2 页经 streamer 真机消费（含迟到页路径）+ v1 0-byte 兼容 golden |
| M93 | cut 空洞注入（父子同选/都不选） | 确定性相机路径逐帧选中簇 id 序列 golden + 迟到页父簇兜底 |
| M94 | 可见集与 BLAS 输入错开一簇；静态帧非零 AS 构建 | 4070 Ti 双腿对拍逐命中一致 + VRAM/构建耗时 evidence |
| M95 | RT/VSM 独立再算可见性的旁路 variant | 蒙皮簇 VisBuffer SW/HW diff=0 + VSM 页标记同源断言 |
| M96 | 改 seed / 跳 RR / 关 MIS | 固定 seed 位级一致 + pbrt-v4 三匹配深度容差带 golden |
| M97 | Card 空洞漏光（负值/黑色裂缝） | 缺失覆盖只丢能量断言 + cook 双构建 hash golden |
| M98 | 逐级强制关闭不可检测；关材质简化开关 | 逐档命中率/耗时计数非空 + 各档对 M96 1-bounce golden |

### 6.3 栈式实现序

1. **PR-Gate**：G9.2 互锁 validator 全绿；重读 ledger actual `next_free`。红即停止。
2. **PR-Spec**：按 §5 materialize 实际 RXS 与 RED 语料；条款 commit 先于实现 commit。
3. **PR-PagesV2**：M91 builder/loader 双侧编解码与 v1 共存；纯 host deterministic RED/GREEN 先绿。
4. **PR-Dag**：M90 误差度量/簇对锁定/蒙皮元数据/CLAS 输入烘焙。
5. **PR-Selection**：M93 误差 cut + 流送联动 + VisibleClusterSet 载荷。
6. **PR-Skinning**：蒙皮 kernel + 保守包围体 + 分级更新率（M92 语义，P1 gate 另立）。
7. **PR-Clas**：M94 双腿 + AsManager 扩展 + capability 门控；随后 M95 单源真相集成与帧末 provenance 校验。
8. **PR-PathTracer**：M96 参照器（G9.4 波内第一顺位，golden 门先行）。
9. **PR-GI**：M97 Surface Cache → M98 降级链 → M99/M101 probe/IF 档位（P1）。
10. **PR-Evidence**：evidence schema 落盘、RTX 4070 Ti validation-on device run，禁止 YAML-only 与 host substitution。

## 7. 备选方案

| 方案 | 裁决 | 理由 |
|---|---|---|
| CPU 蒙皮喂静态 cluster（UE5.5 权宜路线） | 否决（D-1） | 击穿 cluster 包围体/误差度量、吞吐低、有视角 LOD 瑕疵（机制归因引自 D1 草案转述，R4 U3）；Kerbl 2021 有严格解 |
| vendoring nv_lod_cluster_builder | 否决（D-10） | 许可风险与确定性双构建纪律；算法公开，自研同源实现 |
| CLAS 单腿、无回退 | 否决（D-6） | `VK_NV_cluster_acceleration_structure` 当前 NV-only；AMD 无等价物，跨厂商验收必须有传统 BLAS 正确性基线 |
| 运行时探测 capability 后静默选腿 | 否决 | 违 RFC-0019 §4.5 profile 纪律；选择必须发生在构建/装配期，snapshot 不满足 fail-closed |
| 以 FPS 为 CLAS 验收主指标 | 否决（D-9） | AW2 4090 零帧率收益；收益在 VRAM/CPU/构建带宽约束路径，FPS 验收必然假绿 |
| 自研 GI 架构替代 Lumen 全链路 | 否决（D2-Q1） | Lumen 是唯一公开完整工程化细节的实时 GI 架构；UE5 级等价是既定验收基线 |
| M96 直接 wavefront 实现 | 否决（D2-Q8） | 参照器规模受控时 megakernel 工程更简单；接口按 wavefront 阶段化切分已留演进位 |
| Card 图集私定磁盘格式 | 否决 | 复用 M04 版本化 ABI；资产管线纪律禁止 D2 私定 |
| GI 各档复用路径跳验证射线 | 否决（D2-Q4） | GI-1.0 教训：跳验证引入系统性变暗偏置、随场景复杂度放大、事后不可归因；偏置门进验收 |

## 8. 不做（范围红线）

- **DMM / displacement micromap 永久禁止**（D-7；NVIDIA 官方归档、被 Mega Geometry 取代）：任何 micromap 字样提案进 RFC 即一票否决（G9_CAPABILITY_MATRIX §7 登记）。
- **NRC / 神经 radiance cache**：观察项，不进本章（GPU tensor/神经网络属既有 SG 禁止面，训练基建超范围；D2 §2.2 out）。
- **帧生成 FG/MFG（M26）**：不进 G9（RD-041 分项「独立层另判」字面）。
- **CPU 蒙皮权宜路线**：永久否决（§7），不得以「快速打通」名义复活。
- **SVT/RVT/sampler feedback 依赖**：M40/41/42 G8 no-go 维持；本章任何面不得依赖 SVT。
- Morph target 虚拟化（DAG 化）：非虚拟化旁路（D-2），本章不做 DAG 化。
- SER 实现：仅队列化中间层接口预留（D2-Q13）；M108 语言原语归 RFC-0023 章，本章不重复定义。
- RD-034 DXIL RT 腿：blocked 维持；本章全部 RT 走 Vulkan 主腿。
- 多灯直接光（M100 MegaLights/ReSTIR）：语义归 D2 后续 RFC/修订行，本章不冻结（RD-040 触发举证先行）。
- 多 GPU / WebGPU / 编辑器 GUI / USD ingest：承 G8 out-of-scope 口径。
- 不改 G5 `Barrier` EB 三轴、`GpuScene`、`MaterialClosure` 32B、VisBuffer 位格式、M04 v1 页格式 ABI、RXS-0239 单 queue 全序字面、RXS-0311 capability ID 闭集（v1 十项冻结；`rt.clas` 入集须经该条款加性修订行，禁静默扩）；不改 RFC-0018/RFC-0019 已批准语义（除 §9 Q11 显式修订行机制）。
- 不在 G9.1 改 `src/`、`spec/`、`conformance/`、`.github/workflows/`；不 materialize 数字 CI 步骤；不预建空 schema 壳/空脚本占位；不领取 RXS/RD/U/RX 共享在途号；Draft/Approved 状态均不构成实现许可。

## 9. 未决问题 / 关键裁决

下表是本 Draft 的明确裁决提案；Agent Approved 时逐行冻结。若对抗性评审推翻任一项，必须先改正文和本表，再批准。

| ID | 问题 | Draft 裁决 |
|---|---|---|
| Q1 | 误差度量形式与单调不变量 | 每簇 `parent_error`/`cluster_error`，DAG 每边 `parent_error ≥ cluster_error`；builder 校验 typed Err fail-closed |
| Q2 | 蒙皮路线 | GPU cluster 感知蒙皮 + Kerbl 保守包围体离线预计算；CPU 权宜路线否决；Morph 非虚拟化旁路 |
| Q3 | CLAS 厂商分层 | NV CLAS 主腿 + 传统 BLAS 回退腿（正确性基线）；capability `rt.clas` profile 门控，禁静默换腿；抽象按 DXR Part 2 厂商中立设计预留；`rt.clas` 入 RXS-0311 capability ID 闭集须经该条款加性修订行（禁静默扩） |
| Q4 | 页格式演进 | RXPL 新 major（v2）；M04 v1 ABI 0-byte 共存；未知版本/篡改 digest fail-closed；禁中途重定 v1 |
| Q5 | 单源真相载体 | `VisibleClusterSet` 一份三喂（光栅/BLAS/VSM）+ 帧末 provenance 校验负例硬门；动画分级作用于 AS 更新，静态帧零构建 |
| Q6 | 追踪降级链 | L1→L4 逐档独立开关 + 独立计数面；选档结果入 evidence 禁静默；RayQuery 与 RT pipeline 射线流严禁混用 |
| Q7 | probe/IF 内核组织 | L0~L3 共享 probe 着色与八面体编码内核、只换空间索引；DDGI 档 visibility 16×16 优先于 irradiance 8×8 |
| Q8 | 世界 clipmap 级 Radiance Cache | 须 RD-040 measured 触发举证；未举证只做屏幕级 |
| Q9 | M96 架构与确定性 | megakernel 起步 + wavefront 阶段化接口；固定 seed 位级一致 + sample count 导出 + 方差/收敛曲线；pbrt-v4 对照 |
| Q10 | golden 门序 | M96 未绿，M97~M101 任何画质门不得验收（任何 GI 档位画质门）；门序进 ACCEPTANCE_MAP validator 机核，close-out 审计 |
| Q11 | **M28 多层 closure 条件扩展** | M28 若改判 go/strategic_override 而条件扩展，凡触及 `MaterialClosure` 32B 冻结面（G5 冻结，RFC-0019 §4.7 记 0-byte 保持）**必须经本 RFC 或后续 RFC 的显式修订行**（原句→修订后句逐条列出 + golden 零漂移证明计划），**禁止静默扩**；多层 graph 维持编译/资产 IR 定位，不消费 32B 已预留拓扑字段位 |
| Q12 | RFC Approved 是否解锁实现 | 不；G9.2 互锁是独立硬门，不得以 RFC 状态替代机器事实 |

## 9.1 对抗性评审记录（10 §3 / §7 · D-409）

**第 1 轮评审完成，结论「有条件通过」；全部条件（F-1~F-5 正文实改 + F-6 移交登记）落实后翻 Agent Approved。**

| 字段 | 值 |
|---|---|
| 评审者 provenance | `Assisted-by: Kimi Code CLI (Kimi) rfc0022-adversarial-reviewer`（独立实例，与起草无共享上下文；**≠ 起草 Provenance** `Assisted-by: Kimi Code CLI (Kimi) rfc0022-drafter`） |
| 评审轮次 | 第 1 轮，2026-08-09 |

**Findings 与 disposition**（每条 disposition：采纳并修 §X ／ 驳回 + 理由 ／ 移交）：

| # | Finding | 严重度 | Disposition |
|---|---|---|---|
| F-1 | 拟议 `rt.clas` 触 `spec/shader_stages.md` RXS-0311 capability ID 闭集（v1 十项冻结，条款明示加性演进走修订行，闭集外由 `capability.unknown_id` 拒），RFC 全文未声明加性修订义务 | **major** | **采纳，正文实改**：§4.3 capability 门控条末补「加性修订义务」（入集须经 RXS-0311 加性修订行，禁静默扩闭集）；§5 CLAS 行目标 spec 补 `spec/shader_stages.md`（RXS-0311 加性修订行）；§8 冻结面清单补 RXS-0311 闭集；§9 Q3 补同义务 |
| F-2 | golden 门序五处（§1/§3.3/§4.10/§6.1/§9 Q10）写「任何 GI 画质门不得验收」，宽于契约字面「M97~M101 任何画质门不得验收」（G9_PLAN:143 / G-G9-6 / MAP M96 行；矩阵 §6.4 原文为「任何 GI 档位画质门」） | minor | **采纳，正文实改**：五处统一为「M96 未绿，M97~M101 任何画质门不得验收（任何 GI 档位画质门）」逐字对齐；§4.10 锚点补 G9_PLAN:143 / G-G9-6 |
| F-3 | §4.9 蒙皮 MV 新语义未声明与 RD-041「蒙皮/WPO MV 通道资产验证」分项 no-go（G9_CANDIDATE_DECISIONS §3，实现留 G9.7）的关系 | minor | **采纳，正文实改**：§4.9 末尾加「与 RD-041 的边界」段——本节只冻结 M92 蒙皮链路消费时域底座所需 MV 语义面，不触发 RD-041 该分项 |
| F-4 | §7 首行 CPU 蒙皮否决理由丢失 R4 U3 转述标注 | minor | **采纳，正文实改**：理由栏补「（机制归因引自 D1 草案转述，R4 U3）」 |
| F-5 | §4.5「M04 v1 ABI（RXS-0328~0342 冻结面）」归属不精确（RXS-0328~0331 为 M01 逻辑页 ABI） | minor | **采纳，正文实改**：改为「M01/M04 v1 页格式 ABI（RXS-0328~0342 冻结面）」 |
| F-6 | M92 判 go 但无硬门落点（跨文档问题，非本 RFC 正文缺陷） | minor | **移交 G9.1 治理波**：父代理将在 G9_CANDIDATE_DECISIONS §8 总表 M92 行补「验收并入 M93/M95 P0 判据字面」；本 RFC 无需实质改动，在此登记留痕 |

**偏差说明（如实登记）**：首选跨工具评审者在本环境不可得，本轮评审由同工具族独立实例执行（评审 provenance ≠ 起草 provenance），按 RFC-0015 §9.1 / number_ledger v1.29 先例如实登记，不构成对 D-409 字面之外效力的声称。

## 10. 稳定化与 provenance

- **特性生命周期**（10 §5）：RFC Agent Approved 只是语义评审完成；随后仍需 G9.2 互锁 → spec-first/RED → gated implementation → tracking evidence → 至少两个里程碑无重大语义修订 → stabilization report → FCP-lite。
- **稳定面候选**：RXPL v2 schema/digest 规则、DAG 误差单调不变式、`VisibleClusterSet` 契约与 provenance 规则、降级链选档契约、golden 确定性协议字段；是否 stable 由未来 stabilization report 裁决。
- **明确非 stable**：CLAS/BLAS 物理字节与 device address、driver 构建耗时数值、页段物理偏移/padding、SPG/IF 调参阈值、蒙皮 kernel 具体调度。
- **Provenance**：`Assisted-by: Kimi Code CLI (Kimi) rfc0022-drafter`。agent 自主批准后回填记录。

## 11. 规范与实现依据

- 仓库内：[RFC-0019](0019-rendering-platform.md)（§4.5 capability/profile 体例、§4.6 时域运动约定、§4.7 M28 边界）；[G9_CAPABILITY_MATRIX](../milestones/g9/G9_CAPABILITY_MATRIX.md) §1/§2（M90~M98 行、§6.4 成功判据草案、§7 门控维持登记）；G9.0 不可变 ref `1d9460a1` 下 [D1 设计草案](../milestones/g9/design/G9_D1_VIRTUAL_GEOMETRY_RT.md) 与 [D2 设计草案](../milestones/g9/design/G9_D2_GI_LIGHTING.md)；调研 [R4](../milestones/g9/research/R4_VIRTUAL_GEOMETRY_RT.md) / [R5](../milestones/g9/research/R5_GI_LIGHTING.md)。
- 外部（经 R4/R5 复核的一手来源）：SIGGRAPH 2021 Nanite 课程（Karis et al.）；Unterguggenberger/Kerbl et al. 2021（CGF 40(7)，保守 meshlet 包围）；`VK_NV_cluster_acceleration_structure` refpage（multi-indirect / Cluster Template）；DXR Functional Spec Part 2（厂商中立 CLAS）；Remedy Northlight GDC 2024 + AW2 1.2.8 落地数据（Digital Foundry 复核）；SIGGRAPH 2022 Lumen 讲义（Wright et al.）；Majercik et al. JCGT 2019/2021（DDGI）；RTXGI 2.0/SHaRC（GDC 2024）；GI-1.0（arXiv:2310.19855，偏置教训）；pbrt-v4《Wavefront Rendering on GPUs》；Arm GPU Best Practices（RayQuery 甜区与混用禁令）。
- 口径标注：「边塌缩/簇对锁定」术语（R4 U4）、UE5.5 权宜路线机制归因（R4 U3）、Northlight 两级遮挡/bone shader 细节（R4 U1/U2）、「>100 k 实例」AS 阈值（R5 §6.2）为草案转述/调研轮综合口径，引用时保留该标注属性，不冒充逐字复核事实。

---

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| Draft v0.1 | 2026-08-09 | AI 起草初版（G9.1 治理波）：冻结 M90/M91/M93/M94/M95/M96/M97/M98 语义面 + M92/M99/M101 语义先行；8 个 P0 key/脚本/evidence schema 路径三方逐字一致冻结（不预建文件）；§5 spec 映射计划零数字 claim；§9.1 留给独立 provenance 对抗性评审（第 1 轮待进行）；零 `src/`、`spec/`、`conformance/`、workflows 改动 | Full RFC（Draft） |
| v1.0 | 2026-08-09 | **Agent Approved**：D-409 第 1 轮对抗性评审（评审 provenance `Kimi Code CLI (Kimi) rfc0022-adversarial-reviewer` 独立实例 ≠ 起草 `rfc0022-drafter`；同工具族偏差说明见 §9.1 末尾）结论「有条件通过」，6 findings（1 major + 5 minor）全部 disposition 后翻批准。正文实改要点：F-1 补 RXS-0311 capability ID 闭集加性修订义务（§4.3/§5/§8/§9 Q3 四处）；F-2 golden 门序五处统一为「M97~M101 任何画质门不得验收（任何 GI 档位画质门）」逐字对齐契约字面；F-3 §4.9 补 RD-041 蒙皮/WPO MV 分项边界声明；F-4 §7 补 R4 U3 转述标注；F-5 §4.5 改「M01/M04 v1 页格式 ABI」；F-6 跨文档移交 G9.1 治理波（G9_CANDIDATE_DECISIONS §8 M92 行）登记 §9.1。零编号 claim；批准不解锁实现，G9.2 互锁仍为独立硬门。 | Full RFC（Agent Approved） |
