# G8_CANDIDATE_DECISIONS — RD-037~044 候选分项裁决

> **状态**：G8.1 governance-only 裁决记录（2026-08-02）。本表可与 G7 active 并行落盘；**G8.2+ implementation 仍 blocked**。解除实现互锁必须满足 [G8_PLAN §1.0](G8_PLAN.md) 的 G7 closed + RD-038 closed，或在 G7 closed 后把 RD-038 六行互锁终态填满并登记一条独立 RD-038 override。
> **事实源**：[G8_PLAN](G8_PLAN.md) v1.2（承 v1.1 分项与 backfill 纪律的加性解堵裁决）· [G8_CAPABILITY_MATRIX](G8_CAPABILITY_MATRIX.md) v1.2 · [`registry/deferred.json`](../../registry/deferred.json) v1.73（RD-037~044）。
> **证据口径**：“证据路径”只列当前在树事实或作出治理裁决的事实源；“证明 workload”是该分项实施后必须产出的机器证据，未产出前不得把 `go` 写成已交付。`最终期望状态` 是分项 close-out 目标，不是本表落盘时的 registry 状态。

## 0. 总裁决与互锁

- **go**：RD-037/M89 三件套（一个不可拆验收决策）、M04、M25、M72、M83。
- **strategic_override**：仅 RD-040/M50。战略依据是 UE5 前置完成期已经明确需要多材质 hit、SBT 用户数据与稳定 SBT ABI；它们同时是 Path Tracer、材质 hit lighting、SER/OMM 的共同前置，而现有 RXS-0248 仅覆盖单 hit group、单三件套、SBT 无用户数据的最小见证。
- **no-go**：除上述项外全部分项；原 backfill 没有 measured workload、真实资产需求或上游成熟度证据时维持 open。title-only、未给独立 backfill 门槛的分项同样不得静默主线化。
- **RD-038 不 override**：六个接入面当前统一视为 `G7 in-flight / G8 互锁终态 unresolved`；表内 `no-go` 只表示“当前不允许 G8 接管或据此开 G8.2”，不表示否决这些能力。
- **M50 override 与 RD-038 override 严格隔离**：本表对 M50 的 strategic override 不改变 RD-038 status，不替代 §1.0 六行终态，不开放 G8.2，也不解除 RD-034 的 DXIL RT 上游钳制。

逐分项共 **41 行**：go 7 行（其中 RD-037/M89 三行合为一个验收决策）、strategic_override 1 行、no-go 33 行。按能力决策计，go 为 M89、M04、M25、M72、M83 共 5 项。

## 1. RD-037 — 单源 gfx submit（无条件并入）

| RD-id | 分项名 | M## | 原 backfill 字面 | 证明 workload | 证据路径 | 决策 | 承接波次 | 退出门 | 最终期望状态 |
|---|---|---|---|---|---|---|---|---|---|
| RD-037 | `rurixc` gfx pass lowering → artifacts v2 | M89 | “① rurixc lowering 把 gfx pass 顶点供给面纳入 artifacts v2 通道(条款先行,确需自 RXS-0297 顺位)” | 同一 `.rx` 声明式 gfx 图产出 vs/fs SPIR-V 与顶点供给描述；全链不写 Rust 宿主出图代码 | `registry/deferred.json` RD-037；`G8_CAPABILITY_MATRIX.md` M89；`G8_PLAN.md` §1.1/§2 G8.2 | go | G8.2（实现门 blocked） | artifacts v2 产物结构与 `.rx` 图输入逐项一致；三件套全部齐备后才判 RD-037 | closed（组三件全量后） |
| RD-037 | C ABI VB/IB 顶点数据绑定面 | M89 | “② cabi 追加式 VB/IB 绑定符号(RXS-0194 口径,drop_rhi_children 级联模式)” | `.rx` 图使用真实 VB/IB，经追加式 C ABI 绑定并完成 child 级联释放；无 host substitution | `registry/deferred.json` RD-037；`G8_CAPABILITY_MATRIX.md` M89 | go | G8.2（实现门 blocked） | ABI RED/GREEN + 生命周期/级联释放见证；三件套全部齐备后才判 RD-037 | closed（组三件全量后） |
| RD-037 | `rxrt_rhi_submit` 阶段 2 gfx 真派发 | M89 | “③ rxrt_rhi_submit 阶段 2 gfx 派发臂接 render_exec/vk.rs;判据 = .rx gfx 图零 Rust 宿主代码 device 真跑 readback 像素断言” | RTX 4070 Ti 上以 `.rx` 图直接派发 graphics，readback 像素逐值断言 | `registry/deferred.json` RD-037；`src/rurix-rt-cabi/src/lib.rs` 现状锚；`src/rurix-rt/src/render_exec.rs`；`G8_CAPABILITY_MATRIX.md` M89 | go | G8.2（实现门 blocked） | 零 Rust 宿主 `.rx` gfx 图 device 真跑并通过 readback 像素断言 | closed（组三件全量后） |

## 2. RD-038 — 六个 G7 接入面（全部 in-flight / unresolved，无 override）

| RD-id | 分项名 | M## | 原 backfill 字面 | 证明 workload | 证据路径 | 决策 | 承接波次 | 退出门 | 最终期望状态 |
|---|---|---|---|---|---|---|---|---|---|
| RD-038 | compute RayQuery codegen / AS descriptor | M51 | “rurixc vulkan_codegen 效果 kernel 编码通道就位(ray query/u64 atomic/storage image 写等,条款按需自 RXS-0297 顺位)后逐效果兑现” | compute `.rx` 实编 SPIR-V 1.4/KHR RayQuery + AS descriptor 真绑定，RED/GREEN 与 device 数值断言 | `G8_PLAN.md` §1.0：G7 in-flight、互锁终态 unresolved；`registry/deferred.json` RD-038；`spec/shader_stages.md` RXS-0297~0299；`spec/vulkan_backend.md` RXS-0300 | no-go（当前 G8 接入） | G7.2~G7.4 in-flight；G8.2+ blocked | G-G7-4/5；若 G7 close 时仍 open，终态接入 G8.2 后复刻同门 | open-观察（待 G7 终态） |
| RD-038 | `gi_probe` / `rtao` / `hard_shadow` 三核 | M10/M21/M51 | “GI 方向一致性对拍/RTAO 同 TLAS 对拍”以及 title 字面“屏幕探针 GI/RTAO 硬阴影” | 三个真实 `.rx` kernel 使用同一 AS 事实源 device 真跑；GI 方向一致、RTAO/硬阴影同 TLAS 对拍 | `G8_PLAN.md` §1.0：G7 in-flight（子态 open）、互锁终态 unresolved；`registry/deferred.json` RD-038；`G8_CAPABILITY_MATRIX.md` M10/M21/M51 | no-go（当前 G8 接入） | G7.4 in-flight；G8.2+ blocked | G-G7-6；若终态 open，仅可接 G8.2 或 G8.5b 并保留同判据 | open-观察（待 G7 终态） |
| RD-038 | VisBuffer SW/HW 逐像素 diff=0 | M02 | “VisBuffer SW-HW 逐像素 diff 容差 0” | 同场景 SW u64 路与 HW raster 路逐像素对拍，diff=0 | `G8_PLAN.md` §1.0：G7 in-flight（SW 已部分绿、HW/diff 缺）、互锁终态 unresolved；`registry/deferred.json` RD-038；`G8_CAPABILITY_MATRIX.md` M02 | no-go（当前 G8 接入） | G7.5 in-flight；G8.2+ blocked | G-G7-7；若终态 open，接 G8.5a 且 diff=0 不降档 | open-观察（待 G7 终态） |
| RD-038 | VSM depth/sample 真实进 device | M18 | “VSM device 深度对拍” | depth atlas/raster/sample 全部进入真实 device 路径，与 host 金标准对拍 | `G8_PLAN.md` §1.0：G7 in-flight（仅 page-mark 部分绿）、互锁终态 unresolved；`registry/deferred.json` RD-038；`G8_CAPABILITY_MATRIX.md` M18 | no-go（当前 G8 接入） | G7.5 in-flight；G8.2+ blocked | device depth/sample 对拍；若终态 open，接 G8.5a 与 M19 合流 | open-观察（待 G7 终态） |
| RD-038 | TAA/TSR 非 host-only | M23 | title 字面“TAA-TSR 的 GPU compute/raster kernel 化 + device 对拍”；backfill 把“temporal::taa/tsr”列为 host 金标准，但未另写独立退出句 | TAA 与 TSR 各自非 host-only device 真跑；时域序列误差/SSIM 与 host 金标准对拍 | `G8_PLAN.md` §1.0：G7 in-flight（TAA 部分绿、TSR host-only）、互锁终态 unresolved；`registry/deferred.json` RD-038；`G8_CAPABILITY_MATRIX.md` M23 | no-go（当前 G8 接入） | G7.5 in-flight；G8.2+ blocked | 两腿独立 device 对拍；若终态 open，接 G8.5b 与 M24 合流 | open-观察（待 G7 终态） |
| RD-038 | One True Device Frame + soak | — | `RD-038.backfill_condition` 未独立写出；G8_PLAN §1.0 接入字面为“One True Device Frame + soak” | 连续 raster→compute resource provenance 真帧 + 非空预算 + ≥30 min/≥10000 帧 soak | `G8_PLAN.md` §1.0：G7 in-flight（子态 open）、互锁终态 unresolved；`milestones/g7/G7_CONTRACT.md` G-G7-8；`registry/deferred.json` RD-038 | no-go（当前 G8 接入） | G7.6~G7.7 in-flight；G8.2+ blocked | G-G7-8；若终态 open，接 G8.5b 末或 G8.8a，禁止孤立 kernel 充绿 | open-观察（待 G7 终态） |

## 3. RD-039 — 虚拟化几何 P3+

| RD-id | 分项名 | M## | 原 backfill 字面 | 证明 workload | 证据路径 | 决策 | 承接波次 | 退出门 | 最终期望状态 |
|---|---|---|---|---|---|---|---|---|---|
| RD-039 | HZB 两阶段遮挡剔除 | M03 | “HZB 两阶段在剔除效率成为 measured 瓶颈时优先” | uc06/动态场景 cull 计数器证明剔除效率为 measured 瓶颈 | `registry/deferred.json` RD-039；`G8_CAPABILITY_MATRIX.md` M03；截至基准日无 measured artifact | no-go | G8.7 穷举 | 仅 measured 瓶颈证据成立后才可重判并进入 G8.5a | open-留 G8.7 |
| RD-039 | mesh shader 第三光栅路径 | M61 | “mesh shader 在多厂商扩展行为收敛且性能差有 measured 证据时评估” | NVIDIA/AMD 同 workload 行为一致 + 相对 SW/HW 路的 measured 帧时收益 | `registry/deferred.json` RD-039；`G8_CAPABILITY_MATRIX.md` M61；当前无跨厂商 measured 证据 | no-go | G8.7 穷举 | 多厂商收敛与性能差两条件同时成立 | open-留 G8.7 |
| RD-039 | 集群压缩与正式磁盘/内存页格式 | M04 | 原句“cluster 流送 P4 在场景规模超显存时”；G8_PLAN §1.2 明确把**格式 ABI 定版**与超显存运行时触发分离 | 版本化格式 golden + 编解码逐字节往返 + 解码 ABI 兼容拒录 | `G8_PLAN.md` §1.2/§2 G8.3；`G8_CAPABILITY_MATRIX.md` M04；`research/R1_UE5_RENDERER_PANORAMA.md` §3.1/§6.1 | go | G8.3（实现门 blocked） | 磁盘/内存格式分离、版本化、量化与 golden 全绿；G8.4 只消费不重定 | closed |
| RD-039 | cluster 流送 P4 运行时（父页引用/超显存） | M44 | “cluster 流送 P4 在场景规模超显存时” | 场景规模真实超过显存；按需驻留、父页 fallback、迟到页路径与预算计数器齐备 | `registry/deferred.json` RD-039；`G8_CAPABILITY_MATRIX.md` M44；截至基准日无超显存场景证据 | no-go | G8.7 穷举 | 超显存真实场景触发后另行判档；不得用 M04 格式 golden 替代运行时证据 | open-留 G8.7 |
| RD-039 | Nanite Foliage | M06 | “Foliage/骨骼在动态资产面出现时(联动 RD-040 蒙皮 MV)” | 真实动态植被资产集 + 风场/deformer/LOD 需求与帧时证据 | `registry/deferred.json` RD-039；`G8_CAPABILITY_MATRIX.md` M06；未登记真实动态资产证据 | no-go | G8.7 穷举 | 动态资产面出现后独立重判 | open-留 G8.7 |
| RD-039 | 骨骼虚拟几何 / Skinning | M06 | “Foliage/骨骼在动态资产面出现时(联动 RD-040 蒙皮 MV)” | 真实骨骼资产 + skin cache/deformer ABI + MV 全链 | `registry/deferred.json` RD-039；`G8_CAPABILITY_MATRIX.md` M06；未登记真实动态资产证据 | no-go | G8.7 穷举 | 动态资产面与蒙皮 MV 需求同时出现后重判 | open-留 G8.7 |
| RD-039 | 曲面细分位移 | M05 | `backfill_condition` 未单列；title 字面“曲面细分位移” | 真实位移资产清单 + bounds/velocity 语义 + 画质/性能对照 | `registry/deferred.json` RD-039；`G8_CAPABILITY_MATRIX.md` M05（相邻映射）；无独立触发条件与证据 | no-go | G8.7 穷举 | 先补独立 backfill 或 strategic_override，再按 workload 过门 | open-留 G8.7 |
| RD-039 | Assemblies 全功能（嵌套/跨 assembly 去重） | M06 | `backfill_condition` 未单列；title 字面“Assemblies 全功能(嵌套+跨 assembly 去重)” | 真实 assemblies 资产 + 嵌套/去重确定性与 residency 证据 | `registry/deferred.json` RD-039；`G8_CAPABILITY_MATRIX.md` M06；无独立触发条件与证据 | no-go | G8.7 穷举 | 先补独立 backfill 或 strategic_override | open-留 G8.7 |
| RD-039 | Mega Geometry 簇级 BLAS | M09 | “Mega Geometry 在 RT 与虚拟几何合流需求出现时” | 同一虚拟几何数据进入簇级 BLAS，误差、重建成本与 residency measured | `registry/deferred.json` RD-039；`G8_CAPABILITY_MATRIX.md` M09；无合流需求证据 | no-go | G8.7 穷举 | RT 与虚拟几何真实合流需求出现后重判 | open-留 G8.7 |

## 4. RD-040 — 光照 P3+

| RD-id | 分项名 | M## | 原 backfill 字面 | 证明 workload | 证据路径 | 决策 | 承接波次 | 退出门 | 最终期望状态 |
|---|---|---|---|---|---|---|---|---|---|
| RD-040 | SMRT 软阴影完整版 | M20 | “SMRT 在 VSM device 化(RD-038)后可独立 Mini 兑现(采样端沿光线多采样)” | VSM depth/sample device 门先绿；随后 SMRT 多采样软阴影对拍 | `registry/deferred.json` RD-040；`G8_CAPABILITY_MATRIX.md` M20；RD-038 VSM 终态 unresolved | no-go | G8.7 穷举 | VSM device 化前置未满足，不得进入 G8.5a | open-留 G8.7 |
| RD-040 | 世界空间辐射缓存 | M11 | “世界辐射缓存在屏幕探针远场缺失成为画质 measured 问题时” | GI 远场误差指标在代表场景越门，附缓存收益与预算 | `registry/deferred.json` RD-040；`G8_CAPABILITY_MATRIX.md` M11；无 measured 画质证据 | no-go | G8.7 穷举 | 远场画质 measured 问题成立后重判 | open-留 G8.7 |
| RD-040 | 自适应探针 | M11（相邻） | `backfill_condition` 未单列；title 字面“自适应探针(GI P3)” | 探针密度/预算自适应相对固定屏幕探针的画质与性能 measured 对照 | `registry/deferred.json` RD-040；`G8_CAPABILITY_MATRIX.md` M11 相邻面；无独立触发条件与证据 | no-go | G8.7 穷举 | 先补独立 backfill 或 strategic_override | open-留 G8.7 |
| RD-040 | SDF 软追踪 | M13 | `backfill_condition` 未单列；title 字面“SDF 软追踪(GI P4)” | mesh/global DF builder、更新预算与 tiered tracing 画质/性能证据 | `registry/deferred.json` RD-040；`G8_CAPABILITY_MATRIX.md` M13；无独立触发条件与证据 | no-go | G8.7 穷举 | 先补独立 backfill 或 strategic_override | open-留 G8.7 |
| RD-040 | ReSTIR GI/DI 与 MegaLights | M15 | “ReSTIR/MegaLights 在多灯场景需求出现时” | 真实多灯场景 + reservoir 时空复用、去噪、偏差与帧时证据 | `registry/deferred.json` RD-040；`G8_CAPABILITY_MATRIX.md` M15；无真实多灯场景证据 | no-go | G8.7 穷举 | 多灯场景需求出现后独立判档 | open-留 G8.7 |
| RD-040 | 完整 RT pipeline + SBT 增量面 | M50 | “RT pipeline/SBT 在『命中点需多样化材质着色』真实出现时(与 GI hit lighting 同步评估)” | 多 hit group/材质记录 + SBT 用户数据 + stack sizing + pipeline library device 真跑；any-hit/intersection/callable 按 RFC-0019 子集 RED/GREEN | `G8_PLAN.md` §1.2/§2 G8.2；`G8_CAPABILITY_MATRIX.md` M50；`spec/shader_stages.md` RXS-0244/0245；`spec/vulkan_backend.md` RXS-0248（单三件套、SBT 无用户数据）；`research/R3_GPU_API_ASSET_PIPELINE.md` §1/§2.2 | strategic_override | G8.2（实现门 blocked） | 必须是 RXS-0248 **增量**面；最小 `vk_rt` 见证不得充绿 | closed（仅 M50 分项；RD-040 总体仍 open） |
| RD-040 | SER / hit-object 重排 | M52 | `backfill_condition` 未单列；title 字面“SER 与 OMM” | 高分歧 RT workload 上 SER 前后正确性一致与 measured 收益 | `registry/deferred.json` RD-040；`G8_CAPABILITY_MATRIX.md` M52；无 measured workload | no-go | G8.7 穷举 | M50 先绿且 SER workload 有 measured 收益后重判 | open-留 G8.7 |
| RD-040 | Opacity Micromap（OMM） | M53 | `backfill_condition` 未单列；title 字面“SER 与 OMM” | alpha-tested foliage 资产 + OMM build/BLAS attach/baker 正确性与 measured 收益 | `registry/deferred.json` RD-040；`G8_CAPABILITY_MATRIX.md` M53/M84；无真实资产/收益证据 | no-go | G8.7 穷举 | M50 先绿且 OMM 资产需求成立后重判；OMM baker 不抢跑 | open-留 G8.7 |
| RD-040 | NRD 类 vendor 降噪接入 | —（复用 M25 输入族，不等于 M25 go） | “NRD/vendor 降噪经 UpscaleBackend 同构输入契约接入(MV/深度/法线),接入时不改 temporal 底座” | vendor denoiser adapter 的 MV/深度/法线 ABI 契约测试 + 画质/稳定性对照 | `registry/deferred.json` RD-040；`G8_CAPABILITY_MATRIX.md` §11.2 映射未给独立 M##；无 vendor 降噪需求证据 | no-go | G8.7 穷举 | 独立需求成立后接入，且 temporal 底座 0-byte | open-留 G8.7 |

### 4.1 M50 strategic_override 登记文本

> **RD-040/M50 strategic_override（agent，2026-08-02）**：以“UE5 级前置能力完成期必须先冻结可承载多材质 hit 的 RT pipeline/SBT ABI”为战略依据，覆盖 RD-040 原 backfill 对“命中点需多样化材质着色真实出现”的等待条件。覆盖范围仅为 M50 增量面：多 hit group/材质记录、SBT 用户数据、stack sizing、pipeline library，以及 RFC-0019 选定的 any-hit/intersection/callable 子集。现有 RXS-0248 单 hit group、SBT 无用户数据的最小见证只构成差距证据，不构成退出证据。本 strategic_override **不是 RD-038 override**，不改变 G7/RD-038 机器事实，不开放 G8.2，也不承诺 DXIL RT 腿。

## 5. RD-041 — 材质 / 流送 / 时域 P3+

| RD-id | 分项名 | M## | 原 backfill 字面 | 证明 workload | 证据路径 | 决策 | 承接波次 | 退出门 | 最终期望状态 |
|---|---|---|---|---|---|---|---|---|---|
| RD-041 | 多层材质 slab / closure IR | M28 | “多层 slab 在单层闭合表达力成为真实资产瓶颈时(MaterialClosure 已预留拓扑字段位)” | MaterialClosure 无法表达的真实资产用例清单 + 分层/混合/降级/跨路径 lowering 对拍 | `registry/deferred.json` RD-041；`G8_CAPABILITY_MATRIX.md` M28；无真实资产瓶颈证据 | no-go | 语义只在 RFC-0019 留面；实现留 G8.7 | 不得因 RFC 留语义面视为实现 go | open-留 G8.7 |
| RD-041 | SVT 虚拟纹理 | M40 | `backfill_condition` 无独立 SVT 门槛；title 字面“SVT 虚拟纹理” | 真实大纹理资产管线 + residency/feedback/迟到页/atlas 证据 | `registry/deferred.json` RD-041；`G8_CAPABILITY_MATRIX.md` M40；无独立门槛与真实大纹理资产证据 | no-go | G8.7 穷举 | 先追加“真实大纹理资产管线出现”门槛或单独 strategic_override；门-VT 记 SKIP=not-triggered | open-留 G8.7 |
| RD-041 | KTX2/BasisU 真转码器 | M83 | “KTX2/BasisU 在真实纹理资产管线出现时经 PagedResource::transcode 留口接入(解包确定性单测口径不变)” | G8.3 cook 样例资产：KTX2/Basis→BCn/ASTC 转码、mip/normal/alpha coverage 语义与双构建 hash | `G8_PLAN.md` §1.2/§2 G8.3；`G8_CAPABILITY_MATRIX.md` M83；`research/R3_GPU_API_ASSET_PIPELINE.md` §4.3；G8.3 cook 即真实纹理资产管线 | go | G8.3（实现门 blocked） | cook 样例资产 device/host 格式验证 + 确定性 wrapper；许可审计齐备 | closed |
| RD-041 | FSR/DirectSR/DLSS vendor 超分插件面 | M25 | “FSR/DirectSR 经 UpscaleBackend trait 接入(接口已冻结,不改底座)” | MV/深度/曝光/颜色输入 ABI 契约测试 + 至少一个 adapter 见证；temporal 底座零改写 | `registry/deferred.json` RD-041；`G8_PLAN.md` §1.2；`G8_CAPABILITY_MATRIX.md` M25；现有 `UpscaleBackend` 冻结接口构成接入依据 | go | G8.5b（实现门 blocked） | 输入 ABI 契约测试全绿，adapter 仅通过冻结 trait 接入 | closed |
| RD-041 | 帧生成 FG/MFG | M26 | “FG/MFG 为独立层另判” | 独立 frame-generation ABI、latency/pacing、伪影与 vendor 能力 measured 证据 | `registry/deferred.json` RD-041；`G8_CAPABILITY_MATRIX.md` M26；无独立判档证据 | no-go | 不进 G8 | 独立层另立判档，不用 M25 超分插件面代充绿 | open-观察 |
| RD-041 | 蒙皮 / WPO MV 通道资产验证 | M05 | “蒙皮/WPO MV 在动态资产面出现时(接口已按三类速度设计)” | 真实动态资产 + deformation velocity/MV、TSR 序列和 bounds 证据 | `registry/deferred.json` RD-041；`G8_CAPABILITY_MATRIX.md` M05；无真实动态资产证据 | no-go | G8.7 穷举 | 动态资产面出现后重判；不得只以接口预留充绿 | open-留 G8.7 |
| RD-041 | Work Graphs 与 mesh nodes | M56 | “Work Graphs 待 Vulkan 侧对应物成熟且『pass 内部提交单元可替换』接缝已预留” | Vulkan 对应物成熟度 + 可替换接缝验证 + D3D12 探针；不形成生产硬门 | `registry/deferred.json` RD-041；`G8_CAPABILITY_MATRIX.md` M56/§12；Vulkan 对应物条件未满足 | no-go | G8.7 穷举 | 仅评估探针；未满足双条件维持 open | open-留 G8.7 |

## 6. RD-042 — 可微物理 / 机器人批仿研究轨

| RD-id | 分项名 | M## | 原 backfill 字面 | 证明 workload | 证据路径 | 决策 | 承接波次 | 退出门 | 最终期望状态 |
|---|---|---|---|---|---|---|---|---|---|
| RD-042 | Differentiable Physics（明确归 RD-042，不归 RD-044） | M76 | “上游可微物理/机器人批仿生态成熟度(API 稳定性、许可、CPU 或 Vulkan 路径可得性、真实项目采用 measured 证据)达到引擎集成评估门槛,且出现真实 U5 面需求(可微仿真、机器人批仿训练环)时重评估” | 真实可微仿真项目 + API/许可/可用后端/采用度四项证据 | `registry/deferred.json` RD-042；`G8_PLAN.md` §1.1/§1.4；`G8_CAPABILITY_MATRIX.md` M76；`research/R2_PHYSICS_CHAOS_JOLT.md` §4.7/§5.4 | no-go | 不进 G8 硬门 | 重评估仍须 Full RFC；独立仓库或 feature 永不默认 | open-观察 |
| RD-042 | Newton / Genesis / MuJoCo Warp 机器人批仿生态 | M76 | “任何合入形态维持『独立仓库或 feature 永不默认』红线——触发判档 ≠ 合入主仓 CI,重评估经 Full RFC 立项,不进既有硬门” | 真实机器人批仿训练环 + 生产成熟度、许可与 CPU/Vulkan 路径证据 | `registry/deferred.json` RD-042；`G8_CAPABILITY_MATRIX.md` M76；`research/R2_PHYSICS_CHAOS_JOLT.md` §4.2/§5.4 | no-go | 不进 G8 硬门 | 研究隔离红线维持；不作 G8 验收依赖 | open-观察 |

## 7. RD-043 — wgrapier GPU 刚体观察

| RD-id | 分项名 | M## | 原 backfill 字面 | 证明 workload | 证据路径 | 决策 | 承接波次 | 退出门 | 最终期望状态 |
|---|---|---|---|---|---|---|---|---|---|
| RD-043 | wgrapier GPU 刚体生态 | M78 | “wgrapier 上游达到生产成熟度(稳定 API、刚体特性对拍 parity、真实项目采用 measured 证据)且出现 CPU 多核刚体无法承载的 measured 瓶颈场景(如十万级并发动态体)时重评估” | 五项 GPU 主刚体重审条件同时成立，且跨 NVIDIA/AMD 的 end-to-end measured 帧时优于 CPU 扩核/LOD | `registry/deferred.json` RD-043；`G8_CAPABILITY_MATRIX.md` M78/§12；当前无 measured 瓶颈与成熟度证据 | no-go | 不进 G8 | 即便触发也只作 feature 快路径；不作验收依赖/生产默认，不与渲染队列抢车道 | open-观察 |

## 8. RD-044 — 四拆

| RD-id | 分项名 | M## | 原 backfill 字面 | 证明 workload | 证据路径 | 决策 | 承接波次 | 退出门 | 最终期望状态 |
|---|---|---|---|---|---|---|---|---|---|
| RD-044 | Cloth | M72 | “Jolt 软体/布料在真实角色/资产需求出现时(rurix-physics crate 内扩 safe API,FFI 沿 U33~U42 审计模式续号,unsafe 集中绑定 crate 纪律不变)” | 开放 panel/seam/fabric schema + DCC 导入 + 碰撞 + LOD + 独立求解时间线的服装用例 | `G8_PLAN.md` §1.2/§1.4/§2 G8.6d；`G8_CAPABILITY_MATRIX.md` M72/§11.3；`research/R2_PHYSICS_CHAOS_JOLT.md` §2.6/§5；Chaos 对照产品级缺口构成 go 依据 | go | G8.6d（实现门 blocked） | 布料资产 schema、导入、碰撞、LOD、独立时间线五面全绿；unsafe 仍集中绑定 crate | closed（仅 Cloth 分项；RD-044 总体仍 open） |
| RD-044 | Continuum（软体/MPM；含 Taichi 生产 external-import 面） | M49a/M76 | “Taichi MPM/体积场在特效资产管线真实出现时由 spike 走生产 external-import 面(维持 §4.E4 三条禁止:只产粒子/体积场、不进刚体求解、不承担确定性联网)” | 真实特效资产管线 + 多 kernel AOT、external import、预算与渲染消费证据 | `registry/deferred.json` RD-044；`G8_CAPABILITY_MATRIX.md` M49a/M76；`research/R2_PHYSICS_CHAOS_JOLT.md` §4.3/§5.2；无真实资产管线证据 | no-go | P3 观察 | 三条禁止维持；不进 G8 硬门 | open-观察 |
| RD-044 | Fluid（含体积场/FLIP 生产面） | M49a/M76 | 同一原句中的“Taichi MPM/体积场在特效资产管线真实出现时由 spike 走生产 external-import 面”；title 字面“流体生产化” | 真实流体/VFX 资产管线 + FLIP/体积场确定性边界、预算与渲染消费证据 | `registry/deferred.json` RD-044；`G8_CAPABILITY_MATRIX.md` M49a/M76；`research/R2_PHYSICS_CHAOS_JOLT.md` §4.6/§5.2；无真实资产管线证据 | no-go | P3 观察 | 仅副轨生产面评估，不进刚体求解/确定性联网/既有硬门 | open-观察 |
| RD-044 | Rapier 快路径深造 | M65b | “Rapier 深造在快路径被真实 workload 采用时(对拍门判据形态不变,阈值实测标定口径不变)” | 真实 workload 采用记录 + parity 扩展场景 + 功能/性能差距与实测阈值 | `registry/deferred.json` RD-044；`G8_CAPABILITY_MATRIX.md` M65b；当前无生产 workload 采用证据 | no-go | G8.7 穷举 | 真实采用前维持默认 off；原对拍门与实测阈值口径不变 | open-留 G8.7 |

> **归属纠偏**：Differentiable Physics 只在 RD-042/M76 观察；不作为 RD-044 第五拆，也不因 Cloth go 而进入 G8 硬门。

## 9. RD 条目级 close-out 预期

| RD | G8 close-out 预期 | 约束 |
|---|---|---|
| RD-037 | closed | M89 三件套与零 Rust 宿主像素断言全部绿后一次关闭 |
| RD-038 | open-观察（当前） | 等 G7 close ref 重审；本表无 RD-038 override |
| RD-039 | open | 仅 M04 目标 closed，其余 no-go 分项继续留档 |
| RD-040 | open | M50 目标 closed；其余分项继续留档；M50 override 不替代 RD-038 override |
| RD-041 | open | M25/M83 目标 closed；其余分项继续留档 |
| RD-042 | open-观察 | Differentiable/机器人批仿维持研究隔离 |
| RD-043 | open-观察 | GPU 主刚体否决线维持 |
| RD-044 | open | Cloth 目标 closed；Continuum/Fluid 观察，Rapier 深造留 G8.7 |

## 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-02 | 首版：覆盖 RD-037~044 全部可辨分项与 RD-038 六个接入面；登记 M50 单独 strategic_override；M04/M25/M72/M83 与 RD-037/M89 go；其余按 backfill 无证据 no-go/open。明确 G8.1 governance-only 可并行、G8.2+ 互锁维持。 |
| v1.1 | 2026-08-05 | **RD-038 终态落地（裁决字面 0-byte，只追加）**：G7 已 `closed`（close-out `5269f96a` / tag `g7-closed`）且 `registry/deferred.json` 的 **RD-038 = `closed`**（G7.7 逐字审计路径）。因此 §2 六行的「open-观察（待 G7 终态）」预期**已由 G7 自行兑现**：G8 侧不接管这六个分项，`G8_PLAN` §1.0 六行「互锁终态」列维持 `unresolved`（RD-038 未走 override 路径，回填即伪造），§9 的 `RD-038 | open-观察（当前）` 行同为当时快照不回写；当前有效事实以 `registry/deferred.json` 为唯一事实源。连带效果：`G-G8-4` 的「接入本波的 RD-038 分项逐字兑现」在 G8.2 为**空集**（无 G8.2 腿），G8.2 退出门 = M50/M89/M29/M30/M31/M32/M85 七个独立断言 + RD-037 三件套，不因空集而放宽任何一条。M50 strategic_override（§4.1）与全部 go/no-go 裁决**逐字不改**；本次零号消费、零判据改动。 |
