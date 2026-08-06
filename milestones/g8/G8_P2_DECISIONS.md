# G8.7 P2 穷举决策表

> **地位**：G8_PLAN §2.7 冻结 31 行集合的唯一决策事实源；`ci/g8_p2_decisions_check.py` 机核。  
> **纪律**：决策 ∈ {`go`,`no-go`,`defer-to-G9+`}；逐单元格非空；`go` 必有 evidence + 承接波次 + 退出门；`no-go` 必引 RD/矩阵 backfill 字面；`defer` 必有 G9+ 承接锚。不得以空串/TBD/TODO/待定/— 充行。  
> **与候选表**：若本表改判 `go` 而 `G8_CANDIDATE_DECISIONS.md` 为 no-go，须先有 `deferred.json` history 只追加 override，否则聚合红。

## 1. 决策表（31 行，集合 ≡ G8_PLAN §2.7）

| M## | 分项名 | 矩阵 P 级/波次 | 原 backfill/触发条件字面 | 决策 | 一句理由 | 依据/证据路径 | go 时承接波次+退出门 | 最终状态 |
|---|---|---|---|---|---|---|---|---|
| M06 | 骨骼/植被虚拟几何 | P2 / G8.7 | RD-039「动态资产面出现时」 | defer-to-G9+ | 动态资产面未触发；属建造期 | `G8_CAPABILITY_MATRIX.md` M06；`registry/deferred.json` RD-039 | G9+ 虚拟几何评估窗 | open-defer |
| M09 | Mega Geometry 簇级 BLAS | P2 / G8.7 | RD-039「RT 与虚拟几何合流需求出现时」 | defer-to-G9+ | 合流需求未出现 | `G8_CAPABILITY_MATRIX.md` M09；RD-039 | G9+ RT×Nanite 合流窗 | open-defer |
| M12 | Surface Cache | P2 / G8.7 | 矩阵「G8.7 评估」；依赖 GI 建造期 | defer-to-G9+ | 无独立触发；GI 主线属建造期 | `G8_CAPABILITY_MATRIX.md` M12 | G9+ GI 建造期 | open-defer |
| M14 | HWRT hit lighting / Far Field | P2 / G8.7 | 「M50 后评估」；画质 measured 需求 | no-go | M50 已绿但无画质 measured 需求方 | `G8_CAPABILITY_MATRIX.md` M14；`G8_CANDIDATE_DECISIONS.md` RD-040 | — | open-留档 |
| M15 | MegaLights / ReSTIR | P2 / G8.7 | RD-040「多灯场景需求出现时」；G7 out_of_scope | no-go | 多灯需求未触发；G7 out_of_scope 字面维持 | `registry/deferred.json` RD-040；`G7_CONTRACT.md` out_of_scope | — | open-留档 |
| M16 | irradiance field 档位 | P2 / G8.7 | GI 档位化建造期 | defer-to-G9+ | 画质档位属建造期 | `G8_CAPABILITY_MATRIX.md` M16 | G9+ GI 档位 | open-defer |
| M22 | 海量灯阴影统一接口 | P2 / G8.7 | 随 M15 / RD-040 | no-go | M15 前提未触发 | `G8_CAPABILITY_MATRIX.md` M22；RD-040 | — | open-留档 |
| M33 | shader library 组合链接 | P2 / G8.7 | 若 G8.2 未完则评估 | defer-to-G9+ | M85 manifest/DDC 已覆盖打包主需求；完整 library 组合链接未交付、无独立 workload | `evidence/g8_m85_shader_manifest_ddc_20260806T055112Z.json`（g8.2 腿）；`G8_CAPABILITY_MATRIX.md` M33 | G9+ shader library 深化 | open-defer |
| M34 | wave-size / SM6.8 对齐 | P2 / G8.7 | workload 证据；DXIL 腿 | no-go | 无 workload；DXIL RT/mesh 腿 RD-034 blocked | `registry/deferred.json` RD-034；`G8_CAPABILITY_MATRIX.md` M34 | — | open-留档 |
| M41 | sampler feedback | P2 / G8.7 | VT(M40) 消费方 | no-go | M40/SVT = no-go（候选表）；无 feedback 消费方 | `G8_CANDIDATE_DECISIONS.md` M40；`G8_CAPABILITY_MATRIX.md` M41 | — | open-留档 |
| M42 | RVT | P2 / G8.7 | 依赖 SVT | no-go | SVT/M40 no-go 连带 | `G8_CANDIDATE_DECISIONS.md` M40；矩阵 M42 | — | open-留档 |
| M43 | World Partition / HLOD | P2 / G8.7 | 「大世界资产面出现时」 | defer-to-G9+ | 大世界资产面属建造期 | `G8_CAPABILITY_MATRIX.md` M43 | G9+ 大世界分区 | open-defer |
| M48 | 体积雾/云 | P2 / G8.7 | 画质专项建造期 | defer-to-G9+ | 无 G8 硬门触发 | `G8_CAPABILITY_MATRIX.md` M48 | G9+ 大气特效 | open-defer |
| M49 | 水体/毛发/皮肤/地形/贴花族 | P2 / G8.7 | 专项渲染器族建造期 | defer-to-G9+ | 专项族属建造期 | `G8_CAPABILITY_MATRIX.md` M49 | G9+ 专项渲染器 | open-defer |
| M49a | GPU 粒子 VFX 渲染侧 | P2 / G8.7 | RD-044「特效资产管线真实出现时」 | no-go | uc09 仅为 Taichi spike 成功臂；生产特效管线未出现 | `registry/deferred.json` RD-044；矩阵 M49a | — | open-留档 |
| M49b | present pacing / 低延迟 | P2 / G8.7 | latency measured 需求 | no-go | 无 latency measured 证据与需求方 | `G8_CAPABILITY_MATRIX.md` M49b | — | open-留档 |
| M52 | SER | P2 / G8.7 | RD-040 高分歧 RT workload | no-go | M50 刚绿；无高分歧 RT measured 收益 | `registry/deferred.json` RD-040；矩阵 M52 | — | open-留档 |
| M53 | OMM | P2 / G8.7 | alpha-tested 资产需求；RD-040 | no-go | 无 alpha-tested 资产需求；OMM baker 不抢跑 | RD-040；矩阵 M53 | — | open-留档 |
| M54 | RT position fetch | P2 / G8.7 | RT 增量消费方 | no-go | RT 增量面已满足 G8 判据；无独立消费方 | 矩阵 M54；M50 evidence | — | open-留档 |
| M55 | descriptor buffer / DGC | P2 / G8.7 | GPU-driven 提交建造期 | defer-to-G9+ | P1 无 G8 硬门；属建造期渲染器主体 | `G8_CAPABILITY_MATRIX.md` M55 | G9+ GPU-driven 提交 | open-defer |
| M56 | Work Graphs | P2 / G8.7 | RD-041 双条件（Vulkan 对应物成熟+接缝预留） | no-go | RD-041 双条件字面未满足 | `registry/deferred.json` RD-041；矩阵 M56 | — | open-留档 |
| M59 | async compute 第二腿 | P2 / G8.7 | 多队列 measured 收益 | no-go | G8.4 默认单队列；async compute 无 measured 收益证据 | `G8_CANDIDATE_DECISIONS.md` 多队列；矩阵 M59 | — | open-留档 |
| M61 | mesh shader 第三光栅 | P2 / G8.7 | RD-039 双条件（跨厂商收敛+measured） | no-go | 单卡 4070 Ti 无法证跨厂商；双条件未成立 | `registry/deferred.json` RD-039；矩阵 M61 | — | open-留档 |
| M62 | task shader 开放 | P2 / G8.7 | RXS-0270 评估窗；RFC-0019/M50 | no-go | G8.2/M50 实况维持 **不开放** task；非 PASS 门 | `G8_CONTRACT.md` §8.8；RFC-0019；矩阵 M62 | — | open-留档（不开放） |
| M63 | VRS | P2 / G8.7 | 着色率 measured 收益 | no-go | 无着色率 measured 收益 | `G8_CAPABILITY_MATRIX.md` M63 | — | open-留档 |
| M65b | Rapier 深造 | P2 / G8.7 | RD-044「快路径被真实 workload 采用时」 | no-go | 默认 off 维持；真实 workload 未采用 | `registry/deferred.json` RD-044；矩阵 M65b | — | open-留档 |
| M74 | Physics Field | P2 / G8.7 | gameplay 统一空间影响 | defer-to-G9+ | M68 damage/field journal 已覆盖 G8 最小面；统一 Field 属建造期 | RFC-0021；矩阵 M74 | G9+ gameplay Field | open-defer |
| M75 | 异步物理 tick | P2 / G8.7 | RFC-0021 Q6 独立判档 | no-go | 本期只冻结时间域 identity；异步调度须独立判档 | RFC-0021 Q6；矩阵 M75 | — | open-留档 |
| M77 | 水体/浮力 | P2 / G8.7 | ApplyBuoyancyImpulse；联动 M49 | no-go | 未包装且无 gameplay 需求；联动 M49 defer | 矩阵 M77；M49 行 | — | open-留档 |
| M86 | USD ingest | P2 / G8.7 | 法务标注 + 真实 USD 资产需求 | no-go | glTF 主线已满足；无真实 USD 需求；TOST 许可未标注 | 矩阵 M86；RFC-0020 | — | open-留档 |
| M87 | MaterialX | P2 / G8.7 | 联动 M28；单层闭合瓶颈 | no-go | M28 维持 no-go；单层闭合表达力未成瓶颈 | `G8_CANDIDATE_DECISIONS.md` M28；矩阵 M87 | — | open-留档 |

**集合校验**：上表 M## 集合 = `{M06,M09,M12,M14,M15,M16,M22,M33,M34,M41,M42,M43,M48,M49,M49a,M49b,M52,M53,M54,M55,M56,M59,M61,M62,M63,M65b,M74,M75,M77,M86,M87}`（31）。本表 **零 go 行**——G8.7 退出不冒充任何 P2 PASS。

## 2. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-06 | 首版：按 G8_PLAN §2.7 + 设计案 §6.2 预填 31 行；M33=defer（M85 覆盖打包主需求）；M62=no-go（task 不开放，M50 留痕）。零 go 行。 |
