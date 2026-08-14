# G9_CANDIDATE_DECISIONS — RD-039/040/041/044 候选分项与 G8.7 承接锚裁决

> **状态**：G9.1 governance-only 裁决记录（2026-08-09）。G9 已立项（用户立项指令 2026-08-09 + agent 依 10 §7/P-13/D-406 v2.0 完全自主签署立项裁决）；**G9.2+ implementation 仍 blocked**——解除实现互锁须经 [G9_PLAN](G9_PLAN.md) §5 G9.1 治理门全绿 + G9.2 实现门三条件（G9.1 治理门全绿且 interlock validator 输出 READY、编号按实测 `next_free` 重新领取、互锁全绿后 spec 条款 PR 先于实现 PR）。
> **事实源**：[G9_PLAN](G9_PLAN.md) v1.1（§1.0 十条承接锚 / §1.1 追加输入 / §1.2 条件型 RD 纪律 / §2 波次与退出门判据草案 / §5 立项待裁决表）· [G9_CAPABILITY_MATRIX](G9_CAPABILITY_MATRIX.md) v1.0（M90~M127 §1~§7 全部行）· [G8_CANDIDATE_DECISIONS](../g8/G8_CANDIDATE_DECISIONS.md) v1.2（§10 矩阵 P1 未判行字面）· [G8_P2_DECISIONS](../g8/G8_P2_DECISIONS.md) v1.0（十条 defer 承接锚字面）· [`registry/deferred.json`](../../registry/deferred.json)（RD-039/040/041/042/043/044 `backfill_condition` 字面）· `registry/number_ledger.json`（2026-08-09 实测 namespaces.RFC `next_free=22`：RFC-0022=虚拟几何与 GI 语义、RFC-0023=GPU-driven 提交与着色系统、RFC-0024=物理平台修订（RFC-0021 修订）；登记由立项治理统一落）。
> **证据口径**：「证据路径」只列当前在树事实或作出治理裁决的事实源；「证明 workload」是该分项实施后必须产出的机器证据，未产出前不得把 `go` 写成已交付；`最终期望状态` 是分项 close-out 目标，不是本表落盘时的 registry 状态。P0 key 命名空间（`g9.p0.m<##>.<slug>` / `ci/g9_<slug>_smoke.py --gate <key>` / `milestones/g9/g9_m<##>_<slug>_evidence_schema.json`）15 行已冻结，本表只引用，不 materialize CI 步骤、不预建 schema 壳。
> **纪律**：① 任何 defer 出 G9 的分项必须带承接锚（机核进 G9.7 validator，同构 `ci/g8_p2_decisions_check.py`）；② 条件型 RD 触发条件不得被「UE5 目标」静默改写——G9 立项书作为触发证据的分项仅限 G9_PLAN §1.0 已声明者（RD-039 M06/M09），且 deferred history 只追加；③ 本表缺行阻断 G9.2（§8）。

## 0. 总裁决与六项立项裁决

逐分项共 **47 行**：**go 22 行**（含条件制 go：M99 屏幕级、M100 低档、M123 判档制；含研究子轨登记 M127）、**strategic_override 2 行**（RD-039/M61→M109、RD-040/M52→M108）、**no-go 22 行**（含 M65b 条件制维持、RD-034 blocked 维持、RD-042/043 观察维持）、**defer 1 行**（Safe GPU Operator Platform→G10+）。

本表落盘 G9_PLAN §5 立项待裁决表六项裁决（已定案，直接采用）：

1. **立项时机与工作树**：现在立项；G9.0 不可变 ref = `1d9460a1`；G8 遗留 staged 工作树集合「带未提交项立项」，保持 staged 待独立提交，不混入 G9.1 提交。
2. **Safe GPU Operator Platform**：defer 至 G10+，承接锚「G10+ Safe GPU Operator Platform 独立期」（§5 行）。
3. **M52 SER / M61 mesh shader 改判接受**：各记 strategic_override（M52→M108 语言层原语 + capability `rt.ser` 可选；M61→M109 可选 geometry pipeline，顺序硬约束 = 排在 meshlet 页格式 v2 与 GPU-driven 剔除链路之后），deferred.json history 只追加 override，禁静默改判（§1.1/§2.1 登记文本）。
4. **G9 规模**：五模块全进（不分包）。
5. **神经变形**：维持 `rfcs/0021:122` 无归属留痕，不新设 RD；M127 研究子轨，无主线门；边界由 RFC-0024 冻结（§5 行）。
6. **G8.8b 同日放行先例**：继承（8a full-run 先行完成后允许同日进 8b close-out）。

## 1. RD-039 — 虚拟化几何 P3+

| 锚/RD-id | 分项名 | G9 M## | 原 backfill 字面 | 证明 workload | 证据路径 | 决策 | 承接波次 | 承接锚 | 最终期望状态 |
|---|---|---|---|---|---|---|---|---|---|
| RD-039 | Nanite Foliage | M90/M92/M93 | “Foliage/骨骼在动态资产面出现时(联动 RD-040 蒙皮 MV)” | 真实植被资产管线上 GPU cluster 感知蒙皮（LBS kernel/保守包围球/距离分级更新率）+ 蒙皮元数据入 DAG（M90）+ 蒙皮簇 VisBuffer SW/HW diff=0（M93/M95 门） | `registry/deferred.json` RD-039；`G9_PLAN.md` §1.0/§1.2（G9 立项书即触发证据，history 只追加）；`G9_CAPABILITY_MATRIX.md` M90/M92/M93；用户立项指令（2026-08-09） | go | G9.2（M90）→G9.3（M92/M93） | G9+ 虚拟几何评估窗（按锚承接） | closed（仅本分项；RD-039 总体维持 open） |
| RD-039 | 骨骼虚拟几何 / Skinning | M90/M92/M93 | “Foliage/骨骼在动态资产面出现时(联动 RD-040 蒙皮 MV)” | 真实骨骼资产 + GPU 蒙皮全链（LBS/包围体/分级动画更新率）+ Morph 非虚拟化旁路面；UE5.5 CPU 蒙皮权宜路线拒绝（D1 D-1） | `registry/deferred.json` RD-039；`G9_PLAN.md` §1.0/§1.2；`G9_CAPABILITY_MATRIX.md` M92 | go | G9.2（M90）→G9.3（M92/M93） | G9+ 虚拟几何评估窗（按锚承接） | closed（仅本分项；RD-039 总体维持 open） |
| RD-039 | Mega Geometry 簇级 BLAS | M90/M94 | “Mega Geometry 在 RT 与虚拟几何合流需求出现时” | 簇级 CLAS 当帧 multi-indirect 拼装 + Cluster Template 实例化；NV 主腿与传统 BLAS 回退腿 ray query 逐命中一致；静态帧零 AS 构建 | `registry/deferred.json` RD-039；`G9_PLAN.md` §1.0/§1.2（立项书即触发证据，history 只追加）；`G9_CAPABILITY_MATRIX.md` M90/M94；`G9_PLAN.md` §2.9 M94 行 | go | G9.2（M90）→G9.3（M94） | G9+ RT×Nanite 合流窗（按锚承接） | closed（仅本分项；RD-039 总体维持 open） |
| RD-039 | mesh shader 第三光栅路径 | M109 | “mesh shader 在多厂商扩展行为收敛且性能差有 measured 证据时评估” | mesh shader 可选 geometry pipeline：cluster 流入口 + VS 光栅唯一 fallback + `mesh.task` capability 选择律 RED/GREEN；顺序硬约束核验（实现痕迹不得早于约束前置） | `registry/deferred.json` RD-039；`G9_CAPABILITY_MATRIX.md` M109 改判提案；§1.1 override 登记文本；deferred history 只追加 override 行 | strategic_override | G9.3+（P2 可选；顺序硬约束 = 排在 meshlet 页格式 v2（M91，G9.2）与 GPU-driven 剔除链路（G9.3）之后） | —（override 登记，非 G8.7 锚） | closed（仅 M109 可选路径分项；RD-039 总体维持 open） |
| RD-039 | HZB 两阶段遮挡剔除 | — | “HZB 两阶段在剔除效率成为 measured 瓶颈时优先” | 动态场景 cull 计数器证明剔除效率为 measured 瓶颈 | `registry/deferred.json` RD-039；截至基准日无 measured artifact | no-go | G9.7 穷举 | —（留 G9.7） | open-留 G9.7 |
| RD-039 | cluster 流送 P4 运行时（父页引用/超显存） | — | “cluster 流送 P4 在场景规模超显存时” | 场景规模真实超过显存；按需驻留、父页 fallback、迟到页路径与预算计数器齐备 | `registry/deferred.json` RD-039；截至基准日无超显存场景证据 | no-go | G9.7 穷举 | —（留 G9.7） | open-留 G9.7 |
| RD-039 | 曲面细分位移 | — | `backfill_condition` 未单列；title 字面“曲面细分位移” | 真实位移资产清单 + bounds/velocity 语义 + 画质/性能对照 | `registry/deferred.json` RD-039；无独立触发条件与证据 | no-go | G9.7 穷举 | —（留 G9.7） | open-留 G9.7 |
| RD-039 | Assemblies 全功能（嵌套/跨 assembly 去重） | — | `backfill_condition` 未单列；title 字面“Assemblies 全功能(嵌套+跨 assembly 去重)” | 真实 assemblies 资产 + 嵌套/去重确定性与 residency 证据 | `registry/deferred.json` RD-039；无独立触发条件与证据 | no-go | G9.7 穷举 | —（留 G9.7） | open-留 G9.7 |

### 1.1 M61 strategic_override 登记文本

> **RD-039/M61 strategic_override（agent，2026-08-09）**：以「UE5 级正式建造期 D3 GPU-driven 提交需要可选 mesh shader geometry pipeline 作为 cluster 流送入口，且 RD-039 双条件中『多厂商扩展行为收敛』按公开证据实质成立、『性能差 measured 证据』可在本机 RTX 4070 Ti 补齐」（`G9_CAPABILITY_MATRIX.md` M109 行字面）为战略依据，覆盖 RD-039 原 backfill「mesh shader 在多厂商扩展行为收敛且性能差有 measured 证据时评估」的等待条件。覆盖范围仅为 M109 可选 geometry pipeline：cluster 流入口、VS 光栅唯一 fallback、`mesh.task` capability 选择律；**P2 可选**——不承诺性能收益、不构成主线硬门、不替代 SW/HW 既有光栅路径。**顺序硬约束**：排在 meshlet 页格式 v2（M91，G9.2）与 GPU-driven 剔除链路（G9.3）之后，此前不得出现其实现痕迹（G9_PLAN §2 G9.3「M108/M109 仅在 §5 裁决 go 后落本波或后续，未裁决前不得出现其实现痕迹」同口径）。本 override 经 `registry/deferred.json` history **只追加**登记，不改写 RD-039 `backfill_condition` 原文、不替代 HZB/cluster 流送 P4/曲面细分位移/Assemblies 等其余分项判档、不改变 RD-039 总体 open 状态；task shader（M62）不开放维持不动。

## 2. RD-040 — 光照 P3+

| 锚/RD-id | 分项名 | G9 M## | 原 backfill 字面 | 证明 workload | 证据路径 | 决策 | 承接波次 | 承接锚 | 最终期望状态 |
|---|---|---|---|---|---|---|---|---|---|
| RD-040 | SER / hit-object 重排 | M108 | `backfill_condition` 未单列；title 字面“SER 与 OMM” | HitObject 类型面 + `reorderThread`/`hitObjectTraceRay`/`hitObjectInvoke` 语言原语 + capability `rt.ser` 可选（无 capability 设备 fail-closed 降级）+ 材质 flags coherence hint 位段预留；RED/GREEN 契约，不承诺性能收益 | `registry/deferred.json` RD-040；`G9_CAPABILITY_MATRIX.md` M108 改判提案；§2.1 override 登记文本；deferred history 只追加 override 行 | strategic_override | G9.3+（P2 可选；语言层语义面经 RFC-0023 冻结） | —（override 登记，非 G8.7 锚） | closed（仅 M108 分项；RD-040 总体维持 open） |
| RD-040 | 世界空间辐射缓存 | M99 | “世界辐射缓存在屏幕探针远场缺失成为画质 measured 问题时” | 条件制：未 measured 举证只做屏幕级（SPG 自适应细分 + Radiance Cache 屏幕级 + product importance sampling）；世界 clipmap 级须远场画质 measured 举证前置，未举证维持 open 不充绿 | `registry/deferred.json` RD-040；`G9_CAPABILITY_MATRIX.md` M99（「世界 clipmap 级须 measured 触发举证，未举证只做屏幕级」）；无 measured 画质证据 | go（条件制） | G9.4 | —（RD-040 条件分项重判档） | closed（屏幕级）/ open（世界 clipmap 级，待 measured 举证） |
| RD-040 | ReSTIR GI/DI 与 MegaLights（含 M22 海量灯阴影统一接口） | M100 | “ReSTIR/MegaLights 在多灯场景需求出现时” | 条件制：低档 MegaLights 式固定随机选灯为默认 go；高档 ReSTIR reservoir 须附多灯 workload 证据，不足则只做低档、高档维持 open-留档；验证射线零跳过硬契约；海量灯阴影统一接口随动 | `registry/deferred.json` RD-040；`G9_CAPABILITY_MATRIX.md` M100（「立项时须附多灯 workload 证据；不足则 M15 维持 open-留档、只做低档」）；`G8_P2_DECISIONS.md` M15/M22 行 | go（条件制） | G9.4 | —（RD-040 条件分项重判档） | closed（低档默认）/ open-留档（高档 ReSTIR，待多灯 workload 证据） |
| RD-040 | SMRT 软阴影完整版 | — | “SMRT 在 VSM device 化(RD-038)后可独立 Mini 兑现(采样端沿光线多采样)” | VSM depth/sample device 门（已由 RD-038 closed 兑现）+ SMRT 多采样软阴影对拍 | `registry/deferred.json` RD-040/RD-038（closed）；前置条件虽成立，但无 G9 波次需求方与独立 workload 举证 | no-go | G9.7 穷举 | —（留 G9.7） | open-留 G9.7 |
| RD-040 | 自适应探针 | — | `backfill_condition` 未单列；title 字面“自适应探针(GI P3)” | 探针密度/预算自适应相对固定屏幕探针的画质与性能 measured 对照 | `registry/deferred.json` RD-040；无独立触发条件与证据；M99 SPG 自适应细分为独立 G9 分项，不冒充本行 | no-go | G9.7 穷举 | —（留 G9.7） | open-留 G9.7 |
| RD-040 | SDF 软追踪 | — | `backfill_condition` 未单列；title 字面“SDF 软追踪(GI P4)” | mesh/global DF builder、更新预算与 tiered tracing 画质/性能证据 | `registry/deferred.json` RD-040；无独立触发条件与证据；M98 L2 SWRT（Mesh/Global SDF）为降级链档位，不构成本分项完整兑现 | no-go | G9.7 穷举 | —（留 G9.7） | open-留 G9.7 |
| RD-040 | Opacity Micromap（OMM） | — | `backfill_condition` 未单列；title 字面“SER 与 OMM” | alpha-tested foliage 资产 + OMM build/BLAS attach/baker 正确性与 measured 收益 | `registry/deferred.json` RD-040；无真实资产/收益证据；OMM baker 不抢跑；DMM 永久禁止（矩阵 §7）与本行无涉 | no-go | G9.7 穷举 | —（留 G9.7） | open-留 G9.7 |
| RD-040 | NRD 类 vendor 降噪接入 | — | “NRD/vendor 降噪经 UpscaleBackend 同构输入契约接入(MV/深度/法线),接入时不改 temporal 底座” | vendor denoiser adapter 的 MV/深度/法线 ABI 契约测试 + 画质/稳定性对照；temporal 底座 0-byte | `registry/deferred.json` RD-040；无 vendor 降噪需求证据 | no-go | G9.7 穷举 | —（留 G9.7） | open-留 G9.7 |

### 2.1 M52 strategic_override 登记文本

> **RD-040/M52 strategic_override（agent，2026-08-09）**：以「UE5 级正式建造期 D3 着色系统需要语言层 SER 原语作为 capability 可选面，为后续高分歧 RT workload 与渲染器集成预留语义面」（`G9_CAPABILITY_MATRIX.md` M108 行改判提案；`G9_PLAN.md` §1.1 M52 行「D3 建议改判『语言层原语 + capability 可选』」）为战略依据，覆盖 RD-040 对 SER 分项事实上的等待条件（SER 未在 `backfill_condition` 单列，title 字面「SER 与 OMM」）。覆盖范围仅为 M108 语言层原语：HitObject 类型面、`reorderThread`/`hitObjectTraceRay`/`hitObjectInvoke`、capability `rt.ser` 可选、材质 flags coherence hint 位段预留；**P2 可选**——收益集中 NV 不承诺性能、渲染器集成延后、不构成主线硬门。本 override 经 `registry/deferred.json` history **只追加**登记，不改写 RD-040 `backfill_condition` 原文、不替代 OMM/SMRT/世界辐射缓存/ReSTIR/NRD 等其余分项判档、不改变 RD-040 总体 open 状态；语义面经 RFC-0023（GPU-driven 提交与着色系统）冻结。

## 3. RD-041 — 材质 / 流送 / 时域 P3+

| 锚/RD-id | 分项名 | G9 M## | 原 backfill 字面 | 证明 workload | 证据路径 | 决策 | 承接波次 | 承接锚 | 最终期望状态 |
|---|---|---|---|---|---|---|---|---|---|
| RD-041 | 多层材质 slab / closure IR | —（语义面留 RFC-0022） | “多层 slab 在单层闭合表达力成为真实资产瓶颈时(MaterialClosure 已预留拓扑字段位)” | MaterialClosure 无法表达的真实资产用例清单 + 分层/混合/降级/跨路径 lowering 对拍 | `registry/deferred.json` RD-041；`G9_PLAN.md` §2 G9.1（「触 M28 多层 closure 条件扩展时显式修订行」）；无真实资产瓶颈证据 | no-go（实现） | 语义只在 RFC-0022 留条件扩展语义面；实现留 G9.7 穷举 | —（留 G9.7） | open-留 G9.7（不得因 RFC 留语义面视为实现 go） |
| RD-041 | SVT 虚拟纹理 | — | `backfill_condition` 无独立 SVT 门槛；title 字面“SVT 虚拟纹理” | 真实大纹理资产管线 + residency/feedback/迟到页/atlas 证据 | `registry/deferred.json` RD-041；`G9_CAPABILITY_MATRIX.md` §7（真实大纹理资产需求独立判档，不搭 D4 便车，R-D4-7） | no-go | 独立判档；门-VT 记 SKIP=not-triggered | —（留档） | open-留档 |
| RD-041 | 帧生成 FG/MFG | — | “FG/MFG 为独立层另判” | 独立 frame-generation ABI、latency/pacing、伪影与 vendor 能力 measured 证据 | `registry/deferred.json` RD-041；`G9_CAPABILITY_MATRIX.md` §7（不进 G9） | no-go | 不进 G9 | —（独立层另判） | open-观察 |
| RD-041 | 蒙皮 / WPO MV 通道资产验证 | — | “蒙皮/WPO MV 在动态资产面出现时(接口已按三类速度设计)” | 真实动态资产 + deformation velocity/MV、TSR 序列和 bounds 证据 | `registry/deferred.json` RD-041；RD-039 M06 骨骼虚拟几何 go（M92 GPU 蒙皮）不等于本 MV 通道资产验证分项触发；无独立真实动态资产 MV 证据 | no-go | G9.7 穷举 | —（留 G9.7） | open-留 G9.7 |
| RD-041 | Work Graphs 与 mesh nodes | — | “Work Graphs 待 Vulkan 侧对应物成熟且『pass 内部提交单元可替换』接缝已预留” | Vulkan 对应物成熟度 + 可替换接缝验证 + D3D12 探针 | `registry/deferred.json` RD-041；`G9_CAPABILITY_MATRIX.md` §7（双条件字面未满足；render graph schema 预留 `reserved_` 前缀字段不接线） | no-go | 仅评估探针；双条件未满足维持 open | —（留档） | open-留档 |

## 4. RD-044 — 物理 P3+（存续 open 分项）

| 锚/RD-id | 分项名 | G9 M## | 原 backfill 字面 | 证明 workload | 证据路径 | 决策 | 承接波次 | 承接锚 | 最终期望状态 |
|---|---|---|---|---|---|---|---|---|---|
| RD-044 | Rapier 快路径深造 | M126（基准先行） | “Rapier 深造在快路径被真实 workload 采用时(对拍门判据形态不变,阈值实测标定口径不变)” | 对标基准先行（M126）：新 Dynamic BVH / sparse voxel / persistent islands / glam 迁移 A/B 报告 → RD-044 判档申请或维持；基准不作 replay oracle | `registry/deferred.json` RD-044；`G9_CAPABILITY_MATRIX.md` M126（「字面不变；基准先行不作 replay oracle；不成立则维持 no-go 留档」）；当前无生产 workload 采用证据 | no-go（条件制维持） | G9.6（M126 基准）→ 判档不成立维持 no-go | —（RD-044 字面不变） | open-留档（M126 基准报告后重判） |
| RD-044 | Continuum（软体/MPM；含 Taichi 生产 external-import 面） | — | “Taichi MPM/体积场在特效资产管线真实出现时由 spike 走生产 external-import 面(维持 §4.E4 三条禁止:只产粒子/体积场、不进刚体求解、不承担确定性联网)” | 真实特效资产管线 + 多 kernel AOT、external import、预算与渲染消费证据 | `registry/deferred.json` RD-044；无真实资产管线证据；三条禁止维持 | no-go | 观察维持 | —（留档） | open-观察 |
| RD-044 | Fluid（含体积场/FLIP 生产面） | — | 同上句“Taichi MPM/体积场在特效资产管线真实出现时由 spike 走生产 external-import 面”；title 字面“流体生产化” | 真实流体/VFX 资产管线 + FLIP/体积场确定性边界、预算与渲染消费证据 | `registry/deferred.json` RD-044；`G9_PLAN.md` §1.4（真双向流体耦合排除主线）；无真实资产管线证据 | no-go | 观察维持 | —（留档） | open-观察 |

> **归属纠偏（沿 G8 口径）**：Differentiable Physics 只在 RD-042 观察（§7），不作为 RD-044 分项；GPU 主刚体否决线维持（§7 RD-043）。

## 5. G8.7 §10 四行与 §1.1 追加输入裁决

| 锚/RD-id | 分项名 | G9 M## | 原 backfill 字面 | 证明 workload | 证据路径 | 决策 | 承接波次 | 承接锚 | 最终期望状态 |
|---|---|---|---|---|---|---|---|---|---|
| G8.7 §10 / M17 | Path Tracer 参照器 | M96 | “GI/材质画质门需要跨路径 golden 时（G9+ 建造期前置）；G8.7 复审”——**字面已命中** | 单向 PT + NEE/MIS/RR megakernel 起步；固定 seed 位级一致 + pbrt-v4 收敛曲线容差带 + 改 seed/跳 RR/关 MIS 三臂 RED；门序硬约束：M96 未绿 → M97~M101 任何画质门不得验收（D2-Q7） | `G8_CANDIDATE_DECISIONS.md` §10 M17 行；`G9_CAPABILITY_MATRIX.md` M96；`G9_PLAN.md` §2 G9.4 门序/§2.9 M96 行 | go | G9.4（波内第一顺位） | open-留 G8.7/G9+（按锚承接） | closed |
| G8.7 §10 / M45 | HDR 管线（拆两层） | M118 | “HDR 显示设备资产/产品需求出现时” | 管线/插件面（go 为 P0）：SDR/scRGB/PQ 三交换链路径运行时切换 + ACES 1.3/2.0/AgX/中性四内置插件逐一 golden，SDR 上即可全量验证；非 HDR 交换链携带 PQ 输出即 RED。设备标定层：条件触发（需 HDR 设备资产），未触发 SKIP=not-triggered 不充绿 | `G8_CANDIDATE_DECISIONS.md` §10 M45 行；`G9_CAPABILITY_MATRIX.md` M118（拆两层字面）；`G9_PLAN.md` §2.9 M118 行 | go（管线/插件面） | G9.5 | open-留 G8.7/G9+（按锚承接） | closed（管线/插件面）/ open-留痕（设备标定层，未触发不假绿） |
| G8.7 §10 / M46 | 后处理栈 | M119 | “产品级后处理需求（bloom/DOF/曝光分级）随 G9+ 建造期出现时”——G9 立项书即产品需求证据（立项时留痕） | histogram 曝光+EV → bloom → DOF → tonemap → LUT → 输出变换全程 HDR 线性域；曝光状态帧间持久；与 TAA/TSR 时域链显式排序 | `G8_CANDIDATE_DECISIONS.md` §10 M46 行；`G9_CAPABILITY_MATRIX.md` M119；用户立项指令（2026-08-09） | go | G9.5 | open-留 G9+（按锚承接） | closed |
| G8.7 §10 / M47 | 透明/OIT | M120 | “透明资产面出现时；OIT 策略选型需 measured 对照。M24 `transparent_velocity` 最小合成面不冒充本行” | benchmark 门先行（nvpro 七算法 harness，仅测量不定档）；默认 TAA 半透明 / 有界近似（WBOIT 起步、AVBOIT 目标）/ 精确 linked-list 仅毛发三档；**无 benchmark 数据的默认档选型提交判 RED**；排序 fallback 永保留 | `G8_CANDIDATE_DECISIONS.md` §10 M47 行；`G9_CAPABILITY_MATRIX.md` M120；`G9_PLAN.md` §2 G9.5 | go | G9.5（benchmark 波内先行） | open-留 G8.7（按锚承接） | closed |
| G8.7 P2 / M75 | 异步物理 tick（双通道确定性架构） | M123 | “RFC-0021 Q6 独立判档”（G8.7 字面“本期只冻结时间域 identity；异步调度须独立判档”） | 条件制：**判档硬前置 = Jolt 单线程成本 measured**（P-6 测量硬前置）；不足维持 no-go 登记不充绿；lockstep-deterministic 永不异步化 vs async-decorative 零回写 + `deterministic_profile` 运行时断言 | `G8_P2_DECISIONS.md` M75 行；RFC-0021 Q6；`G9_CAPABILITY_MATRIX.md` M123；`G9_PLAN.md` §2 G9.6 | go（条件制） | G9.6 | —（独立判档，测量前置） | closed（判档成立）/ open-留档（测量不足，不充绿） |
| G8.7 P2 / M77 | 水体/浮力（解析浮力模型） | M124 | “ApplyBuoyancyImpulse；联动 M49”（G8.7 字面“未包装且无 gameplay 需求；联动 M49 defer”） | 浸入体积/浸没质心 → 浮力+浮力矩+阻力 impulse，走 Field 通道（persistent field + `Buoyancy` 语义）；**禁旁路 API**（旁路注入即 RED）；capture→replay 逐 tick hash 一致 + 变帧率逐位一致；确定性内置入 corpus（细长/翻滚回归） | `G8_P2_DECISIONS.md` M77 行；`G9_CAPABILITY_MATRIX.md` M124（D5 建议 go：Field 统一抽象第二个真实用户）；`G9_PLAN.md` §2 G9.6 | go | G9.6 | open-留档（按锚承接，随 M49→M113 水体面同步） | closed |
| §1.1 / G8 留痕 | Safe GPU Operator Platform | — | G8「改挂 G9+」留痕；G9_PLAN §5 待裁决表项 2 | —（G9 不交付） | `G9_CAPABILITY_MATRIX.md` §7（与 UE5 渲染/物理前置无依赖）；`G9_PLAN.md` §5 表项 2 / §4 R-G9-1（五模块全进已为本项目史上最大里程碑，范围爆炸止损） | defer | G10+ | **G10+ Safe GPU Operator Platform 独立期** | open-defer |
| §1.1 / `rfcs/0021:122` | 神经变形研究子轨 | M127 | `rfcs/0021` G9+ 研究轨留痕（行 122） | 研究子轨登记：混合架构优先、离线工具链（corpus 即语料）、PhysicsAsset residual 通道预留；**无主线门、无 P0/P1 判据、不进 G9 收口硬门**；NN 权威禁止线（NN 输出不得替代权威状态，D5-12）；边界由 RFC-0024 冻结 | `rfcs/0021:122`；`G9_CAPABILITY_MATRIX.md` M127/§7；G9_PLAN §5 表项 5 裁决（维持无归属留痕，不新设 RD） | go（研究子轨登记） | 全程伴随（G9.2~G9.6），无硬门 | —（维持 `rfcs/0021:122` 无归属留痕，不新设 RD） | open-研究子轨（成果另行判档，不占主线门） |

## 6. G8.7 defer 承接锚承接（十锚中 M06/M09 由 §1 go 行承接；本节八锚 + M14 重判档 + Jolt 5.6 评估窗）

| 锚/RD-id | 分项名 | G9 M## | 原 backfill 字面 | 证明 workload | 证据路径 | 决策 | 承接波次 | 承接锚 | 最终期望状态 |
|---|---|---|---|---|---|---|---|---|---|
| G8.7 P2 / M12 | Surface Cache | M97 | “矩阵「G8.7 评估」；依赖 GI 建造期” | 离线 Card 参数化（≤12/mesh 可配）+ 运行时辐射度缓存；缺失覆盖只丢能量不漏光（负例 RED 臂有效）；Card 图集页格式复用 M04 ABI 不私定 | `G8_P2_DECISIONS.md` M12 行；`G9_CAPABILITY_MATRIX.md` M97；`G9_PLAN.md` §2.9 M97 行 | go | G9.4 | **G9+ GI 建造期**（按锚承接） | closed |
| G8.7 P2 / M14 | HWRT hit lighting / Far Field | M98 | “「M50 后评估」；画质 measured 需求”——M50 已绿（G8.2），需求方 = D2 自身画质门 → 重判档 | 追踪降级链 L1 Screen Trace → L2 SWRT（Mesh/Global SDF）→ L3 HWRT（RayQuery + hit lighting 档）→ L4 Far Field；逐档可关可测禁静默；四级命中率/耗时计数非空；L4 依赖 HLOD 接口未就绪时 SKIP=not-triggered | `G8_P2_DECISIONS.md` M14 行；`G9_CAPABILITY_MATRIX.md` M98；`G9_PLAN.md` §2.9 M98 行 | go（重判档） | G9.4 | —（G8.7 no-go 重判档，非 defer 锚） | closed |
| G8.7 P2 / M16 | irradiance field 档位 | M101 | “GI 档位化建造期” | IF 档位 L0–L3 共享 probe 着色与八面体编码内核只换空间索引；每档 AS 更新预算行消费 AsStats；DDGI 档 visibility 16×16 防漏光优先 | `G8_P2_DECISIONS.md` M16 行；`G9_CAPABILITY_MATRIX.md` M101 | go | G9.4 | **G9+ GI 档位**（按锚承接） | closed |
| G8.7 P2 / M33 | shader library 组合链接 | M106/M107 | “若 G8.2 未完则评估” | IR 函数级组合链接：编译期链接物化 SPIR-V/DXIL、链接拓扑进 manifest、interface hash 重算确定性；变体工程级总预算门（硬失败）+ 死变体检测报告 | `G8_P2_DECISIONS.md` M33 行；`G9_CAPABILITY_MATRIX.md` M106/M107 | go | G9.3 | **G9+ shader library 深化**（按锚承接） | closed |
| G8.7 P2 / M43 | World Partition / HLOD | M110/M111 | “「大世界资产面出现时」” | 单一持久世界 schema + 2D cell + streaming source 距离环 + 三项预算契约逐帧 evidence；HLOD 离线烘焙、运行时零合并、双构建 hash 相等；预算违约注入必排队降级 + hitch p99 soak | `G8_P2_DECISIONS.md` M43 行；`G9_CAPABILITY_MATRIX.md` M110/M111；`G9_PLAN.md` §2.9 M110 行 | go | G9.5（M110 波内先行） | **G9+ 大世界分区**（按锚承接） | closed |
| G8.7 P2 / M48 | 体积雾/云 | M112 | “画质专项建造期” | Froxel 统一基础设施 + 雾前端 + 云前端（Perlin-Worley/weather map/时序上采样默认）；weather map 资产化走 M01/M85 通道 | `G8_P2_DECISIONS.md` M48 行；`G9_CAPABILITY_MATRIX.md` M112 | go | G9.5 | **G9+ 大气特效**（按锚承接） | closed |
| G8.7 P2 / M49 | 水体/毛发/皮肤/地形/贴花族 | M113/M114/M115/M116/M117 | “专项渲染器族建造期” | 水体大洋 Tessendorf IFFT 与浅水波方程双管线分离；毛发 Marschner 三瓣 + strand 档强制精确 OIT（G9.5 末，排序在 M120 精确档之后）；皮肤 Burley 屏单 pass（触 `MaterialClosure` 32B 须 RFC 修订，禁静默扩）；地形 chunk ≡ cell 禁第二套分格、零 SVT 依赖断言；贴花 DBuffer 三通道占位 | `G8_P2_DECISIONS.md` M49 行；`G9_CAPABILITY_MATRIX.md` M113~M117 | go | G9.5（M114 G9.5 末） | **G9+ 专项渲染器**（按锚承接） | closed |
| G8.7 P2 / M55 | descriptor buffer / DGC | M102/M103/M104/M105 | “GPU-driven 提交建造期” | DGC 抽象层三后端映射（token 跨 API 最小公倍数、限制装配期 fail-closed）+ descriptor 全局表（索引与 shader 实际索引双向精确相等）+ AccessKind 新边 `StorageWrite→IndirectCommandRead`（触 G5 Barrier EB 冻结面，RFC-0023 显式修订行）+ Execution Set GPU 侧索引切换（D3D12 诚实降级 CPU 侧 PSO 切换，禁静默模拟） | `G8_P2_DECISIONS.md` M55 行；`G9_CAPABILITY_MATRIX.md` M102~M105；`G9_PLAN.md` §2.9 M102/M103/M104 行 | go | G9.2（M102/M103/M104）→G9.3（M105） | **G9+ GPU-driven 提交**（按锚承接） | closed |
| G8.7 P2 / M74 | Physics Field | M121/M122 | “gameplay 统一空间影响” | 统一 particle view 五域 `ParticleAdapter` + `PhysicsParticleRef` 名义类型 + 写路径仅 impulse/force；Field 三层解耦 + `FieldPhysicsType` 八枚举 + 三生命周期（persistent 显式注销全 journal、replay hash 一致）；过滤默认空匹配零影响断言；M68 damage journal 迁移首个 consumer（digest 一致）；World-Field 唯一出口 = GpuScene 只读 buffer | `G8_P2_DECISIONS.md` M74 行；`G9_CAPABILITY_MATRIX.md` M121/M122；`G9_PLAN.md` §2.9 M121/M122 行 | go | G9.2（骨架）→G9.6（完整） | **G9+ gameplay Field**（按锚承接） | closed |
| G8.6a 评估窗 | Jolt 5.3→5.6 升级 A/B | M125 | “G8.6a 纪律延续（corpus 已建成，评估窗开启）” | RFC-0021 §4.A4 七步程序逐字执行（新摩擦模型重点）；采纳臂三件事 / 失败臂钉 5.3，两臂诚实登记禁写 5.6 PASS 伪绿；GPU compute 接口只评估不接权威；layout 探针工具化 | `G9_CAPABILITY_MATRIX.md` M125/§6.3（Jolt 5.6 评估窗行）；`G9_PLAN.md` §2 G9.6 | go | G9.6 | G8.6a 评估窗延续（按锚承接） | closed |

## 7. 门控维持（各一行，从简）

| 锚/RD-id | 分项名 | G9 M## | 原 backfill 字面 | 证明 workload | 证据路径 | 决策 | 承接波次 | 承接锚 | 最终期望状态 |
|---|---|---|---|---|---|---|---|---|---|
| G8.7 P2 / M59 | async compute 第二腿 | — | “多队列 measured 收益” | 多队列 measured 收益证据（D3-Q7） | `G8_P2_DECISIONS.md` M59 行；`G9_CAPABILITY_MATRIX.md` §7（RXS-0239 单 queue 全序字面不动；DGC 全在单 queue 全序内表达） | no-go | 维持 | — | open-留档 |
| G8.7 P2 / M62 | task shader 开放 | — | “RXS-0270 评估窗；RFC-0019/M50” | Amplification 语义出现真实消费方（当前由 DGC 承担 fan-out） | `G8_P2_DECISIONS.md` M62 行；`G9_CAPABILITY_MATRIX.md` §7（RXS-0270 字面不动） | no-go | 不开放维持 | — | open-留档（不开放） |
| RD-034 | DXIL RT/mesh 腿 | — | 上游钳制（spirv-cross RT 消费或 LLVM 签名钳制解除，二选一） | 上游二选一解锁证据 | `G9_CAPABILITY_MATRIX.md` §0.3/§7（blocked 维持；D1~D3 仅 Vulkan 主腿） | no-go | blocked 维持 | — | blocked |
| RD-042 | 可微物理 / 机器人批仿研究轨 | — | “上游可微物理/机器人批仿生态成熟度…达到引擎集成评估门槛,且出现真实 U5 面需求…时重评估;任何合入形态维持『独立仓库或 feature 永不默认』红线” | 真实可微仿真项目 + API/许可/可用后端/采用度四项证据 | `registry/deferred.json` RD-042；`G9_CAPABILITY_MATRIX.md` §7（观察维持，不进 D5） | no-go | 观察维持；不进 G9 硬门 | — | open-观察 |
| RD-043 | wgrapier GPU 刚体观察 | — | “wgrapier 上游达到生产成熟度…且出现 CPU 多核刚体无法承载的 measured 瓶颈场景…时重评估” | 五条件同时成立 + 跨 NVIDIA/AMD end-to-end measured 帧时优于 CPU 扩核/LOD | `registry/deferred.json` RD-043；`G9_CAPABILITY_MATRIX.md` §7（GPU 主刚体否决线维持；Jolt 5.6 GPU compute 只评估不接权威） | no-go | 观察维持；不进 G9 | — | open-观察 |

## 8. 锚 → G9 M## → 波次 → 退出门 → 最终状态总表（**缺行阻断 G9.2**）

> 覆盖：`G9_CAPABILITY_MATRIX` §6.3 全 24 条映射 + §1.1 追加输入（含 M91/Safe GPU/M56/M59/M62）+ 存续 open RD 分项。P0 行退出门引用冻结 key（`g9.p0.m<##>.<slug>`）；判据细节以 `G9_PLAN` §2.9/§2 各波退出门判据草案为准，G9.1 ACCEPTANCE_MAP 固化为契约门。

| G8 锚/来源 | G9 M## | 波次 | 退出门（判据草案/key） | 最终状态 |
|---|---|---|---|---|
| M06 骨骼/植被虚拟几何（defer「G9+ 虚拟几何评估窗」） | M90/M92/M93 | G9.2→G9.3 | `g9.p0.m90.cluster_dag_deepening`（DAG 误差 monotonic 逐边 + 双构建字节一致）+ `g9.p0.m93.visible_cluster_set`（cut 无重叠无空洞 + 空洞注入 RED）+ M92 蒙皮簇 diff=0（**M92 为已 go P1，其验收并入 M93/M95 P0 判据字面**——蒙皮簇注入 selection cut 与 `g9.p0.m95.single_source_truth` 蒙皮簇 VisBuffer SW/HW diff=0 面，不另立 key；RFC-0022 §9.1 F-6 移交处置） | closed（分项） |
| M09 Mega Geometry 簇级 BLAS（defer「G9+ RT×Nanite 合流窗」） | M90/M94 | G9.2→G9.3 | `g9.p0.m94.clas_rt_convergence`（CLAS 腿与回退腿逐命中一致 + 错开一簇即 RED） | closed（分项） |
| M12 Surface Cache（defer「G9+ GI 建造期」） | M97 | G9.4 | `g9.p0.m97.surface_cache`（Card 空洞漏光检测臂 RED 有效 + 只丢能量不漏光） | closed |
| M14 HWRT hit lighting / Far Field（no-go「M50 后评估」重判档） | M98 | G9.4 | `g9.p0.m98.tracing_fallback_chain`（四级计数非空 + 逐级强关回归可检测 + 禁静默回退） | closed |
| M15/M22 MegaLights/ReSTIR + 海量灯阴影（no-go「多灯场景需求出现时」） | M100 | G9.4 | 低档默认门 + 验证射线零跳过硬契约；高档 ReSTIR 须多灯 workload 证据 | closed（低档）/ open-留档（高档） |
| M16 irradiance field 档位（defer「G9+ GI 档位」） | M101 | G9.4 | 每档 AS 更新预算行消费 AsStats + 按匹配深度对 M96 golden | closed |
| M17 Path Tracer 参照器（no-go「G9+ 建造期前置」，字面已命中） | M96 | G9.4（波内第一顺位） | `g9.p0.m96.path_tracer_reference`（固定 seed 位级一致 + pbrt-v4 容差带 + 三臂 RED） | closed |
| M11 世界辐射缓存（no-go 除非 measured） | M99（世界 clipmap 级） | G9.4 | 屏幕级门（SPG + Radiance Cache 屏幕级）；clipmap 级 measured 举证前置 | closed（屏幕级）/ open（clipmap 级） |
| M33 shader library 组合链接（defer「G9+ shader library 深化」） | M106/M107 | G9.3 | IR 链接 interface hash 确定性 + 链接拓扑可回放 + 变体工程级总预算门硬失败有效 | closed |
| M52 SER（no-go 留档 → **strategic_override**） | M108 | G9.3+（P2 可选） | capability `rt.ser` 可选原语 RED/GREEN；override history 只追加核验 | closed（可选分项） |
| M55 descriptor buffer/DGC（defer「G9+ GPU-driven 提交」） | M102/M103/M104/M105 | G9.2→G9.3 | `g9.p0.m102.dgc_abstraction` + `g9.p0.m103.descriptor_global_table` + `g9.p0.m104.accesskind_indirect_edge` + M105 Execution Set 诚实降级 | closed |
| M61 mesh shader 第三光栅（no-go 留档 → **strategic_override**） | M109 | G9.3+（P2 可选，顺序硬约束后） | VS 光栅唯一 fallback + `mesh.task` 选择律 RED/GREEN；override history 只追加核验；顺序硬约束核验 | closed（可选分项） |
| M43 World Partition/HLOD（defer「G9+ 大世界分区」） | M110/M111 | G9.5 | `g9.p0.m110.world_partition`（预算违约注入必降级 + hitch p99 soak + cell 事件 golden）+ HLOD 双构建 hash 相等 + 运行时零合并 | closed |
| M48 体积雾/云（defer「G9+ 大气特效」） | M112 | G9.5 | Froxel 基础设施 + 雾/云前端 golden | closed |
| M49 专项渲染器族（defer「G9+ 专项渲染器」） | M113/M114/M115/M116/M117 | G9.5（M114 末） | 各专项前端 golden；M114 strand 档强制精确 OIT；M116 零 SVT 依赖断言；M115 触 closure 32B 须 RFC 修订行 | closed |
| M45 HDR 管线（no-go「HDR 显示设备资产/产品需求出现时」） | M118 | G9.5 | `g9.p0.m118.display_pipeline_view_transform`（四插件逐一 golden + PQ 违规 RED）；设备标定未触发 SKIP=not-triggered 不充绿 | closed（管线/插件面）/ open-留痕（标定层） |
| M46 后处理栈（no-go「产品需求随 G9+ 建造期出现」，立项书即证据） | M119 | G9.5 | 后处理链 golden + 曝光状态帧间持久 + 与 TAA/TSR 显式排序 | closed |
| M47 透明/OIT（no-go「OIT 策略选型需 measured 对照」） | M120 | G9.5（benchmark 波内先行） | benchmark harness（仅测量不定档）；无 benchmark 数据的默认档选型提交判 RED | closed |
| M74 Physics Field（defer「G9+ gameplay Field」） | M121/M122 | G9.2→G9.6 | `g9.p0.m121.physics_particle_view`（五域 adapter + M68 journal 迁移 digest 一致）+ `g9.p0.m122.gameplay_field`（过滤默认空匹配零影响 + persistent 全 journal replay hash 一致） | closed |
| M75 异步物理 tick（no-go「须独立判档」） | M123 | G9.6 | Jolt 单线程成本 measured 判档硬前置；不足 no-go 登记不充绿；`deterministic_profile` 运行时断言 | closed（判档成立）/ open-留档（不足） |
| M77 水体/浮力（no-go「ApplyBuoyancyImpulse 未包装」） | M124 | G9.6 | 浮力旁路 API 注入即 RED + capture→replay 逐 tick hash + 变帧率逐位一致 | closed |
| M65b Rapier 深造（no-go「快路径被真实 workload 采用时」） | M126 | G9.6 | 对标 A/B 报告 → RD-044 判档申请或维持；基准不作 replay oracle | open-留档（基准先行后重判） |
| Jolt 5.6 评估窗（G8.6a 纪律延续） | M125 | G9.6 | RFC-0021 §4.A4 七步程序记录完整 + 采纳/失败两臂诚实登记 | closed |
| 神经变形（`rfcs/0021:122` G9+ 研究轨留痕） | M127 | 全程伴随，无硬门 | 无主线门；NN 权威禁止线；边界由 RFC-0024 冻结 | open-研究子轨 |
| D1 D-11（新增 cluster 属性入页触发；无 G8 锚） | M91 | G9.2 | `g9.p0.m91.page_format_v2_abi`（编解码往返无损 + M04 v1 0-byte 兼容 + 篡改 digest 页被拒） | closed |
| Safe GPU Operator Platform（G8「改挂 G9+」留痕） | — | G10+ | —（defer，承接锚机核进 G9.7 validator） | open-defer（承接锚「G10+ Safe GPU Operator Platform 独立期」） |
| M56 Work Graphs（RD-041 双条件未满足） | —（预留 `reserved_` 前缀字段不接线） | 仅评估探针 | — | open-留档 |
| M59 async compute 第二腿（RXS-0239 字面不动） | — | 维持 | — | open-留档 |
| M62 task shader（RXS-0270 字面不动） | — | 不开放维持 | — | open-留档（不开放） |
| RD-039 M03 HZB | — | G9.7 穷举 | — | open-留 G9.7 |
| RD-039 M44 cluster 流送 P4 运行时 | — | G9.7 穷举 | — | open-留 G9.7 |
| RD-039 M05 曲面细分位移 | — | G9.7 穷举 | — | open-留 G9.7 |
| RD-039 M06 Assemblies 全功能 | — | G9.7 穷举 | — | open-留 G9.7 |
| RD-040 M20 SMRT | — | G9.7 穷举 | — | open-留 G9.7 |
| RD-040 自适应探针（GI P3） | — | G9.7 穷举 | — | open-留 G9.7 |
| RD-040 M13 SDF 软追踪 | — | G9.7 穷举 | — | open-留 G9.7 |
| RD-040 M53 OMM | — | G9.7 穷举 | — | open-留 G9.7 |
| RD-040 NRD vendor 降噪 | — | G9.7 穷举 | — | open-留 G9.7 |
| RD-041 M28 多层材质 | —（语义面留 RFC-0022 条件扩展） | G9.7 穷举（实现） | — | open-留 G9.7 |
| RD-041 M40 SVT | — | 独立判档（不搭 D4 便车） | — | open-留档 |
| RD-041 M26 FG/MFG | — | 不进 G9（独立层另判） | — | open-观察 |
| RD-041 M05 蒙皮/WPO MV | — | G9.7 穷举 | — | open-留 G9.7 |
| RD-044 Continuum / Fluid | — | 观察维持（三条禁止维持） | — | open-观察 |
| RD-034 DXIL RT/mesh 腿 | — | blocked 维持（D1~D3 仅 Vulkan 主腿） | — | blocked |
| RD-042 可微物理/机器人批仿 | — | 观察维持，不进 G9 硬门 | — | open-观察 |
| RD-043 wgrapier GPU 刚体 | — | 观察维持，GPU 主刚体否决线维持 | — | open-观察 |

> **已 go P1 的硬门落点纪律（v1.1 追加）**：M92（P1，go）验收并入 M93/M95 P0 判据字面（见 M06 锚行），不另立 key；M124（P1，go）硬门落点走 G9.6 开工前 `G9_ACCEPTANCE_MAP` §1 只追加程序或波次聚合 subject（字面 G9.6 开工时冻结），当前不预造 key——与 MAP §3「已 go P1 集合当前为空集」自洽（RFC-0022 §9.1 F-6 / RFC-0024 §9.1 F-7 移交处置）。

> **已 go P1 的硬门落点校准（v1.2 追加，G9.3 波 P1 全进裁决）**：依 [G9_CONTRACT.md](G9_CONTRACT.md) §8.1 裁决①（P1 全进，逐波经治理流程只追加进 ACCEPTANCE_MAP §3，不静默并入既有 key），M92/M105/M106/M107 四行的 G9 承接状态校准为 **go（G9.3，P1 全进裁决）**，各立独立 P1 key 与 CI 门——M92 → `g9.p1.m92.gpu_skinning_lod_update`、M105 → `g9.p1.m105.command_build_node`、M106 → `g9.p1.m106.execution_set_pso`、M107 → `g9.p1.m107.shader_library_ir_link`（G9_ACCEPTANCE_MAP §3 + CI_GATES §4A 同构登记，2026-08-11；numeric CI step 待 materialize 实测回填）。承接面按 G9.3 执行波口径（G9_PLAN §2 G9.3 D3 链路行）：M105 = command build node 全链路零 CPU 回读（RFC-0023 §4.4 语义面；M104 P0 已冻结的 AccessKind 新边与结构性零回读面不降格）、M106 = Execution Set 与 PSO 衔接（RFC-0023 §4.2；`submit.execution_set` 预留位随 spec RXS-0355 转正——RXS-0349 行「M105 Execution Set」的 M## 引用以矩阵口径为历史留痕，本波消费面落 M106 key）、M107 = IR 链接 + 变体预算合并门（RFC-0023 §4.5/§4.6「同波不延后」字面）。v1.1 注「M92 并入 M93/M95 P0 判据字面、不另立 key」的落点处置由本注校准为独立 P1 key（M93/M95 P0 判据字面不降格、不回写）；M124 落点纪律维持 v1.1 注不变（G9.6 开工前走 MAP §1 只追加程序）。全部 47 行裁决逐字未改。

> **RD-040 条件分项触发举证校准（v1.3 追加，G9.4 波 P1 全进裁决）**：依 [G9_CONTRACT.md](G9_CONTRACT.md) §8.1 裁决①与 §6 RD-040 行「M99 世界 clipmap 级须 measured 触发举证，未举证只做屏幕级；M100 多灯高档须附 workload 证据，不足则只做低档、M15 维持 open-留档」字面——截至本注落盘，世界级 clipmap 远场画质 measured 证据与多灯 ReSTIR workload 证据**均未产出**（树内零对应 measured artifact）。校准登记：**M99 仅屏幕级判 go**（SPG 自适应细分 + Radiance Cache 屏幕级 + product importance sampling；世界 clipmap 级分项维持 open、登记 **not-triggered 不充绿**，待 measured 举证后只追加重判）；**M100 仅低档默认判 go**（MegaLights 式固定随机选灯 + 验证射线零跳过硬契约〔D2-Q4〕+ 海量灯阴影统一接口随动；高档 ReSTIR reservoir 分项维持 open-留档、登记 **not-triggered 不充绿**，待多灯 workload 证据后只追加重判）；**M101 全档判 go**（G8.7 P2/M16 defer 锚「G9+ GI 档位」承接，无 RD-040 条件分项字面，IF 档位 L0~L3 全阶梯进 G9.4）。M99/M100/M101 三分项各立独立 P1 key——`g9.p1.m99.spg_radiance_cache` / `g9.p1.m100.multi_light_low` / `g9.p1.m101.if_tier_ladder`（G9_ACCEPTANCE_MAP §3 + CI_GATES §4A 同构登记，2026-08-12；numeric CI step 待 materialize 实测回填）。RD-040 总体 open 状态与 §2 各行裁决字面 0-byte 不改；条件型 RD 触发条件不得被「UE5 目标」静默改写（§0 纪律②同口径）。

> **G9.5 波 P1 裁决与 D4 伞形 RFC 缺口处置校准（v1.4 追加，G9.5 波 P1 全进裁决）**：依 [G9_CONTRACT.md](G9_CONTRACT.md) §8.1 裁决①，G9.5 波九项 P1（§6 M43/M48/M49 锚行与 §5 M45~M47 行的承接分项）逐项裁决登记——**M111 HLOD / M112 大气（Froxel 统一基础设施 + 雾/云前端）/ M113 水体（大洋 IFFT + 浅水波方程双管线）/ M116 地形（chunk ≡ cell + 零 SVT 依赖断言）/ M117 贴花（DBuffer 三通道帧图占位）/ M119 后处理骨架（显式排序 + HDR 线性域 + 曝光状态帧间持久）判 go**；**M115 皮肤判 go**，触 `MaterialClosure` 32B 冻结面（G5 冻结，RFC-0016 §4.G1；RFC-0019 §4.7/§8 与 RFC-0022 §8 重申 0-byte）经新起草 **RFC-0025**（`rfcs/0025-world-and-specialty-renderers.md`，G9 D4 伞形）§4.L 🔒 显式修订行前置登记——资产化侧表扩展通道按材质槽 ID 索引接入单层 closure 求值，32B 定长布局/字段含义/flags 位段 0-byte、预留拓扑字段位不消费，禁静默扩（M104 修订行先例 = RFC-0023 §4.4.3）；**M114 毛发条件 go**——Marschner R/TT/TRT 三瓣着色与 card/mesh 几何档判 go，**strand 档（强制精确 OIT）依赖 M120 精确 linked-list 档 benchmark 裁决数据，截至本注落盘该数据未产出（M120 本波仅落 benchmark harness 仅测量不定档），strand 档分项登记 not-triggered 不充绿**，承接锚「M120 精确档 benchmark 裁决数据落地后重判，兜底 G9.7 穷举」（G9_PLAN §2 G9.7 候选行集已列 M114）；**M120 OIT benchmark harness 判 go**（仅测量不定档，evidence 非空；默认档选型必须引 benchmark 数据，无数据提交判 RED；排序 fallback 永保留）。**D4 伞形 RFC 缺口处置**：G9.1 三份伞形 RFC（0022=D1/D2、0023=D3、0024=D5）未覆盖 D4——经 Grep 实测 RFC-0016/0019/0022/0023 冻结面与 D4 链路面（M110~M120）无重叠，MR（Mini-RFC）体例不承载新语义面 + G5 冻结面修订行，判档争议向上取严（硬规则 8），按 RFC-0024 最小化先例起草 **RFC-0025**（编号按 2026-08-12 实测 `number_ledger` RFC next_free=25 顺位领取；D-409 第 1 轮对抗性评审完成——单实例偏差如实登记 + 效力自限声明见 RFC-0025 §9.1——4 findings 全部 disposition 后 Agent Approved）。九项各立独立 P1 key（`g9.p1.m111.hlod_baking` / `g9.p1.m112.atmosphere_froxel` / `g9.p1.m113.water_dual_pipeline` / `g9.p1.m114.hair_marschner` / `g9.p1.m115.skin_burley_diffusion` / `g9.p1.m116.terrain_chunk_cell` / `g9.p1.m117.decal_dbuffer` / `g9.p1.m119.post_processing_skeleton` / `g9.p1.m120.oit_benchmark_harness`），经 G9_ACCEPTANCE_MAP §3 + CI_GATES §4A 同构登记（2026-08-12，只追加；numeric CI step 待 materialize 实测回填）。全部 47 行裁决字面 0-byte 未改；M43/M48/M49/M45/M46/M47 承接锚行字面不动；`registry/deferred.json` 0-byte——无整项 defer（M114 strand 档为分项 not-triggered，沿 M99/M100 v1.3 先例不动 RD），RD-039/040/041/044 总体 open 维持。

> **G9.6 波 P1 裁决与 M123 判档校准（v1.5 追加，G9.6 波 P1 全进裁决）**：依 [G9_CONTRACT.md](G9_CONTRACT.md) §8.1 裁决①，G9.6 波 P1 候选（§5 M75/M77 行与 §6 M65b/G8.6a 锚行的承接分项）逐项裁决登记——**M124 浮力判 go**（解析浮力模型走 Field 通道：persistent field + `FieldPhysicsType::Buoyancy` 语义，Field 统一抽象第二个真实用户；**禁旁路 API**——旁路注入即 RED；细长体/翻滚体 corpus fixture；capture→replay 逐 tick hash 一致 + 变帧率逐位一致；M77 锚承接，语义面 RFC-0024 §4.D）；**M125 Jolt 5.3→5.6 A/B 判 go**（RFC-0021 §4.A4 七步程序逐字执行；5.6 独立 vendor 并存不覆盖 5.3 基线；新摩擦模型〔平均接触点〕重点实测；GPU compute 只评估不接权威〔GPU 主刚体禁止线 0-byte〕；采纳臂三件事/失败臂钉 5.3 两臂诚实登记，禁写 5.6 PASS 伪绿；G8.6a 评估窗锚承接，语义面 RFC-0024 §4.E1）；**M126 Rapier 深造对标基准判 go**（新 Dynamic BVH/sparse voxel/persistent islands/manifold ≤4/简化摩擦模型大堆叠场景 A/B 夹具，与 Jolt 同场景同输入同 determinism 画像；measured 报告含确定性偏差统计；基准不作 replay oracle；**RD-044 字面不变**——基准显示 D5 真实 workload measured 优势才按 RD-044 程序申请深造判档，否则维持 no-go 留档；M65b 锚承接，语义面 RFC-0024 §4.E2）。**M123 双通道判档 = no-go 不充绿**：判档硬前置 = Jolt 单线程成本 measured（RFC-0024 R-6 🔒/Q1；「异步调度须独立判档」为 `G8_P2_DECISIONS.md` M75 行理由列字面）——截至本注落盘，**树内零 Jolt 单线程成本 measured artifact**（evidence/ 物理相关件〔G6 physics_core/bridge/rapier_parity、G8 M66/M67、G9.2 M121/M122 骨架〕零单线程成本字段；`g9_budget.json` 无物理段 counter；测量任务 = D5 先行任务 P-6/RFC-0024 §6.3 步 1，归实现波真跑，本治理/spec 波零 cargo 构建不产测量，禁 estimated），判档不成立 → **维持 M75 no-go 留档**（RFC-0024 Q1「测量不足 → 维持 M75 no-go 留档」字面）：lockstep-deterministic 维持唯一通道，`physics-async-decorative` feature 与 `DecorativePhysicsTickId` 维持「仅判档 go 时生效」字面（R-4/R-7 🔒）不启用；no-go 项不入 G9_ACCEPTANCE_MAP §3（§3「no-go/defer 项不入本表」纪律），承接锚 = **G9.7 P2 穷举**（G9_PLAN §2 G9.7 候选行集已列「M123/M126（若判档不成立）」字面；`ci/g9_p2_decisions_check.py` 候选行含 M123）；判档结论经 **RFC-0024 v1.1 修订行**落定（Q1「判档结论以本 RFC 修订行落定，并引判档证据」字面）。三项 go 各立独立 P1 key（`g9.p1.m124.buoyancy_field_channel` / `g9.p1.m125.jolt_56_ab_evaluation` / `g9.p1.m126.rapier_benchmark_ab`），经 G9_ACCEPTANCE_MAP §3 + CI_GATES §4A 同构登记（2026-08-13，只追加；numeric CI step 待 materialize 实测回填）。全部 47 行裁决逐字未改；M75/M77/M65b/G8.6a 承接锚行字面不动；`registry/deferred.json` 0-byte——M123 no-go 承接锚为 G9.7 穷举既有候选行字面，不新设 RD（沿 v1.3/v1.4 先例），RD-039/040/041/044 总体 open 维持（RD-044 M65b 分项经 M126 基准报告后重判，字面不变）。

## 9. RD 条目级 close-out 预期

| RD | G9 close-out 预期 | 约束 |
|---|---|---|
| RD-039 | open | 仅 M06 Foliage/骨骼、M09 Mega Geometry（立项书触发，history 只追加）与 M61→M109（strategic_override）目标 closed；M03/M44/M05 曲面细分位移/M06 Assemblies 继续留档进 G9.7 |
| RD-040 | open | 仅 M52→M108（strategic_override）、M11→M99 屏幕级、M15/M22→M100 低档目标 closed；M20/自适应探针/M13/M53/NRD 继续留档；M99 clipmap 级与 M100 高档未举证维持 open |
| RD-041 | open | 无 go 分项；M28 仅 RFC-0022 留条件扩展语义面（实现不 go）；M40/M26/M05 MV/M56 全部维持 open |
| RD-044 | open | 无 go 分项；M65b 经 M126 基准报告后重判（字面不变）；Continuum/Fluid 观察维持 |
| RD-034 | blocked | DXIL RT/mesh 上游钳制维持；D1~D3 仅 Vulkan 主腿 |
| RD-042 | open-观察 | 可微物理/机器人批仿研究隔离维持，不进 G9 硬门 |
| RD-043 | open-观察 | wgrapier 观察维持；GPU 主刚体否决线维持 |

## 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-09 | 首版（G9.1 governance-only）：六项立项裁决落盘（§0）；RD-039/040/041/044 全部 open 分项逐行裁决——M06 Foliage/骨骼、M09 Mega Geometry 以 G9 立项书为触发证据 go（history 只追加）；M61/M52 各记 strategic_override 转 M109/M108（P2 可选，登记文本 §1.1/§2.1）；M99/M100/M123 条件制 go；其余 no-go 维持 open 留 G9.7；G8.7 §10 四行 M17/M45/M46/M47 go（M118 拆两层，标定层未触发 SKIP 不充绿）；M77→M124 go（禁旁路 API）；Safe GPU Operator Platform defer G10+（承接锚「G10+ Safe GPU Operator Platform 独立期」）；神经变形 M127 研究子轨登记（不新设 RD，RFC-0024 冻结边界）；十条 G8.7 defer 承接锚逐条承接；§8 总表覆盖矩阵 §6.3 全 24 条 + §1.1 追加输入 + 存续 open RD 分项，缺行阻断 G9.2。零 `src/spec/conformance` 改动、零 CI 步骤 materialize、零编号消费（RFC-0022/0023/0024 按实测 `next_free=22` 引用，登记由立项治理统一落）。 |
| v1.1 | 2026-08-09 | **只追加修订（裁决字面 0-byte）**：落实 RFC-0022 §9.1 F-6 / RFC-0024 §9.1 F-7 跨文档移交——§8 总表 M06 锚行显式登记「M92（已 go P1）验收并入 M93/M95 P0 判据字面，不另立 key」；表后追加「已 go P1 的硬门落点纪律」注（M92 并入 M93/M95；M124 走 G9.6 开工前 MAP §1 只追加程序或波次聚合 subject，当前不预造 key）。全部 47 行裁决逐字未改。 |
| v1.2 | 2026-08-11 | **只追加校准（裁决字面 0-byte）**：落实 G9_CONTRACT §8.1 裁决①（G9.3 波 P1 全进）——§8 总表后追加「已 go P1 的硬门落点校准（v1.2）」注：M92/M105/M106/M107 承接状态校准为 go（G9.3，P1 全进裁决），各立独立 P1 key（`g9.p1.m92.gpu_skinning_lod_update` / `g9.p1.m105.command_build_node` / `g9.p1.m106.execution_set_pso` / `g9.p1.m107.shader_library_ir_link`，G9_ACCEPTANCE_MAP §3 + CI_GATES §4A 同构登记，numeric step 待 materialize 实测回填）；M105~M107 承接面按 G9.3 执行波口径登记（command build node 全链路 / Execution Set 与 PSO 衔接〔`submit.execution_set` 预留位随 RXS-0355 转正〕 / IR 链接+变体预算合并门，语义面 RFC-0023 §4.4/§4.2/§4.5/§4.6）；v1.1 注 M92「不另立 key」落点由本注校准，M124 纪律维持。全部 47 行裁决逐字未改。 |
| v1.3 | 2026-08-12 | **只追加校准（裁决字面 0-byte）**：落实 G9.4 波 P1 全进裁决（G9_CONTRACT §8.1 裁决①）RD-040 条件分项触发举证判档——§8 总表后追加「RD-040 条件分项触发举证校准（v1.3）」注：M99 仅屏幕级判 go（世界级 clipmap 未 measured 举证 → 分项 not-triggered 不充绿）、M100 仅低档默认判 go（高档 ReSTIR workload 证据不足 → 分项 not-triggered 不充绿，M15 维持 open-留档）、M101 全档判 go；M99/M100/M101 各立独立 P1 key（`g9.p1.m99.spg_radiance_cache` / `g9.p1.m100.multi_light_low` / `g9.p1.m101.if_tier_ladder`）经 G9_ACCEPTANCE_MAP §3 + CI_GATES §4A 同构登记（numeric step 待 materialize 实测回填）。全部 47 行裁决逐字未改；RD-040 总体 open 维持。 |
| v1.4 | 2026-08-12 | **只追加校准（裁决字面 0-byte）**：落实 G9.5 波 P1 全进裁决（G9_CONTRACT §8.1 裁决①）——§8 总表后追加「G9.5 波 P1 裁决与 D4 伞形 RFC 缺口处置校准（v1.4）」注：M111/M112/M113/M116/M117/M119 判 go、M115 判 go（触 `MaterialClosure` 32B 经 RFC-0025 §4.L 🔒 显式修订行前置登记）、M114 条件 go（strand 档依赖 M120 精确档 benchmark 数据不足 → 分项 not-triggered 不充绿，承接锚「M120 精确档数据落地后重判，兜底 G9.7 穷举」）、M120 判 go（仅测量不定档）；D4 无伞形 RFC 缺口按判档向上取严起草最小伞形 Full RFC-0025（实测 next_free=25 顺位领取，D-409 第 1 轮评审单实例偏差如实登记后 Agent Approved）；九项各立独立 P1 key 经 G9_ACCEPTANCE_MAP §3 + CI_GATES §4A 同构登记（numeric step 待 materialize 实测回填）。全部 47 行裁决逐字未改；deferred.json 0-byte（无整项 defer，M114 strand 档为分项 not-triggered 沿 v1.3 先例）；RD-039/040/041/044 总体 open 维持。 |
| v1.5 | 2026-08-13 | **只追加校准（裁决字面 0-byte）**：落实 G9.6 波 P1 全进裁决（G9_CONTRACT §8.1 裁决①）——§8 总表后追加「G9.6 波 P1 裁决与 M123 判档校准（v1.5）」注：M124 浮力判 go（走 Field 通道 + 禁旁路 API + corpus fixture + capture→replay 逐 tick hash + 变帧率逐位一致；M77 锚承接，RFC-0024 §4.D）、M125 Jolt 5.6 A/B 判 go（七步逐字 + 独立 vendor 并存不覆盖 5.3 + 新摩擦模型重点实测 + GPU compute 只评估不接权威 + 两臂诚实登记；G8.6a 锚承接，RFC-0024 §4.E1）、M126 Rapier 基准判 go（同场景同输入同 determinism 画像 A/B + measured 报告 + 不作 replay oracle + RD-044 字面不变；M65b 锚承接，RFC-0024 §4.E2）；**M123 判档 = no-go 不充绿**（判档硬前置 Jolt 单线程成本 measured 未满足——树内零 measured artifact，治理/spec 波零 cargo 构建不产测量；维持 M75 no-go 留档，`physics-async-decorative`/`DecorativePhysicsTickId` 不启用；承接锚 G9.7 穷举既有候选行字面；判档结论经 RFC-0024 v1.1 修订行落定）；三项 go 各立独立 P1 key（`g9.p1.m124.buoyancy_field_channel` / `g9.p1.m125.jolt_56_ab_evaluation` / `g9.p1.m126.rapier_benchmark_ab`）经 G9_ACCEPTANCE_MAP §3 + CI_GATES §4A 同构登记（numeric step 待 materialize 实测回填）。全部 47 行裁决逐字未改；deferred.json 0-byte（M123 no-go 承接锚 = G9.7 穷举既有候选行字面，不新设 RD，沿 v1.3/v1.4 先例）；RD-039/040/041/044 总体 open 维持。 |
| v1.6 | 2026-08-14 | **只追加校准（裁决字面 0-byte）**：落实 G9.6 实现波 M124/M126 两门 materialize 与 M126→RD-044 判档登记——§8 总表后追加「M124/M126 门 materialize 与 RD-044 重判校准（v1.6）」注：**①** M124（`g9.p1.m124.buoyancy_field_channel`）numeric step 由「待 materialize 实测回填」转为实测 **166**（`ci/g9_buoyancy_field_channel_smoke.py`，host 纯 host 确定性门；判据事实源 = MAP §3 M124 行 + RXS-0376 + RFC-0024 §4.D；ledger v1.95 顺位领取）。**②** M126（`g9.p1.m126.rapier_benchmark_ab`）numeric step 转为实测 **167**（`ci/g9_rapier_benchmark_ab_smoke.py`，host 纯 host 确定性门，rapier feature 默认 off 纪律维持；ledger v1.96 顺位领取）。**③** **M126 基准报告落地 → RD-044 Rapier 快路径深造分项重判 = 维持 no-go（条件制）**：measured 报告 `milestones/g9/g9_m126_rapier_benchmark.json`（measured_local 真实采样，禁 estimated）——canonical 大堆叠 6 层 A/B 同场景同输入同 determinism 画像（input_digest 两臂逐位相等；Jolt/Rapier 各自双跑位级一致），Jolt step 中位 40400ns vs Rapier 197900ns（Jolt 显著占优，Rapier 臂无 measured 优势），跨 solver 差异如实记录（world 链非逐位一致；max translation abs diff ≈ 3.9e-2 m 在 0.05m 容差内；接触事件计数差 11；rest_above_ground 不变量成立）——RD-044 条件字面「快路径被真实 workload 采用时」**不变**，verdict=`maintain_no_go`（§4 RD-044 Rapier 行「open-留档（M126 基准报告后重判）」重判完成，最终期望状态维持 open-留档字面不改）；基准不作 replay oracle（replay 对拍唯一权威 = 同 solver 同版本 capture/replay 逐 tick hash，RFC-0021 §4.A1）；glam 迁移兼容留档（不承诺 bitwise 不变）；本门只产基准报告，不升格深造、不作验收依赖与生产默认；deferred.json RD-044 history 只尾追加（status open 维持，id/title/reason/backfill_condition 四字段 0-byte，沿 v1.76 RD-040 先例）。全部 47 行裁决逐字未改；RD-039/040/041/044 总体 open 维持；M125 门 materialize 归其自身实现波。 |
