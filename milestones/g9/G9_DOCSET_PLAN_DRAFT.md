# G9 文档集制定计划（DRAFT）

> **状态**：**DRAFT 计划提案——G9 未立项**。本文档与 `design/` 下五份模块设计草案均不构成任何契约、验收承诺或编号 claim；G9 正式立项（用户指令 + D-406 裁决 + 治理门）前，零 `src/`/`spec/`/`conformance/` 语义实现、零编号消费。
> **起草日期**：2026-08-08。基于：G8 收口终态 + G8.7 P2 决策表法定承接锚 + 五路联网调研（2023–2026 论文/工业资料）+ 五份模块设计草案。

---

## 1. 立项前现状摘要（事实基线）

- **G8 已收口**：`milestones/g8/G8_CONTRACT.md` §8.26 `status: closed`（2026-08-06，flip commit `b4189e79`）；21/21 P0+go-P1 PASS、wave2~8a 聚合 11/11 PASS。
- **假绿清零战役完成**：2026-08-07~08-08 Track A2/A2.1/A3/A4 + M83 真 vendor 清零全部 PASS；最新 closeout 复核 `evidence/g8_wave8b_closeout_20260808T040705Z.json` `VERDICT=READY`（**未提交**，留工作树）。
- **G9 唯一显性前置已满足**：G8_PLAN:15「正式建造归 G9+」+ G8 closed。
- **存续 open（不阻断立项，为法定输入）**：RD-039/040/041/044 总体 open（G8_CONTRACT §8.26）；RD-034 DXIL RT/mesh blocked；RD-036/042/043 观察；G-MB1-6 AMD 真卡尾门缺硬件。
- **工作树未提交项**：最新 8b closeout evidence、`.phys_baseline/`、`.wip_evidence_hold/`、`src/rurix-physics-sys/tools/layout_hinge*`（Jolt 铰链布局探针）。**立项前应先处置**：证据落库提交 + 探针归档或纳入 G9 物理波次（见 §5 待裁决）。

## 2. G9 定位与法定输入清单

**G9 = UE5 级渲染器与物理引擎的正式建造期**（G8_PLAN:15,113；G8_CONTRACT:128）。「UE5 级」可核对基线沿用 G8 口径 = UE 5.8。

**法定输入（G8.7 P2 决策表 defer-to-G9+ 十条承接锚，`ci/g8_p2_decisions_check.py` 机核「defer 必有 G9+ 承接锚」）：**

| 承接锚 | 分项 | 归属设计草案 |
|---|---|---|
| G9+ 虚拟几何评估窗 | M06 骨骼/植被虚拟几何 | D1 |
| G9+ RT×Nanite 合流窗 | M09 Mega Geometry 簇级 BLAS | D1 |
| G9+ GI 建造期 | M12 Surface Cache | D2 |
| G9+ GI 档位 | M16 irradiance field 档位 | D2 |
| G9+ shader library 深化 | M33 shader library 组合链接 | D3 |
| G9+ 大世界分区 | M43 World Partition/HLOD | D4 |
| G9+ 大气特效 | M48 体积雾/云 | D4 |
| G9+ 专项渲染器 | M49 水体/毛发/皮肤/地形/贴花族 | D4 |
| G9+ GPU-driven 提交 | M55 descriptor buffer/DGC | D3 |
| G9+ gameplay Field | M74 Physics Field | D5 |

**追加输入**：M17 Path Tracer 参照器（backfill 字面「G9+ 建造期前置」已命中，可判 go）、M45 HDR / M46 后处理栈（open-留 G9+）、M47 OIT（measured 选型对照随 D4 benchmark 门）、Safe GPU Operator Platform（改挂 G9+，与 UE5 前置无依赖——**进 G9 还是后续期，待立项裁决**）、神经变形（G9+ 研究轨，rfcs/0021:122）、M75/M77/M65b（D5 草案已给出处置建议）、M52/M61（D3 草案建议由 no-go 调整为语言层支持/可选路径，**需按治理流程走 deferred history 只追加 override 方可改判**）。

## 3. 已完成的设计草案产物（本计划依据）

| 草案 | 模块 | 核心架构决策 |
|---|---|---|
| `design/G9_D1_VIRTUAL_GEOMETRY_RT.md` | 虚拟化几何与 RT 合流 | VisibleClusterSet 单源真相（光栅/BLAS 拼装/VSM 共用）；GPU cluster 感知蒙皮（拒 UE5.5 CPU 权宜路线）；CLAS 离线烘焙 + 传统 BLAS 回退腿；DMM 永禁 |
| `design/G9_D2_GI_LIGHTING.md` | GI 与光照缓存 | Lumen 式全链路（Surface Cache 丢能量不漏光 + 四级追踪降级 + SPG/Radiance Cache）；多灯低档 MegaLights 式/高档 ReSTIR；IF L0–L3 阶梯；M17 golden 门为各档前置 |
| `design/G9_D3_GPU_DRIVEN_SUBMISSION.md` | GPU-driven 提交与着色系统 | DGC 跨 API 最小公倍数抽象 + Execution Set；descriptor buffer 全局表；IR 函数级链接为主轴；SER 语言原语 + capability 可选；mesh shader 可选路径（排在 meshlet 格式后） |
| `design/G9_D4_WORLD_AND_SPECIALTY_RENDERERS.md` | 大世界分区 / 专项渲染器 / 显示管线 | 分区数据模型先行 + HLOD 纯离线烘焙；云雾共用 Froxel；五族专项渲染器分级回退；view transform 可插拔（AgX/ACES 并列）；OIT 三档 benchmark 先行 |
| `design/G9_D5_PHYSICS.md` | 物理建造期 | Field 三层解耦 + 统一 particle view；lockstep/async 双通道确定性架构；浮力走 Field 通道；Jolt 5.3→5.6 七步 A/B；神经变形研究子轨 |

## 4. G9 文档集制定计划（按 G8 先例的四波文档波次）

### G9.0 — 文档集不可变基线（纯文档提交，零编号零 registry）

1. **本计划定稿** → `G9_PLAN.md` v1.0（经评审修订循环，参照 G8_PLAN v1.0→v1.3 过程）。
2. **`G9_CAPABILITY_MATRIX.md`**：能力缺口矩阵，行号体系裁决（沿用 M## 顺延 or G9 独立编号——建议沿用 M## 顺延保持 RD 映射连续性）；五条验收层级沿用 G8 口径（核心等价/功能闭环/可降级/可生产化/Vulkan 主线）。
3. **调研报告落盘 `research/R4~R8.md`**：五路调研（虚拟几何×RT / GI / GPU-driven / 大世界×专项 / 物理）整理为正式调研文档，引用 URL 与日期留痕——本次调研结论已注入五份设计草案，需补独立落盘。
4. 五份 `design/` 草案作为 G9.0 附件冻结引用（只追加修订记录，不回写）。

### G9.1 — 治理包（governance-only，与 G8.1 同构）

1. **用户立项指令留痕 + agent D-406 裁决**；G9.0 不可变 ref 登记。
2. **`G9_CONTRACT.md`**：契约四要素（范围/交付物/验收门/guardrails）+ front matter 状态机（`implementation_status: blocked` 起始）+ §8 只追加条款结构。
3. **`G9_CANDIDATE_DECISIONS.md`**：条件型 RD 逐分项 go/no-go/strategic_override——法定输入 = RD-039/040/041/044 全部 open 分项 + M17/M45/M46/M47 + M52/M61 改判提案（每条改判须 deferred.json history 只追加 override，禁静默改判）+ M75/M77/M65b + Safe GPU Operator Platform 归属裁决。
4. **`G9_ACCEPTANCE_MAP.md`**：全部 P0（及 go 的 P1）的 `M## → CI step → evidence schema → 判据`；缺行阻断 G9.2。
5. **伞形 Full RFC（D-409 对抗性评审）**：建议三份——
   - **RFC-0022 虚拟几何与 GI 语义**（cluster DAG/CLAS/Surface Cache/probe 编码/材质时域；触 M28 多层 closure 条件扩展时显式修订行）
   - **RFC-0023 GPU-driven 提交与着色系统**（DGC 语义/Execution Set/descriptor 全局索引/SER 原语/mesh shader 可选路径；自动 barrier 新依赖边 = G5 Barrier EB 冻结面修订行）
   - **RFC-0024 物理平台修订**（RFC-0021 修订：Field 系统/双通道 tick/浮力/Jolt 5.6 升级路径/神经变形研究轨边界）
   - RFC 实际编号按立项时 `registry/number_ledger.json` 实测 `next_free` 领取，禁止推测号。
6. **RTX 4070 Ti measured baseline → `g9_budget.json` 非空**（P-09：无 measured baseline 不得设性能硬门；D1 验收含 VRAM/AS 构建耗时指标）。
7. **`CI_GATES`** + G9 专属 validator（acceptance map 三向比对、implementation interlock、budget baseline 检查——脚本模式照 G8 五件套，编号 CI 步骤 G9.2 互锁后实测领取）。

### G9.2+ — 实现波次（草案建议，最终归 G9_PLAN 定稿）

```text
G9.2 地基波：D1 cluster 数据格式/页格式 v2 冻结 + D3 descriptor 全局表/DGC 抽象 + D5 Field 骨架与统一 particle view
G9.3 几何×RT 合流波：D1 GPU 蒙皮/LOD/CLAS + D3 command build node/Execution Set
G9.4 GI 波：D2 M17 参照器先行（golden 前置）→ Surface Cache → SPG/Radiance Cache → IF 档位 → 多灯
G9.5 大世界×专项波：D4 分区骨架/OIT benchmark → 大气/地形/贴花 → 云/水体/皮肤/HDR → AVBOIT/毛发
G9.6 物理波：D5 Field 完整语义/浮力/双通道 tick/Jolt 5.6 A/B/Rapier 基准
G9.7 P2 穷举决策表（软门必穷尽，沿袭 G8.7 纪律：defer 必有承接锚）
G9.8a stabilization/soak（≥G7 量级：≥30min/≥10000 帧）→ G9.8b close-out
```

波次内可蜂群并行、波次间串行；spec-first + RED 先行；禁止 stub/mock/host substitution 抢跑；条件实现刚绿不得当日进 8a（G8.8b 的同日放行先例是否继承，立项时显式裁决）。

### G9 收口要件（预判）

- 验收映射终审 + RD 分项最终状态与候选决策表逐字一致；契约 §8 只追加 + status flip。
- RD-039/040/041/044 在 G9 建造期真实资产/画质需求出现时逐条判档（其触发条件不得被「UE5 目标」静默改写——G8_PLAN §1.2 纪律继承）。

## 5. 立项前待裁决事项（建议提交用户决策）

1. **G9 立项指令**：是否现在立项，还是先完成工作树清理（8b evidence 落库提交 + layout_hinge 探针处置）。
2. **Safe GPU Operator Platform 归属**：进 G9（独立轨道，与渲染/物理无依赖）还是 G10+。
3. **M52 SER / M61 mesh shader 改判**：接受 D3 草案的「语言层支持/可选路径」调整，还是维持 G8.7 no-go 留档（改判须走 override 流程）。
4. **G9 规模与分包**：五模块一次性进 G9，还是 G9 只取 D1+D3+D5 地基、GI/大世界归 G10（五模块全进将是项目史上最大里程碑，工期与 soak 成本需 measured 评估）。
5. **神经变形研究轨**：以独立 RD 登记研究轨，还是维持 rfcs/0021:122 的无归属留痕。

## 6. 风险（计划级）

| 风险 | 缓解 |
|---|---|
| 五模块范围爆炸，G9 变成不可收口巨兽 | §5.4 分包裁决；P0 最小集先行，P1/P2 按波次消化与穷举 |
| NV-only 扩展（CLAS/DGC 部分）绑死单厂商 | 每 NV 扩展主腿配跨厂商回退腿作正确性基线；capability profile 门控 |
| 调研引用失效/版本漂移（UE 5.5-5.8 快速演进） | G9.1 立项时复核 R4~R8 关键引用时效；SIGGRAPH/Unreal Fest 2026 新材料复审窗 |
| 防假绿纪律在更大范围下执行成本上升 | 验收门全部机器判读 + 负例 RED 臂 + evidence schema，沿袭 G8 机核模式 |
| M17 参照器工作量被低估（golden 是 GI 各档前置） | G9.4 波内 M17 排第一顺位；megakernel 起步控制范围 |
