# G5_PLAN — 原生渲染器期主线分解

> 契约:[G5_CONTRACT.md](G5_CONTRACT.md) · 上游事实源:渲染器调研/ 七份调研报告 · 门:[CI_GATES.md](CI_GATES.md)
> 推进形态:**波次(wave)推进**——G5.0/G5.1 治理与 RFC 先行,G5.2 底座六面并行,G5.3 效果六面并行(gated on G5.2 集成绿),G5.4 合流 demo 与 smoke,G5.5 close-out。波次内并行、波次间严格串行(每波结束全 workspace build/test 绿才进下一波)。

---

## 0. 依赖图(七报告合流)

```
G5.2-A render graph 底座 ──┬──► G5.3 全部效果面(屏障/transient 公共前置)
G5.2-B RHI 派发桥 ─────────┤
G5.2-H 时域公共底座 ────────┤──► G5.3-E GI 滤波 / G5.3-D 阴影滤波 / G5.3-F RT 降噪
G5.2-G 材质闭合+GPU scene ──┘
G5.2-F AS 管理+ray query ◄──► G5.3-E GI 探针追踪(同通道)
G5.2-C 离线 meshlet ──► G5.3-C GPU 剔除+VisBuffer ──► 材质 classify/resolve
```

## 1. 波次分解

### G5.0 治理包(结构件)
- 本四件套 + number_ledger reserved_in_flight[G5](v1.27)。交付 D-G5-1,门 G-G5-1。

### G5.1 RFC-0016 伞形八章
- Draft(起草 provenance 留痕)→ D-409 跨模型对抗性评审(评审 provenance ≠ 起草)→ findings 逐条 disposition → Agent Approved。交付 D-G5-2,门 G-G5-2。

### G5.2 底座六面(并行)
| 面 | 内容 | 报告 | 主要落点 |
|---|---|---|---|
| A | render graph:声明式 pass/资源、四趟编译(剔除/生命周期/屏障/车道)、EB 三轴 Barrier、transient 池别名、编译期校验、图 dump | 报告5 P0–P1 | `src/rurix-render/src/graph/` |
| B | RHI 派发桥:rxrt_rhi_submit gfx pass 真派发接 vk.rs 图形执行器;VB/IB/descriptor 传递;present handoff | 前置 | `src/rurix-rt/src/{rhi,vk}.rs` + `src/rurix-rt-cabi` |
| C | 离线 meshlet 化 + 层级 DAG + 误差包围球 + 序列化(预留页表字段)+ CPU 参照剔除器 | 报告1 P0 | `src/rurix-geom-build/` |
| G | 单层材质闭合(32B)+ GPU scene 扁平化 + PSO precache/运行时编译告警 | 报告6 P0 | `src/rurix-render/src/material/` `gpu_scene` |
| H | 时域公共底座(MV/Halton jitter/历史验证/邻域裁剪)+ TAA | 报告7 P0 | `src/rurix-render/src/temporal/` |
| F | AS 管理器(BLAS 缓存/refit 分级/TLAS 重建)+ ray query 封装 | 报告4 P0 前半 | `src/rurix-render/src/rt/` |

集成门:全 workspace `cargo build/test` 绿 + 门 G-G5-3(调度底座)/ G-G5-4(派发桥)。

### G5.3 效果六面(并行,gated on G5.2 集成)
| 面 | 内容 | 报告 | 主要落点 |
|---|---|---|---|
| C2 | GPU 实例/簇两级剔除 + VisBuffer(64 位 depth30+cluster27+tri7)SW(atomicMax u64)/HW 双路光栅 + 材质 classify/resolve | 报告1 P1–P2 | `geometry/` + shaders |
| D | VSM clipmap:页标记/分配/失效 + 多视图 shadow_depth_raster + 投影;共享物理页池 | 报告3 P0–P1 | `shadow/` + shaders |
| E | 屏幕探针 GI:1/16 探针 + ray query 单反弹 + SH + 平面插值 + 3×3 滤波 + 时域累积 | 报告2 P0–P1 | `gi/` + shaders |
| F2 | RTAO/硬阴影 + 时域滤波 | 报告4 P0–P1 | `rt/` + shaders |
| K | TSR 类超分 + UpscaleBackend trait(自研主实现,vendor 后端留口) | 报告7 P1 | `temporal/` |
| L | 通用页式流送(128KB 页/三预算)+ 两级实例化 | 报告6 P1–P2 | `streaming/` `scene/` |

集成门:全 workspace 绿 + 门 G-G5-5(几何)/ G-G5-6(光照)/ G-G5-7(时域)。

### G5.4 合流:uc06 demo + smoke
- `apps/uc06-renderer`:meshlet 场景 → 剔除 → VisBuffer → 延迟着色 → GI+VSM+RT → TAA/TSR → readback;异步 compute 车道接 AO/GI 滤波(报告5 P2)。
- CI smoke 步骤 82 起(拟:82 renderer graph host 门 / 83 draw 桥 / 84 visbuffer / 85 lighting / 86 temporal / 87 uc06 全管线;数量随实现回填)+ evidence schema + g5_budget counter + evaluator 同 PR。
- P3+ 项登记 deferred RD-037+。交付 D-G5-5,门 G-G5-8。

### G5.5 close-out
- 全量回归冻结 + 门终审表 + RD/SG 处置 + status flip。交付 D-G5-6,门 G-G5-9。

## 2. 冻结接口(G5.2 开工前固化,波次内不得漂移)

- `Barrier { sync_before/after, access_before/after, layout_before/after }`(EB 三轴,AnKi 简化 stage 集)
- `PassDesc { name, queue: Graphics|AsyncCompute, reads: Vec<ResAccess>, writes: Vec<ResAccess> }` + `ResourceDesc`/`ResourceId`(transient vs imported)
- `ClusterRecord`(≤128 tri/簇,含锥剔除+误差包围球字段)与序列化布局(预留页表字段)
- VisBuffer 位格式:u64 = depth:30 | cluster:27 | tri:7
- `MaterialClosure` 32B 定长(albedo/F0/roughness/normal/emissive 打包)
- `PageRequest`/`StreamingBudget { io, transcode, upload }`
- `UpscaleBackend` trait(输入颜色/深度/MV/reactive/曝光 → 输出目标分辨率颜色)
- 跨帧资源(TAA 历史/VSM 页表/GI 探针历史)一律外部资源 import,不入 transient;流送屏障图外 acquire/release

## 3. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-07-29 | 初版(G5 开工) |
